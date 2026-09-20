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
    pub mod abi;
}
pub use generated::abi::*;

// ---- section 5: errors ------------------------------------------------------------

pub const AK_OK: i32 = 0;
pub const AK_ALREADY_INITIALIZED: i32 = 1;
pub const AK_ERR_HOST: i32 = -1;
pub const AK_ERR_MALFORMED: i32 = -2;
pub const AK_ERR_TRUNCATED: i32 = -3;
pub const AK_ERR_DEPTH: i32 = -4;
pub const AK_ERR_LIMIT: i32 = -5;
pub const AK_ERR_TRANSCODE: i32 = -6;
pub const AK_ERR_CAPACITY: i32 = -7;
pub const AK_ERR_INVALID_STATE: i32 = -8;
pub const AK_ERR_PANIC: i32 = -9;
pub const AK_ERR_UNINITIALIZED: i32 = -10;
pub const AK_ERR_ABI: i32 = -11;

/// ABI v1 section 10: one version for the whole ABI, checked once in `ak_init`.
pub const AK_ABI_VERSION: u32 = 1;

// ---- section 3: the lifecycle -----------------------------------------------------
//
// "Nothing here is implicit. A host initialises the library, then builds a runtime, then a
// context, then a client, and destroys them in the reverse order."

/// The host takes the process log; the core installs no bridge.
pub const AK_INIT_OWN_LOGGING: u32 = 1 << 0;
/// The host keeps its own panic hook; the core installs none.
pub const AK_INIT_NO_PANIC_HOOK: u32 = 1 << 1;
/// The host will not use the RPC half, so the crypto provider is not installed.
/// Not in the specification's flag list, which says "AK_INIT_OWN_LOGGING,
/// AK_INIT_NO_PANIC_HOOK, ...". A codec-only host is the common case in this branch --
/// four of the five slices build a codec arm and no RPC arm -- and without it every such
/// host pays a one-shot install it will never reach.
pub const AK_INIT_NO_CRYPTO: u32 = 1 << 2;

/// A log line from the core. Severity follows `tracing`: 0 error, 1 warn, 2 info, 3 debug,
/// 4 trace. The message is NOT null-terminated; it is a pointer and a length, like every
/// other string in this ABI.
pub type ak_log_fn = unsafe extern "C" fn(
    ctx: *mut c_void,
    level: u32,
    msg: *const u8,
    msg_len: usize,
);

/// ABI v1 section 3. `abi_version` is passed IN rather than only exported, so the check is
/// made by the side that knows what it was generated against, once, at the only point
/// where failing is cheap.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ak_init_opts {
    pub abi_version: u32,
    pub flags: u32,
    pub log: Option<ak_log_fn>,
    pub log_ctx: *mut c_void,
}

/// Section 5's out-parameter form. `ak_init` has no context to put a sticky error in,
/// because a context cannot exist before it returns.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ak_err {
    pub code: i32,
    /// What the core wanted the host to know, as an index into `ak_err_text`. A pointer
    /// would have to be owned by somebody; a code is owned by nobody.
    pub detail: u32,
}

/// `ak_err.detail` values. Named so a host can switch on them rather than on prose.
pub const AK_DETAIL_NONE: u32 = 0;
pub const AK_DETAIL_ABI_MISMATCH: u32 = 1;
pub const AK_DETAIL_OPTS_DIFFER: u32 = 2;
pub const AK_DETAIL_NULL_ARG: u32 = 3;

unsafe extern "C" {
    /// Once per process, before anything else, the codec included.
    ///
    /// Idempotent under IDENTICAL options: a second call with the same options returns
    /// `AK_ALREADY_INITIALIZED`, which is a success. A second call with different options
    /// fails, because the one-shot installs cannot be redone. There is no `ak_shutdown`.
    pub fn ak_init(opts: *const ak_init_opts, err: *mut ak_err) -> i32;
    /// Whether `ak_init` has returned successfully. Not in the specification; it is what
    /// makes "every other entry point requires `ak_init`" testable from outside.
    pub fn ak_initialized() -> i32;
    /// Two copies of the core in one process either share Rust's globals or split-brain
    /// them with no warning, so the build id is checked rather than assumed (section 3).
    /// Null-terminated, static, never freed.
    pub fn ak_build_id() -> *const core::ffi::c_char;
    /// Emit a line through the host's sink, so a host can see the bridge work without
    /// waiting for the core to have something to say. Same path a real line takes.
    /// Returns 1 if a sink took it, 0 if there is none.
    pub fn ak_log_test(level: u32, msg: *const u8, len: usize) -> i32;
    /// Panic inside the core, on purpose, so the hook can be seen working. Does not
    /// return: the unwind is refused at this `extern "C"` boundary and the process aborts.
    pub fn ak_panic_test();
}

// ---- section 4: common vocabulary -------------------------------------------------

/// Encode: a string or bytes field as DATA inside the group, never a call.
///
/// `len` counts SOURCE code units, never bytes and never characters -- for a Rust host
/// that is bytes, because `str` is UTF-8 already. `tc == null` means the field is absent,
/// which is how a group distinguishes absent from empty (`tc` set, `len == 0`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ak_str {
    pub data: *const c_void,
    pub len: usize,
    pub tc: Option<ak_transcode_fn>,
}

/// ABI v1 section 8: a small range of `ak_str.data` values is reserved as sentinels meaning
/// *this field is a direct argument of the call* rather than a pointer into staging.
///
/// It is one sentence in the specification and the one unambiguous win on the JVM: the host
/// can hold `GetPrimitiveArrayCritical` or FFM's `critical(true)` across the whole call,
/// which it can only do if the codec makes no reverse call -- hence the generator-time
/// refusal that goes with it. **On a Rust host it buys nothing**: there is no pinning to
/// avoid and the copy is a copy either way, so this slice can show the path works and cannot
/// confirm the win.
pub const AK_STR_DIRECT: *const core::ffi::c_void = 1usize as *const core::ffi::c_void;

impl Default for ak_str {
    fn default() -> Self {
        ak_str { data: core::ptr::null(), len: 0, tc: None }
    }
}

/// Decode: an OFFSET into the buffer the host handed in, plus a byte length. Eight bytes
/// where a pointer pair was 24, and still meaningful after a JNI host has released a
/// critical section.
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct ak_span {
    pub off: u32,
    pub len: u32,
    /// A host hint (ABI v1 open decision 4). Nothing in this slice reads it.
    pub coder: u32,
}

/// The transcoder asks for more room. May move the buffer, which is why it writes back
/// through pointers.
pub type ak_grow_fn =
    unsafe extern "C" fn(sink: *mut c_void, want: i32, dst: *mut *mut u8, cap: *mut i32) -> i32;

/// Writes the host's own string representation as UTF-8 into the codec's buffer. Returns
/// bytes written, or a negative ak error code.
pub type ak_transcode_fn = unsafe extern "C" fn(
    src: *const c_void,
    len: usize,
    dst: *mut u8,
    cap: i32,
    grow: ak_grow_fn,
    sink: *mut c_void,
) -> i32;

// ---- opaque handles ---------------------------------------------------------------

pub enum ak_enc_ctx {}
pub enum ak_dec_ctx {}

// ABI v1 section 6: what a host calls in the codec are PLAIN EXPORTS, not a table, so the
// host declares the symbols it uses and a missing one is a load failure.
unsafe extern "C" {
    pub fn ak_abi_version() -> u32;

    pub fn ak_enc_ctx_new() -> *mut ak_enc_ctx;
    pub fn ak_enc_ctx_free(ctx: *mut ak_enc_ctx);
    pub fn ak_enc_reset(ctx: *mut ak_enc_ctx);
    /// Borrow what the context has encoded. Valid until the next reset.
    pub fn ak_enc_take(ctx: *mut ak_enc_ctx, ptr: *mut *const u8, len: *mut usize) -> i32;

    pub fn ak_dec_ctx_new() -> *mut ak_dec_ctx;
    pub fn ak_dec_ctx_free(ctx: *mut ak_dec_ctx);

    /// Sticky: the first error wins, so unwinding cannot overwrite the cause. Never
    /// allocates, never throws, safe from inside a reverse-call frame (section 5).
    pub fn ak_fail(ctx: *mut c_void, code: i32, msg: *const u8, msg_len: u32);
    pub fn ak_enc_err(ctx: *const ak_enc_ctx) -> i32;
    /// What the host reported through `ak_fail` on a DECODE context. `ak_fail` takes either
    /// kind: both contexts begin with the same header, so the error slot is at one offset.
    pub fn ak_dec_err(ctx: *const ak_dec_ctx) -> i32;
    pub fn ak_dec_err_reset(ctx: *mut ak_dec_ctx);

    /// The transcoders the core implements once for every language (section 4).
    pub fn ak_tc_utf8() -> ak_transcode_fn;
    /// UTF-8 passthrough with no validation, for a host whose type already carries the
    /// invariant. ABI v1 open decision 3 is about which of these `ak_tc_utf8` should be.
    pub fn ak_tc_utf8_trusted() -> ak_transcode_fn;
    /// Validate-and-fail, with a SIMD validator rather than the scalar one.
    pub fn ak_tc_utf8_simd() -> ak_transcode_fn;
    pub fn ak_tc_bytes() -> ak_transcode_fn;

    /// Counting build only (`--features count`). The counters live in the contexts, not in
    /// a thread-local: a thread-local is a hidden global with a re-entrancy hazard, which
    /// ABI v1 section 7.3 refuses for the arena for the same reason.
    pub fn ak_enc_counters(ctx: *const ak_enc_ctx, out: *mut AkCounters);
    pub fn ak_enc_counters_reset(ctx: *mut ak_enc_ctx);
    pub fn ak_dec_counters(ctx: *const ak_dec_ctx, out: *mut AkCounters);
    pub fn ak_dec_counters_reset(ctx: *mut ak_dec_ctx);
    pub fn ak_enc_site_moves(ctx: *const ak_enc_ctx, out: *mut u32, cap: usize) -> usize;

    /// The boundary, priced on its own, in the same process and the same build.
    pub fn ak_noop(x: u64) -> u64;
    pub fn ak_noop_reverse(f: unsafe extern "C" fn(u64) -> u64, x: u64) -> u64;
}

// ---- section 7.1's PULL family: the host drives, the codec makes no reverse call -------
//
// The push family's `ak_decode_*` hands each value over by calling the host. The pull
// family's `ak_parse_*` writes the same handovers as records into the host-owned decode
// context, and the host reads them afterwards. So a record stream IS the call sequence the
// push family would have made, in the same order, which is what makes the two families two
// deliveries of one traversal rather than two decoders (open decision 2).

/// One record. The host reads these out of a drained chunk, or in place.
///
/// 24 bytes with no padding, so a group that follows it is 8-aligned, which every
/// `ak_dfix_*` needs. `ak-core` static-asserts this against the core's own `ak_rt::bdr::Rec`
/// at compile time; the two are separate declarations on purpose, because that is the
/// situation section 10 exists for and the rust slice could not otherwise reach it.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ak_bdr_rec {
    pub op: u32,
    /// `(outer << 16) | inner`. `outer` is the root's loop slot whose element this belongs
    /// to, 1-based, or 0 for the root object itself; `inner` is the slot within that scope,
    /// 1-based, or 0 for the element itself.
    pub slot: u32,
    /// The element index, or `AK_TOKEN_ROOT`. An index, never an address (section 10).
    pub token: i64,
    pub n: u32,
    /// Payload bytes after this header, already rounded up to a multiple of 8.
    pub bytes: u32,
}

/// The root group: what `apply` would have been called with.
pub const AK_BDR_APPLY: u32 = 1;
/// A run: what `add_<slot>(obj, token, elems, n)` would have been called with.
pub const AK_BDR_ADD: u32 = 2;
/// A non-leaf element begins: what `new_<slot>` would have returned, minted by the codec.
pub const AK_BDR_NEW: u32 = 3;
/// A non-leaf element's group: what `apply_<slot>(obj, token, fix)` would have been called with.
pub const AK_BDR_APPLY_ELEM: u32 = 4;

/// The smallest chunk `ak_bdr_drain` accepts, whatever the schema: section 7.3's 32 KB
/// arena plus one header. A record is never split, because a host walking a chunk has to
/// find a header at its start; a chunk smaller than one record returns `AK_ERR_CAPACITY`.
pub const AK_BDR_MIN_CHUNK: usize = 32 * 1024 + 24;

unsafe extern "C" {
    /// Pre-size the record buffer. Optional.
    pub fn ak_bdr_reserve(ctx: *mut ak_dec_ctx, bytes: usize) -> i32;
    /// What the last parse deposited, so a host can read what it is paying and bound it.
    pub fn ak_bdr_footprint(ctx: *const ak_dec_ctx) -> usize;
    /// Copy whole records out, from `*cursor`. Returns bytes written, 0 at the end, or a
    /// negative error. This is the forward-crossing half of the family, one call per chunk.
    pub fn ak_bdr_drain(
        ctx: *mut ak_dec_ctx,
        dst: *mut u8,
        cap: usize,
        cursor: *mut usize,
    ) -> isize;
    /// The records in place, for a host with no pinning problem. Valid until the next parse.
    pub fn ak_bdr_ptr(ctx: *mut ak_dec_ctx, ptr: *mut *const u8, len: *mut usize) -> i32;
    pub fn ak_bdr_reset(ctx: *mut ak_dec_ctx);
    /// Counting build only: the host records the forward crossings its drain loop made
    /// through `ak_bdr_footprint`, which takes a `const` context and cannot bump a counter.
    /// Same shape and same reason as `ak_enc_count_reverse` (R5: count, do not infer).
    pub fn ak_bdr_count_forward(ctx: *mut ak_dec_ctx, n: u32);
}

// ---- section 9: the RPC half. It moves opaque bytes and dispatches on a path string.

pub enum ak_runtime {}
pub enum ak_client {}

/// A byte range the core owns until the host releases it.
#[repr(C)]
pub struct ak_bytes {
    pub ptr: *const u8,
    pub len: usize,
    /// The core's handle on the allocation. The host passes it back and does not read it.
    pub owner: *mut c_void,
}

impl Default for ak_bytes {
    fn default() -> Self {
        ak_bytes { ptr: core::ptr::null(), len: 0, owner: core::ptr::null_mut() }
    }
}

unsafe extern "C" {
    pub fn ak_runtime_new(worker_threads: u32) -> *mut ak_runtime;
    pub fn ak_runtime_destroy(r: *mut ak_runtime);
    pub fn ak_client_new(r: *mut ak_runtime, uri: *const u8, uri_len: usize) -> *mut ak_client;
    pub fn ak_client_destroy(c: *mut ak_client);
    /// One crossing in.
    pub fn ak_call_unary(
        c: *mut ak_client,
        path: *const u8,
        path_len: usize,
        req: *const u8,
        req_len: usize,
        out: *mut ak_bytes,
    ) -> i32;
    /// The second crossing, and the only other one.
    pub fn ak_bytes_free(b: *mut ak_bytes);
}

/// Boundary-call counts, from the counting build (README R5). Counted in the CORE, so a
/// call that the optimiser removed is not counted, and a count that is right is evidence
/// the arm is running.
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct AkCounters {
    /// Host -> core: every `extern "C"` entry point the host called.
    pub forward: u64,
    /// Core -> host: every function pointer the core invoked (loops, apply, add, transcode).
    pub reverse: u64,
    /// Of `reverse`, the transcoder invocations.
    pub transcode: u64,
    /// Length-prefix resizes: a placeholder whose learned width missed (ABI v1 section 6).
    pub prefix_moves: u64,
    /// Bytes memmoved by those resizes.
    pub prefix_bytes: u64,
    /// Transcoder growth-callback invocations (ABI v1 open decision 5).
    pub grows: u64,
}
