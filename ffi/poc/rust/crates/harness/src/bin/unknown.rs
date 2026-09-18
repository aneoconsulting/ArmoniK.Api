//! ABI v1 open decision 11: what an optional bag of unknown fields costs.
//!
//! Decision 11 records that the core retains nothing while `Google.Protobuf`, protobuf-java,
//! protobuf C++ and upb all retain, so adopting the core removes a protobuf guarantee from
//! four of the five languages. Nobody had priced the fix. This is the price.
//!
//! **The gating question was structural and is answered before anything here runs.**
//! `gen/unknown_predicate.py` puts a bag into the schema under two modellings and runs ABI
//! v1 7.2's batching predicate -- `emit/shapes.py:is_leaf`, imported rather than re-derived
//! (R1) -- over the result. A REPEATED bag takes the schema from 9 leaf messages to **0**:
//! every element type stops batching, including `ResultRaw`, which is what turns 1000 rows
//! into 9 crossings. ONE opaque `bytes` blob changes the leafness of **no** message. So the
//! bag is one blob, and the arm below is built on that and on nothing else.
//!
//! What is built, all beside the default and none of it changing the default path:
//!
//!   encode   one `ak_str` slot per group (`ak_ufix_*`), appended verbatim by the codec
//!            after the known fields. Empty is `tc == NULL`, the existing absent convention.
//!            A nested message gets its own bag, so a child slot is the child's `u` group.
//!   decode   unknown runs captured as spans into the buffer the host handed in and
//!            delivered as a SIDE RUN keyed by token -- not a slot in the element group, so
//!            the group stays a fixed-size POD and the predicate never even sees it. The
//!            core copies nothing and allocates nothing.
//!
//! The four encode arms exist because decision 9 interacts: the bag adds a slot to every
//! group, so the total fill grows on every message -- including P1.3, where the fill IS the
//! cost.

use harness::arms as m1;
use harness::arms_m2 as m2;
use harness::arms_m3 as m3;
use harness::manifest::{sha, Manifest};
use std::time::Instant;

const ROUNDS: usize = 15;
const TARGET_NS: u128 = 25_000_000;

fn main() {
    let man = Manifest::load();
    println!("# the unknown-field bag (ABI v1 open decision 11), as an ARM");
    println!("#   guard: {}", if cfg!(feature = "guard") { "on" } else { "OFF" });
    println!("#   counting build: {}", if cfg!(feature = "count") { "YES -- timings NOT usable" } else { "no" });
    println!("#   rounds: {ROUNDS}, interleaved, one process (R4)");
    println!();

    let ctx: &'static _ = Box::leak(Box::new(m1::core_ffi_arm::Ctx::new()));
    let mut bad = 0usize;

    // ---- 1. the empty bag changes no byte, on any payload, under any fill.
    println!("## 1. byte identity against the validated manifest, EMPTY bag");
    println!("#   in production the bag is always empty: ArmoniK ships both sides from one");
    println!("#   release, so retention is pure overhead until a version skews.");
    println!("{:<7} {:<26} {:>9} {:>9} {:<8} {:<8}", "payload", "arm", "want B", "got B", "bytes", "sha256");
    for pid in ["P1.1", "P1.2", "P1.3"] {
        let row = man.row(pid);
        let v = m1::armonik_arm::value(pid);
        for (arm, b) in [
            ("core-ffi-rust", m1::core_ffi_arm::encode(ctx, &v)),
            ("core-ffi-unk", m1::core_ffi_unk::encode(ctx, &v)),
            ("core-ffi-unk+zeroed", m1::core_ffi_unk::encode_into_zeroed(ctx, &v).to_vec()),
        ] {
            bad += report(pid, arm, row.bytes, &row.sha256, &b);
        }
    }
    for pid in m3::ALL {
        let row = man.row(pid);
        let v = m3::armonik_arm::value(pid);
        for (arm, b) in [
            ("core-ffi-rust", m3::core_ffi_arm::encode(ctx, &v)),
            ("core-ffi-unk", m3::core_ffi_unk::encode(ctx, &v)),
        ] {
            bad += report(pid, arm, row.bytes, &row.sha256, &b);
        }
    }
    for pid in m2::ALL {
        let row = man.row(pid);
        let v = m2::armonik_arm::value(pid);
        for (arm, b) in [
            ("core-ffi-rust", m2::core_ffi_arm::encode(ctx, &v)),
            ("core-ffi-unk", m2::core_ffi_unk::encode(ctx, &v)),
            ("core-ffi-unk+zeroed", m2::core_ffi_unk::encode_into_zeroed(ctx, &v).to_vec()),
        ] {
            bad += report(pid, arm, row.bytes, &row.sha256, &b);
        }
    }

    // ---- 2. the property the whole thing exists for.
    println!();
    println!("## 2. ROUND-TRIP BYTE IDENTITY on messages carrying unknown fields");
    println!("#   decode with the capture on, re-encode with the bag, compare with the input.");
    println!("#   None of the existing vectors test this: today the unknowns are dropped and");
    println!("#   the re-encode is legitimately shorter.");
    println!();
    println!("#   THE ORDERING QUESTION, answered rather than chosen silently: the bag is");
    println!("#   APPENDED AT THE END of the message. Canonical form ascends by tag, so an");
    println!("#   unknown tag that sits numerically between two known tags re-encodes in a");
    println!("#   different ORDER -- same fields, same bytes, different position. The");
    println!("#   `expected` column says which vectors that applies to, and the gate is");
    println!("#   agreement with the expectation, not identity.");
    println!();
    println!("{:<46} {:>6} {:>6} {:<12} {:<12} {:<9} {}",
             "vector", "in B", "out B", "bag == input", "layout ident", "expected", "semantic RT");
    for (name, body, want_bag, expect) in vectors() {
        let v = m3::core_ffi_unk::decode(ctx, &body);
        let mut got_bag: Vec<u8> = v.unknown_fields.clone();
        for p in &v.probes {
            got_bag.extend_from_slice(&p.unknown_fields);
        }
        // (a) THE BAG'S BYTES ARE PRESERVED EXACTLY. This is still a byte property and it
        //     is still checkable: the retained blob against the unknown runs of the input.
        let bag_ok = got_bag == want_bag;
        let out = m3::core_ffi_unk::encode(ctx, &v);
        // (b) THE MESSAGE'S BYTE LAYOUT IS NOT, when an unknown tag sits numerically
        //     between two known ones: the bag is appended, so it moves to the end.
        let ident = out == body;
        // (c) So that case is validated SEMANTICALLY: decode, re-encode, decode again,
        //     compare values -- which includes the retained blob, since it is a field.
        let again = m3::core_ffi_unk::decode(ctx, &out);
        let same = again == v;
        println!("{:<46} {:>6} {:>6} {:<12} {:<12} {:<9} {}", name, body.len(), out.len(),
                 if bag_ok { "yes" } else { "NO" },
                 if ident { "yes" } else { "no" },
                 if expect { "yes" } else { "no" },
                 if same { "yes" } else { "NO" });
        if ident != expect || !same || !bag_ok {
            bad += 1;
        }
    }

    // ---- 3. the positive control: an arm that silently dropped the bag would pass
    //         section 1 and measure identically, which is the failure mode this is for.
    println!();
    println!("## 3. positive control: the DEFAULT decode must NOT retain");
    let (name, body, _, _) = vectors().into_iter().next().unwrap();
    let dropped = m3::core_ffi_arm::decode(ctx, &body);
    let kept: usize = dropped.probes.iter().map(|p| p.unknown_fields.len()).sum();
    let re = m3::core_ffi_arm::encode(ctx, &dropped);
    println!("  vector                 {name}");
    println!("  default decode kept    {kept} bytes  (must be 0)");
    println!("  default re-encode      {} B against {} B in  (must be SHORTER)", re.len(), body.len());
    let ctl = kept == 0 && re.len() < body.len();
    println!("  -> {}", if ctl { "ok: the capture is doing the work, not the default path" }
             else { "FAILED: the default path already retains, so section 2 proves nothing" });
    bad += usize::from(!ctl);

    if bad > 0 {
        println!("\nFAILED: {bad} problem(s). Nothing below is usable.");
        std::process::exit(1);
    }

    // ---- 4. the empty-bag cost.
    println!();
    println!("## 4. the EMPTY-bag cost, one process, interleaved");
    let mut cases: Vec<Case> = Vec::new();
    for pid in ["P1.2", "P1.3"] {
        {
            let v: &'static _ = Box::leak(Box::new(m1::prost_arm::value(pid)));
            let buf: &'static mut Vec<u8> = Box::leak(Box::new(Vec::with_capacity(1 << 20)));
            cases.push(mk(pid, "encode", "prost", move |n| {
                for _ in 0..n { buf.clear(); prost::Message::encode(v, buf).unwrap(); std::hint::black_box(&buf); }
            }));
        }
        let v: &'static _ = Box::leak(Box::new(m1::armonik_arm::value(pid)));
        cases.push(mk(pid, "encode", "total fill", move |n| {
            for _ in 0..n { std::hint::black_box(m1::core_ffi_arm::encode_into(ctx, v)); }
        }));
        cases.push(mk(pid, "encode", "total fill + bag", move |n| {
            for _ in 0..n { std::hint::black_box(m1::core_ffi_unk::encode_into(ctx, v)); }
        }));
        cases.push(mk(pid, "encode", "zeroed", move |n| {
            for _ in 0..n { std::hint::black_box(m1::core_ffi_zeroed::encode_into(ctx, v)); }
        }));
        cases.push(mk(pid, "encode", "zeroed + bag", move |n| {
            for _ in 0..n { std::hint::black_box(m1::core_ffi_unk::encode_into_zeroed(ctx, v)); }
        }));
        let bytes: &'static [u8] = Box::leak(m1::prost_arm::encode(&m1::prost_arm::value(pid)).into_boxed_slice());
        cases.push(mk(pid, "decode", "prost", move |n| {
            for _ in 0..n { std::hint::black_box(m1::prost_arm::decode(bytes)); }
        }));
        cases.push(mk(pid, "decode", "no capture", move |n| {
            for _ in 0..n { std::hint::black_box(m1::core_ffi_arm::decode(ctx, bytes)); }
        }));
        cases.push(mk(pid, "decode", "capture on", move |n| {
            for _ in 0..n { std::hint::black_box(m1::core_ffi_unk::decode(ctx, bytes)); }
        }));
    }
    for pid in ["P2.2", "P2.5"] {
        {
            let v: &'static _ = Box::leak(Box::new(m2::prost_arm::value(pid)));
            let buf: &'static mut Vec<u8> = Box::leak(Box::new(Vec::with_capacity(4 << 20)));
            cases.push(mk(pid, "encode", "prost", move |n| {
                for _ in 0..n { buf.clear(); prost::Message::encode(v, buf).unwrap(); std::hint::black_box(&buf); }
            }));
        }
        let v: &'static _ = Box::leak(Box::new(m2::armonik_arm::value(pid)));
        cases.push(mk(pid, "encode", "total fill", move |n| {
            for _ in 0..n { std::hint::black_box(m2::core_ffi_arm::encode_into(ctx, v)); }
        }));
        cases.push(mk(pid, "encode", "total fill + bag", move |n| {
            for _ in 0..n { std::hint::black_box(m2::core_ffi_unk::encode_into(ctx, v)); }
        }));
        cases.push(mk(pid, "encode", "zeroed", move |n| {
            for _ in 0..n { std::hint::black_box(m2::core_ffi_zeroed::encode_into(ctx, v)); }
        }));
        cases.push(mk(pid, "encode", "zeroed + bag", move |n| {
            for _ in 0..n { std::hint::black_box(m2::core_ffi_unk::encode_into_zeroed(ctx, v)); }
        }));
        let bytes: &'static [u8] = Box::leak(m2::prost_arm::encode(&m2::prost_arm::value(pid)).into_boxed_slice());
        cases.push(mk(pid, "decode", "prost", move |n| {
            for _ in 0..n { std::hint::black_box(m2::prost_arm::decode(bytes)); }
        }));
        cases.push(mk(pid, "decode", "no capture", move |n| {
            for _ in 0..n { std::hint::black_box(m2::core_ffi_arm::decode(ctx, bytes)); }
        }));
        cases.push(mk(pid, "decode", "capture on", move |n| {
            for _ in 0..n { std::hint::black_box(m2::core_ffi_unk::decode(ctx, bytes)); }
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
    let get = |cs: &Vec<Case>, pid: &str, dir: &str, arm: &str| -> Option<f64> {
        cs.iter().find(|c| c.payload == pid && c.dir == dir && c.arm == arm).map(&med)
    };

    println!();
    println!("{:<7} {:<7} {:<18} {:>12} {:>12} {:>9} {:>9}",
             "payload", "dir", "arm", "median ns", "min ns", "/ prost", "spread %");
    for pid in ["P1.2", "P1.3", "P2.2", "P2.5"] {
        for dir in ["encode", "decode"] {
            let base = get(&cases, pid, dir, "prost").unwrap();
            for arm in ["prost", "total fill", "total fill + bag", "zeroed", "zeroed + bag",
                        "no capture", "capture on"] {
                let Some(c) = cases.iter().find(|c| c.payload == pid && c.dir == dir && c.arm == arm) else { continue };
                let m = med(c);
                let lo = c.times.iter().cloned().fold(f64::INFINITY, f64::min);
                let hi = c.times.iter().cloned().fold(0.0f64, f64::max);
                println!("{:<7} {:<7} {:<18} {:>12.1} {:>12.1} {:>9.3} {:>9.1}",
                         pid, dir, arm, m, lo, m / base, 100.0 * (hi - lo) / m);
            }
            println!();
        }
    }

    println!("## 5. what the empty bag costs, per element");
    println!("{:<7} {:>6} {:>18} {:>18} {:>18}", "payload", "elems", "bag/total fill", "bag/zeroed", "capture/no capture");
    for pid in ["P1.2", "P1.3", "P2.2", "P2.5"] {
        let n = match pid { "P1.2" => 1000.0, "P1.3" => 300.0, _ => 500.0 };
        let tf = get(&cases, pid, "encode", "total fill").unwrap();
        let tb = get(&cases, pid, "encode", "total fill + bag").unwrap();
        let z = get(&cases, pid, "encode", "zeroed").unwrap();
        let zb = get(&cases, pid, "encode", "zeroed + bag").unwrap();
        let dn = get(&cases, pid, "decode", "no capture").unwrap();
        let dc = get(&cases, pid, "decode", "capture on").unwrap();
        println!("{:<7} {:>6.0} {:>10.3} {:+.2}ns {:>10.3} {:+.2}ns {:>10.3} {:+.2}ns",
                 pid, n, tb / tf, (tb - tf) / n, zb / z, (zb - z) / n, dc / dn, (dc - dn) / n);
    }
}

fn report(pid: &str, arm: &str, want: usize, want_sha: &str, b: &[u8]) -> usize {
    let (ok_b, ok_h) = (b.len() == want, sha(b) == want_sha);
    println!("{:<7} {:<26} {:>9} {:>9} {:<8} {:<8}", pid, arm, want, b.len(),
             if ok_b { "ok" } else { "DIFFER" }, if ok_h { "ok" } else { "DIFFER" });
    usize::from(!(ok_b && ok_h))
}

// ----------------------------------------------------------------- wire vectors

fn varint(out: &mut Vec<u8>, mut v: u64) {
    while v >= 0x80 { out.push((v as u8) | 0x80); v >>= 7; }
    out.push(v as u8);
}
fn key(out: &mut Vec<u8>, tag: u32, wire: u32) { varint(out, ((tag << 3) | wire) as u64); }
fn ld(out: &mut Vec<u8>, tag: u32, body: &[u8]) {
    key(out, tag, 2);
    varint(out, body.len() as u64);
    out.extend_from_slice(body);
}
fn wrap(body: &[u8]) -> Vec<u8> { let mut o = Vec::new(); ld(&mut o, 1, body); o }

/// One `ListProbeResponse` per vector. The unknown fields sit inside the `Probe` element,
/// which is where the existing seven vectors put them.
/// `(name, whole message, the unknown runs it carries, is the re-encode byte-identical)`
fn vectors() -> Vec<(&'static str, Vec<u8>, Vec<u8>, bool)> {
    let mut v: Vec<(&'static str, Vec<u8>, Vec<u8>, bool)> = Vec::new();

    let mut u = Vec::new();
    key(&mut u, 20, 0); varint(&mut u, 12345);
    ld(&mut u, 21, b"an unknown length-delimited field");
    key(&mut u, 22, 5); u.extend_from_slice(&7u32.to_le_bytes());
    key(&mut u, 23, 1); u.extend_from_slice(&9u64.to_le_bytes());
    let mut b = Vec::new();
    ld(&mut b, 1, b"id");
    b.extend_from_slice(&u);
    v.push(("all four wire types, AFTER every known field", wrap(&b), u, true));

    let mut u = Vec::new();
    key(&mut u, 20, 0); varint(&mut u, 12345);
    let mut b = u.clone();
    ld(&mut b, 1, b"id");
    v.push(("one unknown BEFORE every known field", wrap(&b), u, false));

    // The ordering case. Probe's known tags are 1 (id) and 2..4 (the optional fields);
    // tag 5 sits numerically BETWEEN 4 and the oneof's 10..14.
    let mut u = Vec::new();
    key(&mut u, 5, 0); varint(&mut u, 77);
    let mut b = Vec::new();
    ld(&mut b, 1, b"id");
    b.extend_from_slice(&u);
    key(&mut b, 10, 0); varint(&mut b, 7);
    v.push(("an unknown tag BETWEEN two known tags (the ordering case)", wrap(&b), u, false));

    let mut u1 = Vec::new();
    key(&mut u1, 20, 0); varint(&mut u1, 1);
    let mut u2 = Vec::new();
    key(&mut u2, 21, 0); varint(&mut u2, 2);
    let mut b = Vec::new();
    ld(&mut b, 1, b"a");
    b.extend_from_slice(&u1);
    let mut c = Vec::new();
    ld(&mut c, 1, b"b");
    c.extend_from_slice(&u2);
    let mut o = Vec::new();
    ld(&mut o, 1, &b);
    ld(&mut o, 1, &c);
    let mut all = u1.clone();
    all.extend_from_slice(&u2);
    v.push(("two elements, each with its own unknown field", o, all, true));

    v
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
