//! The content-set pass on the string path (`design/SHAPES.md`, "Content sets").
//!
//! "A slice that reports one string-path number without saying which content set it came
//! from has reported half a number." Every figure in `stage2-four-arms-M1.log` and
//! `stage3-M2.log` is ASCII, so this is the other half for M1 and M2.
//!
//! **What these sets mean in Rust is not what they mean on a managed host.** On .NET or the
//! JVM they make a narrowing transcoder do real work or fail outright, because the host
//! holds UTF-16. A Rust `String` is UTF-8 already: there is no narrowing and no transcoding.
//! What changes is the byte width of the same character count -- 1, 2 and 3 bytes -- and
//! which path `str::from_utf8` takes. So these rows price VALIDATION and WIDTH in Rust and
//! not a transcoder, and a Rust figure here must not be read across to a managed slice.
//!
//! No manifest oracle: `ffi/schema/` emits ASCII payloads only, so there are no hashes for
//! latin1 or wide. Correctness here is byte identity of the four arms against each other,
//! with the prost arm -- an independent encoder, validated against the manifest on ASCII --
//! as the reference the other three are compared to.

use harness::arms as m1;
use harness::arms_m2 as m2;
use shapes_values::{set_content_set, ContentSet};
use std::time::Instant;

const ROUNDS: usize = 11;
const TARGET_NS: u128 = 20_000_000;

const SETS: [(ContentSet, &str); 3] = [
    (ContentSet::Ascii, "ascii"),
    (ContentSet::Latin1, "latin1"),
    (ContentSet::Wide, "wide"),
];

fn main() {
    println!("# content sets on the string path, M1 (P1.2) and M2 (P2.2)");
    println!("#   guard: {}", if cfg!(feature = "guard") { "on" } else { "OFF" });
    println!("#   both messages are REAL (design/SHAPES.md). No control rows here.");
    println!();

    // Leaked so every closure can hold a 'static reference: the contexts live for the
    // whole run, which is also what keeps the learned length widths warm.
    let ctx: &'static _ = Box::leak(Box::new(m1::core_ffi_arm::Ctx::new()));
    let ctx_v: &'static _ = Box::leak(Box::new(m1::core_ffi_arm::Ctx::validating()));
    let ctx_s: &'static _ = Box::leak(Box::new(m1::core_ffi_arm::Ctx::validating_simd()));

    // ---- correctness first, for every set.
    println!("## byte identity across the arms, per content set");
    println!("#   reference: the prost arm. There is no manifest for latin1 or wide.");
    println!();
    println!("{:<7} {:<8} {:>10} {:>9} {:<10} {}", "payload", "set", "bytes", "B/ascii", "arms agree", "decode round trip");
    let mut bad = 0usize;
    let mut sizes = std::collections::BTreeMap::new();
    for (cs, name) in SETS {
        set_content_set(cs);
        for pid in ["P1.2", "P2.2"] {
            let (pb, ab, cb, fb, rt) = if pid == "P1.2" {
                let pv = m1::prost_arm::value(pid);
                let av = m1::armonik_arm::value(pid);
                (
                    m1::prost_arm::encode(&pv),
                    m1::armonik_arm::encode(&av),
                    m1::core_native_arm::encode(&av),
                    m1::core_ffi_arm::encode(ctx, &av),
                    m1::core_native_arm::decode(&m1::armonik_arm::encode(&av)) == av
                        && m1::core_ffi_arm::decode(ctx, &m1::armonik_arm::encode(&av)) == av,
                )
            } else {
                let pv = m2::prost_arm::value(pid);
                let av = m2::armonik_arm::value(pid);
                (
                    m2::prost_arm::encode(&pv),
                    m2::armonik_arm::encode(&av),
                    m2::core_native_arm::encode(&av),
                    m2::core_ffi_arm::encode(ctx, &av),
                    m2::core_native_arm::decode(&m2::armonik_arm::encode(&av)) == av
                        && m2::core_ffi_arm::decode(ctx, &m2::armonik_arm::encode(&av)) == av,
                )
            };
            let ok = ab == pb && cb == pb && fb == pb;
            sizes.insert((pid, name), pb.len());
            let base = *sizes.get(&(pid, "ascii")).unwrap_or(&pb.len()) as f64;
            println!(
                "{:<7} {:<8} {:>10} {:>9.2} {:<10} {}",
                pid, name, pb.len(), pb.len() as f64 / base,
                if ok { "ok" } else { "DIFFER" },
                if rt { "ok" } else { "FAILED" }
            );
            bad += usize::from(!(ok && rt));
        }
    }
    if bad > 0 {
        println!("\nFAILED: {bad} disagreement(s). Nothing below is usable.");
        std::process::exit(1);
    }

    // ---- timings. All three sets in ONE process (R4), interleaved.
    println!();
    println!("## encode and decode, all three sets, one process, rounds interleaved");
    println!();
    let mut cases: Vec<Case> = Vec::new();
    for (cs, name) in SETS {
        set_content_set(cs);
        {
            let v: &'static _ = Box::leak(Box::new(m1::prost_arm::value("P1.2")));
            let av: &'static _ = Box::leak(Box::new(m1::armonik_arm::value("P1.2")));
            let buf: &'static mut Vec<u8> = Box::leak(Box::new(Vec::with_capacity(4 << 20)));
            let e: &'static mut ak_rt::Enc = Box::leak(Box::new(ak_rt::Enc::new(
                facade::generated::core_native::SITES,
            )));
            cases.push(mk("P1.2", name, "encode", "prost", move |n| {
                for _ in 0..n { buf.clear(); prost::Message::encode(v, buf).unwrap(); std::hint::black_box(&buf); }
            }));
            cases.push(mk("P1.2", name, "encode", "core-native", move |n| {
                for _ in 0..n { facade::generated::core_native::encode_into_list_results_response(av, e); std::hint::black_box(&e.buf); }
            }));
            cases.push(mk("P1.2", name, "encode", "core-ffi-rust", move |n| {
                for _ in 0..n { std::hint::black_box(m1::core_ffi_arm::encode_into(ctx, av)); }
            }));
            cases.push(mk("P1.2", name, "encode", "core-ffi (valid utf8)", move |n| {
                for _ in 0..n { std::hint::black_box(m1::core_ffi_arm::encode_into(ctx_v, av)); }
            }));
            cases.push(mk("P1.2", name, "encode", "core-ffi (valid, simd)", move |n| {
                for _ in 0..n { std::hint::black_box(m1::core_ffi_arm::encode_into(ctx_s, av)); }
            }));
            let bytes: &'static [u8] = Box::leak(m1::prost_arm::encode(v).into_boxed_slice());
            cases.push(mk("P1.2", name, "decode", "prost", move |n| {
                for _ in 0..n { std::hint::black_box(m1::prost_arm::decode(bytes)); }
            }));
            cases.push(mk("P1.2", name, "decode", "core-native", move |n| {
                for _ in 0..n { std::hint::black_box(m1::core_native_arm::decode(bytes)); }
            }));
            cases.push(mk("P1.2", name, "decode", "core-ffi-rust", move |n| {
                for _ in 0..n { std::hint::black_box(m1::core_ffi_arm::decode(ctx, bytes)); }
            }));
        }
        {
            let v: &'static _ = Box::leak(Box::new(m2::prost_arm::value("P2.2")));
            let av: &'static _ = Box::leak(Box::new(m2::armonik_arm::value("P2.2")));
            let buf: &'static mut Vec<u8> = Box::leak(Box::new(Vec::with_capacity(8 << 20)));
            let e: &'static mut ak_rt::Enc = Box::leak(Box::new(ak_rt::Enc::new(
                facade::generated::core_native::SITES,
            )));
            cases.push(mk("P2.2", name, "encode", "prost", move |n| {
                for _ in 0..n { buf.clear(); prost::Message::encode(v, buf).unwrap(); std::hint::black_box(&buf); }
            }));
            cases.push(mk("P2.2", name, "encode", "core-native", move |n| {
                for _ in 0..n { facade::generated::core_native::encode_into_list_tasks_detailed_response(av, e); std::hint::black_box(&e.buf); }
            }));
            cases.push(mk("P2.2", name, "encode", "core-ffi-rust", move |n| {
                for _ in 0..n { std::hint::black_box(m2::core_ffi_arm::encode_into(ctx, av)); }
            }));
            cases.push(mk("P2.2", name, "encode", "core-ffi (valid utf8)", move |n| {
                for _ in 0..n { std::hint::black_box(m2::core_ffi_arm::encode_into(ctx_v, av)); }
            }));
            cases.push(mk("P2.2", name, "encode", "core-ffi (valid, simd)", move |n| {
                for _ in 0..n { std::hint::black_box(m2::core_ffi_arm::encode_into(ctx_s, av)); }
            }));
            let bytes: &'static [u8] = Box::leak(m2::prost_arm::encode(v).into_boxed_slice());
            cases.push(mk("P2.2", name, "decode", "prost", move |n| {
                for _ in 0..n { std::hint::black_box(m2::prost_arm::decode(bytes)); }
            }));
            cases.push(mk("P2.2", name, "decode", "core-native", move |n| {
                for _ in 0..n { std::hint::black_box(m2::core_native_arm::decode(bytes)); }
            }));
            cases.push(mk("P2.2", name, "decode", "core-ffi-rust", move |n| {
                for _ in 0..n { std::hint::black_box(m2::core_ffi_arm::decode(ctx, bytes)); }
            }));
        }
    }
    set_content_set(ContentSet::Ascii);

    for c in cases.iter_mut() {
        let mut n = 1usize;
        let mut ns = 0u128;
        while ns < 2_000_000 && n < 10_000_000 {
            let t = Instant::now();
            (c.run)(n);
            ns = t.elapsed().as_nanos().max(1);
            if ns >= 2_000_000 { break; }
            n *= 4;
        }
        c.reps = ((TARGET_NS / (ns / n as u128).max(1)) as usize).clamp(1, 20_000_000);
        (c.run)(c.reps.min(200));
    }
    for _ in 0..ROUNDS {
        for c in cases.iter_mut() {
            let t = Instant::now();
            (c.run)(c.reps);
            c.times.push(t.elapsed().as_nanos() as f64 / c.reps as f64);
        }
    }

    println!("{:<7} {:<9} {:<22} {:>8} {:>13} {:>13} {:>9} {:>9}",
             "payload", "direction", "arm", "set", "median ns", "min ns", "/ prost", "/ its ascii");
    let med = |c: &Case| {
        let mut t = c.times.clone();
        t.sort_by(|a, b| a.partial_cmp(b).unwrap());
        t[t.len() / 2]
    };
    for pid in ["P1.2", "P2.2"] {
        for dir in ["encode", "decode"] {
            for arm in ["prost", "core-native", "core-ffi-rust", "core-ffi (valid utf8)", "core-ffi (valid, simd)"] {
                let mut first = None;
                for (_, name) in SETS {
                    let Some(c) = cases.iter().find(|c| c.payload == pid && c.set == name && c.dir == dir && c.arm == arm) else { continue };
                    let m = med(c);
                    if first.is_none() { first = Some(m); }
                    let base = cases.iter().find(|c| c.payload == pid && c.set == name && c.dir == dir && c.arm == "prost").map(med);
                    println!("{:<7} {:<9} {:<22} {:>8} {:>13.1} {:>13.1} {:>9} {:>9.3}",
                             pid, dir, arm, name, m,
                             c.times.iter().cloned().fold(f64::INFINITY, f64::min),
                             base.map(|b| format!("{:.3}", m / b)).unwrap_or_else(|| "-".into()),
                             m / first.unwrap());
                }
                println!();
            }
        }
    }
    println!("# `/ its ascii` is the same arm against itself on the ascii set, so it isolates");
    println!("#   what the content costs that arm, with the arm held constant.");
}

struct Case {
    payload: &'static str,
    set: &'static str,
    dir: &'static str,
    arm: &'static str,
    reps: usize,
    run: Box<dyn FnMut(usize)>,
    times: Vec<f64>,
}

fn mk(payload: &'static str, set: &'static str, dir: &'static str, arm: &'static str,
      run: impl FnMut(usize) + 'static) -> Case {
    Case { payload, set, dir, arm, reps: 1, run: Box::new(run), times: Vec::new() }
}
