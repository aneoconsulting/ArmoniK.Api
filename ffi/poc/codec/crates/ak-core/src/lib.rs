//! The core, behind the C ABI. **One copy, shared by every slice (README R0).**
//!
//! This file was three files. `crates/ak-core/src/lib.rs` in the rust slice, `core/src/
//! lib.rs` in the cpp slice and `core/src/lib.rs` in the java slice were the same
//! hand-written runtime with three different sets of additions, and the emitted
//! `generated/codec.rs` beside them was byte-identical in all three -- so the *generator*
//! was genuinely shared (R1) and the *runtime* had forked three ways. Neither addition was
//! wrong and neither broke a measurement. The mechanism is the finding: **each fork
//! happened because a slice needed to add something and the shared core had nowhere to
//! accept a contribution**, which is the five-implementations problem this branch exists to
//! argue about, reproduced inside the branch. W10 folded them back; R0 is the rule that
//! keeps them folded, and `gen/one_core.sh` is the test that the rule fails when broken.
//!
//! What each slice contributed, all of it additive and all of it kept exactly as its
//! author wrote it:
//!
//!   * the cpp slice -- `ak_enc_count_reverse`, so a reverse crossing the core cannot see
//!     is counted rather than inferred (R5);
//!   * the java slice -- `ak_tc_utf16` and `ak_tc_latin1`, the two converting transcoders
//!     ABI v1 section 4 specifies for a host that does not already hold UTF-8;
//!   * the cpp and java slices together -- `generated::layout`, section 10's run-time
//!     layout export, which the rust slice could not exercise because both of its sides
//!     compile against one generated header.
//!
//! Two things are behind features rather than unconditional, and for measurement reasons
//! rather than taste. `rpc` (ABI v1 section 9) pulls tonic and tokio into the shared
//! object, so a codec arm must not link it; the rust slice turns it on because its stage-4
//! arm is the RPC one. `count` (R5) puts the counters in the contexts, and a counting build
//! is never a timed build.
//!
//! A separate crate on purpose. Every entry point here is a `#[no_mangle] extern "C"`
//! function that is neither generic nor `#[inline]`, so with cross-crate LTO off rustc has
//! no MIR to inline and the host's call is a real call through the symbol -- which is what
//! makes a `core-ffi` arm a measurement of the boundary rather than of the optimiser. Each
//! host's harness measures a no-op crossing in the same build to show the boundary is
//! there. The one host this cannot reach is a managed one: a JVM or a CLR cannot be inlined
//! into across the boundary at all (findings/rust.md).
#![allow(non_camel_case_types, non_upper_case_globals)]

extern crate alloc;

use ak_abi::*;
use ak_rt::Enc;
use core::ffi::c_void;

/// ABI v1 section 9, behind a feature so the codec arms do not link tonic.
#[cfg(feature = "rpc")]
pub mod rpc;

pub mod generated {
    /// `corpus` (test-only): the codec generated for the conformance corpus's reader
    /// schema, from the same plan layer and the same backend (FIX-PLAN WP5 item 6.1).
    #[cfg_attr(feature = "corpus", path = "../generated_corpus/codec.rs")]
    pub mod codec;
    /// R-G5: the core's RPC definitions checked against the ONE declaration (`plan.rpc`).
    #[cfg(feature = "rpc")]
    pub mod rpc_check;
    /// WP5 step 6 (R-G13): the core's fixed entry points checked against `plan.FIXED`.
    pub mod abi_check;
    /// ABI v1 section 10: the core's own view of every group layout, exported so the
    /// host can compare it with what ITS compiler produced. The rust slice could not
    /// exercise this (both sides compiled against one header); here the two sides
    /// genuinely restate the layout, which is the case section 10 exists for.
    #[cfg_attr(feature = "corpus", path = "../generated_corpus/layout.rs")]
    pub mod layout;
}

/// Referenced by the host so the linker keeps this crate's objects. A host that links the
/// codec and calls nothing would otherwise get its symbols garbage-collected.
pub fn link_anchor() -> u32 {
    AK_ABI_VERSION
}

// ---- section 3: the lifecycle -------------------------------------------------------
//
// "Nothing here is implicit."  Built by the rust slice, which found that every slice in the
// branch had skipped it: the codec half needs none of it, so nobody built it, so section
// 3's central claim -- "every other entry point requires `ak_init` to have returned
// successfully, the codec included" -- had never been exercised anywhere.
//
// WHAT IS REAL HERE AND WHAT IS NOT, stated rather than left to be discovered.  The state
// machine, the idempotence rules, the ABI-version check, the build id, the log bridge and
// the panic hook are real and exercised.  The rustls crypto provider is NOT installed,
// because this crate's codec build does not link rustls at all; `AK_INIT_NO_CRYPTO` names
// that case and the RPC feature is where the install would go.  A one-line install nobody
// has run is not evidence, and pretending otherwise is what this branch exists to avoid.

use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

const INIT_NONE: u32 = 0;
const INIT_RUNNING: u32 = 1;
const INIT_DONE: u32 = 2;

static INIT_STATE: AtomicU32 = AtomicU32::new(INIT_NONE);
/// The options the first successful `ak_init` was given, so a second call with DIFFERENT
/// options can be refused without storing a struct behind a lock. Three words, compared
/// field by field: R-D9 found that the earlier single word (`log ^ log_ctx` in its low
/// bits) made two different option sets compare equal whenever sink and context XOR-ed to
/// the same value. They are written before `INIT_STATE` is released as DONE and read only
/// after it is acquired as DONE, so three plain atomics are as consistent as one.
static INIT_VF: AtomicU64 = AtomicU64::new(0);
static INIT_LOG: AtomicU64 = AtomicU64::new(0);
static INIT_LOGCTX: AtomicU64 = AtomicU64::new(0);

/// The host's log sink, installed once. Read on every log line, which is why it is an
/// atomic pair rather than a mutex: a log call must be safe from inside a reverse-call
/// frame, the same constraint section 5 puts on `ak_fail`.
static LOG_FN: AtomicU64 = AtomicU64::new(0);
static LOG_CTX: AtomicU64 = AtomicU64::new(0);

/// Two copies of the staticlib in one process either share Rust's globals or split-brain
/// them with no warning (section 3), so the build carries an id a host can compare.
/// Null-terminated: it is the one string in this ABI that is, because it is a C string
/// constant and not a wire value.
static BUILD_ID: &[u8] = concat!(
    "ak-core ", env!("CARGO_PKG_VERSION"), " abi1 ", env!("CARGO_PKG_NAME"), "\0"
).as_bytes();

/// The options as three words, one per field, so equality is equality of every field and
/// cannot collide (R-D9). The version and the flags share a word without overlapping: 32
/// bits each.
fn opts_words(o: &ak_init_opts) -> (u64, u64, u64) {
    let log = o.log.map(|f| f as usize as u64).unwrap_or(0);
    (((o.abi_version as u64) << 32) | (o.flags as u64), log, o.log_ctx as u64)
}

#[no_mangle]
pub unsafe extern "C" fn ak_init(opts: *const ak_init_opts, err: *mut ak_err) -> i32 {
    let set = |code: i32, detail: u32| -> i32 {
        if !err.is_null() {
            *err = ak_err { code, detail };
        }
        code
    };
    if opts.is_null() {
        return set(AK_ERR_INVALID_STATE, AK_DETAIL_NULL_ARG);
    }
    let o = *opts;

    // The version the HOST was generated against, checked by the side that knows it, once,
    // at the only point where failing is cheap (section 3).
    if o.abi_version != AK_ABI_VERSION {
        return set(AK_ERR_ABI, AK_DETAIL_ABI_MISMATCH);
    }

    let words = opts_words(&o);
    match INIT_STATE.compare_exchange(INIT_NONE, INIT_RUNNING, Ordering::AcqRel, Ordering::Acquire)
    {
        Ok(_) => {}
        Err(_) => {
            // Already initialising or initialised. Spin until the first call has finished,
            // because a second caller must not return before the installs are visible --
            // "every other entry point requires ak_init to have returned successfully" is
            // a claim about ALL callers, not about the first.
            while INIT_STATE.load(Ordering::Acquire) == INIT_RUNNING {
                core::hint::spin_loop();
            }
            let seen = (
                INIT_VF.load(Ordering::Acquire),
                INIT_LOG.load(Ordering::Acquire),
                INIT_LOGCTX.load(Ordering::Acquire),
            );
            return if seen == words {
                set(AK_ALREADY_INITIALIZED, AK_DETAIL_NONE)
            } else {
                // The one-shot installs cannot be redone, so this is a failure and not a
                // no-op. There is no ak_shutdown for the same reason.
                set(AK_ERR_INVALID_STATE, AK_DETAIL_OPTS_DIFFER)
            };
        }
    }

    // ---- the one-shot installs, in the order section 3 gives them.

    // 1. the crypto provider. NOT DONE, and the flag is how a host says so. The codec build
    //    does not link rustls; see the module comment.
    let _ = o.flags & AK_INIT_NO_CRYPTO;

    // 2. the log bridge. `tracing::set_global_default` and `log::set_logger` are one-shot
    //    per process, so who owns them is decided here or not at all.
    if o.flags & AK_INIT_OWN_LOGGING == 0 {
        LOG_FN.store(o.log.map(|f| f as usize as u64).unwrap_or(0), Ordering::Release);
        LOG_CTX.store(o.log_ctx as u64, Ordering::Release);
    }

    // 3. the panic hook. It changes what a panic PRINTS; it does not stop one. A panic
    //    raised inside an `extern "C"` entry point of this ABI still aborts the process,
    //    because the unwind is refused at the boundary -- measured by the rust slice's
    //    concurrency suite, which reaches that path on purpose.
    if o.flags & AK_INIT_NO_PANIC_HOOK == 0 {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let msg = alloc::format!("panic in ak-core: {info}");
            if !ak_log(0, msg.as_ptr(), msg.len()) {
                prev(info);
            }
        }));
    }

    INIT_VF.store(words.0, Ordering::Release);
    INIT_LOG.store(words.1, Ordering::Release);
    INIT_LOGCTX.store(words.2, Ordering::Release);
    INIT_STATE.store(INIT_DONE, Ordering::Release);
    set(AK_OK, AK_DETAIL_NONE)
}

/// Whether `ak_init` has returned successfully. Not in the specification; it is what makes
/// "every other entry point requires `ak_init`" testable from outside the core.
#[no_mangle]
pub extern "C" fn ak_initialized() -> i32 {
    i32::from(INIT_STATE.load(Ordering::Acquire) == INIT_DONE)
}

#[no_mangle]
pub extern "C" fn ak_build_id() -> *const core::ffi::c_char {
    BUILD_ID.as_ptr() as *const core::ffi::c_char
}

/// The guard section 3 requires on every other entry point.
///
/// Behind a feature, and that is a measurement decision rather than a design one: "every
/// entry point requires `ak_init`" is a claim with a price on the hot path (`ak_elem_*` is
/// called once per chunk, `ak_encode_*` once per message), and the price has never been
/// quoted anywhere in this branch. With `init-guard` off the generated entry points are
/// exactly what they were; with it on they check, and the difference is the price.
#[inline(always)]
pub fn ak_init_ok() -> bool {
    // Relaxed: the store that makes it true is Release and happens-before any host call
    // that could observe it, because the host had to see `ak_init` return first. What this
    // load must not do is cost a fence on every encode.
    INIT_STATE.load(Ordering::Relaxed) == INIT_DONE
}

/// Emit a line through the host's sink. Returns false if there is none, so a caller can
/// fall back. Never allocates a sink, never locks, safe from inside a reverse-call frame.
pub fn ak_log(level: u32, msg: *const u8, len: usize) -> bool {
    let f = LOG_FN.load(Ordering::Acquire);
    if f == 0 {
        return false;
    }
    let f: ak_log_fn = unsafe { core::mem::transmute(f as usize) };
    unsafe { f(LOG_CTX.load(Ordering::Acquire) as *mut c_void, level, msg, len) };
    true
}

/// Emit a line, for a host that wants to see the bridge work without waiting for the core
/// to have something to say. Exercises the same path a real log line takes.
#[no_mangle]
pub unsafe extern "C" fn ak_log_test(level: u32, msg: *const u8, len: usize) -> i32 {
    i32::from(ak_log(level, msg, len))
}

/// Panic INSIDE the core, on purpose, so the panic hook can be seen working.
///
/// It exists because the hook cannot be tested any other way, and that is itself the
/// finding: the hook covers a panic raised in the core, and a Rust host's own panics go to
/// the host's hook, because a cdylib carries its own copy of `std` and the two hooks are
/// two different globals. Section 3 warns about exactly this mechanism one level up ("two
/// copies of the staticlib in one process either share Rust's globals or split-brain them
/// with no warning"); here it is `std`'s globals rather than the core's, and the split is
/// not a defect but the reason the hook is worth installing at all.
///
/// The caller does not get control back. This frame is `extern "C"`, so the unwind is
/// refused at the boundary and the process aborts -- the hook's whole value is that the
/// message reaches the host's log FIRST. A guard with no failing test is a guard nobody has
/// seen work (README R1), and this is the failing test.
#[no_mangle]
pub extern "C" fn ak_panic_test() {
    panic!("deliberate panic inside ak-core, from ak_panic_test");
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
    /// The pull family's deposit target (ABI v1 section 7.1). Empty and unallocated unless
    /// a host calls `ak_parse_*`, so the push family pays nothing for it: one `Vec` header
    /// in a context that is allocated once per host thread and reused.
    ///
    /// It is in the CONTEXT and not in a thread-local for section 7.3's reason, and in the
    /// context rather than a local of the entry point because the whole point of pull is
    /// that the host reads it AFTER the call returns.
    pub bdr: ak_rt::Bdr,
    /// Decision 11 rule 6 (WP5 step 8): the root this context is BOUND to
    /// (`plan.unk_root_id`); decoding another root with it is refused.
    pub root: u32,
    /// Rule 1: the host's options struct, read IN PLACE (never copied); NULL = drop mode.
    /// Its first word is the ONE host pointer handed to every grow.
    pub unk_opts: *mut u8,
    /// One entry per position (plan.unk_positions order): a pointer into the host's struct.
    pub unk: Vec<UnkPos>,
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
    enc_status(cx)
}

/// The encode operation's status: the host's sticky report first (ABI v1 section 5,
/// `ak_fail` from inside a reverse call), then the codec's own. R-D6: before this the
/// encode entry points read only `e.err`, so a host that called `ak_fail` and returned
/// AK_OK got a successful encode.
#[inline(always)]
pub(crate) unsafe fn enc_status(cx: *const EncCtxImpl) -> i32 {
    if (*cx).hdr.err != AK_OK {
        (*cx).hdr.err
    } else {
        (*cx).e.err
    }
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

/// Decision 11 rule 6: a decode context exists only BOUND to a root; the generated
/// `ak_dec_ctx_new_<Root>` calls this. There is no untyped `ak_dec_ctx_new`.
pub(crate) fn dec_ctx_alloc(root: u32) -> *mut ak_dec_ctx {
    Box::into_raw(Box::new(DecCtxImpl {
        hdr: CtxHeader { kind: AK_CTX_DEC, err: AK_OK },
        c: Default::default(),
        bdr: ak_rt::Bdr::new(),
        root,
        unk_opts: core::ptr::null_mut(),
        unk: Vec::new(),
    })) as *mut ak_dec_ctx
}

// ---- the pull family's buffer (ABI v1 section 7.1) ---------------------------------
//
// Schema-free, so it is hand-written here beside the contexts rather than emitted: what
// the generator emits is `ak_parse_<Root>`, one per root, and every root deposits into
// this one buffer through the same four entry points.

/// Pre-size the record buffer. Optional: `ak_parse_*` grows it as it goes. A host that has
/// already decoded one response of a shape calls `ak_bdr_footprint` after it and reserves
/// that much before the next, which is the case this exists for.
#[no_mangle]
pub unsafe extern "C" fn ak_bdr_reserve(ctx: *mut ak_dec_ctx, bytes: usize) -> i32 {
    if ctx.is_null() {
        return AK_ERR_INVALID_STATE;
    }
    let cx = &mut *(ctx as *mut DecCtxImpl);
    ak_rt::bump!(cx.c, forward);
    cx.bdr.reserve(bytes);
    AK_OK
}

/// How many bytes the last parse deposited. ABI v1 section 7.1's "the host can hold two
/// decoded responses, read what it is paying, bound it and release it".
#[no_mangle]
pub unsafe extern "C" fn ak_bdr_footprint(ctx: *const ak_dec_ctx) -> usize {
    let cx = &*(ctx as *const DecCtxImpl);
    // A const pointer, so this one cannot bump a counter without a cast the signature is
    // there to forbid. It is a forward crossing and the host counts it: see
    // `ak_bdr_count_forward`.
    cx.bdr.footprint()
}

/// Counting build only: let the HOST record the forward crossings its own drain loop makes
/// through a `const` entry point. Same shape and same reason as `ak_enc_count_reverse`,
/// which the cpp slice added for the mirror case (R5: count, do not infer).
#[no_mangle]
pub unsafe extern "C" fn ak_bdr_count_forward(ctx: *mut ak_dec_ctx, n: u32) {
    #[cfg(feature = "count")]
    {
        let cx = &mut *(ctx as *mut DecCtxImpl);
        cx.c.forward += n as u64;
    }
    #[cfg(not(feature = "count"))]
    {
        let _ = (ctx, n);
    }
}

/// Copy whole records out, from `*cursor`, into a host buffer of `cap` bytes. Returns the
/// bytes written, 0 when the buffer is exhausted, or a negative error code.
///
/// The host drives, which is the family's defining property: no upcall, and on the JVM the
/// destination is a `byte[]` held under `GetPrimitiveArrayCritical`.
#[no_mangle]
pub unsafe extern "C" fn ak_bdr_drain(
    ctx: *mut ak_dec_ctx,
    dst: *mut u8,
    cap: usize,
    cursor: *mut usize,
) -> isize {
    if ctx.is_null() || dst.is_null() || cursor.is_null() {
        return AK_ERR_INVALID_STATE as isize;
    }
    let cx = &mut *(ctx as *mut DecCtxImpl);
    ak_rt::bump!(cx.c, forward);
    let mut at = *cursor;
    let n = cx.bdr.drain(dst, cap, &mut at);
    *cursor = at;
    n
}

/// The buffer in place, for a host with no pinning problem: C++, and Rust.
///
/// It is not a shortcut around `ak_bdr_drain` but the other half of the measurement. The
/// drain copy is what a managed host pays to get the records into memory it can walk
/// without a crossing; a native host walks them where they are. Reporting pull with the
/// copy folded in would price the family against a cost only some of its hosts have.
///
/// The pointer is valid until the next `ak_parse_*` or `ak_dec_ctx_free` on this context.
#[no_mangle]
pub unsafe extern "C" fn ak_bdr_ptr(
    ctx: *mut ak_dec_ctx,
    ptr: *mut *const u8,
    len: *mut usize,
) -> i32 {
    if ctx.is_null() || ptr.is_null() || len.is_null() {
        return AK_ERR_INVALID_STATE;
    }
    let cx = &mut *(ctx as *mut DecCtxImpl);
    ak_rt::bump!(cx.c, forward);
    let b = cx.bdr.as_bytes();
    *ptr = b.as_ptr();
    *len = b.len();
    AK_OK
}

/// Drop the records, keep the allocation. A parse resets on entry, so this is for a host
/// that wants the memory back between responses rather than for correctness.
#[no_mangle]
pub unsafe extern "C" fn ak_bdr_reset(ctx: *mut ak_dec_ctx) {
    let cx = &mut *(ctx as *mut DecCtxImpl);
    cx.bdr.reset();
}

/// ABI v1 section 10, applied to the record header: the core writes `ak_rt::bdr::Rec` and
/// the host reads `ak_abi::ak_bdr_rec`, and the two are separate declarations in crates
/// that do not depend on each other. That is exactly the disagreement section 10 exists
/// for -- a wrong value in a field, the worst way to find it -- so it is asserted at
/// compile time here, where both are visible, rather than trusted.
const _: () = {
    assert!(core::mem::size_of::<ak_rt::bdr::Rec>() == core::mem::size_of::<ak_bdr_rec>());
    assert!(core::mem::align_of::<ak_rt::bdr::Rec>() == core::mem::align_of::<ak_bdr_rec>());
    assert!(core::mem::size_of::<ak_rt::bdr::Rec>() == 24);
    // 8-aligned, so a group written straight after a header needs no per-record padding.
    assert!(core::mem::align_of::<ak_rt::bdr::Rec>() == 8);
    assert!(ak_rt::bdr::OP_APPLY == AK_BDR_APPLY);
    assert!(ak_rt::bdr::OP_ADD == AK_BDR_ADD);
    assert!(ak_rt::bdr::OP_NEW == AK_BDR_NEW);
    assert!(ak_rt::bdr::OP_APPLY_ELEM == AK_BDR_APPLY_ELEM);
    assert!(ak_rt::bdr::BDR_MIN_CHUNK == AK_BDR_MIN_CHUNK);
};

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
    // R-D9: an empty host string may arrive as (NULL, 0), and `slice::from_raw_parts` /
    // `copy_nonoverlapping` require a non-null pointer even for zero bytes.
    if len == 0 {
        return 0;
    }
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
    // R-D9: an empty host string may arrive as (NULL, 0), and `slice::from_raw_parts` /
    // `copy_nonoverlapping` require a non-null pointer even for zero bytes.
    if len == 0 {
        return 0;
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
    // R-D9: an empty host string may arrive as (NULL, 0), and `slice::from_raw_parts` /
    // `copy_nonoverlapping` require a non-null pointer even for zero bytes.
    if len == 0 {
        return 0;
    }
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
    // R-D9: an empty host string may arrive as (NULL, 0), and `slice::from_raw_parts` /
    // `copy_nonoverlapping` require a non-null pointer even for zero bytes.
    if len == 0 {
        return 0;
    }
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
    // R-D9: an empty host string may arrive as (NULL, 0), and `slice::from_raw_parts` /
    // `copy_nonoverlapping` require a non-null pointer even for zero bytes.
    if len == 0 {
        return 0;
    }
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
        // R-D9: a zero-length direct argument may be (NULL, 0).
        if n != 0 {
            e.buf.extend_from_slice(core::slice::from_raw_parts(p, n));
        }
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
    // R-D6: a transcoder is the host's code when the host supplied it, so it is an upcall
    // like any other and the sticky slot is read after it.
    if (*cx).hdr.err != AK_OK {
        return false;
    }
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

/// Decision 11 (WP5 steps 7-8): one message position of the context's armed options: a
/// pointer to its entry IN THE HOST'S STRUCT (rule 1), an `ak_unk_opts` or, for a position
/// that can occur more than once, an `ak_unk_pool`; `discard` is "the entry was all zero
/// when armed".
#[derive(Clone, Copy)]
pub struct UnkPos {
    pub entry: *mut u8,
    pub pool: bool,
    pub discard: bool,
}

/// What a decoder carries for its message: the context and the message's position, or a
/// NULL position (drop mode). Copy, two words.
#[derive(Clone, Copy)]
pub struct UnkCx {
    pub dcx: *mut DecCtxImpl,
    pub pos: *mut UnkPos,
}

impl UnkCx {
    /// The root's position, if this context is armed.
    #[inline(always)]
    pub unsafe fn root(dcx: *mut DecCtxImpl) -> UnkCx {
        let pos = if !(*dcx).unk.is_empty() { (*dcx).unk.as_mut_ptr() } else { core::ptr::null_mut() };
        UnkCx { dcx, pos }
    }

    /// The position `rel` entries after this one (`plan.unk_offset`).
    #[inline(always)]
    pub unsafe fn at(self, rel: usize) -> UnkCx {
        if self.pos.is_null() {
            self
        } else {
            UnkCx { dcx: self.dcx, pos: self.pos.add(rel) }
        }
    }
}

const NO_BUF: ak_unk_buf = ak_unk_buf { data: core::ptr::null_mut(), len: 0, cap: 0 };

/// Arm a (bound) context from the host's struct, read in place; NULL = drop mode. `layout`
/// is the root's (byte offset, is pool) per position, generated from the plan. Whether a
/// position discards is decided HERE, from its entry as the host armed it.
pub(crate) unsafe fn unk_arm(dcx: *mut DecCtxImpl, opts: *mut u8, layout: &[(usize, bool)]) {
    let cx = &mut *dcx;
    cx.unk.clear();
    cx.unk_opts = opts;
    if opts.is_null() {
        return;
    }
    let mut all_discard = true;
    for &(off, pool) in layout {
        let entry = opts.add(off);
        let discard = if pool {
            let p = &*(entry as *const ak_unk_pool);
            let any = !p.bufs.is_null() && (0..p.n as usize).any(|i| !(*p.bufs.add(i)).data.is_null());
            !any && p.grow.is_none()
        } else {
            let o = &*(entry as *const ak_unk_opts);
            o.buf.data.is_null() && o.grow.is_none()
        };
        all_discard &= discard;
        cx.unk.push(UnkPos { entry, pool, discard });
    }
    if all_discard {
        // Every entry zero: drop mode, the same code path as NULL (one null test per run).
        cx.unk.clear();
    }
}

/// Copy one unknown run of a message into that message's own buffer slot (plan: UNKNOWN
/// FIELDS ON DECODE). Returns AK_OK or the error that fails the decode.
#[inline(never)]
pub(crate) unsafe fn unk_put(u: UnkCx, slot: &mut ak_unk_buf, run: &[u8]) -> i32 {
    let e = *u.pos;
    if e.discard {
        return AK_OK;
    }
    // Rule 1: the entry is read NOW, in the host's struct, so a refill the host wrote
    // since the last delivery is seen.
    let grow = if e.pool { (*(e.entry as *const ak_unk_pool)).grow } else { (*(e.entry as *const ak_unk_opts)).grow };
    if slot.data.is_null() {
        let taken = if e.pool {
            let p = &mut *(e.entry as *mut ak_unk_pool);
            let mut t = NO_BUF;
            if !p.bufs.is_null() {
                for i in 0..p.n as usize {
                    let b = &mut *p.bufs.add(i);
                    if !b.data.is_null() {
                        // Taken in order, and CLEARED in place: never placed twice.
                        t = core::mem::replace(b, NO_BUF);
                        break;
                    }
                }
            }
            t
        } else {
            core::mem::replace(&mut (*(e.entry as *mut ak_unk_opts)).buf, NO_BUF)
        };
        if !taken.data.is_null() {
            *slot = ak_unk_buf { data: taken.data, len: 0, cap: taken.cap };
        } else if grow.is_none() {
            // Rule 2: nothing left and no grow.
            return AK_ERR_CAPACITY;
        }
    }
    let need = slot.len as usize + run.len();
    if need > slot.cap as usize {
        let Some(grow) = grow else { return AK_ERR_CAPACITY };
        if need > i32::MAX as usize {
            return AK_ERR_LIMIT;
        }
        let mut dst = slot.data as *mut u8;
        let mut cap = slot.cap as i32;
        let cx = &mut *u.dcx;
        ak_rt::bump!(cx.c, reverse);
        ak_rt::bump!(cx.c, grows);
        let host = *(cx.unk_opts as *const *mut c_void);
        let rc = grow(host, need as i32, &mut dst, &mut cap);
        if rc < 0 {
            return rc;
        }
        if cx.hdr.err != AK_OK {
            return cx.hdr.err;
        }
        if dst.is_null() || (cap as i64) < need as i64 {
            return AK_ERR_CAPACITY;
        }
        slot.data = dst as *mut c_void;
        slot.cap = cap as u32;
    }
    core::ptr::copy_nonoverlapping(run.as_ptr(), (slot.data as *mut u8).add(slot.len as usize), run.len());
    slot.len += run.len() as u32;
    AK_OK
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

/// A SECOND bare crossing, identical to `ak_noop` in every respect, and the control that
/// says what the `ak_noop` / `ak_noop_guarded` comparison can resolve.
///
/// Two exported functions with the same body are not the same cost: they land at different
/// addresses, in different cache lines, with different alignment, and through different PLT
/// entries. At a crossing of under 3 ns that difference is not small. So this arm must
/// measure ZERO against `ak_noop`, and whatever it measures instead is the FLOOR of the
/// method -- a guard cost below that floor is not measurable this way, and saying so is the
/// result rather than quoting a number the control cannot support.
///
/// It exists because the first version of the comparison measured the GUARDED crossing as
/// cheaper than the bare one, which cannot be true: the guard adds a load and a branch and
/// can only cost. An arm with the wrong sign means the effect is under the noise, and the
/// way to say that with evidence is to measure the noise.
#[no_mangle]
pub extern "C" fn ak_noop2(x: u64) -> u64 {
    x ^ 1
}

/// The same crossing WITH ABI v1 section 3's guard on it, so the guard can be priced as a
/// delta between two arms in one process and one build.
///
/// It exists because the compile-time form could not be measured. `--features init-guard`
/// is a different binary, so the two arms cannot share a process, and the in-process
/// control R4 requires for that case did not hold: `core-native`, which carries no guard at
/// all, moved by up to 30 percent between the two builds. A cross-build ratio whose control
/// moved is not a figure. This pair is the same question asked the way R4's sharpened half
/// says to ask it -- a delta between two arms in the same interleaved rounds -- and it
/// survives what the cross-build form did not.
///
/// The guard here is the SAME code the generator emits into every entry point: one relaxed
/// load of the process-global init word, a compare and a branch. Multiply the delta by the
/// counting build's forward-crossing count for the per-payload cost.
#[no_mangle]
pub extern "C" fn ak_noop_guarded(x: u64) -> u64 {
    if !ak_init_ok() {
        return AK_ERR_UNINITIALIZED as u64;
    }
    x ^ 1
}

/// The same crossing with a reverse call in it: the core calls back through a function
/// pointer the host supplied. That is what a loop callback, an `apply` and an `add` cost
/// before any work is done.
#[no_mangle]
pub unsafe extern "C" fn ak_noop_reverse(f: unsafe extern "C" fn(u64) -> u64, x: u64) -> u64 {
    f(x)
}

#[cfg(test)]
mod tc_empty_tests {
    //! FIX-PLAN WP4 item 10 / R-D9: an empty host string may legally arrive as
    //! `(NULL, 0)`. `slice::from_raw_parts` requires a non-null pointer even for length 0,
    //! so the transcoders must not build a slice (or copy) from it. Run in a DEBUG build,
    //! where the standard library's precondition checks turn the UB into an abort.
    use super::*;

    unsafe extern "C" fn no_grow(_s: *mut c_void, _w: i32, _d: *mut *mut u8, _c: *mut i32) -> i32 {
        AK_ERR_CAPACITY
    }

    #[test]
    fn every_transcoder_accepts_null_and_zero() {
        let mut dst = [0u8; 4];
        let tcs: [(&str, ak_transcode_fn); 6] = [
            ("utf8", tc_utf8),
            ("utf8_trusted", tc_utf8_trusted),
            ("utf8_simd", tc_utf8_simd),
            ("latin1", tc_latin1),
            ("utf16", tc_utf16),
            ("bytes", ak_tc_bytes()),
        ];
        for (name, tc) in tcs {
            let n = unsafe {
                tc(core::ptr::null(), 0, dst.as_mut_ptr(), dst.len() as i32, no_grow, core::ptr::null_mut())
            };
            assert_eq!(n, 0, "{name}: an empty string transcodes to zero bytes");
        }
    }
}
