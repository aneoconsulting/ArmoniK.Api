//! FIX-PLAN D23 (owner, 2026-10-09): the FSM decode family's runtime support.
//!
//! A third decode family beside push (`ak_decode_*`) and pull (`ak_parse_*`): the host calls
//! `ak_fsm_begin_<Root>` once and then `ak_fsm_next_<Root>` until the end event, and each call
//! advances the parse just far enough to return ONE event -- the equivalent of one record of
//! the pull family's log (`plan.pull_records`), in the same order and with the same tokens.
//!
//! **Nothing here is shared with push or pull, by the owner's rule.** The wire reader, the
//! UTF-8 check, the unknown-field placement (decision 11's grow contract), the run arena and
//! the frame stack are this module's own copies, and the per-root state machines are emitted
//! by their own generator module (`gen/rust_fsm.py`) into their own file (`generated/fsm.rs`).
//! The three families are to be compared as independent implementations; a helper factored
//! out of one into another could move either's cost, and the comparison would then compare
//! nothing. What the FSM does read from the context is CONFIGURATION, not code: the root the
//! context is bound to, its counters, its sticky error slot and decision 11's armed options.
//!
//! **State lives in the context, never on a stack and never in a thread-local** (ABI v1
//! section 7.3's reason): `DecCtxImpl.fsm`, allocated on the first `begin` and reused by every
//! later decode on that context. One context per decode in flight, as for pull: the state is
//! the context's, so two decodes on two contexts are independent and re-entrancy needs nothing.
//!
//! * the **frame stack**: a fixed array of `FSM_MAX_FRAMES` frames, one per nested open
//!   message (the root, an inlined singular child, a non-leaf element, a packed body). The
//!   depth a schema can reach is static (frames are schema paths, an unknown group is skipped
//!   by the reader's own bounded skip), so the generator refuses a root needing more than
//!   `FSM_MAX_FRAMES`; the push below the bound is checked anyway and refused with
//!   AK_ERR_DEPTH.
//! * the **run arena**: `FSM_ARENA_BYTES` (32 KB, the push/pull budget), elements decoded
//!   into it in place; a run event's payload points at it. One arena: runs never nest
//!   (ABI v1 section 7.2) and at most one slot holds elements at a time (`cur`, `n`).
//! * the **groups under construction**: the root group and the current non-leaf element's
//!   group, in `grp`; a group event's payload points there.
//!
//! An event's payload is valid until the next call on the context, which may overwrite it.

use crate::DecCtxImpl;
use ak_abi::*;

/// Frames a root may need. Shapes and corpus need at most 4 (generator-checked per root).
pub const FSM_MAX_FRAMES: usize = 16;
/// The run arena: the same byte budget as push's and pull's (`ak_rt::ARENA_BYTES`), so the
/// runs, and so the events, are the same. Restated rather than imported (owner's rule); the
/// equality is asserted below.
pub const FSM_ARENA_BYTES: usize = 32 * 1024;
const _: () = assert!(FSM_ARENA_BYTES == ak_rt::ARENA_BYTES);
/// D23's event (owner, 2026-10-09: the op is the return value, not a member).
const _: () = assert!(core::mem::size_of::<ak_fsm_ev>() == 32);
const _: () = assert!(core::mem::align_of::<ak_fsm_ev>() == 8);
const _: () = assert!(core::mem::offset_of!(ak_fsm_ev, slot) == 0);
const _: () = assert!(core::mem::offset_of!(ak_fsm_ev, n) == 4);
const _: () = assert!(core::mem::offset_of!(ak_fsm_ev, token) == 8);
const _: () = assert!(core::mem::offset_of!(ak_fsm_ev, data) == 16);
const _: () = assert!(core::mem::offset_of!(ak_fsm_ev, bytes) == 24);

/// What begin / next return with an event: its op (AK_BDR_*), positive; the root group
/// (APPLY) is always the last event and so is the end.
pub const FSM_NEW: i32 = AK_BDR_NEW as i32;
pub const FSM_ADD: i32 = AK_BDR_ADD as i32;
pub const FSM_APPLY_ELEM: i32 = AK_BDR_APPLY_ELEM as i32;
pub const FSM_APPLY: i32 = AK_BDR_APPLY as i32;
const _: () = assert!(FSM_NEW > 0 && FSM_ADD > 0 && FSM_APPLY_ELEM > 0 && FSM_APPLY > 0);

/// Elements of `size` bytes a run holds before it is flushed: the byte budget divided by
/// the element size, at least one.
pub const fn fsm_arena_n(size: usize) -> usize {
    let n = FSM_ARENA_BYTES / size;
    if n == 0 {
        1
    } else {
        n
    }
}

pub const ST_IDLE: u8 = 0;
pub const ST_RUN: u8 = 1;
pub const ST_DONE: u8 = 2;
pub const ST_FAILED: u8 = 3;

/// One open message: which generated arm set reads it, and where its body ends (absolute).
#[derive(Clone, Copy, Default)]
pub struct FsmFrame {
    pub kind: u32,
    pub end: usize,
}

/// The FSM's whole per-context state.
pub struct FsmCx {
    pub buf: *const u8,
    pub len: usize,
    /// The one read cursor, absolute. Frames nest strictly, so a child's end is where its
    /// parent resumes.
    pub pos: usize,
    pub state: u8,
    pub depth: usize,
    pub frames: [FsmFrame; FSM_MAX_FRAMES],
    /// The open run: its slot id in the enclosing scope's numbering (0 = none) and count.
    pub cur: u32,
    pub n: usize,
    /// The current non-leaf element's token, and the next one to mint.
    pub tok: i64,
    pub next_token: i64,
    /// An error owed after the events pull writes before it (a non-leaf element whose
    /// length is truncated: pull still writes its NEW and APPLY_ELEM).
    pub pend: i32,
    /// The depth the owed error falls due at: the element's parent, once the element's
    /// frame has closed (its APPLY_ELEM written).
    pub pend_depth: usize,
    /// D20's mask for FSM decodes on this context (`ak_fsm_set_pvt_<Root>`), and the copy
    /// the decode in progress reads (taken at `begin`, so a setter call mid-decode does not
    /// change a decode already started).
    pub utf8_skip: u64,
    pub sk: u64,
    pub arena: Vec<u64>,
    pub grp: Vec<u64>,
}

impl FsmCx {
    fn new() -> Self {
        FsmCx {
            buf: core::ptr::null(),
            len: 0,
            pos: 0,
            state: ST_IDLE,
            depth: 0,
            frames: [FsmFrame::default(); FSM_MAX_FRAMES],
            cur: 0,
            n: 0,
            tok: 0,
            next_token: 0,
            pend: 0,
            pend_depth: 0,
            utf8_skip: 0,
            sk: 0,
            arena: vec![0u64; FSM_ARENA_BYTES / 8],
            grp: Vec::new(),
        }
    }

    /// Start a decode of `buf[..len]` whose groups need `grp_words` words.
    #[inline]
    pub fn start(&mut self, buf: *const u8, len: usize, grp_words: usize) {
        self.buf = buf;
        self.len = len;
        self.pos = 0;
        self.state = ST_RUN;
        self.depth = 1;
        self.frames[0] = FsmFrame { kind: 0, end: len };
        self.cur = 0;
        self.n = 0;
        self.tok = 0;
        self.next_token = 0;
        self.pend = 0;
        if self.grp.len() < grp_words {
            self.grp.resize(grp_words, 0);
        }
    }

    /// Open a frame. Refused past the bound (unreachable for a root the generator accepted).
    #[inline(always)]
    pub fn push(&mut self, kind: u32, end: usize) -> bool {
        if self.depth >= FSM_MAX_FRAMES {
            return false;
        }
        self.frames[self.depth] = FsmFrame { kind, end };
        self.depth += 1;
        true
    }

    #[inline(always)]
    pub fn mint(&mut self) -> i64 {
        let t = self.next_token;
        self.next_token += 1;
        t
    }

    /// The input, as a slice (an empty message may come with a NULL pointer).
    #[inline(always)]
    pub unsafe fn input<'a>(&self) -> &'a [u8] {
        if self.len == 0 {
            &[]
        } else {
            core::slice::from_raw_parts(self.buf, self.len)
        }
    }
}

/// The context's FSM state, created on first use and kept for every later decode.
#[inline(always)]
pub unsafe fn fsm_of<'a>(dcx: *mut DecCtxImpl) -> &'a mut FsmCx {
    if (*dcx).fsm.is_none() {
        (*dcx).fsm = Some(Box::new(FsmCx::new()));
    }
    (*dcx).fsm.as_deref_mut().unwrap_unchecked()
}

/// The FSM's wire reader: a cursor over the ONE input buffer bounded by the open message's
/// end, so every span it returns is absolute. Its rules are the plan's DECODE RULES (the
/// same accept/refuse set and error codes as the other families' reader, which is the
/// differential's to check), written here again.
#[derive(Clone, Copy)]
pub struct FRd<'a> {
    pub buf: &'a [u8],
    pub pos: usize,
    pub end: usize,
    pub err: i32,
}

impl<'a> FRd<'a> {
    #[inline(always)]
    pub fn at_end(&self) -> bool {
        self.pos >= self.end || self.err != 0
    }

    /// More bytes in the open message (D23 fix A's loop condition).
    #[inline(always)]
    pub fn more(&self) -> bool {
        self.pos < self.end
    }

    #[inline(always)]
    pub fn varint(&mut self) -> u64 {
        let mut v = 0u64;
        let mut shift = 0u32;
        loop {
            if self.pos >= self.end {
                self.err = AK_ERR_TRUNCATED;
                return 0;
            }
            let c = self.buf[self.pos];
            self.pos += 1;
            v |= ((c & 0x7f) as u64) << shift;
            if c & 0x80 == 0 {
                return v;
            }
            shift += 7;
            if shift > 63 {
                self.err = AK_ERR_MALFORMED;
                return 0;
            }
        }
    }

    #[inline(always)]
    pub fn f64(&mut self) -> f64 {
        if self.end - self.pos < 8 {
            self.err = AK_ERR_TRUNCATED;
            return 0.0;
        }
        let v = f64::from_le_bytes(self.buf[self.pos..self.pos + 8].try_into().unwrap());
        self.pos += 8;
        v
    }

    #[inline(always)]
    pub fn fixed32(&mut self) -> u32 {
        if self.end - self.pos < 4 {
            self.err = AK_ERR_TRUNCATED;
            return 0;
        }
        let v = u32::from_le_bytes(self.buf[self.pos..self.pos + 4].try_into().unwrap());
        self.pos += 4;
        v
    }

    /// A length-delimited body as (absolute offset, length). A length past the open
    /// message's end is TRUNCATED and yields an empty in-bounds span.
    #[inline(always)]
    pub fn len_body(&mut self) -> (usize, usize) {
        let n = self.varint() as usize;
        if n > self.end - self.pos {
            self.err = AK_ERR_TRUNCATED;
            return (self.pos, 0);
        }
        let off = self.pos;
        self.pos += n;
        (off, n)
    }

    /// A sub-reader over `off..off + n`.
    #[inline(always)]
    pub fn sub(&self, off: usize, n: usize) -> FRd<'a> {
        FRd { buf: self.buf, pos: off, end: off + n, err: 0 }
    }

    /// An unknown field (protobuf's forward compatibility).
    #[inline]
    pub fn skip(&mut self, tag: u32, wire: u32) {
        match wire {
            0 => {
                self.varint();
            }
            1 => self.pos += 8,
            2 => {
                self.len_body();
            }
            3 => self.skip_group(tag, 0),
            5 => self.pos += 4,
            _ => self.err = AK_ERR_MALFORMED,
        }
        if self.pos > self.end {
            self.err = AK_ERR_TRUNCATED;
        }
    }

    /// The GROUP skip: ends at the END_GROUP whose field number matches; bounded nesting.
    fn skip_group(&mut self, tag: u32, depth: u32) {
        if depth >= FSM_GROUP_DEPTH {
            self.err = AK_ERR_DEPTH;
            return;
        }
        loop {
            if self.err != 0 {
                return;
            }
            if self.pos >= self.end {
                self.err = AK_ERR_TRUNCATED;
                return;
            }
            let k = self.varint();
            if self.err != 0 {
                return;
            }
            let (t, w) = ((k >> 3) as u32, (k & 7) as u32);
            if t == 0 || (k >> 3) > FSM_MAX_FIELD_NUMBER {
                self.err = AK_ERR_MALFORMED;
                return;
            }
            if w == 4 {
                if t != tag {
                    self.err = AK_ERR_MALFORMED;
                }
                return;
            }
            if w == 3 {
                self.skip_group(t, depth + 1);
                continue;
            }
            self.skip(t, w);
        }
    }
}

/// plan.MAX_FIELD_NUMBER and plan.GROUP_DEPTH_LIMIT, restated (owner's rule).
pub const FSM_MAX_FIELD_NUMBER: u64 = (1 << 29) - 1;
pub const FSM_GROUP_DEPTH: u32 = 100;
const _: () = assert!(FSM_MAX_FIELD_NUMBER == ak_rt::MAX_FIELD_NUMBER);

/// The plan's UTF-8 rule on decode (utf8="reject"): true when `b` is well-formed UTF-8.
/// simdutf8's validator, the default of the other families' builds (the accept set is the
/// Unicode definition whichever validator runs).
#[inline(always)]
pub fn fsm_utf8_ok(b: &[u8]) -> bool {
    simdutf8::basic::from_utf8(b).is_ok()
}

// ---- decision 11: unknown fields, the FSM's own placement -------------------------------

/// Where a message's unknown runs go: the context and the message's position in the armed
/// options, or a NULL position (drop mode). The positions are the context's configuration
/// (`ak_dec_ctx_new_<Root>` / `ak_dec_reset_<Root>`), read here, not the other families' code.
#[cfg(feature = "unknown-fields")]
#[derive(Clone, Copy)]
pub struct FsmU {
    pub dcx: *mut DecCtxImpl,
    pub pos: *mut crate::UnkPos,
}

#[cfg(feature = "unknown-fields")]
impl FsmU {
    #[inline(always)]
    pub unsafe fn root(dcx: *mut DecCtxImpl) -> FsmU {
        let pos = if !(*dcx).unk.is_empty() { (*dcx).unk.as_mut_ptr() } else { core::ptr::null_mut() };
        FsmU { dcx, pos }
    }
    #[inline(always)]
    pub unsafe fn at(self, rel: usize) -> FsmU {
        if self.pos.is_null() {
            self
        } else {
            FsmU { dcx: self.dcx, pos: self.pos.add(rel) }
        }
    }
}

/// The no-unknown variant: nothing to carry.
#[cfg(not(feature = "unknown-fields"))]
#[derive(Clone, Copy)]
pub struct FsmU;

#[cfg(not(feature = "unknown-fields"))]
impl FsmU {
    #[inline(always)]
    pub unsafe fn root(_dcx: *mut DecCtxImpl) -> FsmU {
        FsmU
    }
    #[inline(always)]
    pub fn at(self, _rel: usize) -> FsmU {
        self
    }
}

#[cfg(feature = "unknown-fields")]
const FSM_NO_BUF: ak_unk_buf = ak_unk_buf { data: core::ptr::null_mut(), len: 0, cap: 0 };

/// Copy one unknown run into the message's own buffer slot, by decision 11's contract
/// (plan: UNKNOWN FIELDS ON DECODE): discard, placement in order from the host's entry
/// (cleared as taken), `grow` with realloc semantics, every capacity capped at INT32_MAX
/// (AK_ERR_LIMIT past it), AK_ERR_CAPACITY when nothing is left and no grow; a grow is one
/// reverse crossing and one grow in the counting build. Returns AK_OK or the error.
#[cfg(feature = "unknown-fields")]
#[inline(never)]
pub unsafe fn fsm_unk_put(u: FsmU, slot: &mut ak_unk_buf, run: &[u8]) -> i32 {
    let e = *u.pos;
    if e.discard {
        return AK_OK;
    }
    let grow = if e.pool { (*(e.entry as *const ak_unk_pool)).grow } else { (*(e.entry as *const ak_unk_opts)).grow };
    if slot.data.is_null() {
        let taken = if e.pool {
            let p = &mut *(e.entry as *mut ak_unk_pool);
            let mut t = FSM_NO_BUF;
            if !p.bufs.is_null() {
                for i in 0..p.n as usize {
                    let b = &mut *p.bufs.add(i);
                    if !b.data.is_null() {
                        t = core::mem::replace(b, FSM_NO_BUF);
                        break;
                    }
                }
            }
            t
        } else {
            core::mem::replace(&mut (*(e.entry as *mut ak_unk_opts)).buf, FSM_NO_BUF)
        };
        if !taken.data.is_null() {
            *slot = ak_unk_buf { data: taken.data, len: 0, cap: taken.cap };
        } else if grow.is_none() {
            return AK_ERR_CAPACITY;
        }
    }
    const MAX: usize = i32::MAX as usize;
    let need = (slot.len as usize).saturating_add(run.len());
    if need > MAX {
        return AK_ERR_LIMIT;
    }
    if need > (slot.cap as usize).min(MAX) {
        let Some(grow) = grow else { return AK_ERR_CAPACITY };
        let mut dst = slot.data as *mut u8;
        let mut cap = slot.cap as i32;
        let cx = &mut *u.dcx;
        ak_rt::bump!(cx.c, reverse);
        ak_rt::bump!(cx.c, grows);
        let host = *(cx.unk_opts as *const *mut core::ffi::c_void);
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
        slot.data = dst as *mut core::ffi::c_void;
        slot.cap = cap as u32;
    }
    core::ptr::copy_nonoverlapping(run.as_ptr(), (slot.data as *mut u8).add(slot.len as usize), run.len());
    slot.len += run.len() as u32;
    AK_OK
}
