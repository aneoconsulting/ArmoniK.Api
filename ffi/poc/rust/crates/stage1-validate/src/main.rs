//! Stage 1 of the rust slice: validate `ffi/schema/generated/manifest.json` against prost.
//!
//! Every hash in that manifest was produced by `ffi/schema/emit/wire.py`, which no protobuf
//! implementation has ever checked. `emit/check.py` proves the framing; nothing proved the
//! semantics. This binary re-derives every payload from the value rules, encodes it with
//! prost, and compares bytes -- not hashes -- against what the emitters produce.
//!
//!   cargo run -p stage1-validate --release -- <dir of regenerated .bin> [--vectors <dir>]
//!
//! `<dir>` comes from `gen/dump_payloads.py`, because `schema/generated/payloads/` commits
//! only the seven vectors at or under 64 KB. `--vectors` additionally checks the committed
//! seven byte-for-byte against the regenerated ones, so a stale `generated/` is caught here
//! rather than in a later slice.

mod build;
mod rawdiff;

use build::Mode;
use prost::Message;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn sha(b: &[u8]) -> String {
    let d = Sha256::digest(b);
    d.iter().map(|c| format!("{c:02x}")).collect()
}

/// What prost encodes for each payload. P7.1 has no entry: prost writes a repeated field
/// contiguously and the payload interleaves two of them on purpose, so no prost value can
/// produce those bytes. It is checked by decode below.
fn encode(pid: &str) -> Option<Vec<u8>> {
    let b = match pid {
        "P1.1" => build::list_results(4, Mode::Full).encode_to_vec(),
        "P1.2" => build::list_results(1000, Mode::Full).encode_to_vec(),
        "P1.3" => build::list_results(300, Mode::AllAbsent).encode_to_vec(),
        "P2.1" => build::list_tasks(1, Mode::Full, &[3]).encode_to_vec(),
        "P2.2" => build::list_tasks(500, Mode::Full, &[3]).encode_to_vec(),
        "P2.3" => build::list_tasks(125, Mode::Full, &[30]).encode_to_vec(),
        "P2.4" => build::list_tasks(80, Mode::Full, &[3, 150]).encode_to_vec(),
        "P2.5" => build::list_tasks(20, Mode::HalfAbsent, &[3]).encode_to_vec(),
        "P3.1" => build::list_probes(200).encode_to_vec(),
        "P4.1" => build::list_summaries(200).encode_to_vec(),
        "P5.1" => build::upload(36).encode_to_vec(),
        "P5.2" => build::upload(65536).encode_to_vec(),
        "P5.3" => build::upload(1048576).encode_to_vec(),
        "P5.4" => build::upload(4194304).encode_to_vec(),
        "P6.1" => build::list_metrics(200).encode_to_vec(),
        _ => return None,
    };
    Some(b)
}

/// P7.1 is legal wire that prost can read and cannot write. Two checks stand in for the
/// byte comparison: prost decodes it into the values the rules predict, and re-encoding
/// that value gives the same payload with the two fields written contiguously.
fn check_interleaved(expected: &[u8]) -> Result<String, String> {
    use shapes_prost::shapes as p;
    let got = p::DualResponse::decode(expected).map_err(|e| format!("prost refused it: {e}"))?;
    let want = build::dual(3);
    if got != want {
        return Err(format!(
            "prost decoded it to a value the rules do not predict:\n    got  {got:?}\n    want {want:?}"
        ));
    }
    let re = want.encode_to_vec();
    if re == expected {
        return Err("re-encoding reproduced the interleaving, which prost cannot do: \
                    the payload is not actually interleaved"
            .into());
    }
    let mut a = rawdiff::flatten_sorted(expected);
    let mut b = rawdiff::flatten_sorted(&re);
    a.sort();
    b.sort();
    if a != b {
        return Err("re-encoding contiguously is not a permutation of the interleaved bytes".into());
    }
    Ok(format!(
        "decoded to the predicted value; contiguous re-encode is {} B and a permutation of the {} B interleaved form",
        re.len(),
        expected.len()
    ))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = PathBuf::from(args.get(1).expect("usage: stage1-validate <dir> [--vectors <dir>]"));
    let vectors = args.iter().position(|a| a == "--vectors").map(|i| PathBuf::from(&args[i + 1]));
    // `--emit <dir>` writes what prost encodes, so a candidate fix to the emitters can be
    // diffed against it outside this binary.
    if let Some(i) = args.iter().position(|a| a == "--emit") {
        let out = PathBuf::from(&args[i + 1]);
        std::fs::create_dir_all(&out).unwrap();
        for pid in ["P1.1", "P1.2", "P1.3", "P2.1", "P2.2", "P2.3", "P2.4", "P2.5", "P3.1",
                    "P4.1", "P5.1", "P5.2", "P5.3", "P5.4", "P6.1"] {
            let b = encode(pid).unwrap();
            std::fs::write(out.join(format!("{}.bin", pid.replace('.', "_"))), &b).unwrap();
        }
        println!("wrote prost encodings to {}", out.display());
        return;
    }

    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    let generated = here.join("../../../../schema/generated").canonicalize().unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(generated.join("manifest.json")).unwrap()).unwrap();
    let rows = manifest["payloads"].as_object().unwrap();

    println!("# stage 1: ffi/schema/generated/manifest.json against prost");
    println!("#");
    println!("# configuration");
    println!("#   rustc          {}", env!("RUSTC_V"));
    println!("#   prost          0.14 (prost-build 0.14, protox 0.9; no protoc in this container)");
    println!("#   description    ffi/schema/shapes.json, ffi/schema/generated/shapes.proto");
    println!("#   manifest       generated_by = {}", manifest["generated_by"]);
    println!("#   maps           prost-build btree_map(\".\"): BTreeMap, so map entries sort by key");
    println!("#   host           {}", std::fs::read_to_string("/proc/sys/kernel/hostname").unwrap_or_default().trim());
    println!();
    println!(
        "{:<6} {:>10} {:>10}  {:<8} {:<8} {}",
        "payload", "manifest B", "prost B", "bytes", "sha256", "note"
    );

    let mut failures: Vec<String> = Vec::new();
    let mut checked = 0usize;

    // A stale generated/payloads/ would make every later slice diff against the wrong thing.
    if let Some(vdir) = &vectors {
        for (pid, row) in rows.iter() {
            let Some(rel) = row.get("vector").and_then(|v| v.as_str()) else { continue };
            let name = Path::new(rel).file_name().unwrap();
            let committed = std::fs::read(vdir.join(name)).unwrap();
            let regenerated = std::fs::read(dir.join(name)).unwrap();
            if committed != regenerated {
                failures.push(format!("{pid}: the committed vector {rel} is not what emit/payloads.py produces today"));
            }
        }
    }

    let mut order: Vec<&String> = rows.keys().collect();
    order.sort();
    for pid in order {
        let row = &rows[pid];
        let want_bytes = row["bytes"].as_u64().unwrap() as usize;
        let want_sha = row["sha256"].as_str().unwrap();
        let file = dir.join(format!("{}.bin", pid.replace('.', "_")));
        let expected = std::fs::read(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));

        // The regenerated file must first agree with the manifest row it is being used for.
        if expected.len() != want_bytes || sha(&expected) != want_sha {
            failures.push(format!("{pid}: the regenerated payload does not match its own manifest row"));
            continue;
        }

        match encode(pid) {
            Some(got) => {
                checked += 1;
                let ok_len = got.len() == want_bytes;
                let ok_sha = sha(&got) == want_sha;
                println!(
                    "{:<6} {:>10} {:>10}  {:<8} {:<8} {}",
                    pid,
                    want_bytes,
                    got.len(),
                    if ok_len { "ok" } else { "DIFFER" },
                    if ok_sha { "ok" } else { "DIFFER" },
                    if got == expected { "identical" } else { "MISMATCH" }
                );
                if got != expected {
                    let at = got.iter().zip(expected.iter()).position(|(a, b)| a != b);
                    failures.push(format!(
                        "{pid}: prost and emit/wire.py disagree\n  first differing byte: {}\n{}",
                        at.map(|n| format!("0x{n:x}")).unwrap_or_else(|| "none (one is a prefix of the other)".into()),
                        rawdiff::first_difference(&expected, &got)
                    ));
                }
            }
            None => {
                // P7.1
                match check_interleaved(&expected) {
                    Ok(note) => {
                        checked += 1;
                        println!(
                            "{:<6} {:>10} {:>10}  {:<8} {:<8} {}",
                            pid, want_bytes, "-", "n/a", "n/a", note
                        );
                    }
                    Err(e) => {
                        println!("{:<6} {:>10} {:>10}  {:<8} {:<8} {}", pid, want_bytes, "-", "n/a", "n/a", "FAILED");
                        failures.push(format!("{pid}: {e}"));
                    }
                }
            }
        }
    }

    // Round-trip the other way as well: prost must read back what it wrote, so a payload
    // that matches by luck of a symmetric defect still has to decode to the same value.
    let mut roundtrip: BTreeMap<&str, bool> = BTreeMap::new();
    {
        use shapes_prost::shapes as p;
        macro_rules! rt {
            ($pid:literal, $ty:ty, $val:expr) => {{
                let v: $ty = $val;
                let b = v.encode_to_vec();
                roundtrip.insert($pid, <$ty>::decode(&b[..]).map(|d| d == v).unwrap_or(false));
            }};
        }
        rt!("P1.2", p::ListResultsResponse, build::list_results(1000, Mode::Full));
        rt!("P1.3", p::ListResultsResponse, build::list_results(300, Mode::AllAbsent));
        rt!("P2.2", p::ListTasksDetailedResponse, build::list_tasks(500, Mode::Full, &[3]));
        rt!("P2.5", p::ListTasksDetailedResponse, build::list_tasks(20, Mode::HalfAbsent, &[3]));
        rt!("P3.1", p::ListProbeResponse, build::list_probes(200));
        rt!("P4.1", p::ListTaskSummaryResponse, build::list_summaries(200));
        rt!("P6.1", p::ListMetricsResponse, build::list_metrics(200));
    }
    println!();
    println!("# prost encode/decode round trip");
    for (pid, ok) in &roundtrip {
        println!("#   {pid:<6} {}", if *ok { "ok" } else { "FAILED" });
        if !ok {
            failures.push(format!("{pid}: prost does not decode its own encoding back to the same value"));
        }
    }

    println!();
    if failures.is_empty() {
        println!("VERDICT: {checked}/16 payloads checked with prost, no disagreement.");
        println!("         The manifest's hashes are confirmed. emit/wire.py agrees with prost on");
        println!("         every payload, including the absent-path ones (P1.3, P2.5), explicit");
        println!("         presence and the payload-free oneof member (P3.1), the adapter site");
        println!("         (P4.1), packed scalars (P6.1) and the 4 MB bulk body (P5.4).");
    } else {
        println!("VERDICT: {} disagreement(s).", failures.len());
        for f in &failures {
            println!("\n{f}");
        }
        std::process::exit(1);
    }
}
