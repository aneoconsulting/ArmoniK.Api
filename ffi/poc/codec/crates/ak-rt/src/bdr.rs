//! The pull family's deposit target (ABI v1 section 7.1).
//!
//! The push family deposits a decoded value by CALLING the host: `apply` for the group,
//! `add_<field>` for a run, `new`/`apply` for a non-leaf element. The pull family deposits
//! it by APPENDING A RECORD here, and the host reads the records afterwards. That is the
//! whole of the difference between the two families, and it is why one traversal emitter
//! can serve both: the traversal, the arena, section 7.2's batching predicate and 7.3's
//! flush-on-a-foreign-tag are the same code, and only the flush and the final apply differ.
//!
//! So a drained buffer is **a log of the reverse calls push would have made**, in the order
//! push would have made them. A host replays it with the same per-slot code it would have
//! registered in the vtable, and gets the same object graph -- which is what makes the two
//! families comparable rather than two decoders.
//!
//! **Nothing in here calls the host.** That is the point on the JVM: `ak_parse_*` can run
//! inside a critical section because it makes no upcall, so the wire buffer does not have
//! to be copied into native scratch first (the copy that cost the Java slice's push arm 1
//! to 14 percent).
//!
//! The buffer lives in the host-owned `ak_dec_ctx` (section 3), never in a thread-local:
//! a thread-local is a hidden global with a re-entrancy hazard, which section 7.3 refuses
//! for the arena and which applies here for the same reason.

/// Record kinds. One per reverse call the push family makes.
pub const OP_APPLY: u32 = 1;
/// A run of leaf elements, strings or packed scalars: `add_<slot>(obj, token, elems, n)`.
pub const OP_ADD: u32 = 2;
/// A non-leaf element begins: `new_<slot>(obj) -> token`. The codec assigns the token
/// rather than receiving one, because there is nobody to ask during a parse.
pub const OP_NEW: u32 = 3;
/// A non-leaf element's own group: `apply_<slot>(obj, token, fix)`.
pub const OP_APPLY_ELEM: u32 = 4;

/// The header of one record. 24 bytes, so a payload that follows it is 8-aligned when the
/// buffer is, which every `ak_dfix_*` needs (they carry `i64` and `f64`).
///
/// `#[repr(C)]` and no padding by inspection: 4 + 4 + 8 + 4 + 4.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rec {
    /// OP_APPLY, OP_ADD, OP_NEW or OP_APPLY_ELEM.
    pub op: u32,
    /// Which loop slot of the root message, 1-based in `loop_slots` order; 0 for the root
    /// group itself. For a run inside a non-leaf element it is the element slot's id in the
    /// high half and the inner slot's id in the low half, so one u32 names both without a
    /// second field: `(outer << 16) | inner`.
    pub slot: u32,
    /// Which element of the enclosing run this belongs to. `AK_TOKEN_ROOT` (-1) for the
    /// root object. A token is an INDEX, never an address (ABI v1 section 10), which is
    /// what lets the codec mint one during a parse: the host's replay pushes elements in
    /// the same order, so index i is the same object on both sides.
    pub token: i64,
    /// Elements in the payload. 1 for OP_APPLY and OP_APPLY_ELEM, 0 for OP_NEW.
    pub n: u32,
    /// Payload bytes following this header, already rounded up to a multiple of 8.
    pub bytes: u32,
}

pub const REC_BYTES: usize = core::mem::size_of::<Rec>();

#[inline(always)]
pub const fn pad8(n: usize) -> usize {
    (n + 7) & !7
}

/// The record buffer. Grows by doubling and is reused across decodes: `reset` keeps the
/// allocation, which is what makes `ak_bdr_reserve` worth having at all.
///
/// Backed by `Vec<u64>` rather than `Vec<u8>` for one reason and it is not taste: the
/// payload of a record is an `ak_dfix_*` group carrying `i64` and `f64`, so it has to be
/// 8-aligned to be read as one, and `Vec<u8>` guarantees alignment 1. Every record is
/// 24 bytes of header plus a payload padded to 8, so the whole buffer is a whole number of
/// words and the alignment is structural rather than incidental.
pub struct Bdr {
    buf: Vec<u64>,
    /// Monotone across the whole parse, so every non-leaf element in the message has a
    /// distinct token. The host's replay uses it as an index into its own element table.
    pub next_token: i64,
    pub err: i32,
}

impl Default for Bdr {
    fn default() -> Self {
        Bdr::new()
    }
}

impl Bdr {
    pub const fn new() -> Self {
        Bdr { buf: Vec::new(), next_token: 0, err: 0 }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.buf.clear();
        self.next_token = 0;
        self.err = 0;
    }

    #[inline]
    pub fn reserve(&mut self, bytes: usize) {
        let words = pad8(bytes) / 8;
        self.buf.reserve(words.saturating_sub(self.buf.len()));
    }

    #[inline]
    pub fn footprint(&self) -> usize {
        self.buf.len() * 8
    }

    /// The records as bytes. 8-aligned by construction.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.buf.as_ptr() as *const u8, self.buf.len() * 8) }
    }

    /// Append a header plus `bytes` of payload copied from `src`. The copy is the pull
    /// family's cost and it is deliberate: this is the intermediate the C# slice priced at
    /// an estimated 12 to 19 percent of a parse, measured here rather than estimated.
    #[inline(always)]
    pub unsafe fn push(
        &mut self,
        op: u32,
        slot: u32,
        token: i64,
        n: u32,
        src: *const u8,
        bytes: usize,
    ) {
        let padded = pad8(bytes);
        let words = (REC_BYTES + padded) / 8;
        let at = self.buf.len();
        self.buf.reserve(words);
        let p = self.buf.as_mut_ptr().add(at) as *mut u8;
        (p as *mut Rec).write(Rec { op, slot, token, n, bytes: padded as u32 });
        if bytes != 0 {
            core::ptr::copy_nonoverlapping(src, p.add(REC_BYTES), bytes);
        }
        if padded != bytes {
            core::ptr::write_bytes(p.add(REC_BYTES + bytes), 0, padded - bytes);
        }
        self.buf.set_len(at + words);
    }

    /// The token a `new_<slot>` would have returned.
    #[inline(always)]
    pub fn mint(&mut self) -> i64 {
        let t = self.next_token;
        self.next_token += 1;
        t
    }

    /// Copy whole records into the host's buffer, starting at `*cursor` (a byte offset).
    ///
    /// Never splits a record: a host walking a chunk has to find a header at its start, and
    /// on the JVM the chunk is a `byte[]` it reads with no further calls. A record larger
    /// than `cap` is therefore a hard error rather than a silent truncation -- the largest
    /// one a schema can produce is section 7.3's 32 KB arena plus a header, so a host that
    /// sizes at `BDR_MIN_CHUNK` never sees it.
    ///
    /// **`dst` must be 8-aligned**, for the same reason the buffer is: the host reads an
    /// `ak_dfix_*` out of it. A JVM host reading through a `VarHandle` does not care and a
    /// C host passing a `malloc`ed buffer already satisfies it.
    ///
    /// Returns bytes written, or a negative error code.
    #[inline]
    pub unsafe fn drain(&self, dst: *mut u8, cap: usize, cursor: &mut usize) -> isize {
        let all = self.as_bytes();
        let mut at = *cursor;
        if at >= all.len() {
            return 0;
        }
        let mut wrote = 0usize;
        while at < all.len() {
            let h = (all.as_ptr().add(at) as *const Rec).read();
            let size = REC_BYTES + h.bytes as usize;
            if wrote + size > cap {
                if wrote == 0 {
                    return crate::ERR_CAPACITY as isize;
                }
                break;
            }
            core::ptr::copy_nonoverlapping(all.as_ptr().add(at), dst.add(wrote), size);
            wrote += size;
            at += size;
        }
        *cursor = at;
        wrote as isize
    }
}

/// The smallest chunk a host may drain into, whatever the schema: one arena plus one
/// header. A host that passes less gets `AK_ERR_CAPACITY` rather than a partial record.
pub const BDR_MIN_CHUNK: usize = crate::ARENA_BYTES + REC_BYTES;

/// A cursor over a record buffer, drained or borrowed. The host side of the replay, shared
/// by the drain-and-copy arm and the borrow arm so the two cannot deposit differently.
///
/// Takes `&[u64]` because a record's payload must be 8-aligned to be read as a group.
pub struct RecIter<'a> {
    pub buf: &'a [u64],
    /// Byte offset into `buf`.
    pub at: usize,
}

impl<'a> RecIter<'a> {
    #[inline]
    pub fn new(buf: &'a [u64]) -> Self {
        RecIter { buf, at: 0 }
    }

    #[inline]
    fn bytes(&self) -> &'a [u8] {
        unsafe { core::slice::from_raw_parts(self.buf.as_ptr() as *const u8, self.buf.len() * 8) }
    }
}

impl<'a> Iterator for RecIter<'a> {
    /// (header, payload pointer). The pointer is 8-aligned and valid for `h.bytes`.
    type Item = (Rec, *const u8);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let all = self.bytes();
        if self.at + REC_BYTES > all.len() {
            return None;
        }
        let h = unsafe { (all.as_ptr().add(self.at) as *const Rec).read() };
        let body = self.at + REC_BYTES;
        let end = body + h.bytes as usize;
        if end > all.len() {
            return None;
        }
        self.at = end;
        Some((h, unsafe { all.as_ptr().add(body) }))
    }
}
