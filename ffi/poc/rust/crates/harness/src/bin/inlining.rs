//! Separating "was not inlined" from "crossed a boundary and materialised a group".
//!
//! **What is being audited.** The per-element interface cost quoted for M1 -- 40.5 ns on
//! P1.1, 44.9 on P1.2, 11.1 on P1.3 -- was obtained by subtracting `core-native` from
//! `core-ffi-rust`. `core-native` is compiled into this harness and the FFI arm is a
//! dynamic call into a cdylib, so that subtraction bundles the inlining advantage into the
//! interface cost. The figure is an upper bound and was not labelled as one.
//!
//! **Two no-boundary variants bracket the inlining term**, both at the granularity the FFI
//! call sits at (the per-message entry point, not the inner per-element traversal):
//!
//!   core-native            fused into the benchmark closure (zero call sites, see
//!                          gen/inline_check.sh)
//!   core-native-noinline   `#[inline(never)]`: a real call, same crate, rustc may still
//!                          reason about the body. A LOWER bound on the penalty.
//!   core-native-opaque     called through a function pointer that has been through
//!                          `black_box`: no inlining, no devirtualisation, no constant
//!                          propagation across. The closer model of the dynamic call,
//!                          MINUS the group.
//!   core-ffi-rust          the real thing: group materialisation + dynamic call.
//!
//! So the one-term subtraction becomes three:
//!
//!   native -> opaque       the cost of not being inlined
//!   opaque -> core-ffi     the group materialisation + the dynamic call
//!
//! prost is carried as the in-process control, because every ratio published for M1 is
//! against it.

use harness::arms::*;
use harness::manifest::{sha, Manifest};
use std::time::Instant;

const ROUNDS: usize = 15;
const TARGET_NS: u128 = 25_000_000;

/// Element count per payload: what the per-element figures divide by.
fn elems(pid: &str) -> f64 {
    match pid {
        P1_1 => 4.0,
        P1_2 => 1000.0,
        P1_3 => 300.0,
        _ => unreachable!(),
    }
}

const ARMS: [&str; 5] = [
    "prost",
    "core-native",
    "core-native-noinline",
    "core-native-opaque",
    "core-ffi-rust",
];

fn main() {
    let man = Manifest::load();
    println!("# the inlining term, separated from the interface term (M1, P1.1 to P1.3)");
    println!("#   guard: {}", if cfg!(feature = "guard") { "on" } else { "OFF" });
    println!("#   counting build: {}", if cfg!(feature = "count") { "YES -- timings NOT usable" } else { "no" });
    println!("#   rounds: {ROUNDS}, interleaved, one process (R4)");
    println!();

    let ctx: &'static core_ffi_arm::Ctx = Box::leak(Box::new(core_ffi_arm::Ctx::new()));

    // ---- 1. correctness first. The two new arms call the SAME generated traversal, so a
    //         byte difference here would be a defect in the wrapper and nothing else --
    //         which is exactly why it is checked rather than assumed.
    println!("## 1. byte identity against the validated manifest, and the decode round trip");
    println!("{:<7} {:<22} {:>9} {:>9} {:<8} {:<8} {}", "payload", "arm", "want B", "got B", "bytes", "sha256", "round trip");
    let mut bad = 0usize;
    for pid in [P1_1, P1_2, P1_3] {
        let row = man.row(pid);
        let v = armonik_arm::value(pid);
        let cases: [(&str, Vec<u8>, bool); 4] = [
            ("core-native", core_native_arm::encode(&v), core_native_arm::decode(&core_native_arm::encode(&v)) == v),
            ("core-native-noinline", core_native_noinline::encode(&v), core_native_noinline::decode(&core_native_noinline::encode(&v)) == v),
            ("core-native-opaque", core_native_opaque::encode(&v), core_native_opaque::decode(&core_native_opaque::encode(&v)) == v),
            ("core-ffi-rust", core_ffi_arm::encode(ctx, &v), core_ffi_arm::decode(ctx, &core_ffi_arm::encode(ctx, &v)) == v),
        ];
        for (arm, b, rt) in cases {
            let ok_b = b.len() == row.bytes;
            let ok_h = sha(&b) == row.sha256;
            println!("{:<7} {:<22} {:>9} {:>9} {:<8} {:<8} {}", pid, arm, row.bytes, b.len(),
                     if ok_b { "ok" } else { "DIFFER" },
                     if ok_h { "ok" } else { "DIFFER" },
                     if rt { "ok" } else { "FAILED" });
            bad += usize::from(!(ok_b && ok_h && rt));
        }
    }
    if bad > 0 {
        println!("\nFAILED: {bad} disagreement(s). Nothing below is usable.");
        std::process::exit(1);
    }

    // ---- 2. timings.
    let mut cases: Vec<Case> = Vec::new();
    for pid in [P1_1, P1_2, P1_3] {
        {
            let v: &'static _ = Box::leak(Box::new(prost_arm::value(pid)));
            let buf: &'static mut Vec<u8> = Box::leak(Box::new(Vec::with_capacity(1 << 20)));
            cases.push(mk(pid, "encode", "prost", move |n| {
                for _ in 0..n { buf.clear(); prost::Message::encode(v, buf).unwrap(); std::hint::black_box(&buf); }
            }));
        }
        {
            let v: &'static _ = Box::leak(Box::new(armonik_arm::value(pid)));
            let e: &'static mut ak_rt::Enc = Box::leak(Box::new(ak_rt::Enc::new(facade::generated::core_native::SITES)));
            cases.push(mk(pid, "encode", "core-native", move |n| {
                for _ in 0..n { facade::generated::core_native::encode_into_list_results_response(v, e); std::hint::black_box(&e.buf); }
            }));
        }
        {
            let v: &'static _ = Box::leak(Box::new(armonik_arm::value(pid)));
            let e: &'static mut ak_rt::Enc = Box::leak(Box::new(ak_rt::Enc::new(facade::generated::core_native::SITES)));
            cases.push(mk(pid, "encode", "core-native-noinline", move |n| {
                for _ in 0..n { core_native_noinline::encode_into(v, e); std::hint::black_box(&e.buf); }
            }));
        }
        {
            let v: &'static _ = Box::leak(Box::new(armonik_arm::value(pid)));
            let e: &'static mut ak_rt::Enc = Box::leak(Box::new(ak_rt::Enc::new(facade::generated::core_native::SITES)));
            // Taken ONCE, here, outside every timed region.
            let f = core_native_opaque::enc_fn();
            cases.push(mk(pid, "encode", "core-native-opaque", move |n| {
                for _ in 0..n { f(v, e); std::hint::black_box(&e.buf); }
            }));
        }
        {
            let v: &'static _ = Box::leak(Box::new(armonik_arm::value(pid)));
            cases.push(mk(pid, "encode", "core-ffi-rust", move |n| {
                for _ in 0..n { std::hint::black_box(core_ffi_arm::encode_into(ctx, v)); }
            }));
        }

        let bytes: &'static [u8] = Box::leak(prost_arm::encode(&prost_arm::value(pid)).into_boxed_slice());
        cases.push(mk(pid, "decode", "prost", move |n| {
            for _ in 0..n { std::hint::black_box(prost_arm::decode(bytes)); }
        }));
        cases.push(mk(pid, "decode", "core-native", move |n| {
            for _ in 0..n { std::hint::black_box(core_native_arm::decode(bytes)); }
        }));
        cases.push(mk(pid, "decode", "core-native-noinline", move |n| {
            for _ in 0..n { std::hint::black_box(core_native_noinline::decode(bytes)); }
        }));
        {
            let f = core_native_opaque::dec_fn();
            cases.push(mk(pid, "decode", "core-native-opaque", move |n| {
                for _ in 0..n { std::hint::black_box(f(bytes).unwrap()); }
            }));
        }
        cases.push(mk(pid, "decode", "core-ffi-rust", move |n| {
            for _ in 0..n { std::hint::black_box(core_ffi_arm::decode(ctx, bytes)); }
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
    let get = |cases: &Vec<Case>, pid: &str, dir: &str, arm: &str| -> f64 {
        med(cases.iter().find(|c| c.payload == pid && c.dir == dir && c.arm == arm).unwrap())
    };

    println!();
    println!("## 2. medians, all five arms, one process, interleaved");
    println!("{:<7} {:<7} {:<22} {:>12} {:>12} {:>9} {:>9}", "payload", "dir", "arm", "median ns", "min ns", "/ prost", "spread %");
    for pid in [P1_1, P1_2, P1_3] {
        for dir in ["encode", "decode"] {
            let base = get(&cases, pid, dir, "prost");
            for arm in ARMS {
                let c = cases.iter().find(|c| c.payload == pid && c.dir == dir && c.arm == arm).unwrap();
                let m = med(c);
                let lo = c.times.iter().cloned().fold(f64::INFINITY, f64::min);
                let hi = c.times.iter().cloned().fold(0.0f64, f64::max);
                println!("{:<7} {:<7} {:<22} {:>12.1} {:>12.1} {:>9.3} {:>9.1}",
                         pid, dir, arm, m, lo, m / base, 100.0 * (hi - lo) / m);
            }
            println!();
        }
    }

    println!("## 3. the decomposition, per element");
    println!("#   inlining      = (core-native-opaque - core-native) / elements");
    println!("#   inlining_lo   = (core-native-noinline - core-native) / elements  [lower bound]");
    println!("#   group+call    = (core-ffi-rust - core-native-opaque) / elements");
    println!("#   old figure    = (core-ffi-rust - core-native) / elements, which is the sum");
    println!();
    println!("{:<7} {:<7} {:>6} {:>12} {:>13} {:>13} {:>13} {:>10}",
             "payload", "dir", "elems", "inlining ns", "inlining_lo", "group+call", "old figure", "inlining %");
    for pid in [P1_1, P1_2, P1_3] {
        for dir in ["encode", "decode"] {
            let n = elems(pid);
            let native = get(&cases, pid, dir, "core-native");
            let noinl = get(&cases, pid, dir, "core-native-noinline");
            let opaque = get(&cases, pid, dir, "core-native-opaque");
            let ffi = get(&cases, pid, dir, "core-ffi-rust");
            let inl = (opaque - native) / n;
            let inl_lo = (noinl - native) / n;
            let grp = (ffi - opaque) / n;
            let old = (ffi - native) / n;
            println!("{:<7} {:<7} {:>6.0} {:>12.3} {:>13.3} {:>13.3} {:>13.3} {:>9.1}%",
                     pid, dir, n, inl, inl_lo, grp, old, 100.0 * inl / old);
        }
    }
    println!();
    println!("# `inlining %` is what share of the previously quoted per-element interface cost");
    println!("#   is the inlining advantage rather than the interface. A negative `old figure`");
    println!("#   means the FFI arm was FASTER than core-native on that row, and the shares");
    println!("#   below it are not meaningful; read the three absolute columns instead.");
}

struct Case {
    payload: &'static str,
    dir: &'static str,
    arm: &'static str,
    reps: usize,
    run: Box<dyn FnMut(usize)>,
    times: Vec<f64>,
}

fn mk(payload: &'static str, dir: &'static str, arm: &'static str,
      run: impl FnMut(usize) + 'static) -> Case {
    Case { payload, dir, arm, reps: 1, run: Box::new(run), times: Vec::new() }
}
