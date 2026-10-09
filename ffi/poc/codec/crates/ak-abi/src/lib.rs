//! The C ABI of `ffi/design/ABI-v1.md`, as Rust sees it.
//!
//! This crate is the generated header: both the core and the host binding compile against
//! it, so a group layout cannot be restated differently on the two sides. The fixed part
//! (sections 4, 5 and 10 of the specification) is here; the per-message part (section 6's
//! groups and vtables, section 7's fixes) is in `generated`.
//!
//! Naming follows the specification rather than Rust convention on purpose: these are C
//! symbols and C struct names, and a reader comparing this file with ABI-v1.md should not
//! have to translate.
#![allow(non_camel_case_types)]

use core::ffi::c_void;

pub mod generated {
    /// `corpus` (test-only, off by default): the same ABI generated for the conformance
    /// corpus's reader schema instead of `ffi/schema/shapes.json`, so the corpus can be
    /// run through the C ABI (FIX-PLAN WP5 item 6.1). It CHANGES the ABI, so it is only
    /// ever enabled by a build of its own (`poc/rust/corpus/`, its own workspace).
    ///
    /// WP5 step 10: without `unknown-fields`, the NO-UNKNOWN variant of the same ABI.
    #[cfg_attr(all(feature = "corpus", feature = "unknown-fields"), path = "../generated_corpus/abi.rs")]
    #[cfg_attr(all(feature = "corpus", not(feature = "unknown-fields")), path = "../generated_corpus_nounk/abi.rs")]
    #[cfg_attr(all(not(feature = "corpus"), not(feature = "unknown-fields")), path = "../generated_nounk/abi.rs")]
    pub mod abi;
}
pub use generated::abi::*;

// ---- the fixed ABI (sections 3, 4, 5, 7.1, 10). The region between the two markers is
// RENDERED from `poc/codec/gen/plan.py`'s `FIXED` (and `plan.lifecycle`'s flag values) by
// `poc/codec/gen/generate.py` (FIX-PLAN WP5 step 6, R-G13): error codes, the vocabulary
// structs, the function types, the constants and every fixed entry point, stated once for
// every backend. The core's definitions are checked against these signatures at compile
// time (`ak-core/src/generated/abi_check.rs`).
// @generated-begin plan.fixed
// Rendered by poc/codec/gen/generate.py from plan.py (FIXED). Do not edit by hand.

/// success
pub const AK_OK: i32 = 0;
/// success: ak_init called again with the same options
pub const AK_ALREADY_INITIALIZED: i32 = 1;
/// the host reported through ak_fail
pub const AK_ERR_HOST: i32 = -1;
/// invalid wire
pub const AK_ERR_MALFORMED: i32 = -2;
/// the input ended inside a value
pub const AK_ERR_TRUNCATED: i32 = -3;
/// the decode recursion limit
pub const AK_ERR_DEPTH: i32 = -4;
/// a size limit
pub const AK_ERR_LIMIT: i32 = -5;
/// the transcoder refused its input
pub const AK_ERR_TRANSCODE: i32 = -6;
/// a transcoder wrote past the capacity given
pub const AK_ERR_CAPACITY: i32 = -7;
/// e.g. two concurrent recv on one call
pub const AK_ERR_INVALID_STATE: i32 = -8;
/// a caught Rust panic
pub const AK_ERR_PANIC: i32 = -9;
/// ak_init was not called
pub const AK_ERR_UNINITIALIZED: i32 = -10;
/// version or group-layout mismatch
pub const AK_ERR_ABI: i32 = -11;
/// the call completed with a non-OK gRPC status (read it with ak_call_recv)
pub const AK_ERR_RPC_STATUS: i32 = -12;
pub const AK_DETAIL_NONE: u32 = 0;
pub const AK_DETAIL_ABI_MISMATCH: u32 = 1;
pub const AK_DETAIL_OPTS_DIFFER: u32 = 2;
pub const AK_DETAIL_NULL_ARG: u32 = 3;
pub const AK_ABI_VERSION: u32 = 1;
pub const AK_INIT_OWN_LOGGING: u32 = 1;
pub const AK_INIT_NO_PANIC_HOOK: u32 = 2;
pub const AK_INIT_NO_CRYPTO: u32 = 4;

/// An opaque context; the host holds a pointer and never looks inside.
pub enum ak_enc_ctx {}
/// An opaque context; the host holds a pointer and never looks inside.
pub enum ak_dec_ctx {}

/// ABI v1 section 4: the transcoder's growth callback; may move the buffer.
pub type ak_grow_fn = unsafe extern "C" fn(sink: *mut c_void, want: i32, dst: *mut *mut u8, cap: *mut i32) -> i32;
/// ABI v1 section 4: source code units -> UTF-8 into dst; returns bytes written or an error.
pub type ak_transcode_fn = unsafe extern "C" fn(src: *const c_void, len: usize, dst: *mut u8, cap: i32, grow: ak_grow_fn, sink: *mut c_void) -> i32;
/// ABI v1 section 3: the host's log sink.
pub type ak_log_fn = unsafe extern "C" fn(ctx: *mut c_void, level: u32, msg: *const u8, msg_len: usize);
/// ABI v1 section 6: a host-driven loop over one repeated/packed/map field.
pub type ak_loop_f = unsafe extern "C" fn(ctx: *mut ak_enc_ctx, obj: *const c_void, token: i64) -> i32;

/// ABI v1 section 4, encode: a blob as DATA in the group. `len` counts SOURCE code units; `tc == NULL` means the field is ABSENT.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ak_str {
    pub data: *const c_void,
    pub len: usize,
    pub tc: Option<ak_transcode_fn>,
}
impl Default for ak_str {
    fn default() -> Self {
        ak_str {
            data: ::core::ptr::null(),
            len: 0,
            tc: None,
        }
    }
}

/// ABI v1 section 4, decode: an OFFSET into the buffer the host handed in.
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct ak_span {
    pub off: u32,
    pub len: u32,
    pub coder: u32,
}

/// Decision 11's unknown-field bag: two words, raw tag-and-value runs.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ak_blob {
    pub data: *const c_void,
    pub len: usize,
}
impl Default for ak_blob {
    fn default() -> Self {
        ak_blob {
            data: ::core::ptr::null(),
            len: 0,
        }
    }
}

/// Decision 11 (WP5 step 7): one message occurrence's unknown-field buffer, host memory the core copies the runs into.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ak_unk_buf {
    pub data: *mut c_void,
    pub len: u32,
    pub cap: u32,
}
impl Default for ak_unk_buf {
    fn default() -> Self {
        ak_unk_buf {
            data: ::core::ptr::null_mut(),
            len: 0,
            cap: 0,
        }
    }
}

/// Decision 11: one message position's configuration; all zero = its unknowns are discarded.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ak_unk_opts {
    pub buf: ak_unk_buf,
    pub grow: Option<ak_grow_fn>,
}
impl Default for ak_unk_opts {
    fn default() -> Self {
        ak_unk_opts {
            buf: ak_unk_buf::default(),
            grow: None,
        }
    }
}

/// Decision 11 rule 1: a REPEATED position's configuration: buffers taken in order, cleared in place as they are taken; `grow` the fallback.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ak_unk_pool {
    pub bufs: *mut ak_unk_buf,
    pub n: u32,
    pub grow: Option<ak_grow_fn>,
}
impl Default for ak_unk_pool {
    fn default() -> Self {
        ak_unk_pool {
            bufs: ::core::ptr::null_mut(),
            n: 0,
            grow: None,
        }
    }
}

/// ak_init's out-parameter (ABI v1 section 3/5): a code and a detail.
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct ak_err {
    pub code: i32,
    pub detail: u32,
}

/// ak_init's options (ABI v1 section 3).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ak_init_opts {
    pub abi_version: u32,
    pub flags: u32,
    pub log: Option<ak_log_fn>,
    pub log_ctx: *mut c_void,
}
impl Default for ak_init_opts {
    fn default() -> Self {
        ak_init_opts {
            abi_version: 0,
            flags: 0,
            log: None,
            log_ctx: ::core::ptr::null_mut(),
        }
    }
}

/// README R5: boundary-call counts, counted in the CORE (zero unless the core is a counting build).
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct AkCounters {
    pub forward: u64,
    pub reverse: u64,
    pub transcode: u64,
    pub prefix_moves: u64,
    pub prefix_bytes: u64,
    pub grows: u64,
}

/// ABI v1 section 7.1: one pull record header, 24 bytes; the payload follows, padded to 8.
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct ak_bdr_rec {
    pub op: u32,
    pub slot: u32,
    pub token: i64,
    pub n: u32,
    pub bytes: u32,
}

/// FIX-PLAN D23: one FSM event; its op is the return value of ak_fsm_begin_* / ak_fsm_next_*. `bytes` exact, not padded; `data` NULL for AK_BDR_NEW.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ak_fsm_ev {
    pub slot: u32,
    pub n: u32,
    pub token: i64,
    pub data: *const c_void,
    pub bytes: u32,
}
impl Default for ak_fsm_ev {
    fn default() -> Self {
        ak_fsm_ev {
            slot: 0,
            n: 0,
            token: 0,
            data: ::core::ptr::null(),
            bytes: 0,
        }
    }
}

/// ABI v1 section 8: ak_str.data meaning 'a direct argument of the call'.
pub const AK_STR_DIRECT: *const c_void = 1usize as *const c_void;
/// a token naming the root object itself
pub const AK_TOKEN_ROOT: i64 = -1;
/// pull record: the root group (last record)
pub const AK_BDR_APPLY: u32 = 1;
/// pull record: a batched run
pub const AK_BDR_ADD: u32 = 2;
/// pull record: a non-leaf element begins (minted token)
pub const AK_BDR_NEW: u32 = 3;
/// pull record: a non-leaf element's group
pub const AK_BDR_APPLY_ELEM: u32 = 4;
/// the smallest drain chunk that holds any record
pub const AK_BDR_MIN_CHUNK: usize = 32792;

unsafe extern "C" {
    /// Once per process, before anything else (plan.lifecycle).
    pub fn ak_init(opts: *const ak_init_opts, err: *mut ak_err) -> i32;
    /// 1 once ak_init has returned successfully.
    pub fn ak_initialized() -> i32;
    /// A NUL-terminated build id.
    pub fn ak_build_id() -> *const core::ffi::c_char;
    /// Test hook: one line through the installed log bridge.
    pub fn ak_log_test(level: u32, msg: *const u8, len: usize) -> i32;
    /// Test hook: a panic inside the core.
    pub fn ak_panic_test();
    pub fn ak_abi_version() -> u32;
    pub fn ak_enc_ctx_new() -> *mut ak_enc_ctx;
    pub fn ak_enc_ctx_free(ctx: *mut ak_enc_ctx);
    /// Clears the buffer and the sticky slot.
    pub fn ak_enc_reset(ctx: *mut ak_enc_ctx);
    /// Borrow what the context has encoded, valid until the next reset; returns the status.
    pub fn ak_enc_take(ctx: *mut ak_enc_ctx, ptr: *mut *const u8, len: *mut usize) -> i32;
    pub fn ak_dec_ctx_free(ctx: *mut ak_dec_ctx);
    /// Sticky, first error wins; takes either context (ABI v1 section 5).
    pub fn ak_fail(ctx: *mut c_void, code: i32, msg: *const u8, msg_len: u32);
    pub fn ak_enc_err(ctx: *const ak_enc_ctx) -> i32;
    pub fn ak_dec_err(ctx: *const ak_dec_ctx) -> i32;
    pub fn ak_dec_err_reset(ctx: *mut ak_dec_ctx);
    /// UTF-8, validate and refuse.
    pub fn ak_tc_utf8() -> ak_transcode_fn;
    /// UTF-8, no validation.
    pub fn ak_tc_utf8_trusted() -> ak_transcode_fn;
    /// UTF-8, SIMD validator.
    pub fn ak_tc_utf8_simd() -> ak_transcode_fn;
    /// bytes: a copy.
    pub fn ak_tc_bytes() -> ak_transcode_fn;
    /// UTF-16 code units -> UTF-8.
    pub fn ak_tc_utf16() -> ak_transcode_fn;
    /// Latin-1 bytes -> UTF-8.
    pub fn ak_tc_latin1() -> ak_transcode_fn;
    pub fn ak_enc_counters(ctx: *const ak_enc_ctx, out: *mut AkCounters);
    /// Counting build: the HOST reports a reverse crossing the core cannot see.
    pub fn ak_enc_count_reverse(ctx: *mut ak_enc_ctx);
    pub fn ak_enc_counters_reset(ctx: *mut ak_enc_ctx);
    pub fn ak_dec_counters(ctx: *const ak_dec_ctx, out: *mut AkCounters);
    pub fn ak_dec_counters_reset(ctx: *mut ak_dec_ctx);
    pub fn ak_enc_site_moves(ctx: *const ak_enc_ctx, out: *mut u32, cap: usize) -> usize;
    /// A crossing and nothing else.
    pub fn ak_noop(x: u64) -> u64;
    /// An identical twin of ak_noop.
    pub fn ak_noop2(x: u64) -> u64;
    /// ak_noop with the init guard.
    pub fn ak_noop_guarded(x: u64) -> u64;
    /// One forward and one reverse crossing.
    pub fn ak_noop_reverse(f: unsafe extern "C" fn(u64) -> u64, x: u64) -> u64;
    pub fn ak_bdr_reserve(ctx: *mut ak_dec_ctx, bytes: usize) -> i32;
    pub fn ak_bdr_footprint(ctx: *const ak_dec_ctx) -> usize;
    pub fn ak_bdr_drain(ctx: *mut ak_dec_ctx, dst: *mut u8, cap: usize, cursor: *mut usize) -> isize;
    pub fn ak_bdr_ptr(ctx: *mut ak_dec_ctx, ptr: *mut *const u8, len: *mut usize) -> i32;
    pub fn ak_bdr_reset(ctx: *mut ak_dec_ctx);
    /// Counting build: the host reports the forward crossings of a drain loop.
    pub fn ak_bdr_count_forward(ctx: *mut ak_dec_ctx, n: u32);
    /// ABI v1 section 10: the core's own view of every group layout.
    pub fn ak_layout_facts(out: *mut u32, cap: usize) -> usize;
}
// @generated-end plan.fixed

// ---- section 9: the RPC half, moving opaque bytes and dispatching on a path string.
// The region between the two markers is RENDERED from `poc/codec/gen/plan.py`'s `RpcAbi`
// by `poc/codec/gen/generate.py` (R-G5), not written by hand: the structs, the callback,
// the constants and the prototypes are stated once, and the core's definitions are
// checked against them at compile time (`ak-core/src/generated/rpc_check.rs`). It stays
// in this file, rather than in `generated/`, because the cpp slice's header generator
// reads `ak_client_opts` from here (R-D2) until its backend renders plans (WP5 step 2).
// @generated-begin plan.rpc
// Rendered by poc/codec/gen/generate.py from plan.py (RpcAbi). Do not edit by hand.

/// An opaque handle; the host holds a pointer and never looks inside.
pub enum ak_runtime {}
/// An opaque handle; the host holds a pointer and never looks inside.
pub enum ak_client {}
/// An opaque handle; the host holds a pointer and never looks inside.
pub enum ak_call {}
/// An opaque handle; the host holds a pointer and never looks inside.
pub enum ak_queue {}

/// A byte range the core owns until the host releases it with `ak_bytes_free`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ak_bytes {
    pub ptr: *const u8,
    pub len: usize,
    /// The core's handle on the allocation. The host passes it back and does not read it.
    pub owner: *mut c_void,
}

impl Default for ak_bytes {
    fn default() -> Self {
        ak_bytes {
            ptr: ::core::ptr::null(),
            len: 0,
            owner: ::core::ptr::null_mut(),
        }
    }
}

/// What a completion carries. Released with `ak_bytes_free`, as the blocking delivery's bytes are, so a host has one release path whichever delivery it takes.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ak_completion {
    pub tag: u64,
    /// AK_OK iff grpc_status is 0; AK_ERR_RPC_STATUS for another code; the core's own errors as before.
    pub status: i32,
    /// The gRPC status code (0-16); -1 when no call reached the transport.
    pub grpc_status: i32,
    pub bytes: ak_bytes,
}

impl Default for ak_completion {
    fn default() -> Self {
        ak_completion {
            tag: 0,
            status: 0,
            grpc_status: 0,
            bytes: ak_bytes::default(),
        }
    }
}

/// One metadata entry of a call (ABI v1 section 9, streaming as built). ASCII metadata; a key ending in "-bin" carries raw bytes.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ak_kv {
    pub key: *const u8,
    pub key_len: usize,
    pub val: *const u8,
    pub val_len: usize,
}

impl Default for ak_kv {
    fn default() -> Self {
        ak_kv {
            key: ::core::ptr::null(),
            key_len: 0,
            val: ::core::ptr::null(),
            val_len: 0,
        }
    }
}

/// A streamed call's options (section 9). NULL = no deadline, no metadata.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ak_call_opts {
    /// 0 = none; otherwise sent as grpc-timeout.
    pub deadline_ms: u64,
    /// ASCII metadata; a key ending in "-bin" carries raw bytes.
    pub metadata: *const ak_kv,
    pub n_metadata: usize,
}

impl Default for ak_call_opts {
    fn default() -> Self {
        ak_call_opts {
            deadline_ms: 0,
            metadata: ::core::ptr::null(),
            n_metadata: 0,
        }
    }
}

/// The transport settings ArmoniK pins. The stream and the connection window are separate settings on tonic/hyper, so both are here. Zero means "the stack's".
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ak_client_opts {
    /// SETTINGS_INITIAL_WINDOW_SIZE per stream; 0 = default. ArmoniK: 4 MiB.
    pub stream_window: u32,
    /// The connection-level window, a SEPARATE setting; 0 = default.
    pub connection_window: u32,
    /// 1 on, 0 off, -1 default (off). Overrides both windows when on.
    pub adaptive_window: i32,
    /// Largest message accepted, bytes; 0 = default.
    pub max_recv_message: u32,
    /// Largest message sent, bytes; 0 = default.
    pub max_send_message: u32,
    /// ArmoniK's tcp_nagle_algorithm: 1 Nagle on, 0 off, -1 default. ArmoniK ships it OFF.
    pub tcp_nagle: i32,
}

impl Default for ak_client_opts {
    fn default() -> Self {
        ak_client_opts {
            stream_window: 0,
            connection_window: 0,
            adaptive_window: 0,
            max_recv_message: 0,
            max_send_message: 0,
            tcp_nagle: 0,
        }
    }
}

/// Called ONCE per call, on a thread the core owns.
pub type ak_completion_cb = extern "C" fn(user_data: *mut c_void, comp: *mut ak_completion);

/// `ak_queue_next` returned a completion.
pub const AK_QUEUE_OK: i32 = 0;
/// The timeout expired with no completion. Not an error.
pub const AK_QUEUE_TIMEOUT: i32 = 1;
/// The queue is shutting down and is drained.
pub const AK_QUEUE_SHUTDOWN: i32 = 2;
/// ak_call_kind: many request messages, one response. BUILT.
pub const AK_CALL_CLIENT_STREAM: i32 = 1;
/// ak_call_kind, reserved: ak_call_open returns NULL.
pub const AK_CALL_SERVER_STREAM: i32 = 2;
/// ak_call_kind, reserved: ak_call_open returns NULL.
pub const AK_CALL_BIDI_STREAM: i32 = 3;

/// RPC boundary-call counts (counting build).
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct ak_rpc_counters {
    pub forward: u64,
    pub reverse: u64,
}

unsafe extern "C" {
    pub fn ak_runtime_new(worker_threads: u32) -> *mut ak_runtime;
    pub fn ak_runtime_destroy(r: *mut ak_runtime);
    pub fn ak_client_new(r: *mut ak_runtime, uri: *const u8, uri_len: usize) -> *mut ak_client;
    /// A client with the transport pinned. NULL options = `ak_client_new`.
    pub fn ak_client_new_opts(r: *mut ak_runtime, uri: *const u8, uri_len: usize, opts: *const ak_client_opts) -> *mut ak_client;
    pub fn ak_client_destroy(c: *mut ak_client);
    /// Choose the send path of every later call on `c`: 1 = FRAMED, the DEFAULT (tonic's Channel, each request message sent as ONE body frame, its 5-byte gRPC prefix written into the encode context's headroom or into the copy a copying entry makes; no copy into tonic's buffer), 0 = the reference (tonic's Grpc::unary / client_streaming with a raw-bytes codec, one copy into tonic's buffer). Request headers, response status and trailer handling, compression (off) and the send limit are the reference path's. Another value or NULL `c` is AK_ERR_INVALID_STATE.
    pub fn ak_client_set_framed(c: *mut ak_client, on: i32) -> i32;
    /// Blocking delivery: one crossing in, `ak_bytes_free` the only other. Writes the gRPC status code to `*grpc_status` (may be NULL; -1 when no call reached the transport); returns AK_OK iff it is 0, AK_ERR_RPC_STATUS for another code, the core's own errors as before (AK_ERR_LIMIT above the client's send limit, nothing sent, or its receive limit).
    pub fn ak_call_unary(c: *mut ak_client, path: *const u8, path_len: usize, req: *const u8, req_len: usize, out: *mut ak_bytes, grpc_status: *mut i32) -> i32;
    /// Blocking delivery whose request is the encode context's output, MOVED (not copied); the context's encoded bytes are consumed. Additive (optimisation R2).
    pub fn ak_call_unary_enc(c: *mut ak_client, path: *const u8, path_len: usize, enc: *mut ak_enc_ctx, out: *mut ak_bytes, grpc_status: *mut i32) -> i32;
    pub fn ak_bytes_free(b: *mut ak_bytes);
    /// The encode context's output handed to the host as an owned buffer, MOVED (not copied): `out` holds exactly the encoded bytes until the host releases them with `ak_bytes_free`, on any thread, at any time; the context's encoded bytes are consumed, it continues on its spare buffer (a fresh one while the spare is still out), and the released buffer becomes its spare. A context in error is refused with its error and `out` left empty; NULL `enc` or `out` is AK_ERR_INVALID_STATE. One crossing, `ak_bytes_free` the only other. Additive (optimisation T1).
    pub fn ak_enc_take_owned(enc: *mut ak_enc_ctx, out: *mut ak_bytes) -> i32;
    /// Callback delivery: 2 forward crossings and 1 reverse.
    pub fn ak_call_unary_cb(c: *mut ak_client, path: *const u8, path_len: usize, req: *const u8, req_len: usize, cb: ak_completion_cb, user_data: *mut c_void, tag: u64) -> *mut ak_call;
    /// Completion-queue delivery: no upcall. 3 forward crossings and 0 reverse.
    pub fn ak_call_unary_q(c: *mut ak_client, path: *const u8, path_len: usize, req: *const u8, req_len: usize, q: *mut ak_queue, tag: u64) -> *mut ak_call;
    /// Callback delivery of ak_call_unary_enc: the request is the encode context's output, MOVED before this returns. NULL (and no completion) on a NULL argument, an invalid path or a context in error. Additive.
    pub fn ak_call_unary_enc_cb(c: *mut ak_client, path: *const u8, path_len: usize, enc: *mut ak_enc_ctx, cb: ak_completion_cb, user_data: *mut c_void, tag: u64) -> *mut ak_call;
    /// Queue delivery of ak_call_unary_enc: as ak_call_unary_enc_cb, the completion pushed onto `q`. Additive.
    pub fn ak_call_unary_enc_q(c: *mut ak_client, path: *const u8, path_len: usize, enc: *mut ak_enc_ctx, q: *mut ak_queue, tag: u64) -> *mut ak_call;
    pub fn ak_queue_new() -> *mut ak_queue;
    /// Wait up to `timeout_ms` for one completion. R-G5: `u64`, as the core defines it.
    pub fn ak_queue_next(q: *mut ak_queue, out: *mut ak_completion, timeout_ms: u64) -> i32;
    pub fn ak_queue_shutdown(q: *mut ak_queue);
    pub fn ak_queue_destroy(q: *mut ak_queue);
    /// Cancel any call handle, a stream's included: unblocks a pending ak_call_send or ak_call_recv; does NOT free (ak_call_destroy does, after every operation returned).
    pub fn ak_call_cancel(h: *mut ak_call);
    /// Open a streamed call on `path` (section 9, streaming as built). `kind` AK_CALL_CLIENT_STREAM is built; AK_CALL_SERVER_STREAM and AK_CALL_BIDI_STREAM are reserved and return NULL, as does another kind, NULL `c` or invalid metadata. `opts` may be NULL (no deadline, no metadata). The client's send path (ak_client_set_framed) and limits (ak_client_opts) apply to every message.
    pub fn ak_call_open(c: *mut ak_client, path: *const u8, path_len: usize, kind: i32, opts: *const ak_call_opts) -> *mut ak_call;
    /// Send one request message (copied); `last` nonzero ends the request stream after it. Blocks while the transport has not taken the previous message. Above the client's send limit: AK_ERR_LIMIT, nothing sent. After `last`: AK_ERR_INVALID_STATE. On a stream that has already failed: AK_ERR_HOST (the status is read with ak_call_recv).
    pub fn ak_call_send(h: *mut ak_call, msg: *const u8, len: usize, last: i32) -> i32;
    /// `ak_call_send` whose message is the encode context's output, MOVED (not copied); a context in error is refused with its error. Additive.
    pub fn ak_call_send_enc(h: *mut ak_call, enc: *mut ak_enc_ctx, last: i32) -> i32;
    /// Block for the call's response, released with ak_bytes_free. Writes the gRPC status code (0 to 16) to `*grpc_status` (may be NULL) whenever the call completed; returns AK_OK iff it is 0, else AK_ERR_RPC_STATUS (a transport failure is UNAVAILABLE, a cancelled call CANCELLED, a response above the receive limit RESOURCE_EXHAUSTED). A second recv: AK_ERR_INVALID_STATE, `*grpc_status` untouched.
    pub fn ak_call_recv(h: *mut ak_call, out: *mut ak_bytes, grpc_status: *mut i32) -> i32;
    /// Callback delivery of ak_call_send: the message is copied before this returns; the completion (empty bytes) fires ONCE, on a core thread, when the call has accepted the message, so the host may send the next one from it. Returns AK_OK and a completion follows, or the refusal and none: above the send limit AK_ERR_LIMIT (nothing sent), after `last` or while a send is pending AK_ERR_INVALID_STATE. Completion status AK_OK (grpc_status 0), or AK_ERR_HOST (grpc_status -1) when the call has already ended, failed or cancelled (its status is read with a recv).
    pub fn ak_call_send_cb(h: *mut ak_call, msg: *const u8, len: usize, last: i32, cb: ak_completion_cb, user_data: *mut c_void, tag: u64) -> i32;
    /// ak_call_send_cb whose message is the encode context's output, MOVED before this returns (the context may be reused at once); a context in error is refused with its error.
    pub fn ak_call_send_enc_cb(h: *mut ak_call, enc: *mut ak_enc_ctx, last: i32, cb: ak_completion_cb, user_data: *mut c_void, tag: u64) -> i32;
    /// Callback delivery of ak_call_recv: the completion fires ONCE, on a core thread, when the call completes: status AK_OK iff grpc_status is 0, else AK_ERR_RPC_STATUS with the code (cancelled: CANCELLED; above the receive limit: RESOURCE_EXHAUSTED); `bytes` the response, released with ak_bytes_free. A second recv of any delivery: AK_ERR_INVALID_STATE and no completion.
    pub fn ak_call_recv_cb(h: *mut ak_call, cb: ak_completion_cb, user_data: *mut c_void, tag: u64) -> i32;
    /// Queue delivery of ak_call_send: as ak_call_send_cb, the completion pushed onto `q`.
    pub fn ak_call_send_q(h: *mut ak_call, msg: *const u8, len: usize, last: i32, q: *mut ak_queue, tag: u64) -> i32;
    /// Queue delivery of ak_call_send_enc: as ak_call_send_enc_cb, the completion pushed onto `q`.
    pub fn ak_call_send_enc_q(h: *mut ak_call, enc: *mut ak_enc_ctx, last: i32, q: *mut ak_queue, tag: u64) -> i32;
    /// Queue delivery of ak_call_recv: as ak_call_recv_cb, the completion pushed onto `q`.
    pub fn ak_call_recv_q(h: *mut ak_call, q: *mut ak_queue, tag: u64) -> i32;
    /// Frees the handle, only after every operation returned and every completion was delivered.
    pub fn ak_call_destroy(h: *mut ak_call);
    /// 1 if this core counts RPC crossings.
    pub fn ak_rpc_counting() -> i32;
    pub fn ak_rpc_counters(out: *mut ak_rpc_counters);
    pub fn ak_rpc_counters_reset();
}
// @generated-end plan.rpc
