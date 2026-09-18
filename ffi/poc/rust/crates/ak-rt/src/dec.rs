//! The decode reader. Shared by both backends of the one traversal emitter, so a wire-level
//! disagreement between the `core-native` arm and the core behind the ABI is impossible by
//! construction; what the two differ in is only where the decoded values are deposited.

pub struct Dec<'a> {
    pub buf: &'a [u8],
    pub pos: usize,
    pub err: i32,
}

impl<'a> Dec<'a> {
    #[inline]
    pub fn new(buf: &'a [u8]) -> Self {
        Dec { buf, pos: 0, err: 0 }
    }

    #[inline(always)]
    pub fn at_end(&self) -> bool {
        self.pos >= self.buf.len() || self.err != 0
    }

    #[inline(always)]
    pub fn varint(&mut self) -> u64 {
        let mut n = 0u64;
        let mut shift = 0u32;
        loop {
            if self.pos >= self.buf.len() {
                self.err = crate::ERR_TRUNCATED;
                return 0;
            }
            let c = self.buf[self.pos];
            self.pos += 1;
            n |= ((c & 0x7f) as u64) << shift;
            if c & 0x80 == 0 {
                return n;
            }
            shift += 7;
            if shift > 63 {
                self.err = crate::ERR_MALFORMED;
                return 0;
            }
        }
    }

    #[inline(always)]
    pub fn f64(&mut self) -> f64 {
        if self.pos + 8 > self.buf.len() {
            self.err = crate::ERR_TRUNCATED;
            return 0.0;
        }
        let v = f64::from_le_bytes(self.buf[self.pos..self.pos + 8].try_into().unwrap());
        self.pos += 8;
        v
    }

    /// The body of a length-delimited field, as (offset, length) into the ONE buffer the
    /// host handed in. That is what `ak_span` is (ABI v1 section 4) and what lets a host
    /// resolve a string against a base pointer it already holds (7.4).
    #[inline(always)]
    pub fn len_body(&mut self) -> (usize, usize) {
        let n = self.varint() as usize;
        if self.pos + n > self.buf.len() {
            self.err = crate::ERR_TRUNCATED;
            return (self.pos, 0);
        }
        let off = self.pos;
        self.pos += n;
        (off, n)
    }

    /// An unknown field: protobuf's forward compatibility, and the path a corpus generated
    /// from the schema that reads it never executes (README section 10).
    #[inline]
    pub fn skip(&mut self, wire: u32) {
        match wire {
            0 => {
                self.varint();
            }
            1 => self.pos += 8,
            2 => {
                self.len_body();
            }
            5 => self.pos += 4,
            _ => self.err = crate::ERR_MALFORMED,
        }
        if self.pos > self.buf.len() {
            self.err = crate::ERR_TRUNCATED;
        }
    }
}
