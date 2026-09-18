//! Boundary-call counters (README R5).
//!
//! A thread-local would be a hidden global with a re-entrancy hazard, which ABI v1 section
//! 7.3 refuses for the arena and which applies here for the same reason; these live in the
//! encode and decode contexts and are read through one entry point. Compiled out entirely
//! unless the `count` feature is on, so the timed build carries none of it.

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Counters {
    pub forward: u64,
    pub reverse: u64,
    pub transcode: u64,
    pub prefix_moves: u64,
    /// Bytes memmoved by those resizes. A move of a 24 KB element body and a move of a
    /// 30-byte map entry are both one miss and are not the same cost.
    pub prefix_bytes: u64,
    pub grows: u64,
}

#[macro_export]
macro_rules! bump {
    ($c:expr, $field:ident) => {
        #[cfg(feature = "count")]
        {
            $c.$field += 1;
        }
    };
    ($c:expr, $field:ident, $n:expr) => {
        #[cfg(feature = "count")]
        {
            $c.$field += $n as u64;
        }
    };
}
