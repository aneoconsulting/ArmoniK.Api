//! The content sets on **every** payload, not only P1.2 and P2.2.
//!
//! `content.rs` priced `latin1` and `wide` on one M1 payload and one M2 payload, and the
//! slice's own "what is not measured" has said ever since that **every other payload's
//! string-path figure in every other log is an ASCII figure**. `design/SHAPES.md` is blunt
//! about what that costs: "a slice that reports one string-path number without saying which
//! content set it came from has reported half a number." This is the other half for the
//! rest of the set.
//!
//! **What a content set means in Rust, restated because it is the thing most likely to be
//! read across wrongly.** On .NET or the JVM these sets make a narrowing transcoder do real
//! work or fail outright, because the host holds UTF-16. A Rust `String` is UTF-8 already:
//! there is no narrowing and no transcoder on the encode path at all since decision 3
//! settled (`ak_tc_utf8` is `ak_tc_bytes`). What changes here is the **byte width** of the
//! same character count -- 1, 2 and 3 bytes -- and which path `str::from_utf8` takes on
//! decode. These rows price WIDTH and VALIDATION in Rust. A managed slice must not read
//! them across.
//!
//! **Two of the payloads are controls and that is the point of including them.** P1.3 and
//! P2.5 are the absent paths: their strings are mostly or entirely absent, so the content
//! set has almost nothing to act on and the rows must barely move. If an absent-path row
//! DOES move with the content set, something is constructing strings that should not exist,
//! and that is a defect the string-dense payloads cannot show.
//!
//! **This pass found defect D20 on its first run**, and not the way it was aimed. The
//! disagreement it reported on P3.1 was not a content-set effect at all -- ASCII reproduces
//! it. What the extension actually changed was the ORDER in which payloads share one encode
//! context, and that is what exposed an empty string taking ABI v1 section 8's
//! direct-argument path. See `ffi/logs/rust/stage5-content-all.log` and the regression now
//! standing in `conformance`.
//!
//! Correctness covers all 16 payloads including the bulk-bytes ones; timings cover the 11
//! that carry strings. No manifest oracle: `ffi/schema/` emits ASCII payloads only, so
//! there are no hashes for latin1 or wide, and the reference is the prost arm -- an
//! independent encoder over an independently built object graph, validated against the
//! manifest on ASCII.

use harness::arms as m1;
use harness::arms_m2 as m2;
use harness::arms_m3 as m3;
use harness::arms_rest as rest;
use shapes_values::{set_content_set, ContentSet};
use std::time::Instant;

const ROUNDS: usize = 9;
const TARGET_NS: u128 = 12_000_000;

const SETS: [(ContentSet, &str); 3] = [
    (ContentSet::Ascii, "ascii"),
    (ContentSet::Latin1, "latin1"),
    (ContentSet::Wide, "wide"),
];

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

fn main() {
    println!("# content sets on EVERY payload (design/SHAPES.md, \"Content sets\")");
    println!("#   guard: {}", if cfg!(feature = "guard") { "on" } else { "OFF" });
    println!("#   init-guard: {}", if cfg!(feature = "init-guard") { "on" } else { "off" });
    println!("#   arms: prost | core-native | core-ffi-rust. The two validating encode arms");
    println!("#         are NOT here: decision 3 settled and encode no longer validates, so");
    println!("#         they would be priced against a path nothing takes. content.rs keeps");
    println!("#         them for the record on P1.2 and P2.2.");
    println!("#   P1.3 and P2.5 are the ABSENT-PATH CONTROLS: their rows must barely move.");
    println!();

    let ctx: &'static _ = Box::leak(Box::new(m1::core_ffi_arm::Ctx::new()));

    let bad = correctness(ctx);
    if bad > 0 {
        println!("\nFAILED: {bad} disagreement(s). Nothing below would be usable.");
        std::process::exit(1);
    }
    timings(ctx);
    std::process::exit(0);
}

/// Byte identity across the arms on every payload and every set, plus a decode round trip.
fn correctness(ctx: &'static m1::core_ffi_arm::Ctx) -> usize {
    println!("## 1. byte identity across the arms, every payload, every content set");
    println!("#    reference: the prost arm. There is no manifest for latin1 or wide.");
    println!();
    println!("{:<7} {:<8} {:>10} {:>9} {:<12} {}",
             "payload", "set", "bytes", "B/ascii", "arms agree", "decode round trip");
    let mut bad = 0usize;
    let mut ascii_size = std::collections::BTreeMap::new();

    macro_rules! check {
        ($pid:expr, $set:expr, $pb:expr, $ab:expr, $cb:expr, $fb:expr, $rt:expr) => {{
            let (pb, ab, cb, fb): (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) = ($pb, $ab, $cb, $fb);
            let ok = ab == pb && cb == pb && fb == pb;
            if $set == "ascii" { ascii_size.insert($pid, pb.len()); }
            let base = *ascii_size.get($pid).unwrap_or(&pb.len()) as f64;
            println!("{:<7} {:<8} {:>10} {:>9.2} {:<12} {}",
                     $pid, $set, pb.len(), pb.len() as f64 / base,
                     if ok { "ok" } else { "DIFFER" },
                     if $rt { "ok" } else { "FAILED" });
            bad += usize::from(!(ok && $rt));
        }};
    }

    for (cs, set) in SETS {
        set_content_set(cs);
        for pid in [m1::P1_1, m1::P1_2, m1::P1_3] {
            let av = m1::armonik_arm::value(pid);
            let pb = m1::prost_arm::encode(&m1::prost_arm::value(pid));
            check!(pid, set, pb.clone(), m1::armonik_arm::encode(&av),
                   m1::core_native_arm::encode(&av), m1::core_ffi_arm::encode(ctx, &av),
                   m1::core_native_arm::decode(&pb) == av
                       && m1::core_ffi_arm::decode(ctx, &pb) == av);
        }
        for pid in m2::ALL {
            let av = m2::armonik_arm::value(pid);
            let pb = m2::prost_arm::encode(&m2::prost_arm::value(pid));
            check!(pid, set, pb.clone(), m2::armonik_arm::encode(&av),
                   m2::core_native_arm::encode(&av), m2::core_ffi_arm::encode(ctx, &av),
                   m2::core_native_arm::decode(&pb) == av
                       && m2::core_ffi_arm::decode(ctx, &pb) == av);
        }
        for pid in m3::ALL {
            let av = m3::armonik_arm::value(pid);
            let pb = m3::prost_arm::encode(&m3::prost_arm::value(pid));
            check!(pid, set, pb.clone(), m3::armonik_arm::encode(&av),
                   m3::core_native_arm::encode(&av), m3::core_ffi_arm::encode(ctx, &av),
                   m3::core_native_arm::decode(&pb) == av
                       && m3::core_ffi_arm::decode(ctx, &pb) == av);
        }
        {
            let pid = rest::P4_1;
            let av = rest::m4::facade_value(pid);
            let pb = rest::m4::prost_encode(&rest::m4::prost_value(pid));
            check!(pid, set, pb.clone(), rest::m4::armonik_encode(&av),
                   rest::m4::native_encode(&av), rest::ffi::enc_m4(ctx, &av).to_vec(),
                   rest::m4::native_decode(&pb) == av && rest::ffi::dec_m4(ctx, &pb) == av);
        }
        for pid in [rest::P5_1, rest::P5_2] {
            // Bulk bytes. Included so the pass covers all 16 payloads: `bytes` fields carry
            // no content set at all, so these rows must be IDENTICAL across sets, which is
            // the tightest control in this table.
            let av = rest::m5::facade_value(pid);
            let pb = rest::m5::prost_encode(&rest::m5::prost_value(pid));
            check!(pid, set, pb.clone(), rest::m5::armonik_encode(&av),
                   rest::m5::native_encode(&av), rest::ffi::enc_m5(ctx, &av).to_vec(),
                   rest::m5::native_decode(&pb) == av && rest::ffi::dec_m5(ctx, &pb) == av);
        }
        {
            let pid = rest::P6_1;
            let av = rest::m6::facade_value(pid);
            let pb = rest::m6::prost_encode(&rest::m6::prost_value(pid));
            check!(pid, set, pb.clone(), rest::m6::armonik_encode(&av),
                   rest::m6::native_encode(&av), rest::ffi::enc_m6(ctx, &av).to_vec(),
                   rest::m6::native_decode(&pb) == av && rest::ffi::dec_m6(ctx, &pb) == av);
        }
        println!();
    }
    set_content_set(ContentSet::Ascii);
    println!("# P5.x must read 1.00 on every set: a `bytes` field has no content set, so a");
    println!("# row that moved there would mean the harness was recoding something binary.");
    println!();
    bad
}

fn timings(ctx: &'static m1::core_ffi_arm::Ctx) {
    println!("## 2. encode and decode, all three sets, ONE process, rounds interleaved");
    println!();
    let mut cases: Vec<Case> = Vec::new();

    macro_rules! arms {
        ($pid:expr, $set:expr, $pv:expr, $av:expr, $pdec:path, $ndec:path, $fdec:path,
         $nenc:path, $fenc:expr, $cap:expr) => {{
            let pv: &'static _ = Box::leak(Box::new($pv));
            let av: &'static _ = Box::leak(Box::new($av));
            let buf: &'static mut Vec<u8> = Box::leak(Box::new(Vec::with_capacity($cap)));
            let e: &'static mut ak_rt::Enc = Box::leak(Box::new(ak_rt::Enc::new(
                facade::generated::core_native::SITES,
            )));
            cases.push(mk($pid, $set, "encode", "prost", move |n| {
                for _ in 0..n {
                    buf.clear();
                    prost::Message::encode(pv, buf).unwrap();
                    std::hint::black_box(&buf);
                }
            }));
            cases.push(mk($pid, $set, "encode", "core-native", move |n| {
                for _ in 0..n { $nenc(av, e); std::hint::black_box(&e.buf); }
            }));
            let f = $fenc;
            cases.push(mk($pid, $set, "encode", "core-ffi-rust", move |n| {
                for _ in 0..n { std::hint::black_box(f(ctx, av)); }
            }));
            let bytes: &'static [u8] =
                Box::leak(prost::Message::encode_to_vec(pv).into_boxed_slice());
            cases.push(mk($pid, $set, "decode", "prost", move |n| {
                for _ in 0..n { std::hint::black_box($pdec(bytes)); }
            }));
            cases.push(mk($pid, $set, "decode", "core-native", move |n| {
                for _ in 0..n { std::hint::black_box($ndec(bytes)); }
            }));
            cases.push(mk($pid, $set, "decode", "core-ffi-rust", move |n| {
                for _ in 0..n { std::hint::black_box($fdec(ctx, bytes)); }
            }));
        }};
    }

    for (cs, set) in SETS {
        set_content_set(cs);
        for pid in [m1::P1_1, m1::P1_2, m1::P1_3] {
            arms!(pid, set, m1::prost_arm::value(pid), m1::armonik_arm::value(pid),
                  m1::prost_arm::decode, m1::core_native_arm::decode, m1::core_ffi_arm::decode,
                  facade::generated::core_native::encode_into_list_results_response,
                  m1::core_ffi_arm::encode_into, 4 << 20);
        }
        for pid in m2::ALL {
            arms!(pid, set, m2::prost_arm::value(pid), m2::armonik_arm::value(pid),
                  m2::prost_arm::decode, m2::core_native_arm::decode, m2::core_ffi_arm::decode,
                  facade::generated::core_native::encode_into_list_tasks_detailed_response,
                  m2::core_ffi_arm::encode_into, 8 << 20);
        }
        for pid in m3::ALL {
            arms!(pid, set, m3::prost_arm::value(pid), m3::armonik_arm::value(pid),
                  m3::prost_arm::decode, m3::core_native_arm::decode, m3::core_ffi_arm::decode,
                  facade::generated::core_native::encode_into_list_probe_response,
                  m3::core_ffi_arm::encode_into, 1 << 20);
        }
        arms!(rest::P4_1, set, rest::m4::prost_value(rest::P4_1),
              rest::m4::facade_value(rest::P4_1),
              rest::m4::prost_decode, rest::m4::native_decode, rest::ffi::dec_m4,
              facade::generated::core_native::encode_into_list_task_summary_response,
              |c: &'static m1::core_ffi_arm::Ctx, v: &facade::ListTaskSummaryResponse| {
                  rest::ffi::enc_m4(c, v)
              }, 4 << 20);
        arms!(rest::P6_1, set, rest::m6::prost_value(rest::P6_1),
              rest::m6::facade_value(rest::P6_1),
              rest::m6::prost_decode, rest::m6::native_decode, rest::ffi::dec_m6,
              facade::generated::core_native::encode_into_list_metrics_response,
              |c: &'static m1::core_ffi_arm::Ctx, v: &facade::ListMetricsResponse| {
                  rest::ffi::enc_m6(c, v)
              }, 4 << 20);
    }
    set_content_set(ContentSet::Ascii);

    for c in cases.iter_mut() {
        let mut n = 1usize;
        let mut ns = 0u128;
        while ns < 1_500_000 && n < 10_000_000 {
            let t = Instant::now();
            (c.run)(n);
            ns = t.elapsed().as_nanos().max(1);
            if ns >= 1_500_000 { break; }
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

    let med = |c: &Case| {
        let mut t = c.times.clone();
        t.sort_by(|a, b| a.partial_cmp(b).unwrap());
        t[t.len() / 2]
    };
    println!("{:<7} {:<9} {:<16} {:>8} {:>13} {:>9} {:>12}",
             "payload", "direction", "arm", "set", "median ns", "/ prost", "/ its ascii");
    let pids: Vec<&str> = {
        let mut v: Vec<&str> = Vec::new();
        for c in &cases { if !v.contains(&c.payload) { v.push(c.payload); } }
        v
    };
    for pid in pids {
        for dir in ["encode", "decode"] {
            for arm in ["prost", "core-native", "core-ffi-rust"] {
                let mut ascii = None;
                for (_, set) in SETS {
                    let Some(c) = cases.iter().find(|c| {
                        c.payload == pid && c.set == set && c.dir == dir && c.arm == arm
                    }) else { continue };
                    let m = med(c);
                    if ascii.is_none() { ascii = Some(m); }
                    let base = cases.iter().find(|c| {
                        c.payload == pid && c.set == set && c.dir == dir && c.arm == "prost"
                    }).map(med);
                    println!("{:<7} {:<9} {:<16} {:>8} {:>13.1} {:>9} {:>12.3}",
                             pid, dir, arm, set, m,
                             base.map(|b| format!("{:.3}", m / b)).unwrap_or_else(|| "-".into()),
                             m / ascii.unwrap());
                }
            }
            println!();
        }
    }
    println!("# `/ its ascii` is the same arm against ITSELF on the ascii set, so it isolates");
    println!("# what the content costs that arm with the arm held constant. R4's sharpened");
    println!("# half says to prefer that form: this slice has already seen a within-arm delta");
    println!("# reproduce across builds where the ratio to a third arm drifted.");
}
