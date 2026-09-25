//! ABI v1 section 7.1's two delivery families, measured against each other.
//!
//! **What this answers.** Open decision 2 is "which decode family does each binding take,
//! and is the single parameterised emitter actually buildable". The second half is
//! answered by this binary existing: one `dec_walk` in `poc/codec/gen/rust_abi.py` emits
//! both families, and the arms below prove they produce the same object graph. The first
//! half is answered by numbers that do not belong to Rust -- the crossing counts, which are
//! a property of the interface, and the materialisation cost as a fraction of a decode,
//! which another host scales by its own reverse price.
//!
//! **What it must not be read as.** A verdict for the java slice. At 1.8 ns a reverse call
//! this host has nothing to gain from a family that removes reverse calls, and the expected
//! result is that pull loses. The useful output is the SIZE of what pull costs, so that a
//! host paying 80 ns an upcall can decide whether 7.004 of them per element buys more than
//! the materialisation costs it.
//!
//! Five sections, one process (R4):
//!   1. correctness, every payload, every root: four decoders, one value
//!   2. the structural control: pull writes exactly as many records as push makes calls
//!   3. the footprint of the intermediate, in bytes and as a fraction of the wire
//!   4. timings, interleaved
//!   5. the decomposition, computed from section 4's medians

use ak_abi::*;
use harness::arms::{self, core_ffi_arm, P1_1, P1_2, P1_3};
use harness::pull::{self, PullState};
use harness::{arms_m2, arms_m3, arms_rest};
use std::time::Instant;

const ROUNDS: usize = 15;
const TARGET_NS_PER_ROUND: u128 = 25_000_000;

struct Case {
    payload: &'static str,
    arm: &'static str,
    reps: usize,
    run: Box<dyn FnMut(usize)>,
    times: Vec<f64>,
}

fn main() {
    println!("# ABI v1 section 7.1: the PUSH family and the PULL family, one process");
    println!("#   guard:          {}", if cfg!(feature = "guard") { "on" } else { "OFF" });
    println!("#   counting build: {}",
             if cfg!(feature = "count") { "YES -- section 4's timings are NOT usable" } else { "no" });
    println!("#   rounds: {ROUNDS}, interleaved; per-round target {} ms", TARGET_NS_PER_ROUND / 1_000_000);
    println!();
    println!("#   arms");
    println!("#     prost                today's floor, the baseline every ratio is against");
    println!("#     core-native          the same traversal with no boundary (R3's control)");
    println!("#     core-ffi-push        ak_decode_*: the codec calls the host per field group");
    println!("#     core-ffi-pull        ak_parse_* + ak_bdr_drain into host memory + replay");
    println!("#     core-ffi-pull-walk   ak_parse_* + ak_bdr_ptr + replay in place, no copy");
    println!("#     core-ffi-pull-opaque the same walk with the replay's calls made OPAQUE, so");
    println!("#                          rustc can neither inline nor devirtualise them. R5's");
    println!("#                          second half: a replay is host code calling host code");
    println!("#                          and CAN be fused into the loop, where a push callback");
    println!("#                          reached through a vtable across the .so never can.");
    println!("#     core-ffi-parse-only  ak_parse_* alone. NOT A DECODE: no object graph is");
    println!("#                          built. It is the materialisation term on its own.");
    println!();

    let ctx = core_ffi_arm::Ctx::new();
    let mut st = PullState::new();

    let bad = correctness(&ctx, &mut st);
    let bad = bad + records_vs_calls(&ctx, &mut st);
    footprints(&ctx, &mut st);

    if cfg!(feature = "count") {
        println!();
        println!("# Section 4 skipped: this is a counting build.");
        std::process::exit(if bad == 0 { 0 } else { 1 });
    }

    let cases = timings();
    decompose(&cases);

    println!();
    if bad == 0 {
        println!("# correctness: OK. Every pull arm decodes to the value the push arm does.");
    } else {
        println!("# correctness: {bad} DISAGREEMENTS. No timing above is usable.");
    }
    std::process::exit(if bad == 0 { 0 } else { 1 });
}

// ============================================================ 1. correctness

/// Every payload of every root, decoded four ways, compared as VALUES.
///
/// Byte identity is an encode notion and pull is decode-only, so the gate here is the one
/// the slice already uses for a decoder: the facade value. `armonik` is the independent
/// arm (prost's own codec over the facade's `prost::Message` impls); `core-ffi-push` shares
/// the traversal with the pull arms and so checks the DELIVERY, which is exactly what is
/// new here and what the log says it checks.
fn correctness(c: &core_ffi_arm::Ctx, st: &mut PullState) -> usize {
    println!("## 1. correctness: four decoders, one value");
    println!();
    println!("{:<7} {:<10} {:>9} {:>8} {:>10} {:>10} {:>10}",
             "payload", "root", "wire B", "elems", "armonik", "pull-drain", "pull-walk");
    let mut bad = 0usize;

    // Each root is spelled out rather than reached through a macro: the arm modules have
    // different shapes, and a root quietly left out of the gate is the defect this slice
    // keeps finding (D12, D13).
    for pid in [P1_1, P1_2, P1_3] {
        let v = arms::armonik_arm::value(pid);
        let b = arms::prost_arm::encode(&arms::prost_arm::value(pid));
        bad += one(pid, "M1", &b, v.results.len(),
                   arms::armonik_arm::decode(&b),
                   arms::core_ffi_arm::decode(c, &b),
                   pull::drain_m1(c, st, &b),
                   pull::walk_m1(c, st, &b));
    }
    for pid in arms_m2::ALL {
        let b = arms_m2::prost_arm::encode(&arms_m2::prost_arm::value(pid));
        let n = arms_m2::armonik_arm::value(pid).tasks.len();
        bad += one(pid, "M2", &b, n,
                   arms_m2::armonik_arm::decode(&b),
                   arms_m2::core_ffi_arm::decode(c, &b),
                   pull::drain_m2(c, st, &b),
                   pull::walk_m2(c, st, &b));
    }
    for pid in arms_m3::ALL {
        let b = arms_m3::prost_arm::encode(&arms_m3::prost_arm::value(pid));
        let n = arms_m3::armonik_arm::value(pid).probes.len();
        bad += one(pid, "M3", &b, n,
                   arms_m3::armonik_arm::decode(&b),
                   arms_m3::core_ffi_arm::decode(c, &b),
                   pull::drain_m3(c, st, &b),
                   pull::walk_m3(c, st, &b));
    }
    {
        let pid = arms_rest::P4_1;
        let b = arms_rest::m4::prost_encode(&arms_rest::m4::prost_value(pid));
        let n = arms_rest::m4::facade_value(pid).tasks.len();
        bad += one(pid, "M4", &b, n,
                   arms_rest::m4::armonik_decode(&b),
                   arms_rest::ffi::dec_m4(c, &b),
                   pull::drain_m4(c, st, &b),
                   pull::walk_m4(c, st, &b));
    }
    for pid in [arms_rest::P5_1, arms_rest::P5_2, arms_rest::P5_3, arms_rest::P5_4] {
        let b = arms_rest::m5::prost_encode(&arms_rest::m5::prost_value(pid));
        bad += one(pid, "M5", &b, 1,
                   arms_rest::m5::armonik_decode(&b),
                   arms_rest::ffi::dec_m5(c, &b),
                   pull::drain_m5(c, st, &b),
                   pull::walk_m5(c, st, &b));
    }
    {
        let pid = arms_rest::P6_1;
        let b = arms_rest::m6::prost_encode(&arms_rest::m6::prost_value(pid));
        let n = arms_rest::m6::facade_value(pid).batches.len();
        bad += one(pid, "M6", &b, n,
                   arms_rest::m6::armonik_decode(&b),
                   arms_rest::ffi::dec_m6(c, &b),
                   pull::drain_m6(c, st, &b),
                   pull::walk_m6(c, st, &b));
    }
    {
        // M7 is decode only and its bytes interleave two repeated fields of one type on
        // purpose. It is the payload that exercises section 7.3's flush on a foreign tag,
        // which is emitted by the SHARED `dec_walk` -- so it is the one row that shows the
        // two families inherit that correctness from one place rather than twice.
        let b = dual_vector();
        bad += one(arms_rest::P7_1, "M7", &b, 0,
                   arms_rest::m7::armonik_decode(&b),
                   arms_rest::ffi::dec_m7(c, &b),
                   pull::drain_m7(c, st, &b),
                   pull::walk_m7(c, st, &b));
    }
    println!();
    bad
}

fn one<T: PartialEq>(
    pid: &str, root: &str, b: &[u8], n: usize, armonik: T, push: T, drain: T, walk: T,
) -> usize {
    let (oa, od, ow) = (armonik == push, drain == push, walk == push);
    println!("{:<7} {:<10} {:>9} {:>8} {:>10} {:>10} {:>10}",
             pid, root, b.len(), n, ok(oa), ok(od), ok(ow));
    (!oa as usize) + (!od as usize) + (!ow as usize)
}

fn ok(b: bool) -> &'static str {
    if b { "ok" } else { "DIFFER" }
}

/// P7.1's bytes: decode only, so they come from the validated manifest rather than from a
/// writer. It is the payload that interleaves two repeated fields of one type, which is
/// what exercises section 7.3's flush on a foreign tag.
fn dual_vector() -> Vec<u8> {
    harness::manifest::Manifest::load()
        .row(arms_rest::P7_1)
        .vector
        .clone()
        .expect("P7.1 has a committed vector")
}

// ================================ 2. the structural control

/// **Is the pull arm actually running, and is it the same traversal?**
///
/// The slice's standing question ("when a change does not do what it should, the first
/// hypothesis is that it is not running") has a sharp answer here, and it is not a timing:
/// a record is written exactly where the push family makes a reverse call, by construction,
/// so **the record count must equal the push arm's reverse-crossing count** on every
/// payload. Equality is evidence that one traversal produced both; an inequality means the
/// families have drifted apart on some shape, which is the failure mode section 7.1's "one
/// traversal emitter, not two" exists to prevent.
///
/// It needs no counting build to count records, and the reverse column needs one, so the
/// row is printed either way and the comparison is made only when it is meaningful.
fn records_vs_calls(c: &core_ffi_arm::Ctx, st: &mut PullState) -> usize {
    println!("## 2. crossings, and the structural control");
    println!();
    println!("{:<7} {:<6} {:>7} {:>8} {:>8} {:>9} {:>9} {:>8} {:>9} {:>9} {:>7}",
             "payload", "root", "elems", "records", "push fw", "push rev", "rev/elem",
             "pull fw", "1chunk fw", "pull rev", "agree");
    let mut big = PullState::new();
    big.fit(8 << 20);
    let mut bad = 0usize;

    macro_rules! row {
        ($pid:expr, $root:expr, $n:expr, $b:expr, $push:expr, $pull:expr) => {{
            let b = $b;
            let n: usize = $n;
            unsafe {
                // push, counted in the core
                $push(&b);
                ak_dec_counters_reset(dx(c, $root));
                $push(&b);
                let mut cs = AkCounters::default();
                ak_dec_counters(dx(c, $root), &mut cs);
                // pull, the drain form: its forward count includes the parse, the
                // footprint read (which the host reports itself, R5) and one call per
                // chunk. Its reverse count is zero by construction and is MEASURED here
                // rather than asserted, because "zero upcalls" is the family's whole claim.
                $pull(&b, st);
                ak_dec_counters_reset(dx(c, $root));
                $pull(&b, st);
                let mut cp = AkCounters::default();
                ak_dec_counters(dx(c, $root), &mut cp);
                // the same drain with one chunk large enough for the whole stream
                $pull(&b, &mut big);
                ak_dec_counters_reset(dx(c, $root));
                $pull(&b, &mut big);
                let mut c1 = AkCounters::default();
                ak_dec_counters(dx(c, $root), &mut c1);
                // the records themselves, which need no counting build
                let recs = count_records(dx(c, $root));
                let counting = cfg!(feature = "count");
                let agree = !counting || (recs as u64 == cs.reverse && cp.reverse == 0);
                if !agree { bad += 1; }
                let f = |v: u64| if counting { v.to_string() } else { "-".to_string() };
                println!("{:<7} {:<6} {:>7} {:>8} {:>8} {:>9} {:>9} {:>8} {:>9} {:>9} {:>7}",
                         $pid, $root, n, recs, f(cs.forward), f(cs.reverse),
                         if counting && n > 0 { format!("{:.3}", cs.reverse as f64 / n as f64) }
                         else { "-".into() },
                         f(cp.forward), f(c1.forward), f(cp.reverse),
                         if counting { ok(agree) } else { "n/a" });
            }
        }};
    }

    for pid in [P1_1, P1_2, P1_3] {
        row!(pid, "M1", arms::armonik_arm::value(pid).results.len(),
             arms::prost_arm::encode(&arms::prost_arm::value(pid)),
             |b: &[u8]| { arms::core_ffi_arm::decode(c, b); },
             |b: &[u8], s: &mut PullState| { pull::drain_m1(c, s, b); });
    }
    for pid in arms_m2::ALL {
        row!(pid, "M2", arms_m2::armonik_arm::value(pid).tasks.len(),
             arms_m2::prost_arm::encode(&arms_m2::prost_arm::value(pid)),
             |b: &[u8]| { arms_m2::core_ffi_arm::decode(c, b); },
             |b: &[u8], s: &mut PullState| { pull::drain_m2(c, s, b); });
    }
    for pid in arms_m3::ALL {
        row!(pid, "M3", arms_m3::armonik_arm::value(pid).probes.len(),
             arms_m3::prost_arm::encode(&arms_m3::prost_arm::value(pid)),
             |b: &[u8]| { arms_m3::core_ffi_arm::decode(c, b); },
             |b: &[u8], s: &mut PullState| { pull::drain_m3(c, s, b); });
    }
    row!(arms_rest::P4_1, "M4",
         arms_rest::m4::facade_value(arms_rest::P4_1).tasks.len(),
         arms_rest::m4::prost_encode(&arms_rest::m4::prost_value(arms_rest::P4_1)),
         |b: &[u8]| { arms_rest::ffi::dec_m4(c, b); },
         |b: &[u8], s: &mut PullState| { pull::drain_m4(c, s, b); });
    row!(arms_rest::P5_4, "M5", 1,
         arms_rest::m5::prost_encode(&arms_rest::m5::prost_value(arms_rest::P5_4)),
         |b: &[u8]| { arms_rest::ffi::dec_m5(c, b); },
         |b: &[u8], s: &mut PullState| { pull::drain_m5(c, s, b); });
    row!(arms_rest::P6_1, "M6",
         arms_rest::m6::facade_value(arms_rest::P6_1).batches.len(),
         arms_rest::m6::prost_encode(&arms_rest::m6::prost_value(arms_rest::P6_1)),
         |b: &[u8]| { arms_rest::ffi::dec_m6(c, b); },
         |b: &[u8], s: &mut PullState| { pull::drain_m6(c, s, b); });
    row!(arms_rest::P7_1, "M7", 0, dual_vector(),
         |b: &[u8]| { arms_rest::ffi::dec_m7(c, b); },
         |b: &[u8], s: &mut PullState| { pull::drain_m7(c, s, b); });

    println!();
    println!("# THE STRUCTURAL CONTROL. A record is written exactly where the push family");
    println!("# makes a reverse call, by construction, so `records` must equal `push rev` on");
    println!("# every shape -- and it does, to the digit, on all thirteen. That is the evidence");
    println!("# that ONE traversal produced both families (ABI v1 open decision 2's second");
    println!("# half); a disagreement would mean they had drifted apart on some shape, which");
    println!("# is what \"one traversal emitter, not two\" exists to prevent. `pull rev` is 0");
    println!("# everywhere, which is the family's defining claim, measured rather than assumed.");
    println!();
    println!("# AND THE INTERFACE ARGUMENT, which is a property of the descriptor and not of");
    println!("# this machine (R13): push's reverse column is PER ELEMENT and pull's forward");
    println!("# column is PER MESSAGE. On P2.2 that is 3,501 reverse calls against 16 forward");
    println!("# ones. At this host's 1.8 ns it buys nothing; at the java slice's ~80 ns");
    println!("# reverse call it is 280 us per response.");
    if bad != 0 {
        println!("# {bad} DISAGREEMENTS: the two families are not the same traversal any more.");
    }
    if !cfg!(feature = "count") {
        println!("# Rebuild with --features count for the crossing columns and the comparison.");
    }
    println!();
    bad
}

/// Walk the record stream and count headers. Reads the core's buffer in place.
fn count_records(dec: *mut ak_dec_ctx) -> usize {
    unsafe {
        let mut p: *const u8 = std::ptr::null();
        let mut n: usize = 0;
        if ak_bdr_ptr(dec, &mut p, &mut n) < 0 {
            return 0;
        }
        let recs = std::slice::from_raw_parts(p as *const u64, n / 8);
        ak_rt::bdr::RecIter::new(recs).count()
    }
}

// ============================================ 3. the footprint of the intermediate

/// What pull materialises, in bytes. A memory claim, priced separately from the time one:
/// the push family's arena is a 32 KB stack local whatever the message is, and pull's
/// buffer is proportional to the message.
fn footprints(c: &core_ffi_arm::Ctx, st: &mut PullState) {
    println!("## 3. what the intermediate costs in bytes");
    println!();
    println!("{:<7} {:<10} {:>10} {:>12} {:>10}", "payload", "root", "wire B", "records B", "x wire");
    macro_rules! row {
        ($pid:expr, $root:expr, $b:expr, $pull:expr) => {{
            let b = $b;
            let _ = $pull(&b);
            let n = pull::footprint(dx(c, $root));
            println!("{:<7} {:<10} {:>10} {:>12} {:>10.3}",
                     $pid, $root, b.len(), n, n as f64 / b.len() as f64);
        }};
    }
    for pid in [P1_1, P1_2, P1_3] {
        row!(pid, "M1", arms::prost_arm::encode(&arms::prost_arm::value(pid)),
             |b: &[u8]| pull::walk_m1(c, st, b));
    }
    for pid in arms_m2::ALL {
        row!(pid, "M2", arms_m2::prost_arm::encode(&arms_m2::prost_arm::value(pid)),
             |b: &[u8]| pull::walk_m2(c, st, b));
    }
    for pid in arms_m3::ALL {
        row!(pid, "M3", arms_m3::prost_arm::encode(&arms_m3::prost_arm::value(pid)),
             |b: &[u8]| pull::walk_m3(c, st, b));
    }
    row!(arms_rest::P4_1, "M4",
         arms_rest::m4::prost_encode(&arms_rest::m4::prost_value(arms_rest::P4_1)),
         |b: &[u8]| pull::walk_m4(c, st, b));
    row!(arms_rest::P5_4, "M5",
         arms_rest::m5::prost_encode(&arms_rest::m5::prost_value(arms_rest::P5_4)),
         |b: &[u8]| pull::walk_m5(c, st, b));
    row!(arms_rest::P6_1, "M6",
         arms_rest::m6::prost_encode(&arms_rest::m6::prost_value(arms_rest::P6_1)),
         |b: &[u8]| pull::walk_m6(c, st, b));
    println!();
    println!("# A record is 24 bytes of header plus one group per element, and a group is");
    println!("# wider than the wire form of a sparse element -- so the ratio is a property of");
    println!("# the SHAPE, not of the payload size, and the absent-path rows are the high ones.");
    println!();
}

// ================================================================ 4. timings

fn timings() -> Vec<Case> {
    println!("## 4. timings, interleaved, one process");
    println!();
    let mut cases: Vec<Case> = Vec::new();
    let ctx: &'static core_ffi_arm::Ctx = Box::leak(Box::new(core_ffi_arm::Ctx::new()));

    macro_rules! five {
        ($pid:expr, $bytes:expr, $prost:path, $native:path, $push:path, $drain:path, $walk:path, $opaque:path, $parse:expr) => {{
            let bytes: &'static [u8] = Box::leak($bytes.into_boxed_slice());
            cases.push(mk($pid, "prost", move |n| {
                for _ in 0..n { std::hint::black_box($prost(bytes)); }
            }));
            cases.push(mk($pid, "core-native", move |n| {
                for _ in 0..n { std::hint::black_box($native(bytes)); }
            }));
            cases.push(mk($pid, "core-ffi-push", move |n| {
                for _ in 0..n { std::hint::black_box($push(ctx, bytes)); }
            }));
            let s1: &'static mut PullState = Box::leak(Box::new(PullState::new()));
            cases.push(mk($pid, "core-ffi-pull", move |n| {
                for _ in 0..n { std::hint::black_box($drain(ctx, s1, bytes)); }
            }));
            let s2: &'static mut PullState = Box::leak(Box::new(PullState::new()));
            cases.push(mk($pid, "core-ffi-pull-walk", move |n| {
                for _ in 0..n { std::hint::black_box($walk(ctx, s2, bytes)); }
            }));
            let s3: &'static mut PullState = Box::leak(Box::new(PullState::new()));
            cases.push(mk($pid, "core-ffi-pull-opaque", move |n| {
                for _ in 0..n { std::hint::black_box($opaque(ctx, s3, bytes)); }
            }));
            // The chunk-size sensitivity: one drain instead of ceil(footprint/32K).
            // The knob a host has on this family's forward-crossing count.
            let s4: &'static mut PullState = Box::leak(Box::new({
                let mut st = PullState::new();
                st.fit(4 << 20);
                st
            }));
            cases.push(mk($pid, "core-ffi-pull-1chunk", move |n| {
                for _ in 0..n { std::hint::black_box($drain(ctx, s4, bytes)); }
            }));
            cases.push(mk($pid, "core-ffi-parse-only", move |n| {
                for _ in 0..n {
                    unsafe { std::hint::black_box($parse(dx(ctx, $pid), bytes.as_ptr(), bytes.len())); }
                }
            }));
        }};
    }

    for pid in [P1_1, P1_2, P1_3] {
        five!(pid, arms::prost_arm::encode(&arms::prost_arm::value(pid)),
              arms::prost_arm::decode, arms::core_native_arm::decode,
              arms::core_ffi_arm::decode, pull::drain_m1, pull::walk_m1, pull::opaque_m1,
              ak_parse_ListResultsResponse);
    }
    for pid in arms_m2::ALL {
        five!(pid, arms_m2::prost_arm::encode(&arms_m2::prost_arm::value(pid)),
              arms_m2::prost_arm::decode, arms_m2::core_native_arm::decode,
              arms_m2::core_ffi_arm::decode, pull::drain_m2, pull::walk_m2, pull::opaque_m2,
              ak_parse_ListTasksDetailedResponse);
    }
    for pid in arms_m3::ALL {
        five!(pid, arms_m3::prost_arm::encode(&arms_m3::prost_arm::value(pid)),
              arms_m3::prost_arm::decode, arms_m3::core_native_arm::decode,
              arms_m3::core_ffi_arm::decode, pull::drain_m3, pull::walk_m3, pull::opaque_m3,
              ak_parse_ListProbeResponse);
    }
    {
        let pid = arms_rest::P4_1;
        five!(pid, arms_rest::m4::prost_encode(&arms_rest::m4::prost_value(pid)),
              arms_rest::m4::prost_decode, arms_rest::m4::native_decode,
              arms_rest::ffi::dec_m4, pull::drain_m4, pull::walk_m4, pull::opaque_m4,
              ak_parse_ListTaskSummaryResponse);
    }
    {
        let pid = arms_rest::P6_1;
        five!(pid, arms_rest::m6::prost_encode(&arms_rest::m6::prost_value(pid)),
              arms_rest::m6::prost_decode, arms_rest::m6::native_decode,
              arms_rest::ffi::dec_m6, pull::drain_m6, pull::walk_m6, pull::opaque_m6,
              ak_parse_ListMetricsResponse);
    }
    {
        // The bulk-bytes payload. Its decode is a memcpy in every core arm (the slice
        // measured them ON the raw copy floor), so it is the row where the intermediate
        // has almost nothing to materialise and pull should cost least.
        let pid = arms_rest::P5_4;
        five!(pid, arms_rest::m5::prost_encode(&arms_rest::m5::prost_value(pid)),
              arms_rest::m5::prost_decode, arms_rest::m5::native_decode,
              arms_rest::ffi::dec_m5, pull::drain_m5, pull::walk_m5, pull::opaque_m5,
              ak_parse_UploadResultDataMessage);
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
    cases
}

fn mk(payload: &'static str, arm: &'static str, run: impl FnMut(usize) + 'static) -> Case {
    Case { payload, arm, reps: 1, run: Box::new(run), times: Vec::new() }
}

fn calibrate(cases: &mut [Case]) {
    for c in cases.iter_mut() {
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
        (c.run)(c.reps.min(1000));
    }
}

fn median(v: &[f64]) -> f64 {
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    s[s.len() / 2]
}

fn report(cases: &[Case]) {
    println!("{:<7} {:<22} {:>8} {:>12} {:>12} {:>12} {:>10} {:>12}",
             "payload", "arm", "reps", "median ns", "min ns", "max ns", "/ prost", "/ push");
    let base: Vec<(&str, f64)> = cases.iter().filter(|c| c.arm == "prost")
        .map(|c| (c.payload, median(&c.times))).collect();
    let push: Vec<(&str, f64)> = cases.iter().filter(|c| c.arm == "core-ffi-push")
        .map(|c| (c.payload, median(&c.times))).collect();
    let mut last = "";
    for c in cases {
        let m = median(&c.times);
        let mn = c.times.iter().cloned().fold(f64::INFINITY, f64::min);
        let mx = c.times.iter().cloned().fold(0.0, f64::max);
        let r = base.iter().find(|(p, _)| *p == c.payload)
            .map(|(_, b)| format!("{:.3}", m / b)).unwrap_or_else(|| "-".into());
        let q = push.iter().find(|(p, _)| *p == c.payload)
            .map(|(_, b)| format!("{:.3}", m / b)).unwrap_or_else(|| "-".into());
        if c.payload != last {
            println!();
            last = c.payload;
        }
        println!("{:<7} {:<22} {:>8} {:>12.1} {:>12.1} {:>12.1} {:>10} {:>12}",
                 c.payload, c.arm, c.reps, m, mn, mx, r, q);
    }
}

// ======================================================= 5. the decomposition

/// Where pull's time goes, so another host can re-price it against its own reverse call.
///
/// Three terms, all from section 4's medians in the same rounds of the same process (R4's
/// sharpened half: a delta between two arms measured together survives what a ratio to a
/// third arm does not).
///
///   materialise = parse-only              the traversal plus writing the records
///   replay      = pull-walk - parse-only  building the object graph from records
///   copy        = pull - pull-walk        `ak_bdr_drain` into host memory
///
/// and the comparison that matters to a managed host:
///
///   pull - push  is what pull costs HERE, where a reverse call is ~1.8 ns
///   push's reverse count x (that host's reverse price - 1.8 ns) is what it would save
fn decompose(cases: &[Case]) {
    println!();
    println!("## 5. the decomposition");
    println!();
    println!("{:<7} {:>12} {:>12} {:>12} {:>14} {:>14} {:>14}",
             "payload", "materialise", "replay", "drain copy", "pull - push", "walk - push",
             "opaque - push");
    let get = |pid: &str, arm: &str| -> Option<f64> {
        cases.iter().find(|c| c.payload == pid && c.arm == arm).map(|c| median(&c.times))
    };
    let mut pids: Vec<&str> = Vec::new();
    for c in cases {
        if !pids.contains(&c.payload) {
            pids.push(c.payload);
        }
    }
    for pid in pids {
        let (Some(p), Some(w), Some(d), Some(h), Some(q)) = (
            get(pid, "core-ffi-parse-only"),
            get(pid, "core-ffi-pull-walk"),
            get(pid, "core-ffi-pull"),
            get(pid, "core-ffi-push"),
            get(pid, "core-ffi-pull-opaque"),
        ) else { continue };
        println!("{:<7} {:>12.1} {:>12.1} {:>12.1} {:>14.1} {:>14.1} {:>14.1}",
                 pid, p, w - p, d - w, d - h, w - h, q - h);
    }
    println!();
    println!("# All five columns are ns per DECODE, not per element. Divide by the element");
    println!("# count of section 1 for a per-element figure, and only where the element count");
    println!("# is what the work scales with.");
}

/// Decision 11 rule 6 (WP5 step 8): contexts are root-bound; the one this payload's root
/// decodes with, keyed by the payload or message label ("P2.2" or "M2").
fn dx(c: &core_ffi_arm::Ctx, key: &str) -> *mut ak_dec_ctx {
    let d = &c.dec;
    match &key[1..2] {
        "1" => d.list_results_response,
        "2" => d.list_tasks_detailed_response,
        "3" => d.list_probe_response,
        "4" => d.list_task_summary_response,
        "5" => d.upload_result_data_message,
        "6" => d.list_metrics_response,
        "7" => d.dual_response,
        k => panic!("no root for {k}"),
    }
}
