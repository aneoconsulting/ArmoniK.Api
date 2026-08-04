//! Creating and releasing the connection itself.

use std::sync::OnceLock;

use crate::buffer::ak_bytes;
use crate::config::{self, ak_client_config};
use crate::error::{guard, AK_ERR_CONNECTION_FAILED, AK_ERR_NULL_POINTER};

/// One tokio runtime for every client this crate creates, since a connection outlives the single
/// call that made it and needs somewhere to drive its background I/O.
static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

fn runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("building the tokio runtime armonik-ffi shares across every client")
    })
}

/// A connected channel, opaque to the caller. Owned through [`ak_client_create`]/[`ak_client_free`]
/// only; there is no other way to construct or destroy one.
pub struct ak_client {
    // Nothing in this crate dispatches a call over it.
    #[allow(dead_code)]
    pub(crate) channel: armonik_transport::reexports::tonic::transport::Channel,
}

/// Connects using `config`, writing the new client to `*out` and, on failure, an error message to
/// `*out_err` (release it with [`ak_bytes_free`](crate::buffer::ak_bytes_free)). Returns `0` on
/// success, one of the `AK_ERR_*` codes in [`crate::error`] otherwise.
///
/// # Safety
///
/// `config` must be non-null and satisfy [`config::build`]'s contract on every field. `out` and
/// `out_err` must be non-null and valid to write to. `*out` is written only on success; `*out_err`
/// is written only for [`AK_ERR_INVALID_CONFIG`](crate::error::AK_ERR_INVALID_CONFIG),
/// [`AK_ERR_INVALID_UTF8`](crate::error::AK_ERR_INVALID_UTF8) and
/// [`AK_ERR_CONNECTION_FAILED`](crate::error::AK_ERR_CONNECTION_FAILED) - not for
/// [`AK_ERR_NULL_POINTER`](crate::error::AK_ERR_NULL_POINTER), which finds nothing valid to write
/// an error through, nor for [`AK_ERR_PANIC`](crate::error::AK_ERR_PANIC), which is a caught panic
/// with no message of its own. A caller must initialise both before the call if it means to
/// branch on more than the return code.
#[no_mangle]
pub unsafe extern "C" fn ak_client_create(
    config: *const ak_client_config,
    out: *mut *mut ak_client,
    out_err: *mut ak_bytes,
) -> i32 {
    guard(|| {
        if config.is_null() || out.is_null() || out_err.is_null() {
            return AK_ERR_NULL_POINTER;
        }
        // SAFETY: `config` is non-null per the check above; the rest of the contract is this
        // function's own, forwarded from its caller.
        let built = unsafe { config::build(&*config) };
        let http_config = match built {
            Ok(config) => config,
            Err(invalid) => {
                let code = invalid.code();
                // SAFETY: `out_err` is non-null per the check above.
                unsafe { *out_err = invalid.into_bytes() };
                return code;
            }
        };

        match runtime().block_on(armonik_transport::connect(http_config)) {
            Ok(channel) => {
                let client = Box::new(ak_client { channel });
                // SAFETY: `out` is non-null per the check above; ownership of `client` transfers to
                // the caller, who must release it exactly once through `ak_client_free`.
                unsafe { *out = Box::into_raw(client) };
                0
            }
            Err(error) => {
                // SAFETY: `out_err` is non-null per the check above.
                unsafe { *out_err = ak_bytes::from_string(crate::error::describe(&error)) };
                AK_ERR_CONNECTION_FAILED
            }
        }
    })
}

/// Releases a client [`ak_client_create`] returned. Safe to call with a null pointer, a no-op in
/// that case; calling it twice on the same non-null pointer is undefined behaviour.
///
/// # Safety
///
/// `client`, if non-null, must be exactly a pointer [`ak_client_create`] wrote to `*out`, not
/// already freed.
#[no_mangle]
pub unsafe extern "C" fn ak_client_free(client: *mut ak_client) {
    let _ = guard(|| {
        if !client.is_null() {
            // SAFETY: per this function's own contract, `client` names a box this crate leaked in
            // `ak_client_create`; reconstructing it here and letting it drop is exactly undoing
            // that leak.
            drop(unsafe { Box::from_raw(client) });
        }
        0
    });
}
