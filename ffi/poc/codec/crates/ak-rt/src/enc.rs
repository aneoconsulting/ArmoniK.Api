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

/// An open length prefix. Held by value so a miss cannot be attributed to the wrong site.
pub struct Mark {
    site: u32,
    /// Where the placeholder starts.
    hdr: usize,
    /// How many bytes were reserved for it.
    w: usize,
}

pub struct Enc {
    pub buf: Vec<u8>,
    /// One learned width per length-prefix site in the generated code. Per context: a
    /// global table made two encoding threads slower than one (ABI v1 section 6).
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
            widths: vec![1u8; sites].into_boxed_slice(),
            c: Counters::default(),
            err: 0,
            last_cap: 0,
            #[cfg(feature = "count")]
            site_moves: vec![0u32; sites].into_boxed_slice(),
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.buf.clear();
        self.err = 0;
        // The learned widths deliberately SURVIVE a reset: that is what makes them learned.
    }

    #[inline]
    pub fn fail(&mut self, code: i32) {
        if self.err == 0 {
            self.err = code;
        }
    }

    #[inline(always)]
    pub fn varint(&mut self, mut v: u64) {
        while v >= 0x80 {
            self.buf.push((v as u8) | 0x80);
            v >>= 7;
        }
        self.buf.push(v as u8);
    }

    #[inline(always)]
    pub fn key(&mut self, tag: u32, wire: u32) {
        self.varint(key(tag, wire));
    }

    #[inline(always)]
    pub fn varint_field(&mut self, tag: u32, v: u64) {
        self.key(tag, crate::WIRE_VARINT);
        self.varint(v);
    }

    #[inline(always)]
    pub fn f64_field(&mut self, tag: u32, v: f64) {
        self.key(tag, crate::WIRE_I64);
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    /// A length-delimited field whose length is known before the body is written, which is
    /// the case for every blob the HOST hands over by value: it owns the bytes and knows
    /// how many there are. The core reaching a host string through a transcoder does not,
    /// which is what `begin`/`end` exist for.
    #[inline(always)]
    pub fn blob_field(&mut self, tag: u32, b: &[u8]) {
        self.key(tag, WIRE_LEN);
        self.varint(b.len() as u64);
        self.buf.extend_from_slice(b);
    }

    /// Open a length-delimited field whose body length is not yet known.
    #[inline(always)]
    pub fn begin(&mut self, tag: u32, site: u32) -> Mark {
        self.key(tag, WIRE_LEN);
        let w = self.widths[site as usize] as usize;
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
        self.widths[m.site as usize] = need as u8;
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
