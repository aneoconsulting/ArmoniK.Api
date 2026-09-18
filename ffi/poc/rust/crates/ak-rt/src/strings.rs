//! The decode side's UTF-8 policy, in one place, selected at build time.
//!
//! ABI v1 open decision 3, third framing: the encode-side check buys the encoder nothing
//! (a `string` field is a length prefix and a byte copy) and is redundant with the one the
//! parser must do anyway, because the decoder cannot trust bytes that came off a wire
//! anything may have written. So validation belongs here and nowhere else.
//!
//! Three policies, one per build, because a runtime branch per string would be a cost of its
//! own and would confound the measurement it exists to make:
//!
//! - default: **lossy**. What this slice shipped before and what a facade written the
//!   obvious way does: `from_utf8_lossy`, U+FFFD substituted, never fails. proto3 says a
//!   parser must validate, and this does not.
//! - `dec-reject`: **validate and reject**, `core::str::from_utf8`. What prost does, so it
//!   is the like-for-like comparison.
//! - `dec-reject-simd`: the same contract with `simdutf8::basic`. The validator question,
//!   moved to the side where validation is mandatory.

/// Materialise a decoded string. `Err` carries an ak error code.
#[inline(always)]
pub fn decode_str(b: &[u8]) -> Result<String, i32> {
    #[cfg(all(not(feature = "dec-reject"), not(feature = "dec-reject-simd")))]
    {
        // Lossy. Cannot fail, and that is the problem with it.
        Ok(String::from_utf8_lossy(b).into_owned())
    }
    #[cfg(feature = "dec-reject")]
    {
        match core::str::from_utf8(b) {
            Ok(s) => Ok(s.to_owned()),
            Err(_) => Err(crate::ERR_TRANSCODE),
        }
    }
    #[cfg(feature = "dec-reject-simd")]
    {
        match simdutf8::basic::from_utf8(b) {
            Ok(s) => Ok(s.to_owned()),
            Err(_) => Err(crate::ERR_TRANSCODE),
        }
    }
}

/// Which policy this build carries, for a log to name rather than infer.
pub const POLICY: &str = if cfg!(feature = "dec-reject-simd") {
    "reject (simdutf8::basic)"
} else if cfg!(feature = "dec-reject") {
    "reject (core::str::from_utf8)"
} else {
    "lossy (from_utf8_lossy, U+FFFD)"
};
