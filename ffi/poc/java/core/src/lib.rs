//! The core, behind the C ABI. **java slice copy.**
//!
//! The rust slice's `crates/ak-core/src/lib.rs` (by way of the cpp slice's copy of it),
//! with the `rpc` module dropped and **two transcoders added**. It is hand-written runtime
//! support, not a codec, so R1 is satisfied by `generated/codec.rs` being emitted by the
//! SAME emitter the rust and cpp slices use (`ffi/poc/rust/gen/rust_abi.py`, imported
//! read-only by `gen/generate.py`). Three hosts, one core.
//!
//! **What this slice had to add, and why no slice needed it before.** ABI v1 section 4
//! specifies five transcoders and the core implemented two of them: `ak_tc_utf8` and
//! `ak_tc_bytes`, which are the same memcpy once decision 3 settled. Both are enough for a
//! host whose string representation is already UTF-8, which C++ (`std::string`) and Rust
//! (`String`) both are. **A JVM host holds UTF-16**, so `ak_tc_utf16` and `ak_tc_latin1`
//! are the entries the specification wrote for it and the first slice to reach them is
//! this one. They are the whole of "every managed host stops maintaining a UTF-8 encoder"
//! (section 4's provenance row), so a Java slice that skipped them would be measuring a
//! host transcoder and calling it the ABI.
//!
//! A separate crate on purpose. Every entry point here is a `#[no_mangle] extern "C"`
//! function that is neither generic nor `#[inline]`. The managed hosts cannot be inlined
//! into across the boundary at all (findings/rust.md), which makes the JVM the one host
//! R5's first hazard cannot reach; the harness still measures a no-op crossing in the same
//! build, because "an arm is not what its name says until the artifact agrees".
#![allow(non_camel_case_types, non_upper_case_globals)]

use ak_abi::*;
use ak_rt::Enc;
use core::ffi::c_void;

pub mod generated {
    pub mod codec;
    /// ABI v1 section 10: the core's own view of every group layout, exported so the
    /// host can compare it with what ITS compiler produced. The rust slice could not
    /// exercise this (both sides compiled against one header); here the two sides
    /// genuinely restate the layout, which is the case section 10 exists for.
    pub mod layout;
}

/// Referenced by the host so the linker keeps this crate's objects. A host that links the
/// codec and calls nothing would otherwise get its symbols garbage-collected.
pub fn link_anchor() -> u32 {
    AK_ABI_VERSION
}

// ---- contexts ---------------------------------------------------------------------

/// The first field of every context, so `ak_fail` can be the one entry point ABI v1
/// section 5 says it is -- "any host code holding a context may fail the operation" --
/// without knowing which kind of context it was handed.
///
/// Before this, `ak_fail` cast unconditionally to `EncCtxImpl` (defect D7). It was not
/// reachable while nothing on the decode path could fail, and M3 is where that stopped
/// being true: an accessor for a oneof member can be handed a case the host does not know.
#[repr(C)]
pub struct CtxHeader {
    /// AK_CTX_ENC or AK_CTX_DEC. Read by nothing in the fast path; it exists so a wrong
    /// pointer is a diagnosable bug rather than a silent one.
    pub kind: u32,
    /// Sticky: the first error wins, so unwinding cannot overwrite the cause.
    pub err: i32,
}

pub const AK_CTX_ENC: u32 = 0x41_4B_45_43;
pub const AK_CTX_DEC: u32 = 0x41_4B_44_43;

#[repr(C)]
pub struct EncCtxImpl {
    pub hdr: CtxHeader,
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
    /// The direct argument of the call (ABI v1 section 8), if this message tree has one.
    pub direct: *const u8,
    pub direct_len: usize,
}

#[repr(C)]
pub struct DecCtxImpl {
    pub hdr: CtxHeader,
    pub c: ak_rt::Counters,
}

#[no_mangle]
pub extern "C" fn ak_abi_version() -> u32 {
    AK_ABI_VERSION
}

#[no_mangle]
pub extern "C" fn ak_enc_ctx_new() -> *mut ak_enc_ctx {
    let b = Box::new(EncCtxImpl {
        hdr: CtxHeader { kind: AK_CTX_ENC, err: AK_OK },
        e: Enc::new(generated::codec::SITES),
        open_tag: 0,
        open_site: 0,
        open_kind: 0,
        open_vt: core::ptr::null(),
        open_obj: core::ptr::null(),
        direct: core::ptr::null(),
        direct_len: 0,
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
    let cx = &mut *(ctx as *mut EncCtxImpl);
    cx.e.reset();
    cx.hdr.err = AK_OK;
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
    let cx = &*(ctx as *const EncCtxImpl);
    if cx.hdr.err != AK_OK {
        cx.hdr.err
    } else {
        cx.e.err
    }
}

#[no_mangle]
pub extern "C" fn ak_dec_ctx_new() -> *mut ak_dec_ctx {
    Box::into_raw(Box::new(DecCtxImpl {
        hdr: CtxHeader { kind: AK_CTX_DEC, err: AK_OK },
        c: Default::default(),
    })) as *mut ak_dec_ctx
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
///
/// It takes `void*` because it takes EITHER context. Both begin with a `CtxHeader`, so the
/// error slot is at one offset and this does not have to know which it was given.
#[no_mangle]
pub unsafe extern "C" fn ak_fail(ctx: *mut c_void, code: i32, _msg: *const u8, _msg_len: u32) {
    if ctx.is_null() || code >= 0 {
        return;
    }
    let h = &mut *(ctx as *mut CtxHeader);
    if h.err == AK_OK {
        h.err = code;
    }
}

/// What the host reported through `ak_fail` on a decode context, or AK_OK.
#[no_mangle]
pub unsafe extern "C" fn ak_dec_err(ctx: *const ak_dec_ctx) -> i32 {
    (*(ctx as *const DecCtxImpl)).hdr.err
}

#[no_mangle]
pub unsafe extern "C" fn ak_dec_err_reset(ctx: *mut ak_dec_ctx) {
    (*(ctx as *mut DecCtxImpl)).hdr.err = AK_OK;
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

/// Counting build only: let the HOST report a reverse crossing the core cannot see.
///
/// `ak_str.tc` may point at a function in the host image or at one of the core's own, and
/// the counter lives in the core, so the two are indistinguishable from inside. Without
/// this the host-transcoder arm's 17,167 reverse crossings on a P2.2 encode were an
/// argument, and README R5 says count, do not infer. The host calls this from inside its
/// own transcoder, and only in the counting build: it is one more crossing, which is
/// exactly why the timed build must not have it.
#[no_mangle]
pub unsafe extern "C" fn ak_enc_count_reverse(ctx: *mut ak_enc_ctx) {
    #[cfg(feature = "count")]
    {
        let cx = &mut *(ctx as *mut EncCtxImpl);
        cx.e.c.reverse += 1;
    }
    #[cfg(not(feature = "count"))]
    {
        let _ = ctx;
    }
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

// ---- the converting transcoders ABI v1 section 4 specifies for a managed host -------
//
// `ak_tc_utf8` and `ak_tc_bytes` above are a memcpy: a host whose representation is
// already UTF-8 hands over its own bytes. A JVM host does not have one. A `java.lang.
// String` is UTF-16 code units, or on JDK 9 and later a LATIN1 byte per character when
// every character fits, and section 4's whole argument is that the CORE converts so that
// "every managed host stops maintaining a UTF-8 encoder".
//
// `len` counts SOURCE CODE UNITS in both, which is what section 4 says and what makes the
// two entries one contract rather than two: chars for UTF-16, bytes for Latin-1.
//
// Malformed input: an unpaired surrogate is not representable in UTF-8, and ABI v1
// section 7 fixes the answer for a CONVERTING transcoder -- U+FFFD, in both halves of the
// transcode pair. protobuf-java writes `?` for the same input, today, silently. That
// divergence is a migration note rather than a defect (section 7 says so) and it is
// unreachable from the manifest's content sets, all three of which are well formed.

/// How many UTF-8 bytes `n` UTF-16 code units at `src` will produce.
///
/// A pre-pass rather than a guess. The alternative is to write and then discover the
/// buffer was short, and section 4 removed the declared expansion bound precisely because
/// an under-declared one was measured as *silent wire corruption*. Counting is one pass
/// over data that is about to be read again from L1.
#[inline]
unsafe fn utf16_utf8_len(src: *const u16, n: usize) -> usize {
    let s = core::slice::from_raw_parts(src, n);
    let mut out = 0usize;
    let mut i = 0usize;
    while i < n {
        let c = s[i] as u32;
        if c < 0x80 {
            out += 1;
        } else if c < 0x800 {
            out += 2;
        } else if (0xD800..0xDC00).contains(&c)
            && i + 1 < n
            && (0xDC00..0xE000).contains(&(s[i + 1] as u32))
        {
            out += 4;
            i += 1;
        } else {
            // A lone surrogate becomes U+FFFD, which is three bytes, so the count is
            // right for the substitution the write below performs.
            out += 3;
        }
        i += 1;
    }
    out
}

#[inline]
unsafe fn write_cp(dst: *mut u8, at: usize, cp: u32) -> usize {
    if cp < 0x80 {
        *dst.add(at) = cp as u8;
        1
    } else if cp < 0x800 {
        *dst.add(at) = 0xC0 | (cp >> 6) as u8;
        *dst.add(at + 1) = 0x80 | (cp & 0x3F) as u8;
        2
    } else if cp < 0x10000 {
        *dst.add(at) = 0xE0 | (cp >> 12) as u8;
        *dst.add(at + 1) = 0x80 | ((cp >> 6) & 0x3F) as u8;
        *dst.add(at + 2) = 0x80 | (cp & 0x3F) as u8;
        3
    } else {
        *dst.add(at) = 0xF0 | (cp >> 18) as u8;
        *dst.add(at + 1) = 0x80 | ((cp >> 12) & 0x3F) as u8;
        *dst.add(at + 2) = 0x80 | ((cp >> 6) & 0x3F) as u8;
        *dst.add(at + 3) = 0x80 | (cp & 0x3F) as u8;
        4
    }
}

/// UTF-16 code units in, UTF-8 out. `len` is the number of `u16`s.
unsafe extern "C" fn tc_utf16(
    src: *const c_void,
    len: usize,
    mut dst: *mut u8,
    mut cap: i32,
    grow: ak_grow_fn,
    sink: *mut c_void,
) -> i32 {
    let p = src as *const u16;
    let need = utf16_utf8_len(p, len);
    if need as i64 > cap as i64 {
        let rc = grow(sink, need as i32, &mut dst, &mut cap);
        if rc < 0 {
            return rc;
        }
        if need as i64 > cap as i64 {
            return AK_ERR_CAPACITY;
        }
    }
    let s = core::slice::from_raw_parts(p, len);
    let mut at = 0usize;
    let mut i = 0usize;
    while i < len {
        let c = s[i] as u32;
        let cp = if (0xD800..0xDC00).contains(&c) {
            if i + 1 < len && (0xDC00..0xE000).contains(&(s[i + 1] as u32)) {
                let lo = s[i + 1] as u32;
                i += 1;
                0x10000 + ((c - 0xD800) << 10) + (lo - 0xDC00)
            } else {
                0xFFFD
            }
        } else if (0xDC00..0xE000).contains(&c) {
            0xFFFD
        } else {
            c
        };
        at += write_cp(dst, at, cp);
        i += 1;
    }
    at as i32
}

/// Latin-1 bytes in, UTF-8 out: the JVM's compact string form, and CPython's 1-byte kind.
///
/// Every input byte is a code point below U+0100, so there is nothing to validate and
/// nothing that can fail to be representable; the length is exact without a pre-pass,
/// because a byte below 0x80 makes one UTF-8 byte and everything else makes two.
unsafe extern "C" fn tc_latin1(
    src: *const c_void,
    len: usize,
    mut dst: *mut u8,
    mut cap: i32,
    grow: ak_grow_fn,
    sink: *mut c_void,
) -> i32 {
    let s = core::slice::from_raw_parts(src as *const u8, len);
    let mut need = len;
    for &b in s {
        if b >= 0x80 {
            need += 1;
        }
    }
    if need as i64 > cap as i64 {
        let rc = grow(sink, need as i32, &mut dst, &mut cap);
        if rc < 0 {
            return rc;
        }
        if need as i64 > cap as i64 {
            return AK_ERR_CAPACITY;
        }
    }
    let mut at = 0usize;
    for &b in s {
        if b < 0x80 {
            *dst.add(at) = b;
            at += 1;
        } else {
            *dst.add(at) = 0xC0 | (b >> 6);
            *dst.add(at + 1) = 0x80 | (b & 0x3F);
            at += 2;
        }
    }
    at as i32
}

#[no_mangle]
pub extern "C" fn ak_tc_utf16() -> ak_transcode_fn {
    tc_utf16
}

#[no_mangle]
pub extern "C" fn ak_tc_latin1() -> ak_transcode_fn {
    tc_latin1
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
    // The direct-argument path: the bytes are an argument of the call rather than a pointer
    // into staging, so there is no transcoder to invoke and nothing to validate.
    if s.data == ak_abi::AK_STR_DIRECT {
        let (p, n) = ((*cx).direct, (*cx).direct_len);
        let e = &mut (*cx).e;
        e.key(tag, ak_rt::WIRE_LEN);
        e.varint(n as u64);
        e.buf.extend_from_slice(core::slice::from_raw_parts(p, n));
        let _ = site;
        return true;
    }
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

/// ABI v1 open decision 11 candidate: the decode side's capture buffer.
///
/// Unknown runs are collected as spans into the buffer the host handed in -- the core
/// copies nothing and allocates nothing, which is the property the whole decode design
/// rests on -- and delivered in batches, the way an element run is, so the cost is
/// crossings per chunk and not per field. The host materialises them if it intends to
/// re-encode, because that buffer may be recycled.
///
/// `cb == None` is today's behaviour and the case that has to stay free: an unknown tag is
/// skipped and dropped, and the only thing the capture costs is one null test on a branch
/// the existing payload set never takes.
pub(crate) struct UnkBuf {
    pub cb: Option<ak_unk_f>,
    pub ctx: *mut ak_dec_ctx,
    pub obj: *mut c_void,
    /// Which element of the enclosing run the spans being collected belong to.
    pub token: i64,
    pub spans: [ak_uspan; UNK_CHUNK],
    pub n: usize,
}

pub(crate) const UNK_CHUNK: usize = 32;

impl UnkBuf {
    #[inline]
    pub fn new(cb: Option<ak_unk_f>, ctx: *mut ak_dec_ctx, obj: *mut c_void) -> Self {
        UnkBuf {
            cb,
            ctx,
            obj,
            token: AK_TOKEN_ROOT,
            spans: [ak_uspan { token: 0, off: 0, len: 0 }; UNK_CHUNK],
            n: 0,
        }
    }

    #[inline(always)]
    pub unsafe fn push(&mut self, off: usize, len: usize) {
        if self.cb.is_none() {
            return;
        }
        self.spans[self.n] = ak_uspan { token: self.token, off: off as u32, len: len as u32 };
        self.n += 1;
        if self.n == UNK_CHUNK {
            self.flush();
        }
    }

    #[inline]
    pub unsafe fn flush(&mut self) {
        if self.n == 0 {
            return;
        }
        if let Some(cb) = self.cb {
            ak_rt::bump!((*(self.ctx as *mut DecCtxImpl)).c, reverse);
            cb(self.ctx, self.obj, self.spans.as_ptr(), self.n as i32);
        }
        self.n = 0;
    }
}

/// ABI v1 open decision 11 candidate: the unknown-field bag, appended verbatim.
///
/// The bytes ALREADY carry their own tags and lengths -- they are the raw runs a decoder
/// captured and could not name -- so this is `enc_blob` with the key and the length prefix
/// removed, and nothing else. Empty is `tc == NULL`, the existing absent convention, so a
/// message with no unknown fields pays one null test and writes nothing.
///
/// It does NOT go through a transcoder, and the slot is two words rather than three.
/// Every other blob slot carries a transcoder because the host's representation may differ
/// from the wire's; the bag's cannot, because it IS wire bytes the decoder captured. With
/// decision 3 settled, `ak_tc_bytes` and `ak_tc_utf8_trusted` are already the same memcpy,
/// so a transcoder here would be a pointer whose only legal value is the identity -- dead
/// weight on every group of every message, and an invitation to set it wrong.
#[inline]
pub(crate) unsafe fn enc_raw(cx: *mut EncCtxImpl, s: &ak_blob) -> bool {
    if s.len == 0 {
        return true;
    }
    let e = &mut (*cx).e;
    e.buf
        .extend_from_slice(core::slice::from_raw_parts(s.data as *const u8, s.len));
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
