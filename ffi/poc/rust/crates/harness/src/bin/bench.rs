//! Stage 2 timings: four arms, one process, one runtime (README R3 and R4).
//!
//! Not criterion, deliberately. Every ratio this slice reports has to be formed inside one
//! process (R4), and the arms have to be interleaved rather than run in blocks, because on
//! a shared 4-vCPU VM a block of one arm can sit in a different frequency or cache state
//! from the block after it. So: round-robin over the cases, one round at a time, and a
//! median with its spread rather than a mean.
//!
//! What is reported per case is the median of the per-round ns/op, and the min and max
//! round, so a figure carries its own spread. A ratio is computed from medians of two
//! cases measured in the same rounds of the same process.

use ak_abi::*;
use facade::ListResultsResponse;
use harness::arms::*;
use std::time::Instant;

const ROUNDS: usize = 15;
const TARGET_NS_PER_ROUND: u128 = 25_000_000;

struct Case {
    payload: &'static str,
    dir: &'static str,
    arm: &'static str,
    reps: usize,
    run: Box<dyn FnMut(usize)>,
    times: Vec<f64>,
}

fn main() {
    let guard = cfg!(feature = "guard");
    println!("# stage 2 timings");
    println!("#   arms:   prost | armonik | core-native | core-ffi-rust, one process");
    println!("#   guard:  {}", if guard { "ON (ABI v1 section 5)" } else { "OFF" });
    println!("#   counting build: {}", if cfg!(feature = "count") { "YES -- these timings are NOT usable" } else { "no" });
    println!("#   rounds: {ROUNDS}, interleaved; per-round target {} ms", TARGET_NS_PER_ROUND / 1_000_000);
    println!("#   shapes: M1 (P1.x) and M2 (P2.x), both REAL messages. No control rows here:");
    println!("#           M6 and M7 are the controls in design/SHAPES.md and are not built yet.");
    println!();

    let mut cases: Vec<Case> = Vec::new();
    // Leaked so every closure can hold a 'static reference and the set-up cost is outside
    // every timed region.
    let ctx: &'static core_ffi_arm::Ctx = Box::leak(Box::new(core_ffi_arm::Ctx::new()));
    let ctx_v: &'static core_ffi_arm::Ctx = Box::leak(Box::new(core_ffi_arm::Ctx::validating()));

    for pid in [P1_1, P1_2, P1_3] {
        // ---- encode. Every arm encodes into a buffer it reuses, which is what a tonic
        // codec does with the BytesMut the framework hands it, and it is the only way the
        // four arms are doing the same work.
        {
            let v: &'static _ = Box::leak(Box::new(prost_arm::value(pid)));
            let buf: &'static mut Vec<u8> = Box::leak(Box::new(Vec::with_capacity(1 << 20)));
            cases.push(mk(pid, "encode", "prost", move |n| {
                for _ in 0..n {
                    buf.clear();
                    prost::Message::encode(v, buf).unwrap();
                    std::hint::black_box(&buf);
                }
            }));
        }
        {
            let v: &'static _ = Box::leak(Box::new(armonik_arm::value(pid)));
            let buf: &'static mut Vec<u8> = Box::leak(Box::new(Vec::with_capacity(1 << 20)));
            cases.push(mk(pid, "encode", "armonik", move |n| {
                for _ in 0..n {
                    buf.clear();
                    prost::Message::encode(v, buf).unwrap();
                    std::hint::black_box(&buf);
                }
            }));
        }
        {
            let v: &'static _ = Box::leak(Box::new(armonik_arm::value(pid)));
            let e: &'static mut ak_rt::Enc = Box::leak(Box::new(ak_rt::Enc::new(
                facade::generated::core_native::SITES,
            )));
            cases.push(mk(pid, "encode", "core-native", move |n| {
                for _ in 0..n {
                    facade::generated::core_native::encode_into_list_results_response(v, e);
                    std::hint::black_box(&e.buf);
                }
            }));
        }
        {
            let v: &'static _ = Box::leak(Box::new(armonik_arm::value(pid)));
            cases.push(mk(pid, "encode", "core-ffi-rust", move |n| {
                for _ in 0..n {
                    std::hint::black_box(core_ffi_arm::encode_into(ctx, v));
                }
            }));
        }
        {
            let v: &'static _ = Box::leak(Box::new(armonik_arm::value(pid)));
            cases.push(mk(pid, "encode", "core-ffi (valid utf8)", move |n| {
                for _ in 0..n {
                    std::hint::black_box(core_ffi_arm::encode_into(ctx_v, v));
                }
            }));
        }

        // ---- decode. Every arm builds the whole object graph; that is inherent and it is
        // what the incumbent does too.
        let bytes: &'static [u8] = Box::leak(
            prost_arm::encode(&prost_arm::value(pid)).into_boxed_slice(),
        );
        cases.push(mk(pid, "decode", "prost", move |n| {
            for _ in 0..n {
                std::hint::black_box(prost_arm::decode(bytes));
            }
        }));
        cases.push(mk(pid, "decode", "armonik", move |n| {
            for _ in 0..n {
                std::hint::black_box(armonik_arm::decode(bytes));
            }
        }));
        cases.push(mk(pid, "decode", "core-native", move |n| {
            for _ in 0..n {
                std::hint::black_box(core_native_arm::decode(bytes));
            }
        }));
        cases.push(mk(pid, "decode", "core-ffi-rust", move |n| {
            for _ in 0..n {
                let v: ListResultsResponse = core_ffi_arm::decode(ctx, bytes);
                std::hint::black_box(v);
            }
        }));
    }

    // ---- M2: the shape the control plane actually moves. Its element type fails the
    // batching predicate, so the arms are doing a different KIND of work here, not just
    // more of it.
    for pid in harness::arms_m2::ALL {
        use harness::arms_m2 as m2;
        {
            let v: &'static _ = Box::leak(Box::new(m2::prost_arm::value(pid)));
            let buf: &'static mut Vec<u8> = Box::leak(Box::new(Vec::with_capacity(4 << 20)));
            cases.push(mk(pid, "encode", "prost", move |n| {
                for _ in 0..n {
                    buf.clear();
                    prost::Message::encode(v, buf).unwrap();
                    std::hint::black_box(&buf);
                }
            }));
        }
        {
            let v: &'static _ = Box::leak(Box::new(m2::armonik_arm::value(pid)));
            let buf: &'static mut Vec<u8> = Box::leak(Box::new(Vec::with_capacity(4 << 20)));
            cases.push(mk(pid, "encode", "armonik", move |n| {
                for _ in 0..n {
                    buf.clear();
                    prost::Message::encode(v, buf).unwrap();
                    std::hint::black_box(&buf);
                }
            }));
        }
        {
            let v: &'static _ = Box::leak(Box::new(m2::armonik_arm::value(pid)));
            let e: &'static mut ak_rt::Enc = Box::leak(Box::new(ak_rt::Enc::new(
                facade::generated::core_native::SITES,
            )));
            cases.push(mk(pid, "encode", "core-native", move |n| {
                for _ in 0..n {
                    facade::generated::core_native::encode_into_list_tasks_detailed_response(v, e);
                    std::hint::black_box(&e.buf);
                }
            }));
        }
        {
            let v: &'static _ = Box::leak(Box::new(m2::armonik_arm::value(pid)));
            cases.push(mk(pid, "encode", "core-ffi-rust", move |n| {
                for _ in 0..n {
                    std::hint::black_box(m2::core_ffi_arm::encode_into(ctx, v));
                }
            }));
        }
        {
            let v: &'static _ = Box::leak(Box::new(m2::armonik_arm::value(pid)));
            cases.push(mk(pid, "encode", "core-ffi (valid utf8)", move |n| {
                for _ in 0..n {
                    std::hint::black_box(m2::core_ffi_arm::encode_into(ctx_v, v));
                }
            }));
        }
        let bytes: &'static [u8] =
            Box::leak(m2::prost_arm::encode(&m2::prost_arm::value(pid)).into_boxed_slice());
        cases.push(mk(pid, "decode", "prost", move |n| {
            for _ in 0..n {
                std::hint::black_box(m2::prost_arm::decode(bytes));
            }
        }));
        cases.push(mk(pid, "decode", "armonik", move |n| {
            for _ in 0..n {
                std::hint::black_box(m2::armonik_arm::decode(bytes));
            }
        }));
        cases.push(mk(pid, "decode", "core-native", move |n| {
            for _ in 0..n {
                std::hint::black_box(m2::core_native_arm::decode(bytes));
            }
        }));
        cases.push(mk(pid, "decode", "core-ffi-rust", move |n| {
            for _ in 0..n {
                std::hint::black_box(m2::core_ffi_arm::decode(ctx, bytes));
            }
        }));
    }

    // ---- M3. One payload, both directions, and nothing else: the value of M3 is shape
    // coverage, which crates/harness/src/bin/shapes.rs carries, not another timing table.
    {
        use harness::arms_m3 as m3;
        let pid = m3::P3_1;
        {
            let v: &'static _ = Box::leak(Box::new(m3::prost_arm::value(pid)));
            let buf: &'static mut Vec<u8> = Box::leak(Box::new(Vec::with_capacity(1 << 20)));
            cases.push(mk(pid, "encode", "prost", move |n| {
                for _ in 0..n { buf.clear(); prost::Message::encode(v, buf).unwrap(); std::hint::black_box(&buf); }
            }));
        }
        {
            let v: &'static _ = Box::leak(Box::new(m3::armonik_arm::value(pid)));
            let buf: &'static mut Vec<u8> = Box::leak(Box::new(Vec::with_capacity(1 << 20)));
            let e: &'static mut ak_rt::Enc = Box::leak(Box::new(ak_rt::Enc::new(
                facade::generated::core_native::SITES,
            )));
            cases.push(mk(pid, "encode", "armonik", move |n| {
                for _ in 0..n { buf.clear(); prost::Message::encode(v, buf).unwrap(); std::hint::black_box(&buf); }
            }));
            cases.push(mk(pid, "encode", "core-native", move |n| {
                for _ in 0..n { facade::generated::core_native::encode_into_list_probe_response(v, e); std::hint::black_box(&e.buf); }
            }));
            cases.push(mk(pid, "encode", "core-ffi-rust", move |n| {
                for _ in 0..n { std::hint::black_box(m3::core_ffi_arm::encode_into(ctx, v)); }
            }));
        }
        let bytes: &'static [u8] =
            Box::leak(m3::prost_arm::encode(&m3::prost_arm::value(pid)).into_boxed_slice());
        cases.push(mk(pid, "decode", "prost", move |n| {
            for _ in 0..n { std::hint::black_box(m3::prost_arm::decode(bytes)); }
        }));
        cases.push(mk(pid, "decode", "armonik", move |n| {
            for _ in 0..n { std::hint::black_box(m3::armonik_arm::decode(bytes)); }
        }));
        cases.push(mk(pid, "decode", "core-native", move |n| {
            for _ in 0..n { std::hint::black_box(m3::core_native_arm::decode(bytes)); }
        }));
        cases.push(mk(pid, "decode", "core-ffi-rust", move |n| {
            for _ in 0..n { std::hint::black_box(m3::core_ffi_arm::decode(ctx, bytes)); }
        }));
    }

    // The two added arms that isolate ABI v1 open decision 5.
    for pid in harness::arms_m2::ADDED {
        use harness::arms_m2 as m2;
        {
            let v: &'static _ = Box::leak(Box::new(m2::added::prost_value(pid)));
            let buf: &'static mut Vec<u8> = Box::leak(Box::new(Vec::with_capacity(4 << 20)));
            cases.push(mk(pid, "encode", "prost", move |n| {
                for _ in 0..n {
                    buf.clear();
                    prost::Message::encode(v, buf).unwrap();
                    std::hint::black_box(&buf);
                }
            }));
        }
        {
            let v: &'static _ = Box::leak(Box::new(m2::added::facade_value(pid)));
            let e: &'static mut ak_rt::Enc = Box::leak(Box::new(ak_rt::Enc::new(
                facade::generated::core_native::SITES,
            )));
            cases.push(mk(pid, "encode", "core-native", move |n| {
                for _ in 0..n {
                    facade::generated::core_native::encode_into_list_tasks_detailed_response(v, e);
                    std::hint::black_box(&e.buf);
                }
            }));
        }
        {
            let v: &'static _ = Box::leak(Box::new(m2::added::facade_value(pid)));
            cases.push(mk(pid, "encode", "core-ffi-rust", move |n| {
                for _ in 0..n {
                    std::hint::black_box(m2::core_ffi_arm::encode_into(ctx, v));
                }
            }));
        }
    }

    // ---- M4 to M7. Small: one payload each where the shape is the point, plus the two
    // bulk sizes the direct-argument path exists for.
    {
        use harness::arms_rest as r;
        macro_rules! four {
            ($pid:expr, $m:ident, $encf:ident, $decf:ident) => {{
                let pid: &'static str = $pid;
                let pv: &'static _ = Box::leak(Box::new(r::$m::prost_value(pid)));
                let av: &'static _ = Box::leak(Box::new(r::$m::facade_value(pid)));
                let buf: &'static mut Vec<u8> = Box::leak(Box::new(Vec::with_capacity(8 << 20)));
                let e: &'static mut ak_rt::Enc = Box::leak(Box::new(ak_rt::Enc::new(
                    facade::generated::core_native::SITES,
                )));
                cases.push(mk(pid, "encode", "prost", move |n| {
                    for _ in 0..n { buf.clear(); prost::Message::encode(pv, buf).unwrap(); std::hint::black_box(&buf); }
                }));
                cases.push(mk(pid, "encode", "core-native", move |n| {
                    // Into a REUSED buffer, like every other arm. Allocating a fresh Vec and
                    // growing it by doubling made this arm 1.41 of prost on a 4 MB payload,
                    // which was the harness and not the codec.
                    for _ in 0..n { r::$m::native_encode_into(av, e); std::hint::black_box(&e.buf); }
                }));
                cases.push(mk(pid, "encode", "core-ffi-rust", move |n| {
                    for _ in 0..n { std::hint::black_box(r::ffi::$encf(ctx, av)); }
                }));
                let bytes: &'static [u8] =
                    Box::leak(r::$m::prost_encode(pv).into_boxed_slice());
                cases.push(mk(pid, "decode", "prost", move |n| {
                    for _ in 0..n { std::hint::black_box(r::$m::prost_decode(bytes)); }
                }));
                cases.push(mk(pid, "decode", "core-native", move |n| {
                    for _ in 0..n { std::hint::black_box(r::$m::native_decode(bytes)); }
                }));
                cases.push(mk(pid, "decode", "core-ffi-rust", move |n| {
                    for _ in 0..n { std::hint::black_box(r::ffi::$decf(ctx, bytes)); }
                }));
            }};
        }
        four!(r::P4_1, m4, enc_m4, dec_m4);
        four!(r::P5_3, m5, enc_m5, dec_m5);
        four!(r::P5_4, m5, enc_m5, dec_m5);
        four!(r::P6_1, m6, enc_m6, dec_m6);
    }

    // A raw 4 MB copy, as the floor for every P5.4 row. A bulk payload IS a copy plus a
    // handful of varints, so any arm far from this number is doing something other than
    // copying, and any RATIO between two arms on P5.4 is a ratio between two copies unless
    // one of them is not copying.
    {
        let src: &'static [u8] = Box::leak(vec![7u8; 4 * 1024 * 1024].into_boxed_slice());
        cases.push(mk("P5.4", "decode", "memcpy control (Bytes)", move |n| {
            for _ in 0..n {
                std::hint::black_box(::bytes::Bytes::copy_from_slice(src));
            }
        }));
        cases.push(mk("P5.4", "decode", "memcpy control (Vec)", move |n| {
            for _ in 0..n {
                std::hint::black_box(src.to_vec());
            }
        }));
    }

    // The boundary, priced on its own, in this same process and this same build. If these
    // are not a plausible handful of nanoseconds, the core-ffi arm is not crossing anything.
    cases.push(mk("-", "crossing", "forward (no-op)", |n| {
        let mut x = 1u64;
        for _ in 0..n {
            x = unsafe { ak_noop(std::hint::black_box(x)) };
        }
        std::hint::black_box(x);
    }));
    cases.push(mk("-", "crossing", "fwd+reverse (no-op)", |n| {
        let mut x = 1u64;
        for _ in 0..n {
            x = unsafe { ak_noop_reverse(host_noop, std::hint::black_box(x)) };
        }
        std::hint::black_box(x);
    }));

    // AK_BENCH_ONLY=P4.1,P5.3 restricts the run to those payloads. The interleaving and the
    // ratio are still formed inside one process over exactly the cases that remain, so a
    // filtered run is as valid as a full one for the payloads it keeps -- it is simply a
    // smaller set. A log that needs four payloads should not pay for sixteen.
    if let Ok(only) = std::env::var("AK_BENCH_ONLY") {
        let keep: Vec<&str> = only.split(',').map(|s| s.trim()).collect();
        cases.retain(|c| keep.contains(&c.payload) || c.dir == "crossing");
        println!("#   filtered: AK_BENCH_ONLY={only} ({} cases)", cases.len());
    }

    calibrate(&mut cases);
    for _ in 0..ROUNDS {
        for c in cases.iter_mut() {
            let t = Instant::now();
            (c.run)(c.reps);
            c.times.push(t.elapsed().as_nanos() as f64 / c.reps as f64);
        }
    }

    report(&cases);
    decision5(&cases);
}

unsafe extern "C" fn host_noop(x: u64) -> u64 {
    x ^ 2
}

fn mk(
    payload: &'static str,
    dir: &'static str,
    arm: &'static str,
    run: impl FnMut(usize) + 'static,
) -> Case {
    Case { payload, dir, arm, reps: 1, run: Box::new(run), times: Vec::new() }
}

/// Pick a repetition count per case so every round takes about the same wall time. A fixed
/// count would give the small payloads a round of microseconds, where the clock itself is
/// the measurement.
fn calibrate(cases: &mut [Case]) {
    for c in cases.iter_mut() {
        // Ramp rather than a single shot. A single shot on a cold path gave one case 119
        // repetitions where its neighbour got 1,776, and the spread that produced was the
        // measurement rather than the arm.
        let mut n = 1usize;
        let mut ns = 0u128;
        while ns < 2_000_000 && n < 10_000_000 {
            let t = Instant::now();
            (c.run)(n);
            ns = t.elapsed().as_nanos().max(1);
            if ns >= 2_000_000 {
                break;
            }
            n *= 4;
        }
        let per = (ns / n as u128).max(1);
        c.reps = ((TARGET_NS_PER_ROUND / per) as usize).clamp(1, 20_000_000);
        // Warm: touch every path once at the chosen size before anything is recorded, so
        // the learned length widths are settled and the allocator has stopped growing.
        (c.run)(c.reps.min(1000));
    }
}

fn median(v: &mut Vec<f64>) -> f64 {
    let mut s = v.clone();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    s[s.len() / 2]
}

/// ABI v1 open decision 5, isolated rather than attributed.
///
/// P2.4 alternates 3 and 150 repeated strings per element; P2.4a is all 3 and P2.4b is all
/// 150, 80 elements each. By construction the alternating payload has exactly the mean of
/// the two uniform ones in bytes, elements, fields and strings -- 87,422 and 1,875,022 sum
/// to twice 981,222 -- so the only thing left between `t(P2.4)` and the mean of the two is
/// the length-prefix thrashing. prost is carried through the same arithmetic as a control:
/// it computes lengths in a first pass and has no learned width at all, so whatever it
/// shows is the construction's own non-linearity and not the mechanism.
fn decision5(cases: &[Case]) {
    println!();
    println!("# ABI v1 open decision 5: what the grow path costs, isolated");
    println!();
    println!("{:<22} {:>12} {:>12} {:>12} {:>10} {:>9}", "arm", "P2.4 ns", "P2.4a ns", "P2.4b ns", "mean(a,b)", "excess");
    let get = |pid: &str, arm: &str| -> Option<f64> {
        cases
            .iter()
            .find(|c| c.payload == pid && c.dir == "encode" && c.arm == arm)
            .map(|c| median(&mut c.times.clone()))
    };
    for arm in ["prost", "core-native", "core-ffi-rust"] {
        let (Some(x), Some(a), Some(b)) = (get("P2.4", arm), get("P2.4a", arm), get("P2.4b", arm))
        else {
            continue;
        };
        let mean = (a + b) / 2.0;
        println!(
            "{:<22} {:>12.1} {:>12.1} {:>12.1} {:>10.1} {:>8.2}%",
            arm, x, a, b, mean, (x - mean) / mean * 100.0
        );
    }
    println!();
    println!("# prost's excess is the floor: it has no learned width, so its number is what");
    println!("# this construction costs on its own. The mechanism's cost is the difference");
    println!("# between a core arm's excess and prost's.");
}

fn report(cases: &[Case]) {
    println!(
        "{:<7} {:<9} {:<22} {:>8} {:>12} {:>12} {:>12} {:>10}",
        "payload", "direction", "arm", "reps", "median ns", "min ns", "max ns", "/ prost"
    );
    let mut base: Vec<(&str, &str, f64)> = Vec::new();
    for c in cases {
        if c.arm == "prost" {
            base.push((c.payload, c.dir, median(&mut c.times.clone())));
        }
    }
    let mut last = ("", "");
    for c in cases {
        let mut t = c.times.clone();
        let m = median(&mut t);
        let mn = t.iter().cloned().fold(f64::INFINITY, f64::min);
        let mx = t.iter().cloned().fold(0.0, f64::max);
        let r = base
            .iter()
            .find(|(p, d, _)| *p == c.payload && *d == c.dir)
            .map(|(_, _, b)| format!("{:.3}", m / b))
            .unwrap_or_else(|| "-".into());
        if (c.payload, c.dir) != last {
            println!();
            last = (c.payload, c.dir);
        }
        println!(
            "{:<7} {:<9} {:<22} {:>8} {:>12.1} {:>12.1} {:>12.1} {:>10}",
            c.payload, c.dir, c.arm, c.reps, m, mn, mx, r
        );
    }
}
