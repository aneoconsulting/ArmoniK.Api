//! The RPC half of ABI v1 section 9, reduced to what stage 4 measures.
//!
//! "The RPC half does not know the schema. It moves opaque bytes and dispatches on a path
//! string, so it serves every RPC unchanged and adding one is a table row." Nothing in this
//! file mentions a message type, and that is the property stage 4's crossing count tests:
//! if the count per RPC were a function of field count, something here would have to know
//! about fields.
//!
//! **What is NOT here, and is in STATE.md's not-measured list rather than stubbed**: TLS,
//! retry and backoff, metadata, deadlines, the gRPC status code as a number, cancellation,
//! the completion queue and the callback delivery modes, streaming, and `ak_init` with its
//! one-shot installs. Section 9's case is behavioural and this is not a test of it.

use crate::{AK_ERR_HOST, AK_ERR_INVALID_STATE, AK_OK};
use bytes::Bytes;
use core::ffi::c_void;

pub enum ak_runtime {}
pub enum ak_client {}

/// A borrowed byte range the core owns until the host releases it.
#[repr(C)]
pub struct ak_bytes {
    pub ptr: *const u8,
    pub len: usize,
    /// The core's handle on the allocation. The host passes it back and does not read it.
    pub owner: *mut c_void,
}

pub struct RuntimeImpl {
    pub rt: tokio::runtime::Runtime,
}

pub struct ClientImpl {
    pub rt: *const RuntimeImpl,
    /// The CHANNEL, not a `Grpc`. `Grpc::unary` takes `&mut self`, so holding one here made
    /// `ak_call_unary` a mutating call on shared state, and two host threads calling it at
    /// once a data race -- which is what stage 4's 8-in-flight arm failed on. A `Channel` is
    /// cheap to clone and clones share the connection, so a call builds its own `Grpc` and
    /// the client handle is what ABI v1 section 9 says it is: usable from many threads at
    /// once, with ownership between handles internal.
    pub chan: tonic::transport::Channel,
}

/// `worker_threads` comes from the host with a small explicit default, never from
/// `Runtime::new()`: Rust reads the cgroup quota, so a requests-only pod takes every CPU on
/// the node (ABI v1 section 3). Stage 4 passes it explicitly for that reason and not to tune.
#[no_mangle]
pub extern "C" fn ak_runtime_new(worker_threads: u32) -> *mut ak_runtime {
    let n = if worker_threads == 0 { 2 } else { worker_threads as usize };
    match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(n)
        .enable_all()
        .build()
    {
        Ok(rt) => Box::into_raw(Box::new(RuntimeImpl { rt })) as *mut ak_runtime,
        Err(_) => core::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn ak_runtime_destroy(r: *mut ak_runtime) {
    if !r.is_null() {
        drop(Box::from_raw(r as *mut RuntimeImpl));
    }
}

#[no_mangle]
pub unsafe extern "C" fn ak_client_new(
    r: *mut ak_runtime,
    uri: *const u8,
    uri_len: usize,
) -> *mut ak_client {
    let rt = &*(r as *const RuntimeImpl);
    let s = match core::str::from_utf8(core::slice::from_raw_parts(uri, uri_len)) {
        Ok(s) => s.to_string(),
        Err(_) => return core::ptr::null_mut(),
    };
    let chan = rt.rt.block_on(async move {
        tonic::transport::Endpoint::from_shared(s)
            .ok()?
            .connect()
            .await
            .ok()
    });
    match chan {
        Some(chan) => Box::into_raw(Box::new(ClientImpl {
            rt: r as *const RuntimeImpl,
            chan,
        })) as *mut ak_client,
        None => core::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn ak_client_destroy(c: *mut ak_client) {
    if !c.is_null() {
        drop(Box::from_raw(c as *mut ClientImpl));
    }
}

/// The blocking unary call. ONE crossing in.
///
/// ABI v1 section 9 gives the blocking form a handle so it can be cancelled; stage 4 does
/// not build cancellation, so this takes none and that omission is recorded rather than
/// papered over.
#[no_mangle]
pub unsafe extern "C" fn ak_call_unary(
    c: *mut ak_client,
    path: *const u8,
    path_len: usize,
    req: *const u8,
    req_len: usize,
    out: *mut ak_bytes,
) -> i32 {
    // Shared, not exclusive: many host threads may be inside this at once.
    let cl = &*(c as *const ClientImpl);
    let rt = &*cl.rt;
    let p = match core::str::from_utf8(core::slice::from_raw_parts(path, path_len)) {
        Ok(p) => p,
        Err(_) => return AK_ERR_INVALID_STATE,
    };
    let path = match http::uri::PathAndQuery::from_maybe_shared(p.to_string()) {
        Ok(p) => p,
        Err(_) => return AK_ERR_INVALID_STATE,
    };
    let body = Bytes::copy_from_slice(core::slice::from_raw_parts(req, req_len));
    let mut grpc = tonic::client::Grpc::new(cl.chan.clone());
    let res = rt.rt.block_on(async {
        grpc.ready().await.map_err(|e| format!("ready: {e}"))?;
        grpc.unary(tonic::Request::new(body), path, rpc::RawCodec)
            .await
            .map_err(|e| format!("unary: {e}"))
    });
    match res {
        Ok(r) => {
            let b = Box::new(r.into_inner());
            (*out).ptr = b.as_ptr();
            (*out).len = b.len();
            (*out).owner = Box::into_raw(b) as *mut c_void;
            AK_OK
        }
        Err(e) => {
            if std::env::var_os("AK_RPC_TRACE").is_some() {
                eprintln!("ak_call_unary: {e}");
            }
            AK_ERR_HOST
        }
    }
}

/// The second crossing, and the only other one: the host is done with the response bytes.
#[no_mangle]
pub unsafe extern "C" fn ak_bytes_free(b: *mut ak_bytes) {
    if !b.is_null() && !(*b).owner.is_null() {
        drop(Box::from_raw((*b).owner as *mut Bytes));
        (*b).owner = core::ptr::null_mut();
        (*b).ptr = core::ptr::null();
        (*b).len = 0;
    }
}
