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
use bytes::Bytes;
use std::sync::{Arc, Mutex};

/// An open length prefix. Held by value so a miss cannot be attributed to the wrong site.
pub struct Mark {
    site: u32,
    /// Where the placeholder starts.
    hdr: usize,
    /// How many bytes were reserved for it.
    w: usize,
}

/// A taken body (`Enc::take`): returns its buffer to the context's spare ring when its last
/// `Bytes` clone is dropped, unless the ring is already full or locked (then it is freed).
struct Recycle {
    v: Vec<u8>,
    slot: Arc<Mutex<Vec<Vec<u8>>>>,
}

/// How many taken buffers the spare ring keeps (the stream probes, 2026-09-28: with ONE
/// spare, a transport that still holds the previous message when the next encode starts
/// left the slot empty and every such encode wrote into a fresh buffer; host encode per
/// 2 MiB chunk on the framed stream 506 / 437 / 396 / 401 us with 1 / 2 / 3 / 4 spares,
/// logs/rust/opt/framed-default/ring-size).
pub const SPARES: usize = 3;

/// Bytes kept free at the start of the buffer when `head` is on: the gRPC message prefix
/// (1 flag byte, 4 length bytes), written in place by `take_framed`.
pub const FRAME_HEAD: usize = 5;
impl AsRef<[u8]> for Recycle {
    fn as_ref(&self) -> &[u8] {
        &self.v
    }
}
impl Drop for Recycle {
    fn drop(&mut self) {
        if let Ok(mut g) = self.slot.try_lock() {
            if g.len() < SPARES {
                g.push(core::mem::take(&mut self.v));
            }
        }
    }
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
    /// The encoded message is `buf[head..]`.
    pub buf: Vec<u8>,
    /// 0, or FRAME_HEAD: bytes `reset` keeps free before the message so a transport can
    /// write the gRPC prefix in place and send prefix and message as ONE buffer
    /// (`take_framed`). The encode itself only appends, so nothing else sees them; every
    /// reader of the encoded message goes through `msg()` / `msg_len()` / `take()`.
    pub head: usize,
    /// Optimisation T1: the buffers taken bodies (`take`) hand back when their last `Bytes`
    /// clone is dropped -- on whatever thread the transport drops it, hence the lock -- so
    /// the next `take` swaps one in: neither the take nor the next encode copies or
    /// allocates a buffer while the ring has one. (A `BytesMut` split was tried first and
    /// cost 5-20% on the reused-buffer encode: logs/rust/opt/t1-native-ab.) A ring of
    /// SPARES rather than one slot: logs/rust/opt/stream-probe2.
    spare: Arc<Mutex<Vec<Vec<u8>>>>,
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
            buf: Vec::with_capacity(4096),
            head: 0,
            spare: Arc::new(Mutex::new(Vec::with_capacity(SPARES))),
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
        self.buf.clear();
        if self.head != 0 {
            self.buf.resize(self.head, 0);
        }
        self.err = 0;
        // The learned widths deliberately SURVIVE a reset: that is what makes them learned.
    }

    /// T1: the encoded bytes as a `Bytes`, MOVED out (no copy): O(1). The context gets the
    /// spare buffer a dropped body returned, or a fresh one of the same capacity when the
    /// previous body is still held; when this body's last clone is dropped its buffer
    /// becomes the spare. What a tonic request body is (cell F), what `ak_call_unary_enc`
    /// sends (cell C) and what `ak_enc_take_owned` hands a host.
    pub fn take(&mut self) -> Bytes {
        let h = self.head;
        let b = self.take_all();
        if h == 0 { b } else { b.slice(h..) }
    }

    /// The encoded message with its 5-byte gRPC prefix (flag 0, big-endian length) written
    /// into the headroom, as ONE buffer, moved out like `take`. Requires `head` ==
    /// FRAME_HEAD (else it is `take`, and the caller must frame it itself).
    pub fn take_framed(&mut self) -> Bytes {
        if self.head != FRAME_HEAD || self.buf.len() < FRAME_HEAD {
            return self.take();
        }
        let n = (self.buf.len() - FRAME_HEAD) as u32;
        self.buf[0] = 0;
        self.buf[1..FRAME_HEAD].copy_from_slice(&n.to_be_bytes());
        self.take_all()
    }

    fn take_all(&mut self) -> Bytes {
        let fresh = match self.spare.lock().ok().and_then(|mut g| g.pop()) {
            Some(mut v) => {
                v.clear();
                v
            }
            None => Vec::with_capacity(self.buf.capacity()),
        };
        let v = core::mem::replace(&mut self.buf, fresh);
        if self.head != 0 {
            self.buf.resize(self.head, 0);
        }
        Bytes::from_owner(Recycle { v, slot: self.spare.clone() })
    }

    /// The encoded message (after the headroom).
    #[inline]
    pub fn msg(&self) -> &[u8] {
        &self.buf[self.head.min(self.buf.len())..]
    }

    /// The encoded message's length.
    #[inline]
    pub fn msg_len(&self) -> usize {
        self.buf.len().saturating_sub(self.head)
    }

    /// Append bytes: reserve once, copy. Every append to `buf` from the core and the
    /// generated encoders goes through it.
    #[inline(always)]
    pub fn put(&mut self, b: &[u8]) {
        self.buf.extend_from_slice(b);
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
            let n = put_varint(self.buf.as_mut_ptr().add(len), v);
            self.buf.set_len(len + n);
        }
    }

    /// A packed run of varints (E5): one reservation for the whole run (at most 10 bytes an
    /// element), then raw stores.
    #[inline(always)]
    pub fn varint_run<I: Iterator<Item = u64>>(&mut self, n: usize, it: I) {
        self.buf.reserve(n.saturating_mul(10));
        unsafe {
            let base = self.buf.as_mut_ptr();
            let mut at = self.buf.len();
            for v in it.take(n) {
                at += put_varint(base.add(at), v);
            }
            self.buf.set_len(at);
        }
    }

    /// A packed run of doubles (E5): little-endian, so on a little-endian target it is one
    /// copy of the host's array.
    #[inline(always)]
    pub fn f64_run(&mut self, v: &[f64]) {
        #[cfg(target_endian = "little")]
        {
            let b = unsafe { core::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 8) };
            self.put(b);
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
            let p = self.buf.as_mut_ptr().add(len);
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
            let p = self.buf.as_mut_ptr().add(len);
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
        self.buf.resize(hdr + w, 0);
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
        unsafe { (self.buf.as_mut_ptr().add(len), cap) }
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

    /// T1: `take` hands out exactly the encoded bytes without a copy (the `Bytes` points at
    /// the buffer the encode wrote), and once it is dropped its buffer comes back: the take
    /// after next writes into the first buffer again (two buffers alternate).
    #[test]
    fn take_moves_and_buffers_alternate() {
        let mut e = Enc::new(1);
        fill(&mut e, 5000);
        let want = e.buf.clone();
        let p0 = e.buf.as_ptr();
        let b = e.take();
        assert_eq!(&b[..], &want[..]);
        assert_eq!(b.as_ptr(), p0, "take copied");
        assert!(e.buf.is_empty());
        drop(b);
        fill(&mut e, 5000);
        let p1 = e.buf.as_ptr();
        assert_eq!(&e.buf[..], &want[..]);
        drop(e.take());
        fill(&mut e, 5000);
        assert_eq!(e.buf.as_ptr(), p0, "the dropped body's buffer did not come back");
        let _ = p1;
    }

    /// With the taken `Bytes` still held, the next take allocates one fresh buffer of the
    /// same capacity, and the held bytes are untouched by the next encode.
    #[test]
    fn take_held_allocates_once() {
        let mut e = Enc::new(1);
        fill(&mut e, 5000);
        let want = e.buf.clone();
        let cap = e.buf.capacity();
        let b = e.take();
        assert!(e.buf.capacity() >= cap);
        fill(&mut e, 5000);
        assert_eq!(&b[..], &want[..]);
    }

    /// With the headroom on, the encoded bytes are the same as without it, `take` hands
    /// back the message alone, and `take_framed` the gRPC prefix and the message as one
    /// buffer, moved (no copy).
    #[test]
    fn headroom_and_take_framed() {
        let mut a = Enc::new(1);
        fill(&mut a, 5000);
        let want = a.buf.clone();
        let mut e = Enc::new(1);
        e.head = super::FRAME_HEAD;
        fill(&mut e, 5000);
        assert_eq!(e.msg(), &want[..]);
        assert_eq!(e.msg_len(), want.len());
        let p0 = e.buf.as_ptr();
        let b = e.take_framed();
        assert_eq!(b.as_ptr(), p0, "take_framed copied");
        assert_eq!(&b[..1], &[0u8]);
        assert_eq!(u32::from_be_bytes(b[1..5].try_into().unwrap()) as usize, want.len());
        assert_eq!(&b[5..], &want[..]);
        assert_eq!(e.buf.len(), super::FRAME_HEAD, "the next encode starts after the headroom");
        fill(&mut e, 5000);
        let m = e.take();
        assert_eq!(&m[..], &want[..]);
    }

    /// The reused-buffer form never takes, and the buffer stays put.
    #[test]
    fn reused_buffer_stays_put() {
        let mut e = Enc::new(1);
        fill(&mut e, 5000);
        let p0 = e.buf.as_ptr();
        fill(&mut e, 5000);
        assert_eq!(e.buf.as_ptr(), p0);
    }
}
