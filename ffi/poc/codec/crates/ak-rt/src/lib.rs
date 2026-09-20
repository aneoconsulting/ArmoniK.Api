//! The codec's runtime support, shared by the two backends the one traversal emitter
//! produces: the `core-native` arm (emitted into the host, no boundary) and the core
//! behind the C ABI. It is runtime support, not a codec: every per-message encoder and
//! decoder in this slice is generated.
//!
//! Sharing it is what makes `core-native` a control for `core-ffi-rust` rather than a
//! different codec: the buffer, the varints and the learned length widths are the same
//! code in both, and the only difference between the arms is where the values come from.

pub mod counters;
pub mod enc;

pub use bdr::Bdr;
pub use counters::Counters;
pub use enc::{Enc, Mark};

/// Bytes a varint needs.
#[inline(always)]
pub const fn varint_len(v: u64) -> usize {
    ((63 - (v | 1).leading_zeros() as usize) / 7) + 1
}

#[inline(always)]
pub const fn key(tag: u32, wire: u32) -> u64 {
    ((tag << 3) | wire) as u64
}

pub const WIRE_VARINT: u32 = 0;
pub const WIRE_I64: u32 = 1;
pub const WIRE_LEN: u32 = 2;
pub const WIRE_I32: u32 = 5;

pub mod bdr;
pub mod dec;
pub mod strings;

/// Mirrors ak-abi's AK_ERR_CAPACITY. ak-rt does not depend on ak-abi: the native arm uses
/// this crate without any C ABI in the picture at all, which is the point of that arm.
pub const ERR_CAPACITY: i32 = -7;
pub const ERR_MALFORMED: i32 = -2;
pub const ERR_TRUNCATED: i32 = -3;
/// Mirrors ak-abi's AK_ERR_DEPTH: the decode recursion limit, which nested unknown
/// GROUP fields are the one path in this crate that can reach.
pub const ERR_DEPTH: i32 = -4;
/// ABI v1 section 6: "the transcoder refused its input". The design text names this code
/// for the malformed-string case (section, decision 3); `AK_ERR_MALFORMED` ("invalid wire")
/// is the other defensible reading, since proto3 makes invalid UTF-8 a PARSE error and the
/// rejecting path has no transcoder on it. That choice is not a slice's to settle.
pub const ERR_TRANSCODE: i32 = -6;

/// ABI v1 section 7.3: a run is materialised in a fixed-size arena sized as a BYTE BUDGET
/// divided by the group size, not an element count, so the scratch is the same 32 KB
/// whatever the schema does. Bounding at 32 KB measured free because the destination stays
/// in L2 however large the message is.
pub const ARENA_BYTES: usize = 32 * 1024;

pub const fn arena_n(group_size: usize) -> usize {
    let n = ARENA_BYTES / group_size;
    if n == 0 {
        1
    } else {
        n
    }
}
