//! Byte identity across every arm, before anything is timed (README R2).
//!
//! Every arm is checked against `ffi/schema/generated/manifest.json`, which stage 1
//! validated, rather than against another arm: a defect then shows up as a disagreement
//! with a known-good vector instead of as two of this slice's own components agreeing with
//! each other.

use harness::arms::*;
use harness::manifest::{sha, Manifest};

fn main() {
    let man = Manifest::load();
    let ffi_ctx = core_ffi_arm::Ctx::new();
    println!("# stage 2 conformance: byte identity across the arms");
    println!("#   reference: ffi/schema/generated/manifest.json (VALIDATED, stage 1)");
    println!();
    println!("{:<7} {:<16} {:>9} {:>9}  {:<8} {:<10} {}", "payload", "arm", "want B", "got B", "bytes", "sha256", "decode round trip");

    let mut bad = 0usize;
    for pid in [P1_1, P1_2, P1_3] {
        let row = man.row(pid);

        let pv = prost_arm::value(pid);
        let pb = prost_arm::encode(&pv);
        bad += report(pid, "prost", row.bytes, &row.sha256, &pb, prost_arm::decode(&pb) == pv);

        let av = armonik_arm::value(pid);
        let ab = armonik_arm::encode(&av);
        bad += report(pid, "armonik", row.bytes, &row.sha256, &ab, armonik_arm::decode(&ab) == av);

        let cv = core_native_arm::value(pid);
        let cb = core_native_arm::encode(&cv);
        bad += report(pid, "core-native", row.bytes, &row.sha256, &cb, core_native_arm::decode(&cb) == cv);

        let fv = core_ffi_arm::value(pid);
        let fb = core_ffi_arm::encode(&ffi_ctx, &fv);
        bad += report(pid, "core-ffi-rust", row.bytes, &row.sha256, &fb,
                      core_ffi_arm::decode(&ffi_ctx, &fb) == fv);

        if let Some(v) = &row.vector {
            if v != &pb || v != &ab || v != &cb || v != &fb {
                println!("{pid:<7} {:<16} committed vector disagrees with an arm", "VECTOR");
                bad += 1;
            }
        }
    }
    // ---- M2. The shape the control plane actually moves, and the first non-leaf element
    // type in this slice.
    for pid in harness::arms_m2::ALL {
        use harness::arms_m2::*;
        let row = man.row(pid);

        let pv = prost_arm::value(pid);
        let pb = prost_arm::encode(&pv);
        bad += report(pid, "prost", row.bytes, &row.sha256, &pb, prost_arm::decode(&pb) == pv);

        let av = armonik_arm::value(pid);
        let ab = armonik_arm::encode(&av);
        bad += report(pid, "armonik", row.bytes, &row.sha256, &ab, armonik_arm::decode(&ab) == av);

        let cb = core_native_arm::encode(&av);
        bad += report(pid, "core-native", row.bytes, &row.sha256, &cb,
                      core_native_arm::decode(&cb) == av);

        let fb = core_ffi_arm::encode(&ffi_ctx, &av);
        bad += report(pid, "core-ffi-rust", row.bytes, &row.sha256, &fb,
                      core_ffi_arm::decode(&ffi_ctx, &fb) == av);

        // Cross-arm value identity, which byte identity alone does not give: three arms
        // decode the same bytes through three different decoders and must land on the same
        // facade value.
        let d_a = armonik_arm::decode(&pb);
        let d_c = core_native_arm::decode(&pb);
        let d_f = core_ffi_arm::decode(&ffi_ctx, &pb);
        if d_a != d_c || d_a != d_f {
            println!("{pid:<7} {:<16} three decoders disagree on the same bytes", "VALUES");
            bad += 1;
        }

        if let Some(v) = &row.vector {
            if v != &pb || v != &ab || v != &cb || v != &fb {
                println!("{pid:<7} {:<16} committed vector disagrees with an arm", "VECTOR");
                bad += 1;
            }
        }
    }

    // ---- M3. A oneof of five members including a payload-free one, and three optional
    // scalars. Both shapes are unmeasured on .NET.
    for pid in harness::arms_m3::ALL {
        use harness::arms_m3::*;
        let row = man.row(pid);
        let pv = prost_arm::value(pid);
        let pb = prost_arm::encode(&pv);
        bad += report(pid, "prost", row.bytes, &row.sha256, &pb, prost_arm::decode(&pb) == pv);
        let av = armonik_arm::value(pid);
        let ab = armonik_arm::encode(&av);
        bad += report(pid, "armonik", row.bytes, &row.sha256, &ab, armonik_arm::decode(&ab) == av);
        let cb = core_native_arm::encode(&av);
        bad += report(pid, "core-native", row.bytes, &row.sha256, &cb, core_native_arm::decode(&cb) == av);
        let fb = core_ffi_arm::encode(&ffi_ctx, &av);
        bad += report(pid, "core-ffi-rust", row.bytes, &row.sha256, &fb,
                      core_ffi_arm::decode(&ffi_ctx, &fb) == av);
        let (d_a, d_c, d_f) = (armonik_arm::decode(&pb), core_native_arm::decode(&pb),
                               core_ffi_arm::decode(&ffi_ctx, &pb));
        if d_a != d_c || d_a != d_f {
            println!("{pid:<7} {:<16} three decoders disagree on the same bytes", "VALUES");
            bad += 1;
        }
        if let Some(v) = &row.vector {
            if v != &pb || v != &ab || v != &cb || v != &fb {
                println!("{pid:<7} {:<16} committed vector disagrees with an arm", "VECTOR");
                bad += 1;
            }
        }
    }

    // The two added arms (P2.4a, P2.4b) have no manifest row. Their check is byte identity
    // against the prost arm, which the manifest validated on every payload that does have
    // one, plus value identity across the three facade decoders.
    for pid in harness::arms_m2::ADDED {
        use harness::arms_m2::{added, armonik_arm, core_ffi_arm as ffi, core_native_arm, prost_arm};
        let pv = added::prost_value(pid);
        let pb = prost_arm::encode(&pv);
        let av = added::facade_value(pid);
        let ab = armonik_arm::encode(&av);
        let cb = core_native_arm::encode(&av);
        let fb = ffi::encode(&ffi_ctx, &av);
        let ok = ab == pb && cb == pb && fb == pb;
        let rt = core_native_arm::decode(&pb) == av && ffi::decode(&ffi_ctx, &pb) == av;
        println!(
            "{:<7} {:<16} {:>9} {:>9}  {:<8} {:<8} {}",
            pid, "added (4 arms)", pb.len(), pb.len(),
            if ok { "ok" } else { "DIFFER" }, "n/a",
            if rt { "ok" } else { "FAILED" }
        );
        bad += usize::from(!(ok && rt));
    }

    // ---- M4 to M7. The class column is per ROW: M6's four scalar packed fields are a
    // control and its packed enum is not, so labelling the table would be wrong.
    println!();
    println!("{:<7} {:<9} {:<16} {:>9} {:>9}  {:<8} {:<8} {}",
             "payload", "class", "arm", "want B", "got B", "bytes", "sha256", "decode round trip");
    {
        use harness::arms_rest::*;
        macro_rules! four {
            ($pid:expr, $class:expr, $m:ident, $encf:ident, $decf:ident) => {{
                let pid = $pid;
                let row = man.row(pid);
                let pv = $m::prost_value(pid);
                let pb = $m::prost_encode(&pv);
                bad += report4(pid, $class, "prost", row.bytes, &row.sha256, &pb,
                               $m::prost_decode(&pb) == pv);
                let av = $m::facade_value(pid);
                let ab = $m::armonik_encode(&av);
                bad += report4(pid, $class, "armonik", row.bytes, &row.sha256, &ab,
                               $m::armonik_decode(&ab) == av);
                let cb = $m::native_encode(&av);
                bad += report4(pid, $class, "core-native", row.bytes, &row.sha256, &cb,
                               $m::native_decode(&cb) == av);
                let fb = ffi::$encf(&ffi_ctx, &av).to_vec();
                bad += report4(pid, $class, "core-ffi-rust", row.bytes, &row.sha256, &fb,
                               ffi::$decf(&ffi_ctx, &fb) == av);
                if let Some(v) = &row.vector {
                    if v != &pb || v != &ab || v != &cb || v != &fb {
                        println!("{pid:<7} {:<9} {:<16} committed vector disagrees", "", "VECTOR");
                        bad += 1;
                    }
                }
            }};
        }
        four!(P4_1, "real", m4, enc_m4, dec_m4);
        for pid in [P5_1, P5_2, P5_3, P5_4] {
            four!(pid, "real", m5, enc_m5, dec_m5);
        }
        four!(P6_1, "mixed", m6, enc_m6, dec_m6);
        println!("{:<7} {:<9} {}", "", "", "^ M6 is MIXED: ticks/values/codes/flags are CONTROL");
        println!("{:<7} {:<9} {}", "", "", "  (the schema has no packed scalar); statuses is REAL");
        println!("{:<7} {:<9} {}", "", "", "  (all 3 packed fields in the schema are enums).");

        // M7: decode only. No canonical writer can produce it, so the check is that every
        // arm decodes the committed vector to the same value, and that a contiguous
        // re-encode is a PERMUTATION of the same (tag, wire type, body) triples -- the same
        // method stage 1 used, and the only statement that can be made about it.
        let row = man.row(P7_1);
        let wire = row.vector.clone().expect("P7.1 has a committed vector");
        let pd = m7::prost_decode(&wire);
        let ad = m7::armonik_decode(&wire);
        let cd = m7::native_decode(&wire);
        let fd = ffi::dec_m7(&ffi_ctx, &wire);
        let agree = ad == cd && ad == fd
            && pd.left.len() == ad.left.len() && pd.right.len() == ad.right.len();
        let re_a = m7::armonik_encode(&ad);
        let re_c = m7::native_encode(&cd);
        let re_f = ffi::enc_m7(&ffi_ctx, &fd).to_vec();
        let re_p = prost::Message::encode_to_vec(&pd);
        let contiguous = re_a == re_c && re_a == re_f && re_a == re_p;
        let permutation = re_a != wire && triples(&re_a) == triples(&wire);
        println!("{:<7} {:<9} {:<16} {:>9} {:>9}  {:<8} {:<8} {}",
                 P7_1, "CONTROL", "4 arms, decode", wire.len(), re_a.len(),
                 if agree { "ok" } else { "DIFFER" },
                 if contiguous { "ok" } else { "DIFFER" },
                 if permutation { "re-encode is a permutation of the same triples" }
                 else { "PERMUTATION CHECK FAILED" });
        println!("{:<7} {:<9} {}", "", "", "^ CONTROL, and DECODE ONLY: no canonical writer can");
        println!("{:<7} {:<9} {}", "", "", "  produce these bytes, so no arm is asked to.");
        bad += usize::from(!(agree && contiguous && permutation));
    }

    println!();
    if bad == 0 {
        println!("VERDICT: every arm is byte-identical to the validated manifest on every payload it covers.");
    } else {
        println!("VERDICT: {bad} disagreement(s). Nothing is timed until this is zero.");
        std::process::exit(1);
    }
}

/// Every (tag path, wire type, body) triple in a buffer, sorted. Two encodings of one value
/// that differ only in field ORDER are the same multiset of these.
fn triples(b: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    walk(b, "", &mut out);
    out.sort();
    out
}

fn walk(b: &[u8], prefix: &str, out: &mut Vec<String>) {
    let mut i = 0usize;
    while i < b.len() {
        let mut k = 0u64;
        let mut sh = 0u32;
        loop {
            if i >= b.len() { return; }
            let c = b[i];
            i += 1;
            k |= ((c & 0x7f) as u64) << sh;
            if c & 0x80 == 0 { break; }
            sh += 7;
        }
        let (tag, wire) = (k >> 3, (k & 7) as u8);
        let path = if prefix.is_empty() { format!("{tag}") } else { format!("{prefix}.{tag}") };
        match wire {
            0 => { while i < b.len() && b[i] & 0x80 != 0 { i += 1; } i += 1; }
            1 => i += 8,
            5 => i += 4,
            2 => {
                let mut n = 0u64;
                let mut s2 = 0u32;
                loop {
                    if i >= b.len() { return; }
                    let c = b[i];
                    i += 1;
                    n |= ((c & 0x7f) as u64) << s2;
                    if c & 0x80 == 0 { break; }
                    s2 += 7;
                }
                let n = n as usize;
                if i + n > b.len() { return; }
                out.push(format!("{path}/2/{}", hex(&b[i..i + n])));
                walk(&b[i..i + n], &path, out);
                i += n;
            }
            _ => return,
        }
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|c| format!("{c:02x}")).collect()
}

fn report4(pid: &str, class: &str, arm: &str, want_b: usize, want_sha: &str, got: &[u8], rt: bool) -> usize {
    let (ok_b, ok_s) = (got.len() == want_b, sha(got) == want_sha);
    println!("{:<7} {:<9} {:<16} {:>9} {:>9}  {:<8} {:<8} {}",
             pid, class, arm, want_b, got.len(),
             if ok_b { "ok" } else { "DIFFER" },
             if ok_s { "ok" } else { "DIFFER" },
             if rt { "ok" } else { "FAILED" });
    usize::from(!(ok_b && ok_s && rt))
}

fn report(pid: &str, arm: &str, want_b: usize, want_sha: &str, got: &[u8], rt: bool) -> usize {
    let ok_b = got.len() == want_b;
    let ok_s = sha(got) == want_sha;
    println!(
        "{:<7} {:<16} {:>9} {:>9}  {:<8} {:<10} {}",
        pid, arm, want_b, got.len(),
        if ok_b { "ok" } else { "DIFFER" },
        if ok_s { "ok" } else { "DIFFER" },
        if rt { "ok" } else { "FAILED" }
    );
    usize::from(!(ok_b && ok_s && rt))
}
