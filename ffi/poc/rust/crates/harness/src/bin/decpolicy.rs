//! The decode side's UTF-8 policy, priced. ABI v1 open decision 3, third framing.
//!
//! The framing inverts the earlier work: a `string` field on the wire is a length prefix
//! and a byte copy, so there is nothing for the ENCODER to validate that the receiving
//! parser does not have to validate anyway. Validation belongs on decode, which is the
//! mandatory side. What this bin answers is what it costs there.
//!
//! Three policies, one per BUILD (`ak_rt::strings`), because a runtime branch per string
//! would be a cost of its own inside the measurement it exists to make:
//!
//!   default              lossy      `String::from_utf8_lossy`, U+FFFD, never fails
//!   --features dec-reject         reject     `core::str::from_utf8`
//!   --features dec-reject-simd    reject     `simdutf8::basic`
//!
//! **So the three columns are three processes and R4 does not hold across them.** The
//! in-process control is the `prost` row: prost validates UTF-8 on decode and this build
//! flag cannot reach it, so `arm / prost` inside each process is the figure that travels,
//! and it is also the answer to "what does a rejecting decode do to the ratio against the
//! incumbent". Absolutes are printed but must be read down a column, never across.

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
    println!("# decode UTF-8 policy (ABI v1 open decision 3, third framing)");
    println!("#   POLICY IN THIS BUILD: {}", ak_rt::strings::POLICY);
    println!("#   guard: {}", if cfg!(feature = "guard") { "on" } else { "OFF" });
    println!("#   control column: prost, which validates on decode under every policy here.");
    println!();

    let ctx: &'static _ = Box::leak(Box::new(m1::core_ffi_arm::Ctx::new()));
    let ctx2: &'static _ = Box::leak(Box::new(m1::core_ffi_arm::Ctx::new()));

    let mut bad = 0usize;

    // ---- 1. valid input still decodes to the same value under every policy.
    println!("## 1. correctness on VALID input: value identity, every set, every arm");
    println!("{:<7} {:<8} {:>9} {:<14} {}", "payload", "set", "bytes", "native == v", "ffi == v");
    for (cs, name) in SETS {
        set_content_set(cs);
        for pid in ["P1.2", "P2.2"] {
            let (n, ok_n, ok_f) = if pid == "P1.2" {
                let av = m1::armonik_arm::value(pid);
                let b = m1::prost_arm::encode(&m1::prost_arm::value(pid));
                (b.len(),
                 m1::core_native_arm::decode(&b) == av,
                 m1::core_ffi_arm::decode(ctx, &b) == av)
            } else {
                let av = m2::armonik_arm::value(pid);
                let b = m2::prost_arm::encode(&m2::prost_arm::value(pid));
                (b.len(),
                 m2::core_native_arm::decode(&b) == av,
                 m2::core_ffi_arm::decode(ctx2, &b) == av)
            };
            println!("{:<7} {:<8} {:>9} {:<14} {}", pid, name, n,
                     if ok_n { "ok" } else { "DIFFER" }, if ok_f { "ok" } else { "DIFFER" });
            bad += usize::from(!(ok_n && ok_f));
        }
    }
    set_content_set(ContentSet::Ascii);

    // ---- 2. the case that must FAIL (D13: a refusal is tested by a case that must fail).
    //         One string field's first byte is replaced with 0xFF, which is never a legal
    //         UTF-8 lead byte. The length is unchanged, so the WIRE is still well formed
    //         and nothing but the UTF-8 policy can notice.
    println!();
    println!("## 2. correctness on MALFORMED input: one string byte replaced with 0xFF");
    let bad_bytes = malformed_payload();
    let p = m1::prost_arm::decode_res(&bad_bytes);
    let n = facade::generated::core_native::decode_list_results_response(&bad_bytes);
    let f = harness::generated::binding::decode_with_list_results_response(ctx.dec, &bad_bytes);
    let say = |r: &Result<_, i32>| match r {
        Ok(_) => "ACCEPTED (substituted)".to_string(),
        Err(e) => format!("rejected, code {e}"),
    };
    println!("{:<16} {}", "prost", match &p { Ok(_) => "ACCEPTED".into(), Err(e) => format!("rejected: {e}") });
    println!("{:<16} {}", "core-native", say(&n));
    println!("{:<16} {}", "core-ffi-rust", say(&f));
    let rejecting = cfg!(feature = "dec-reject") || cfg!(feature = "dec-reject-simd");
    let want = if rejecting { "both core arms must REJECT" } else { "both core arms must ACCEPT (lossy)" };
    let got = if rejecting { n.is_err() && f.is_err() } else { n.is_ok() && f.is_ok() };
    println!("expected for this build: {want} -> {}", if got { "ok" } else { "FAILED" });
    bad += usize::from(!got);
    if p.is_ok() {
        println!("NOTE: prost ACCEPTED malformed UTF-8, which contradicts what this test assumes.");
        bad += 1;
    }
    // The sticky slot is cleared by decode_with_*; prove it, or every later row is a lie.
    let good = m1::prost_arm::encode(&m1::prost_arm::value("P1.1"));
    let after = harness::generated::binding::decode_with_list_results_response(ctx.dec, &good);
    println!("a good decode after a rejected one: {}",
             if after.is_ok() { "ok" } else { "POISONED CONTEXT" });
    bad += usize::from(after.is_err());

    if bad > 0 {
        println!("\nFAILED: {bad} problem(s). Nothing below is usable.");
        std::process::exit(1);
    }

    // ---- 3. timings. Decode only: the encode side has no validator any more.
    println!();
    println!("## 3. decode, one process, three content sets, rounds interleaved");
    println!();
    let mut cases: Vec<Case> = Vec::new();
    for (cs, name) in SETS {
        set_content_set(cs);
        {
            let v: &'static _ = Box::leak(Box::new(m1::prost_arm::value("P1.2")));
            let bytes: &'static [u8] = Box::leak(m1::prost_arm::encode(v).into_boxed_slice());
            cases.push(mk("P1.2", name, "prost", move |n| {
                for _ in 0..n { std::hint::black_box(m1::prost_arm::decode(bytes)); }
            }));
            cases.push(mk("P1.2", name, "core-native", move |n| {
                for _ in 0..n { std::hint::black_box(m1::core_native_arm::decode(bytes)); }
            }));
            cases.push(mk("P1.2", name, "core-ffi-rust", move |n| {
                for _ in 0..n { std::hint::black_box(m1::core_ffi_arm::decode(ctx, bytes)); }
            }));
        }
        {
            let v: &'static _ = Box::leak(Box::new(m2::prost_arm::value("P2.2")));
            let bytes: &'static [u8] = Box::leak(m2::prost_arm::encode(v).into_boxed_slice());
            cases.push(mk("P2.2", name, "prost", move |n| {
                for _ in 0..n { std::hint::black_box(m2::prost_arm::decode(bytes)); }
            }));
            cases.push(mk("P2.2", name, "core-native", move |n| {
                for _ in 0..n { std::hint::black_box(m2::core_native_arm::decode(bytes)); }
            }));
            cases.push(mk("P2.2", name, "core-ffi-rust", move |n| {
                for _ in 0..n { std::hint::black_box(m2::core_ffi_arm::decode(ctx2, bytes)); }
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

    let med = |c: &Case| {
        let mut t = c.times.clone();
        t.sort_by(|a, b| a.partial_cmp(b).unwrap());
        t[t.len() / 2]
    };
    println!("{:<7} {:<8} {:<14} {:>12} {:>12} {:>10} {:>12}",
             "payload", "set", "arm", "median ns", "min ns", "/ prost", "spread %");
    for pid in ["P1.2", "P2.2"] {
        for (_, name) in SETS {
            let base = cases.iter()
                .find(|c| c.payload == pid && c.set == name && c.arm == "prost")
                .map(med).unwrap();
            for arm in ["prost", "core-native", "core-ffi-rust"] {
                let c = cases.iter().find(|c| c.payload == pid && c.set == name && c.arm == arm).unwrap();
                let m = med(c);
                let lo = c.times.iter().cloned().fold(f64::INFINITY, f64::min);
                let hi = c.times.iter().cloned().fold(0.0f64, f64::max);
                println!("{:<7} {:<8} {:<14} {:>12.1} {:>12.1} {:>10.3} {:>12.1}",
                         pid, name, arm, m, lo, m / base, 100.0 * (hi - lo) / m);
            }
            println!();
        }
    }
    println!("# `/ prost` is the column to carry between the three policy builds. The absolute");
    println!("#   nanoseconds are per-process and must not be compared across them.");

    // ---- 4. the policy ALONE, all three in ONE process, so R4 holds for this table.
    //
    // Section 3's three columns are three builds, and a rebuilt binary moves by more than
    // the effect on some rows: the same lossy build measured `core-native` on P1.2 ascii at
    // 0.778 of prost in one invocation and 0.88 in the next, with prost unmoved. That is
    // code placement, not policy. This section removes the confound by calling all three
    // policies from one binary over one corpus, at the cost of not being a whole decode.
    // Read section 4 for what the policy costs and section 3 for what it does to a decode.
    println!();
    println!("## 4. the policy alone, ONE process, over every string in the payload");
    println!("#   corpus: every `string` field of the decoded payload, harvested by");
    println!("#   REFLECTION over the descriptor, so completeness is a fact and not a claim.");
    println!();
    println!("{:<7} {:<7} {:>8} {:>10} {:>9} {:>11} {:>11} {:>11} {:>8} {:>8}",
             "payload", "set", "strings", "bytes", "B/str",
             "lossy ns", "scalar ns", "simd ns", "sc/loss", "si/loss");
    for (cs, name) in SETS {
        set_content_set(cs);
        for pid in ["P1.2", "P2.2"] {
            let (bytes, root) = if pid == "P1.2" {
                (m1::prost_arm::encode(&m1::prost_arm::value(pid)), "armonik.ffi.shapes.v1.ListResultsResponse")
            } else {
                (m2::prost_arm::encode(&m2::prost_arm::value(pid)), "armonik.ffi.shapes.v1.ListTasksDetailedResponse")
            };
            let corpus = harvest(root, &bytes);
            let n_str = corpus.len();
            let n_b: usize = corpus.iter().map(|s| s.len()).sum();
            let refs: Vec<&[u8]> = corpus.iter().map(|s| s.as_bytes()).collect();
            let lossy = time_policy(&refs, |b| { std::hint::black_box(String::from_utf8_lossy(b).into_owned()); });
            let scalar = time_policy(&refs, |b| { std::hint::black_box(core::str::from_utf8(b).map(|s| s.to_owned())).ok(); });
            let simd = time_policy(&refs, |b| { std::hint::black_box(simdutf8::basic::from_utf8(b).map(|s| s.to_owned())).ok(); });
            println!("{:<7} {:<7} {:>8} {:>10} {:>9.1} {:>11.1} {:>11.1} {:>11.1} {:>8.3} {:>8.3}",
                     pid, name, n_str, n_b, n_b as f64 / n_str as f64,
                     lossy, scalar, simd, scalar / lossy, simd / lossy);
        }
    }
    set_content_set(ContentSet::Ascii);
    println!("#   ns are for the WHOLE corpus of that row, one pass, median of 11.");
    println!("#   `sc/loss` and `si/loss` are the two rejecting policies against today's lossy one.");
}

/// Every `string` field of a decoded message, found by walking the descriptor rather than
/// the generated type, so nothing can be left out by hand.
fn harvest(root: &str, bytes: &[u8]) -> Vec<String> {
    use prost_reflect::{DescriptorPool, DynamicMessage, Value};
    let pool = DescriptorPool::decode(shapes_prost::DESCRIPTOR).expect("descriptor");
    let md = pool.get_message_by_name(root).expect("root message");
    let dm = DynamicMessage::decode(md, bytes).expect("decode");
    let mut out = Vec::new();
    fn walk(v: &Value, out: &mut Vec<String>) {
        match v {
            Value::String(s) => out.push(s.clone()),
            Value::Message(m) => {
                for (_, fv) in m.fields() {
                    walk(fv, out);
                }
            }
            Value::List(l) => for e in l { walk(e, out) },
            Value::Map(m) => for (k, e) in m {
                if let prost_reflect::MapKey::String(s) = k { out.push(s.clone()) }
                walk(e, out);
            },
            _ => {}
        }
    }
    for (_, fv) in dm.fields() {
        walk(fv, &mut out);
    }
    out
}

/// Median of 11 passes over the whole corpus, in nanoseconds.
fn time_policy(refs: &[&[u8]], mut f: impl FnMut(&[u8])) -> f64 {
    let mut t = Vec::with_capacity(11);
    for _ in 0..3 { for b in refs { f(b); } }
    for _ in 0..11 {
        let s = Instant::now();
        for b in refs { f(b); }
        t.push(s.elapsed().as_nanos() as f64);
    }
    t.sort_by(|a, b| a.partial_cmp(b).unwrap());
    t[5]
}

/// A `ListResultsResponse` whose first `ResultRaw.name` is not valid UTF-8, built by
/// planting a marker through prost and then overwriting one byte of it. The length is
/// unchanged, so this is a well formed message with an ill formed string in it.
fn malformed_payload() -> Vec<u8> {
    const MARK: &[u8] = b"ZZZZZZZZZZ";
    let mut v = m1::prost_arm::value("P1.1");
    v.results[0].name = String::from_utf8(MARK.to_vec()).unwrap();
    let mut b = m1::prost_arm::encode(&v);
    let hits: Vec<usize> = b.windows(MARK.len()).enumerate()
        .filter(|(_, w)| *w == MARK).map(|(i, _)| i).collect();
    assert_eq!(hits.len(), 1, "the marker must occur exactly once, found {}", hits.len());
    b[hits[0]] = 0xFF; // never a legal UTF-8 lead byte
    b
}

struct Case {
    payload: &'static str,
    set: &'static str,
    arm: &'static str,
    reps: usize,
    run: Box<dyn FnMut(usize)>,
    times: Vec<f64>,
}

fn mk(payload: &'static str, set: &'static str, arm: &'static str,
      run: impl FnMut(usize) + 'static) -> Case {
    Case { payload, set, arm, reps: 1, run: Box::new(run), times: Vec::new() }
}
