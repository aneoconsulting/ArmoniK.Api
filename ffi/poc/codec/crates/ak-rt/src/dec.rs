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
        // `pos <= buf.len()` always holds (every read advances `pos` only after a bounds
        // check), so `buf.len() - pos` cannot underflow; comparing the REMAINING bytes
        // against 8 avoids the `pos + 8` overflow the checked form has near `usize::MAX`.
        if self.buf.len() - self.pos < 8 {
            self.err = crate::ERR_TRUNCATED;
            return 0.0;
        }
        let v = f64::from_le_bytes(self.buf[self.pos..self.pos + 8].try_into().unwrap());
        self.pos += 8;
        v
    }

    /// A `fixed32` value (wire type 5). Same remaining-bytes check as `f64`.
    #[inline(always)]
    pub fn fixed32(&mut self) -> u32 {
        if self.buf.len() - self.pos < 4 {
            self.err = crate::ERR_TRUNCATED;
            return 0;
        }
        let v = u32::from_le_bytes(self.buf[self.pos..self.pos + 4].try_into().unwrap());
        self.pos += 4;
        v
    }

    /// The body of a length-delimited field, as (offset, length) into the ONE buffer the
    /// host handed in. That is what `ak_span` is (ABI v1 section 4) and what lets a host
    /// resolve a string against a base pointer it already holds (7.4).
    #[inline(always)]
    pub fn len_body(&mut self) -> (usize, usize) {
        let n = self.varint() as usize;
        // R-D1: the earlier `self.pos + n > self.buf.len()` wraps in release builds
        // (overflow-checks off), so a length varint near 2^64 passed the check, `pos`
        // moved backwards or off the end, and callers built `&buf[off..off + n]` with a
        // wrapped `n` -- a hang in the root loop, a span with len 0xFFFF_FFFF handed to
        // the host, or a panic across `extern "C"`. `pos <= buf.len()` always holds here
        // (`varint` advances `pos` only after a bounds check), so `buf.len() - pos` cannot
        // underflow, and comparing the REMAINING bytes against `n` cannot overflow. On
        // rejection it returns `(pos, 0)`, an empty in-bounds span, so no caller's slice
        // can panic even when the length was hostile.
        if n > self.buf.len() - self.pos {
            self.err = crate::ERR_TRUNCATED;
            return (self.pos, 0);
        }
        let off = self.pos;
        self.pos += n;
        (off, n)
    }

    /// An unknown field: protobuf's forward compatibility, and the path a corpus generated
    /// from the schema that reads it never executes (README section 10).
    ///
    /// `tag` is the field number the wire type arrived with, and it is not decoration: the
    /// deprecated GROUP form carries no length, so the only way to find a group's end is to
    /// read fields until an `END_GROUP` **whose field number matches the one that opened
    /// it**. A skipper that counts depth instead accepts a mismatched end tag and then
    /// mis-nests every group after it, which is corpus vector `X-group-mismatched-end`.
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
            // 4 is END_GROUP with nothing open; 6 and 7 do not exist.
            _ => self.err = crate::ERR_MALFORMED,
        }
        if self.pos > self.buf.len() {
            self.err = crate::ERR_TRUNCATED;
        }
    }

    /// The GROUP skip. Recursive, because groups nest, and bounded, because a payload of
    /// nothing but start tags would otherwise be a stack overflow rather than an error --
    /// which is what `AK_ERR_DEPTH` is for (ABI v1 section 5).
    fn skip_group(&mut self, tag: u32, depth: u32) {
        if depth >= Self::MAX_GROUP_DEPTH {
            self.err = crate::ERR_DEPTH;
            return;
        }
        loop {
            if self.err != 0 {
                return;
            }
            if self.pos >= self.buf.len() {
                // An unterminated group: `X-group-unterminated`.
                self.err = crate::ERR_TRUNCATED;
                return;
            }
            let k = self.varint();
            if self.err != 0 {
                return;
            }
            let (t, w) = ((k >> 3) as u32, (k & 7) as u32);
            // plan.MAX_FIELD_NUMBER, inside a group too (WP5 step 6).
            if t == 0 || (k >> 3) > crate::MAX_FIELD_NUMBER {
                self.err = crate::ERR_MALFORMED;
                return;
            }
            if w == 4 {
                if t != tag {
                    self.err = crate::ERR_MALFORMED;
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

    /// plan.GROUP_DEPTH_LIMIT: nested groups within ONE skipped field, counted apart from
    /// message depth (protobuf's own default recursion limit is the same number).
    const MAX_GROUP_DEPTH: u32 = 100;
}

#[cfg(test)]
mod len_body_tests {
    use super::Dec;

    /// R-D1: a length varint near 2^64 must be rejected, not wrapped. The 10-byte
    /// over-long varint below carries `u64::MAX`; the old `pos + n` wrapped and passed.
    #[test]
    fn a_near_2_64_length_is_truncated_not_wrapped() {
        let mut buf = vec![0xFFu8; 10];
        buf[9] = 0x01; // 10th byte's low bit -> value has bit 63 set, ~2^64
        let mut d = Dec::new(&buf);
        let (off, n) = d.len_body();
        assert_eq!(d.err, crate::ERR_TRUNCATED);
        assert_eq!(n, 0, "a rejected length yields an empty span");
        assert!(off <= d.buf.len(), "the span offset stays in bounds");
    }

    /// The offsets the returned span carries are always in bounds, so `&buf[off..off + n]`
    /// cannot panic even for a hostile length.
    #[test]
    fn the_returned_span_is_always_sliceable() {
        for tail in [u64::MAX, u64::MAX - 3, (u32::MAX as u64) + 1, 1 << 40] {
            let mut buf = Vec::new();
            for i in 0..10u32 {
                let mut b = ((tail >> (7 * i)) & 0x7f) as u8;
                if i < 9 {
                    b |= 0x80;
                }
                buf.push(b);
            }
            buf.extend([0u8; 4]);
            let mut d = Dec::new(&buf);
            let (off, n) = d.len_body();
            // must not panic:
            let _ = &d.buf[off..off + n];
        }
    }

    /// A well-formed length still works.
    #[test]
    fn a_valid_length_still_decodes() {
        let buf = [0x03u8, 0xaa, 0xbb, 0xcc, 0xdd];
        let mut d = Dec::new(&buf);
        let (off, n) = d.len_body();
        assert_eq!(d.err, 0);
        assert_eq!((off, n), (1, 3));
        assert_eq!(d.pos, 4);
    }
}

#[cfg(test)]
mod group_skip_tests {
    use super::Dec;

    /// A field header: field number and wire type, varint-encoded. Every tag in these
    /// tests is below 16, so one byte.
    fn tag(field: u32, wire: u32) -> u8 {
        assert!(field < 16);
        ((field << 3) | wire) as u8
    }

    /// What a generated decoder does with a field it does not know: read the header, then
    /// hand both halves to `skip`.
    fn skip_all(buf: &[u8]) -> (i32, usize) {
        let mut d = Dec::new(buf);
        while !d.at_end() {
            let k = d.varint();
            if d.err != 0 {
                break;
            }
            let (t, w) = ((k >> 3) as u32, (k & 7) as u32);
            // plan.MAX_FIELD_NUMBER, inside a group too (WP5 step 6).
            if t == 0 || (k >> 3) > crate::MAX_FIELD_NUMBER {
                d.err = crate::ERR_MALFORMED;
                break;
            }
            d.skip(t, w);
        }
        (d.err, d.pos)
    }

    #[test]
    fn a_group_is_skipped_whole() {
        // group 15 { varint 1 = 1; } then a known-shaped varint field after it, so the
        // test fails if the skip stops in the wrong place rather than only if it errors.
        let buf = [tag(15, 3), tag(1, 0), 0x01, tag(15, 4), tag(2, 0), 0x7f];
        assert_eq!(skip_all(&buf), (0, buf.len()));
    }

    #[test]
    fn groups_nest() {
        let buf = [
            tag(15, 3),
            tag(14, 3),
            tag(1, 0),
            0x01,
            tag(14, 4),
            tag(15, 4),
        ];
        assert_eq!(skip_all(&buf), (0, buf.len()));
    }

    #[test]
    fn an_unterminated_group_is_truncated_not_accepted() {
        // corpus X-group-unterminated
        let buf = [tag(15, 3), tag(1, 0), 0x01];
        assert_eq!(skip_all(&buf).0, crate::ERR_TRUNCATED);
    }

    #[test]
    fn a_mismatched_end_tag_is_malformed() {
        // corpus X-group-mismatched-end. A skipper that counts depth accepts this, which
        // is exactly the defect that vector exists to catch.
        let buf = [tag(15, 3), tag(1, 0), 0x01, tag(14, 4)];
        assert_eq!(skip_all(&buf).0, crate::ERR_MALFORMED);
    }

    #[test]
    fn a_mismatched_end_inside_a_nest_is_malformed() {
        let buf = [tag(15, 3), tag(14, 3), tag(13, 4), tag(15, 4)];
        assert_eq!(skip_all(&buf).0, crate::ERR_MALFORMED);
    }

    #[test]
    fn an_end_tag_with_nothing_open_is_malformed() {
        assert_eq!(skip_all(&[tag(15, 4)]).0, crate::ERR_MALFORMED);
    }

    #[test]
    fn nesting_past_the_limit_is_depth_not_a_stack_overflow() {
        let buf: Vec<u8> = (0..200).map(|_| tag(15, 3)).collect();
        assert_eq!(skip_all(&buf).0, crate::ERR_DEPTH);
    }

    /// WP5 step 6: a field number above 2^29 - 1 inside a skipped group is malformed, as
    /// upb and protobuf C++ refuse it (logs/rust/wp5s6-oracles.log).
    #[test]
    fn a_field_number_above_the_maximum_inside_a_group_is_malformed() {
        // group 15 { field 2^29, varint 1 } end 15
        let mut buf = vec![tag(15, 3)];
        let mut k: u64 = (1u64 << 29) << 3;
        while k >= 0x80 {
            buf.push((k as u8) | 0x80);
            k >>= 7;
        }
        buf.push(k as u8);
        buf.extend([0x01, tag(15, 4)]);
        assert_eq!(skip_all(&buf).0, crate::ERR_MALFORMED);
    }

    #[test]
    fn the_other_wire_types_still_skip() {
        let buf = [
            tag(1, 0),
            0x96,
            0x01, // varint 150
            tag(2, 1),
            0, 0, 0, 0, 0, 0, 0, 0, // 64-bit
            tag(3, 2),
            0x02,
            0xaa,
            0xbb, // length-delimited
            tag(4, 5),
            0, 0, 0, 0, // 32-bit
        ];
        assert_eq!(skip_all(&buf), (0, buf.len()));
    }
}
