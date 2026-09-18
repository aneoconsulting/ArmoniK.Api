//! The core, behind the C ABI.
//!
//! A separate crate on purpose. Every entry point here is a `#[no_mangle] extern "C"`
//! function that is neither generic nor `#[inline]`, so with cross-crate LTO off rustc has
//! no MIR to inline and the host's call is a real call through the symbol -- which is what
//! makes `core-ffi-rust` a measurement of the boundary rather than of the optimiser. The
//! harness measures a no-op crossing in the same build to show that the boundary is there.
#![allow(non_camel_case_types, non_upper_case_globals)]

use ak_abi::*;
use ak_rt::Enc;
use core::ffi::c_void;

pub mod generated {
    pub mod codec;
}

/// Referenced by the host so the linker keeps this crate's objects. A host that links the
/// codec and calls nothing would otherwise get its symbols garbage-collected.
pub fn link_anchor() -> u32 {
    AK_ABI_VERSION
}

// ---- contexts ---------------------------------------------------------------------

pub struct EncCtxImpl {
    pub e: Enc,
    /// Which repeated field is open, so the element entry point does not have to be told
    /// its own tag: the codec drives, and it knows the field (ABI v1 rule 2). Read into
    /// locals at the top of a run, because encoding a non-leaf element makes reverse calls
    /// that open fields of their own.
    pub open_tag: u32,
    pub open_site: u32,
    /// The wire encoding of a packed field comes from the schema and lives here, so `bool`
    /// and `enum` need no symbols of their own (ABI v1 section 6).
    pub open_kind: u32,
    /// The element vtable and the root object, for a non-leaf element run: the codec passes
    /// the object pointer back to the host and never dereferences it.
    pub open_vt: *const c_void,
    pub open_obj: *const c_void,
}

pub struct DecCtxImpl {
    pub err: i32,
    pub c: ak_rt::Counters,
}

#[no_mangle]
pub extern "C" fn ak_abi_version() -> u32 {
    AK_ABI_VERSION
}

#[no_mangle]
pub extern "C" fn ak_enc_ctx_new() -> *mut ak_enc_ctx {
    let b = Box::new(EncCtxImpl {
        e: Enc::new(generated::codec::SITES),
        open_tag: 0,
        open_site: 0,
        open_kind: 0,
        open_vt: core::ptr::null(),
        open_obj: core::ptr::null(),
    });
    Box::into_raw(b) as *mut ak_enc_ctx
}

#[no_mangle]
pub unsafe extern "C" fn ak_enc_ctx_free(ctx: *mut ak_enc_ctx) {
    if !ctx.is_null() {
        drop(Box::from_raw(ctx as *mut EncCtxImpl));
    }
}

#[no_mangle]
pub unsafe extern "C" fn ak_enc_reset(ctx: *mut ak_enc_ctx) {
    (*(ctx as *mut EncCtxImpl)).e.reset();
}

#[no_mangle]
pub unsafe extern "C" fn ak_enc_take(
    ctx: *mut ak_enc_ctx,
    ptr: *mut *const u8,
    len: *mut usize,
) -> i32 {
    let cx = &mut *(ctx as *mut EncCtxImpl);
    *ptr = cx.e.buf.as_ptr();
    *len = cx.e.buf.len();
    cx.e.err
}

#[no_mangle]
pub unsafe extern "C" fn ak_enc_err(ctx: *const ak_enc_ctx) -> i32 {
    (*(ctx as *const EncCtxImpl)).e.err
}

#[no_mangle]
pub extern "C" fn ak_dec_ctx_new() -> *mut ak_dec_ctx {
    Box::into_raw(Box::new(DecCtxImpl { err: 0, c: Default::default() })) as *mut ak_dec_ctx
}

#[no_mangle]
pub unsafe extern "C" fn ak_dec_ctx_free(ctx: *mut ak_dec_ctx) {
    if !ctx.is_null() {
        drop(Box::from_raw(ctx as *mut DecCtxImpl));
    }
}

/// Sticky: the first error wins, so unwinding cannot overwrite the cause. Never allocates,
/// never throws, safe from inside a reverse-call frame (ABI v1 section 5). The message is
/// dropped in this slice; nothing here reports one.
#[no_mangle]
pub unsafe extern "C" fn ak_fail(ctx: *mut c_void, code: i32, _msg: *const u8, _msg_len: u32) {
    // Both contexts start with their error slot reachable through this one entry point;
    // the harness only ever hands an encode context, which is what the loop callbacks get.
    let cx = &mut *(ctx as *mut EncCtxImpl);
    cx.e.fail(code);
}

// ---- counters ---------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn ak_enc_counters(ctx: *const ak_enc_ctx, out: *mut AkCounters) {
    let c = (*(ctx as *const EncCtxImpl)).e.c;
    *out = AkCounters {
        forward: c.forward,
        reverse: c.reverse,
        transcode: c.transcode,
        prefix_moves: c.prefix_moves,
        prefix_bytes: c.prefix_bytes,
        grows: c.grows,
    };
}

#[no_mangle]
pub unsafe extern "C" fn ak_enc_counters_reset(ctx: *mut ak_enc_ctx) {
    (*(ctx as *mut EncCtxImpl)).e.c = Default::default();
}

/// Counting build only: the per-site length-prefix miss counts, so decision 5's figure can
/// name the field it comes from. Writes at most `cap` entries and returns how many sites
/// exist.
#[no_mangle]
pub unsafe extern "C" fn ak_enc_site_moves(ctx: *const ak_enc_ctx, out: *mut u32, cap: usize) -> usize {
    #[cfg(feature = "count")]
    {
        let m = &(*(ctx as *const EncCtxImpl)).e.site_moves;
        let n = m.len().min(cap);
        core::ptr::copy_nonoverlapping(m.as_ptr(), out, n);
        return m.len();
    }
    #[allow(unreachable_code)]
    {
        let _ = (ctx, out, cap);
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn ak_dec_counters(ctx: *const ak_dec_ctx, out: *mut AkCounters) {
    let c = (*(ctx as *const DecCtxImpl)).c;
    *out = AkCounters {
        forward: c.forward,
        reverse: c.reverse,
        transcode: c.transcode,
        prefix_moves: c.prefix_moves,
        prefix_bytes: c.prefix_bytes,
        grows: c.grows,
    };
}

#[no_mangle]
pub unsafe extern "C" fn ak_dec_counters_reset(ctx: *mut ak_dec_ctx) {
    (*(ctx as *mut DecCtxImpl)).c = Default::default();
}

// ---- transcoders ------------------------------------------------------------------

/// The growth callback of ABI v1 section 4. May move the buffer, which is why it writes
/// back through pointers.
unsafe extern "C" fn ak_grow(sink: *mut c_void, want: i32, dst: *mut *mut u8, cap: *mut i32) -> i32 {
    let cx = &mut *(sink as *mut EncCtxImpl);
    let (p, c) = cx.e.grow(want);
    *dst = p;
    *cap = c;
    AK_OK
}

/// UTF-8 passthrough, validate-and-fail. ABI v1 open decision 3 is whether this or
/// substitution is the rule; the Java slice chose fail, so that is what this is.
unsafe extern "C" fn tc_utf8(
    src: *const c_void,
    len: usize,
    mut dst: *mut u8,
    mut cap: i32,
    grow: ak_grow_fn,
    sink: *mut c_void,
) -> i32 {
    let s = core::slice::from_raw_parts(src as *const u8, len);
    if core::str::from_utf8(s).is_err() {
        return AK_ERR_TRANSCODE;
    }
    if (len as i64) > cap as i64 {
        let rc = grow(sink, len as i32, &mut dst, &mut cap);
        if rc < 0 {
            return rc;
        }
        if (len as i64) > cap as i64 {
            return AK_ERR_CAPACITY;
        }
    }
    core::ptr::copy_nonoverlapping(s.as_ptr(), dst, len);
    len as i32
}

/// UTF-8 passthrough with no validation, for a host whose own type carries the invariant.
/// A Rust `String` is UTF-8 by construction, so validating it is the core re-checking what
/// the host's type system already proved. The two are measured separately rather than
/// assumed equal.
unsafe extern "C" fn tc_utf8_trusted(
    src: *const c_void,
    len: usize,
    mut dst: *mut u8,
    mut cap: i32,
    grow: ak_grow_fn,
    sink: *mut c_void,
) -> i32 {
    if (len as i64) > cap as i64 {
        let rc = grow(sink, len as i32, &mut dst, &mut cap);
        if rc < 0 {
            return rc;
        }
        if (len as i64) > cap as i64 {
            return AK_ERR_CAPACITY;
        }
    }
    core::ptr::copy_nonoverlapping(src as *const u8, dst, len);
    len as i32
}

/// The same contract as `tc_utf8` -- validate, and refuse malformed input -- with a SIMD
/// validator instead of the standard library's scalar one.
///
/// This exists to separate two questions the content-set pass showed were being asked as
/// one. `core::str::from_utf8` has an ASCII fast path that consumes a `usize` at a time and
/// a byte-at-a-time DFA for everything else, so its cost is not proportional to bytes: it
/// is proportional to NON-ASCII bytes with a much worse constant. Measured, that is the
/// difference between validation costing 25 to 30 percent of an encode on ASCII and costing
/// more than the whole encode on latin1 or above U+00FF. If a SIMD validator closes that,
/// then ABI v1 open decision 3 is not "validate or trust the host" but "which validator",
/// and the correctness contract does not have to be given up to get the speed.
unsafe extern "C" fn tc_utf8_simd(
    src: *const c_void,
    len: usize,
    mut dst: *mut u8,
    mut cap: i32,
    grow: ak_grow_fn,
    sink: *mut c_void,
) -> i32 {
    let s = core::slice::from_raw_parts(src as *const u8, len);
    if simdutf8::basic::from_utf8(s).is_err() {
        return AK_ERR_TRANSCODE;
    }
    if (len as i64) > cap as i64 {
        let rc = grow(sink, len as i32, &mut dst, &mut cap);
        if rc < 0 {
            return rc;
        }
        if (len as i64) > cap as i64 {
            return AK_ERR_CAPACITY;
        }
    }
    core::ptr::copy_nonoverlapping(s.as_ptr(), dst, len);
    len as i32
}

#[no_mangle]
pub extern "C" fn ak_tc_utf8() -> ak_transcode_fn {
    tc_utf8
}

#[no_mangle]
pub extern "C" fn ak_tc_utf8_simd() -> ak_transcode_fn {
    tc_utf8_simd
}
#[no_mangle]
pub extern "C" fn ak_tc_utf8_trusted() -> ak_transcode_fn {
    tc_utf8_trusted
}
#[no_mangle]
pub extern "C" fn ak_tc_bytes() -> ak_transcode_fn {
    tc_utf8_trusted
}

/// One length-delimited blob, reached through its transcoder.
///
/// The prefix is opened BEFORE the transcode and resolved after it, because the core does
/// not know how many bytes the host's representation will produce -- there is no declared
/// expansion bound any more (ABI v1 section 4). That is what makes the learned width of
/// section 6 load-bearing rather than a tuning knob, and it is what ABI v1 open decision 5
/// asks the price of.
#[inline]
pub(crate) unsafe fn enc_blob(cx: *mut EncCtxImpl, tag: u32, site: u32, s: &ak_str) -> bool {
    let tc = match s.tc {
        Some(tc) => tc,
        None => return true,
    };
    let e = &mut (*cx).e;
    let mk = e.begin(tag, site);
    let (dst, cap) = e.space();
    ak_rt::bump!(e.c, transcode);
    let n = tc(s.data, s.len, dst, cap, ak_grow, cx as *mut c_void);
    let e = &mut (*cx).e;
    if n < 0 {
        e.fail(n);
        return false;
    }
    if !e.commit(n as usize) {
        return false;
    }
    e.end(mk);
    true
}

/// A crossing and nothing else, so the harness can price the boundary itself in the same
/// process and the same build as the arms. If this measures zero, the `core-ffi-rust` arm
/// is not crossing anything and every figure taken from it is a figure about the
/// optimiser: "when a change does not do what it should, the first hypothesis is that it
/// is not running."
#[no_mangle]
pub extern "C" fn ak_noop(x: u64) -> u64 {
    x ^ 1
}

/// The same crossing with a reverse call in it: the core calls back through a function
/// pointer the host supplied. That is what a loop callback, an `apply` and an `add` cost
/// before any work is done.
#[no_mangle]
pub unsafe extern "C" fn ak_noop_reverse(f: unsafe extern "C" fn(u64) -> u64, x: u64) -> u64 {
    f(x)
}
