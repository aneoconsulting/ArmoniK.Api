//! The encode buffer, the varints, and the learned length-placeholder width.
//!
//! ABI v1 section 6: "Length placeholders use a learned width, held in the encode context.
//! [...] The table lives in the context, never process-global". And section 4: the codec
//! hands the transcoder whatever the buffer has left rather than a reservation sized from a
//! declared bound, so the prefix width is resolved AFTER the transcode returns. Open
//! decision 5 is what that costs, and `Counters::prefix_moves` and `Counters::grows` are
//! how this slice answers it.

use crate::counters::Counters;
use crate::{key, varint_len, WIRE_LEN};
use bytes::{Bytes, BytesMut};

/// An open length prefix. Held by value so a miss cannot be attributed to the wrong site.
pub struct Mark {
    site: u32,
    /// Where the placeholder starts.
    hdr: usize,
    /// How many bytes were reserved for it.
    w: usize,
}

/// ABI v1 section 6's refused arrangement, built so it can be measured (feature
/// `global-widths`, off by default). One table for the whole process, shared by every
/// context on every thread, sized at the largest site count any caller asks for.
///
/// **This is a TEST-ONLY construction of a configuration the specification refuses. It is
/// not an option a host may pick**, and no shipped build enables it. It exists so
/// obligation 12.5's suite can be seen failing, and so section 6's claim is measured on a
/// second host rather than inherited.
///
/// Relaxed atomics, so this is a real program rather than a data race: what it exposes is
/// the CONTENTION section 6's claim is about -- one cache line written by every encoding
/// thread -- and not undefined behaviour. The bytes stay correct because `Mark` carries its
/// own width by value, which is the invariant that makes a missed width a cost and not a
/// defect.
#[cfg(feature = "global-widths")]
pub static GLOBAL_WIDTHS: [core::sync::atomic::AtomicU8; 4096] =
    [const { core::sync::atomic::AtomicU8::new(1) }; 4096];

/// Store `v` as a varint at `p` (which must have 10 writable bytes); returns its length.
#[inline(always)]
unsafe fn put_varint(p: *mut u8, mut v: u64) -> usize {
    let mut i = 0usize;
    while v >= 0x80 {
        *p.add(i) = (v as u8) | 0x80;
        v >>= 7;
        i += 1;
    }
    *p.add(i) = v as u8;
    i + 1
}

pub struct Enc {
    /// Optimisation T1: a `BytesMut` rather than a `Vec<u8>`, so the transport-ready form
    /// (`take`, what a tonic request body is) is a split and a freeze, O(1), as prost's
    /// is out of tonic's encode buffer, and not a copy. Read and written as a byte vector
    /// everywhere else; the reused-buffer form (`reset`, encode, read `buf`) never splits
    /// and stays allocation-free.
    pub buf: BytesMut,
    /// T1: how many bytes the last `take` handed out, so `reset` reserves them ONCE (the
    /// buffer reclaimed when the taken `Bytes` is gone, replaced once when it is still
    /// held) instead of the encode regrowing from the split's remainder.
    want: usize,
    /// One learned width per length-prefix site in the generated code. Per context: a
    /// global table made two encoding threads slower than one (ABI v1 section 6).
    #[cfg(not(feature = "global-widths"))]
    widths: Box<[u8]>,
    pub c: Counters,
    /// Sticky, first error wins (ABI v1 section 5).
    pub err: i32,
    /// The capacity most recently handed to a transcoder. The codec refuses a returned
    /// count larger than it, which is the one check ABI v1 section 4 says cannot be
    /// removed, and a grow may have changed it after the call started.
    pub last_cap: i32,
    /// Counting build only: which SITE missed, so "the grow path costs X" can name the
    /// field rather than the message. ABI v1 open decision 5 asks what the learned width
    /// is worth, and an aggregate that does not say where it thrashes cannot answer it.
    #[cfg(feature = "count")]
    pub site_moves: Box<[u32]>,
}

impl Enc {
    pub fn new(sites: usize) -> Self {
        Enc {
            buf: BytesMut::with_capacity(4096),
            want: 0,
            #[cfg(not(feature = "global-widths"))]
            widths: vec![1u8; sites].into_boxed_slice(),
            c: Counters::default(),
            err: 0,
            last_cap: 0,
            #[cfg(feature = "count")]
            site_moves: vec![0u32; sites].into_boxed_slice(),
        }
    }

    /// The learned width for one site. One indirection either way, so the default build's
    /// code is what it was: a load from a `Box<[u8]>` this context owns.
    #[cfg(not(feature = "global-widths"))]
    #[inline(always)]
    fn width(&self, site: u32) -> u8 {
        self.widths[site as usize]
    }

    #[cfg(not(feature = "global-widths"))]
    #[inline(always)]
    fn set_width(&mut self, site: u32, w: u8) {
        self.widths[site as usize] = w;
    }

    #[cfg(feature = "global-widths")]
    #[inline(always)]
    fn width(&self, site: u32) -> u8 {
        GLOBAL_WIDTHS[site as usize].load(core::sync::atomic::Ordering::Relaxed)
    }

    #[cfg(feature = "global-widths")]
    #[inline(always)]
    fn set_width(&mut self, site: u32, w: u8) {
        GLOBAL_WIDTHS[site as usize].store(w, core::sync::atomic::Ordering::Relaxed);
    }

    #[inline]
    pub fn reset(&mut self) {
        // `BytesMut::clear` is not `#[inline]` (nor are `truncate` and `resize`), and
        // without LTO a non-inline, non-generic method of another crate is a call on every
        // use: measured +20-40% on the small-field payloads (logs/rust/opt/t1-native-first).
        // A length of 0 exposes nothing, so `set_len(0)` is `clear`.
        unsafe { self.buf.set_len(0) };
        // T1: after a `take` the buffer is the split's remainder. Reserve what the taken
        // message needed: with the taken `Bytes` dropped this reclaims the whole allocation
        // (nothing to copy, the length is 0); with it still held it allocates once. Never
        // taken, `want` is 0 and this is one compare.
        if self.want > self.buf.capacity() {
            self.buf.reserve(self.want);
        }
        self.err = 0;
        // The learned widths deliberately SURVIVE a reset: that is what makes them learned.
    }

    /// T1: the encoded bytes as a `Bytes`, split off the buffer and frozen: O(1), no copy.
    /// The buffer keeps the allocation's remainder; the next `reset` reclaims or replaces it
    /// (see `want`). What a tonic request body is, and what `ak_call_unary_enc` sends.
    #[inline]
    pub fn take(&mut self) -> Bytes {
        self.want = self.buf.len();
        self.buf.split().freeze()
    }

    /// Where the next byte goes: the spare capacity's start (which, unlike a pointer taken
    /// from the initialised slice, may be written through for the whole spare capacity).
    #[inline(always)]
    fn tail(&mut self) -> *mut u8 {
        self.buf.spare_capacity_mut().as_mut_ptr() as *mut u8
    }

    #[inline]
    pub fn fail(&mut self, code: i32) {
        if self.err == 0 {
            self.err = code;
        }
    }

    /// Optimisation E5: every writer below reserves ONCE for what it writes and then stores
    /// through a raw pointer and `set_len`, instead of a capacity check per byte (`push`).
    /// The bytes are the ones the byte-at-a-time form wrote.
    #[inline(always)]
    pub fn varint(&mut self, v: u64) {
        self.buf.reserve(10);
        unsafe {
            let len = self.buf.len();
            let n = put_varint(self.tail(), v);
            self.buf.set_len(len + n);
        }
    }

    /// A packed run of varints (E5): one reservation for the whole run (at most 10 bytes an
    /// element), then raw stores.
    #[inline(always)]
    pub fn varint_run<I: Iterator<Item = u64>>(&mut self, n: usize, it: I) {
        self.buf.reserve(n.saturating_mul(10));
        unsafe {
            let len = self.buf.len();
            let base = self.tail();
            let mut at = 0usize;
            for v in it.take(n) {
                at += put_varint(base.add(at), v);
            }
            self.buf.set_len(len + at);
        }
    }

    /// A packed run of doubles (E5): little-endian, so on a little-endian target it is one
    /// copy of the host's array.
    #[inline(always)]
    pub fn f64_run(&mut self, v: &[f64]) {
        #[cfg(target_endian = "little")]
        {
            let b = unsafe { core::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 8) };
            self.buf.extend_from_slice(b);
        }
        #[cfg(not(target_endian = "little"))]
        for x in v {
            self.buf.extend_from_slice(&x.to_le_bytes());
        }
    }

    #[inline(always)]
    pub fn key(&mut self, tag: u32, wire: u32) {
        self.varint(key(tag, wire));
    }

    #[inline(always)]
    pub fn varint_field(&mut self, tag: u32, v: u64) {
        // E5: one reservation for key and value.
        self.buf.reserve(20);
        unsafe {
            let len = self.buf.len();
            let p = self.tail();
            let k = put_varint(p, key(tag, crate::WIRE_VARINT));
            let n = put_varint(p.add(k), v);
            self.buf.set_len(len + k + n);
        }
    }

    #[inline(always)]
    pub fn f64_field(&mut self, tag: u32, v: f64) {
        self.key(tag, crate::WIRE_I64);
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    /// A `fixed32` field (wire type 5): four bytes, little-endian. R-E3: the corpus's
    /// `WireZoo.v_fixed32`, which a generator with no wire-type-5 case could not reach.
    #[inline(always)]
    pub fn fixed32_field(&mut self, tag: u32, v: u32) {
        self.key(tag, crate::WIRE_I32);
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    /// A length-delimited field whose length is known before the body is written, which is
    /// the case for every blob the HOST hands over by value: it owns the bytes and knows
    /// how many there are. The core reaching a host string through a transcoder does not,
    /// which is what `begin`/`end` exist for.
    #[inline(always)]
    pub fn blob_field(&mut self, tag: u32, b: &[u8]) {
        // E5: one reservation for key, length and body.
        self.buf.reserve(20 + b.len());
        unsafe {
            let len = self.buf.len();
            let p = self.tail();
            let k = put_varint(p, key(tag, WIRE_LEN));
            let n = put_varint(p.add(k), b.len() as u64);
            core::ptr::copy_nonoverlapping(b.as_ptr(), p.add(k + n), b.len());
            self.buf.set_len(len + k + n + b.len());
        }
    }

    /// Open a length-delimited field whose body length is not yet known.
    #[inline(always)]
    pub fn begin(&mut self, tag: u32, site: u32) -> Mark {
        self.key(tag, WIRE_LEN);
        let w = self.width(site) as usize;
        let hdr = self.buf.len();
        // T1: `BytesMut::resize` is a call (see `reset`); a learned width is at most 10
        // bytes, so zero 16 (a constant-size store) and take `w` of them.
        debug_assert!(w <= 16);
        self.buf.reserve(16);
        unsafe {
            core::ptr::write_bytes(self.tail(), 0, 16);
            self.buf.set_len(hdr + w);
        }
        Mark { site, hdr, w }
    }

    /// Close it, resolving the prefix width now that the body is written. A miss moves the
    /// body; it is never padded, because padding to a learned width makes the encoder's
    /// output depend on its own history (ABI v1 section 6 refuses it explicitly).
    #[inline(always)]
    pub fn end(&mut self, m: Mark) {
        let body = self.buf.len() - m.hdr - m.w;
        let need = varint_len(body as u64);

        // ABI v1 section 6's SECOND refusal, built so it can be seen failing (feature
        // `pad-widths`, off by default). Instead of moving the body when the guess was too
        // wide, keep the reservation and write a NON-MINIMAL varint into it. That is legal
        // wire -- every parser accepts a padded varint -- and it is exactly why it is
        // refused: the encoder's output now depends on its own history, so two threads of
        // one process emit two different legal encodings of one message, and a byte-vector
        // corpus cannot express either.
        //
        // This is the refusal that corrupts BYTES. The process-global table (feature
        // `global-widths`) is a data race and a throughput defect and not a byte defect,
        // because this branch is what rewrites the prefix to the width the body actually
        // needs whatever the guess was. The two are independent and only their combination
        // is the worst case; the cpp slice separated them first
        // (`ffi/logs/cpp/concurrency.log`) and section 6 was rewritten to say so.
        #[cfg(feature = "pad-widths")]
        if need < m.w {
            let mut v = body as u64;
            for i in 0..m.w {
                let last = i + 1 == m.w;
                self.buf[m.hdr + i] = ((v as u8) & 0x7f) | if last { 0 } else { 0x80 };
                v >>= 7;
            }
            return;
        }

        if need != m.w {
            self.resize_prefix(&m, body, need);
        }
        let mut v = body as u64;
        let mut i = m.hdr;
        while v >= 0x80 {
            self.buf[i] = (v as u8) | 0x80;
            v >>= 7;
            i += 1;
        }
        self.buf[i] = v as u8;
    }

    #[cold]
    fn resize_prefix(&mut self, m: &Mark, body: usize, need: usize) {
        crate::bump!(self.c, prefix_moves);
        crate::bump!(self.c, prefix_bytes, body);
        #[cfg(feature = "count")]
        {
            self.site_moves[m.site as usize] += 1;
        }
        self.set_width(m.site, need as u8);
        let src = m.hdr + m.w;
        if need > m.w {
            self.buf.resize(self.buf.len() + (need - m.w), 0);
        }
        let dst = m.hdr + need;
        self.buf.copy_within(src..src + body, dst);
        if need < m.w {
            self.buf.truncate(dst + body);
        }
    }

    /// What the transcoder is handed: the whole remaining buffer, not a per-string
    /// reservation, so a grow is the exception and not the rhythm (ABI v1 section 4).
    #[inline(always)]
    pub fn space(&mut self) -> (*mut u8, i32) {
        let len = self.buf.len();
        let cap = (self.buf.capacity() - len).min(i32::MAX as usize) as i32;
        self.last_cap = cap;
        (self.tail(), cap)
    }

    /// The transcoder asked for more. May move the buffer.
    #[inline]
    pub fn grow(&mut self, want: i32) -> (*mut u8, i32) {
        crate::bump!(self.c, grows);
        self.buf.reserve(want.max(0) as usize);
        self.space()
    }

    /// Accept what a transcoder wrote. The codec refuses a count larger than the capacity
    /// it gave, because nothing can make a transcoder that writes past its buffer safe
    /// (ABI v1 section 4, AK_ERR_CAPACITY).
    #[inline(always)]
    pub fn commit(&mut self, n: usize) -> bool {
        if n > self.last_cap as usize {
            self.fail(crate::ERR_CAPACITY);
            return false;
        }
        unsafe { self.buf.set_len(self.buf.len() + n) };
        true
    }
}

#[cfg(test)]
mod take_tests {
    use super::Enc;

    fn fill(e: &mut Enc, n: usize) {
        e.reset();
        for i in 0..n {
            e.varint_field(1, i as u64);
        }
    }

    /// T1: `take` hands out exactly the encoded bytes, without a copy (the `Bytes` points
    /// at the buffer the encode wrote), and after the `Bytes` is dropped the next `reset`
    /// reclaims that allocation: the next encode writes where the first one did.
    #[test]
    fn take_is_a_split_and_reset_reclaims() {
        let mut e = Enc::new(1);
        fill(&mut e, 5000);
        let want = e.buf.to_vec();
        let p0 = e.buf.as_ptr();
        let b = e.take();
        assert_eq!(&b[..], &want[..]);
        assert_eq!(b.as_ptr(), p0, "take copied");
        assert_eq!(e.buf.len(), 0);
        drop(b);
        fill(&mut e, 5000);
        assert_eq!(&e.buf[..], &want[..]);
        assert_eq!(e.buf.as_ptr(), p0, "reset did not reclaim the dropped body's allocation");
    }

    /// With the taken `Bytes` still held, the next `reset` allocates ONE buffer of at least
    /// the taken length, and the held bytes are untouched by the next encode.
    #[test]
    fn take_held_allocates_once() {
        let mut e = Enc::new(1);
        fill(&mut e, 5000);
        let want = e.buf.to_vec();
        let b = e.take();
        e.reset();
        assert!(e.buf.capacity() >= want.len());
        let p1 = e.buf.as_ptr();
        for i in 0..5000 {
            e.varint_field(1, (i + 1) as u64);
        }
        assert_eq!(e.buf.as_ptr(), p1, "the encode after a held take regrew");
        assert_eq!(&b[..], &want[..]);
    }

    /// The reused-buffer form never splits: no `take`, and the buffer stays put.
    #[test]
    fn reused_buffer_stays_put() {
        let mut e = Enc::new(1);
        fill(&mut e, 5000);
        let p0 = e.buf.as_ptr();
        fill(&mut e, 5000);
        assert_eq!(e.buf.as_ptr(), p0);
    }
}
