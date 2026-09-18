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

    /// The transcoders the core implements once for every language (section 4).
    pub fn ak_tc_utf8() -> ak_transcode_fn;
    /// UTF-8 passthrough with no validation, for a host whose type already carries the
    /// invariant. ABI v1 open decision 3 is about which of these `ak_tc_utf8` should be.
    pub fn ak_tc_utf8_trusted() -> ak_transcode_fn;
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
