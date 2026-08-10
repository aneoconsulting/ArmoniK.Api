//! Deterministic randomness for the harness: seeds derive from the proto
//! name and iteration index, and are printed on failure so any case can be
//! replayed exactly.

pub struct SplitMix64(u64);

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    /// Uniform-ish value in `0..n` (modulo bias is irrelevant here).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }

    /// `true` with probability `permille`/1000.
    pub fn chance(&mut self, permille: u64) -> bool {
        self.below(1000) < permille
    }
}

/// Deterministic seed for one (message, iteration) case.
pub fn seed(name: &str, iteration: u64) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in name.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash ^ iteration.wrapping_mul(0x9e3779b97f4a7c15)
}
