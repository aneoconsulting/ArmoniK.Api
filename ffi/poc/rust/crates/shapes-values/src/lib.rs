//! The deterministic value rules of `ffi/schema/emit/values.py`, re-derived in Rust.
//!
//! Deliberately hand-written from the rules the schema README and `values.py` state, and
//! deliberately NOT generated from the Python. Stage 1 exists to check `emit/wire.py`'s
//! semantics with prost; a Rust side transliterated automatically from the Python side
//! would share whatever the Python side assumed. An independent re-derivation of the same
//! stated rules is the thing that can disagree.
//!
//! No RNG: a value is a pure function of (field path, element index).

use sha2::{Digest, Sha256};

pub const VOCAB: [&str; 16] = [
    "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel", "india", "juliet",
    "kilo", "lima", "mike", "november", "oscar", "papa",
];

fn digest(s: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    h.finalize().into()
}

fn hex(d: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for b in d {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// `int.from_bytes(sha256("path#idx")[:8], "little")`
pub fn h64(path: &str, idx: i64) -> u64 {
    let d = digest(&format!("{path}#{idx}"));
    u64::from_le_bytes(d[0..8].try_into().unwrap())
}

/// 36 ASCII characters, the shape of every id in the real schema.
pub fn guid(path: &str, idx: i64) -> String {
    let d = hex(&digest(&format!("{path}#{idx}")));
    format!(
        "{}-{}-{}-{}-{}",
        &d[0..8],
        &d[8..12],
        &d[12..16],
        &d[16..20],
        &d[20..32]
    )
}

pub fn word(path: &str, idx: i64) -> String {
    let h = h64(path, idx);
    format!("{}{}", VOCAB[(h % 16) as usize], h % 1000)
}

pub fn sentence(path: &str, idx: i64) -> String {
    let h = h64(path, idx);
    (0..5)
        .map(|i| VOCAB[((h >> (4 * i)) % 16) as usize])
        .collect::<Vec<_>>()
        .join(" ")
}

/// 16 bytes by default: sha256("path#idx#i") concatenated, truncated.
pub fn blob(path: &str, idx: i64, n: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(n + 32);
    let mut i = 0;
    while out.len() < n {
        out.extend_from_slice(&digest(&format!("{path}#{idx}#{i}")));
        i += 1;
    }
    out.truncate(n);
    out
}

/// The bulk body of M5: sha256("bulk#i") concatenated, truncated.
pub fn bulk(n: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(n + 32);
    let mut i = 0usize;
    while out.len() < n {
        let mut h = Sha256::new();
        h.update(format!("bulk#{i}").as_bytes());
        out.extend_from_slice(&h.finalize());
        i += 1;
    }
    out.truncate(n);
    out
}

pub fn scalar_i32(path: &str, idx: i64) -> i32 {
    (h64(path, idx) % 100_000) as i32
}

pub fn scalar_i64(path: &str, idx: i64) -> i64 {
    (h64(path, idx) % 1_000_000_000_000) as i64
}

pub fn scalar_bool(path: &str, idx: i64) -> bool {
    h64(path, idx) & 1 != 0
}

pub fn scalar_f64(path: &str, idx: i64) -> f64 {
    (h64(path, idx) % 1_000_000) as f64 / 1000.0
}

/// Declaration order of `ResultStatus`. 127 is why the enum is in the schema at all:
/// a two-byte varint that must round-trip.
pub const RESULT_STATUS: [i32; 6] = [0, 1, 2, 3, 4, 127];
/// Declaration order of `TaskStatus`.
pub const TASK_STATUS: [i32; 14] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13];

pub fn enum_value(values: &[i32], idx: i64) -> i32 {
    values[(idx as usize) % values.len()]
}

pub fn timestamp(idx: i64) -> (i64, i32) {
    (1_700_000_000 + idx * 37, ((idx * 7919) % 1_000_000_000) as i32)
}

pub fn duration(idx: i64) -> (i64, i32) {
    (idx % 3600, ((idx * 104_729) % 1_000_000_000) as i32)
}
