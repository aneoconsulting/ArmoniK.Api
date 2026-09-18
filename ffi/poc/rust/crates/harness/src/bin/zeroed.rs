//! ABI v1 open decision 9 candidate: the ZEROED-GROUP variant of the element fill.
//!
//! **The proposal.** The element-group array is zeroed once per chunk with a memset and the
//! host then assigns only the fields that are not at their default, instead of the total
//! fill section 6 requires.
//!
//! **What it reverses.** The total fill was bought so the codec never has to reset the group
//! between elements -- worth 5.4 ns per `ResultRaw` and 24.4 per `TaskDetailed`. This pays
//! that back on every element of every payload in order to save the scattered stores on the
//! empty ones. So it is not a free win on the absent path; it is a trade, and the question
//! is where the cross-over sits relative to traffic that is actually moved.
//!
//! **Where it should win**: P1.3 and P2.5, the absent-path payloads.
//! **Where it should lose**: P1.2 and P2.2. P2.2 is the one that decides it, because that
//! is the shape the control plane actually moves.
//!
//! **This is an arm, not an adoption.** It changes the ABI contract, so the generator emits
//! it alongside the default and nothing the default path uses changed. Only the TOP-LEVEL
//! element group is built this way; a nested loop inside an element keeps the total fill,
//! so on M2 this prices the outer `TaskDetailed` group and not the inner runs.

use harness::arms as m1;
use harness::arms_m2 as m2;
use harness::arms_m3 as m3;
use harness::manifest::{sha, Manifest};
use std::time::Instant;

const ROUNDS: usize = 15;
const TARGET_NS: u128 = 25_000_000;

/// Elements per payload, for the per-element column.
fn elems(pid: &str) -> f64 {
    match pid {
        "P1.1" => 4.0, "P1.2" => 1000.0, "P1.3" => 300.0,
        "P2.1" => 1.0, "P2.2" => 500.0, "P2.3" => 500.0, "P2.4" => 500.0, "P2.5" => 500.0,
        _ => unreachable!(),
    }
}

fn main() {
    let man = Manifest::load();
    println!("# the zeroed-group variant (ABI v1 open decision 9 candidate), as an ARM");
    println!("#   guard: {}", if cfg!(feature = "guard") { "on" } else { "OFF" });
    println!("#   counting build: {}", if cfg!(feature = "count") { "YES -- timings NOT usable" } else { "no" });
    println!("#   rounds: {ROUNDS}, interleaved, one process (R4)");
    println!();

    let ctx: &'static _ = Box::leak(Box::new(m1::core_ffi_arm::Ctx::new()));

    // ---- 1. correctness first, on EVERY M1 and M2 payload, not only the timed ones.
    println!("## 1. byte identity against the validated manifest, total fill and zeroed");
    println!("{:<7} {:<18} {:>9} {:>9} {:<8} {:<8}", "payload", "arm", "want B", "got B", "bytes", "sha256");
    let mut bad = 0usize;
    for pid in ["P1.1", "P1.2", "P1.3"] {
        let row = man.row(pid);
        let v = m1::armonik_arm::value(pid);
        for (arm, b) in [("core-ffi-rust", m1::core_ffi_arm::encode(ctx, &v)),
                         ("core-ffi-zeroed", m1::core_ffi_zeroed::encode(ctx, &v))] {
            let (ok_b, ok_h) = (b.len() == row.bytes, sha(&b) == row.sha256);
            println!("{:<7} {:<18} {:>9} {:>9} {:<8} {:<8}", pid, arm, row.bytes, b.len(),
                     if ok_b { "ok" } else { "DIFFER" }, if ok_h { "ok" } else { "DIFFER" });
            bad += usize::from(!(ok_b && ok_h));
        }
    }
    // M3 is where a sparse fill can be WRONG and not merely slow: explicit presence and the
    // oneof. `Some(0)` and `Some("")` are at their default VALUE and must still be written,
    // so the generated fill tests presence for those fields, and this row is what catches it
    // if it ever stops doing so.
    for pid in m3::ALL {
        let row = man.row(pid);
        let v = m3::armonik_arm::value(pid);
        for (arm, b) in [("core-ffi-rust", m3::core_ffi_arm::encode(ctx, &v)),
                         ("core-ffi-zeroed", m3::core_ffi_zeroed::encode(ctx, &v))] {
            let (ok_b, ok_h) = (b.len() == row.bytes, sha(&b) == row.sha256);
            println!("{:<7} {:<18} {:>9} {:>9} {:<8} {:<8}", pid, arm, row.bytes, b.len(),
                     if ok_b { "ok" } else { "DIFFER" }, if ok_h { "ok" } else { "DIFFER" });
            bad += usize::from(!(ok_b && ok_h));
        }
    }
    for pid in m2::ALL {
        let row = man.row(pid);
        let v = m2::armonik_arm::value(pid);
        for (arm, b) in [("core-ffi-rust", m2::core_ffi_arm::encode(ctx, &v)),
                         ("core-ffi-zeroed", m2::core_ffi_zeroed::encode(ctx, &v))] {
            let (ok_b, ok_h) = (b.len() == row.bytes, sha(&b) == row.sha256);
            println!("{:<7} {:<18} {:>9} {:>9} {:<8} {:<8}", pid, arm, row.bytes, b.len(),
                     if ok_b { "ok" } else { "DIFFER" }, if ok_h { "ok" } else { "DIFFER" });
            bad += usize::from(!(ok_b && ok_h));
        }
    }
    if bad > 0 {
        println!("\nFAILED: {bad} disagreement(s). Nothing below is usable.");
        std::process::exit(1);
    }

    // ---- 2. timings.
    let mut cases: Vec<Case> = Vec::new();
    for pid in ["P1.2", "P1.3"] {
        {
            let v: &'static _ = Box::leak(Box::new(m1::prost_arm::value(pid)));
            let buf: &'static mut Vec<u8> = Box::leak(Box::new(Vec::with_capacity(1 << 20)));
            cases.push(mk(pid, "prost", move |n| {
                for _ in 0..n { buf.clear(); prost::Message::encode(v, buf).unwrap(); std::hint::black_box(&buf); }
            }));
        }
        let v: &'static _ = Box::leak(Box::new(m1::armonik_arm::value(pid)));
        cases.push(mk(pid, "core-ffi-rust", move |n| {
            for _ in 0..n { std::hint::black_box(m1::core_ffi_arm::encode_into(ctx, v)); }
        }));
        cases.push(mk(pid, "core-ffi-zeroed", move |n| {
            for _ in 0..n { std::hint::black_box(m1::core_ffi_zeroed::encode_into(ctx, v)); }
        }));
    }
    for pid in ["P2.2", "P2.5"] {
        {
            let v: &'static _ = Box::leak(Box::new(m2::prost_arm::value(pid)));
            let buf: &'static mut Vec<u8> = Box::leak(Box::new(Vec::with_capacity(4 << 20)));
            cases.push(mk(pid, "prost", move |n| {
                for _ in 0..n { buf.clear(); prost::Message::encode(v, buf).unwrap(); std::hint::black_box(&buf); }
            }));
        }
        let v: &'static _ = Box::leak(Box::new(m2::armonik_arm::value(pid)));
        cases.push(mk(pid, "core-ffi-rust", move |n| {
            for _ in 0..n { std::hint::black_box(m2::core_ffi_arm::encode_into(ctx, v)); }
        }));
        cases.push(mk(pid, "core-ffi-zeroed", move |n| {
            for _ in 0..n { std::hint::black_box(m2::core_ffi_zeroed::encode_into(ctx, v)); }
        }));
    }

    for c in cases.iter_mut() {
        let mut n = 1usize;
        let mut ns = 0u128;
        while ns < 2_000_000 && n < 20_000_000 {
            let t = Instant::now();
            (c.run)(n);
            ns = t.elapsed().as_nanos().max(1);
            if ns >= 2_000_000 { break; }
            n *= 4;
        }
        c.reps = ((TARGET_NS / (ns / n as u128).max(1)) as usize).clamp(1, 50_000_000);
        (c.run)(c.reps.min(200));
    }
    for _ in 0..ROUNDS {
        for c in cases.iter_mut() {
            let t = Instant::now();
            (c.run)(c.reps);
            c.times.push(t.elapsed().as_nanos() as f64 / c.reps as f64);
        }
    }

    let med = |c: &Case| {
        let mut t = c.times.clone();
        t.sort_by(|a, b| a.partial_cmp(b).unwrap());
        t[t.len() / 2]
    };
    let get = |cs: &Vec<Case>, pid: &str, arm: &str| -> f64 {
        med(cs.iter().find(|c| c.payload == pid && c.arm == arm).unwrap())
    };

    println!();
    println!("## 2. encode medians, one process, interleaved");
    println!("{:<7} {:<18} {:>12} {:>12} {:>9} {:>9}", "payload", "arm", "median ns", "min ns", "/ prost", "spread %");
    for pid in ["P1.2", "P1.3", "P2.2", "P2.5"] {
        let base = get(&cases, pid, "prost");
        for arm in ["prost", "core-ffi-rust", "core-ffi-zeroed"] {
            let c = cases.iter().find(|c| c.payload == pid && c.arm == arm).unwrap();
            let m = med(c);
            let lo = c.times.iter().cloned().fold(f64::INFINITY, f64::min);
            let hi = c.times.iter().cloned().fold(0.0f64, f64::max);
            println!("{:<7} {:<18} {:>12.1} {:>12.1} {:>9.3} {:>9.1}", pid, arm, m, lo, m / base, 100.0 * (hi - lo) / m);
        }
        println!();
    }

    println!("## 3. what the zeroed group costs or saves, per element");
    println!("#   positive = the zeroed variant is SLOWER than the total fill.");
    println!("#   The trade it reverses is 5.4 ns per ResultRaw and 24.4 per TaskDetailed.");
    println!();
    println!("{:<7} {:>6} {:>14} {:>14} {:>16} {:>10}", "payload", "elems", "total fill ns", "zeroed ns", "zeroed-total/el", "zeroed/tot");
    for pid in ["P1.2", "P1.3", "P2.2", "P2.5"] {
        let n = elems(pid);
        let tot = get(&cases, pid, "core-ffi-rust");
        let zer = get(&cases, pid, "core-ffi-zeroed");
        println!("{:<7} {:>6.0} {:>14.1} {:>14.1} {:>16.3} {:>10.3}", pid, n, tot, zer, (zer - tot) / n, zer / tot);
    }
}

struct Case {
    payload: &'static str,
    arm: &'static str,
    reps: usize,
    run: Box<dyn FnMut(usize)>,
    times: Vec<f64>,
}

fn mk(payload: &'static str, arm: &'static str, run: impl FnMut(usize) + 'static) -> Case {
    Case { payload, arm, reps: 1, run: Box::new(run), times: Vec::new() }
}
