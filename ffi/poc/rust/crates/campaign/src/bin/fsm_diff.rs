//! FIX-PLAN D23: the FSM decode family's differential on the codec suite's inputs.
//!
//! For every input of the codec suite (the 16 shape payloads, the Latin-1 and wide content
//! sets, every accepted `U-*` corpus row the shapes roots carry) and every unknown-field mode
//! of this build (drop and retain, or no-unknown):
//!
//!   1. the FSM's event stream equals the pull family's log record for record (op, slot,
//!      token, n, payload bytes; pull pads to 8, the FSM's `bytes` is exact), the return
//!      codes agree, a call after the end is refused, and in retain mode the unknown-field
//!      buffers are byte-identical (`fsm_diff_<root>` in the generated binding);
//!   2. the Rust FSM consumer's graph equals push's (`fsm_graph_<root>`);
//!   2b. VALID re-orderings of each input: an unknown varint field (number 9999) spliced
//!      between every two top-level records (every open run flushed by a foreign tag, the
//!      FSM's rewind path), and the top-level records in reverse order; plus synthetic
//!      inputs whose packed runs exceed the 32 KB arena inside ONE packed body (the FSM's
//!      resume-inside-a-packed-body path) and whose string runs exceed it;
//!   3. MALFORMED variants of each input -- truncations at up to 48 points and single-byte
//!      mutations at up to 48 points (deterministic) -- under the D20 mask 0 and all-ones
//!      (given to BOTH families): the same error code, the same events before it.
//!
//! Prints one line per (input, mode): records, events, FSM calls (begin + nexts), and the
//! counting build's forward / reverse crossings of each family. Exit 1 on any failure.
//!
//!   fsm_diff [--only PREFIX,...] [--no-malformed]

use campaign::*;
use harness::generated::binding;

/// The top-level records of `b` (key + value bytes), or None if `b` is not well-formed.
fn records(b: &[u8]) -> Option<Vec<Vec<u8>>> {
    let mut d = ak_rt::dec::Dec::new(b);
    let mut out = Vec::new();
    while !d.at_end() {
        let s0 = d.pos;
        let k = d.varint();
        if d.err != 0 {
            return None;
        }
        d.skip((k >> 3) as u32, (k & 7) as u32);
        if d.err != 0 {
            return None;
        }
        out.push(b[s0..d.pos].to_vec());
    }
    Some(out)
}

fn varint(out: &mut Vec<u8>, mut v: u64) {
    while v >= 0x80 {
        out.push((v as u8) | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

fn ld(out: &mut Vec<u8>, field: u64, body: &[u8]) {
    varint(out, field << 3 | 2);
    varint(out, body.len() as u64);
    out.extend_from_slice(body);
}

/// Synthetic inputs (root, id, bytes): runs past the arena budget INSIDE one packed body and
/// across many string elements. ListMetricsResponse { batches = 1: MetricsBatch { ticks = 2
/// packed int64, values = 3 packed double, codes = 4 packed int32, flags = 5 packed bool,
/// statuses = 6 packed enum } }; ListTasksDetailedResponse { tasks = 1: TaskDetailed
/// { 4 and 5: repeated string, ... } } (field numbers read from the generated FSM).
fn synthetic() -> Vec<(&'static str, String, Vec<u8>)> {
    let mut v = Vec::new();
    for (n_ticks, n_vals, n_flags) in [(5000usize, 4097usize, 40000usize), (4096, 4096, 32768), (70000, 9000, 70000)] {
        let mut batch = Vec::new();
        let mut t = Vec::new();
        for i in 0..n_ticks { varint(&mut t, (i as u64 * 7919) % 300); }
        ld(&mut batch, 2, &t);
        let mut f = Vec::new();
        for i in 0..n_vals { f.extend_from_slice(&(i as f64 * 0.5).to_le_bytes()); }
        ld(&mut batch, 3, &f);
        let mut c = Vec::new();
        for i in 0..n_ticks / 2 { varint(&mut c, i as u64 % 5); }
        ld(&mut batch, 4, &c);
        let mut g = Vec::new();
        for i in 0..n_flags { varint(&mut g, (i % 3 == 0) as u64); }
        ld(&mut batch, 5, &g);
        // the same packed field twice in a row: the run continues across the two bodies
        ld(&mut batch, 5, &g[..100]);
        let mut st = Vec::new();
        for i in 0..1000 { varint(&mut st, i % 3); }
        ld(&mut batch, 6, &st);
        let mut root = Vec::new();
        ld(&mut root, 1, &batch);
        // a second, smaller batch: the first two records of the first one
        let r2: Vec<u8> = records(&batch).unwrap().into_iter().take(2).flatten().collect();
        ld(&mut root, 1, &r2);
        v.push(("ListMetricsResponse", format!("SYN-packed-{n_ticks}-{n_vals}-{n_flags}"), root));
    }
    let mut task = Vec::new();
    for i in 0..5000u32 { ld(&mut task, 4, format!("parent-{i:05}").as_bytes()); }
    for i in 0..3000u32 { ld(&mut task, 5, format!("dep-{i:05}").as_bytes()); }
    let mut root = Vec::new();
    ld(&mut root, 1, &task);
    let t2: Vec<u8> = records(&task).unwrap().into_iter().take(7).flatten().collect();
    ld(&mut root, 1, &t2);
    v.push(("ListTasksDetailedResponse", "SYN-strings-5000".to_string(), root));
    v
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let only: Vec<String> = args.iter().position(|a| a == "--only")
        .map(|i| args[i + 1].split(',').map(String::from).collect()).unwrap_or_default();
    let malformed = !args.iter().any(|a| a == "--no-malformed");
    let ctx: &'static _ = Box::leak(Box::new(harness::arms::core_ffi_arm::Ctx::new()));
    let counting = cfg!(feature = "count");
    println!("# FIX-PLAN D23 FSM differential (shapes core), build {}, counting {}",
             if cfg!(feature = "unknown-fields") { "full (drop, retain)" } else { "no-unknown" }, counting);
    let (n_api, api_bad) = binding::fsm_api_checks();
    println!("# API checks: {n_api} checks, {} failures", api_bad.len());
    for b in &api_bad {
        println!("API FAIL {b}");
    }
    let mut fails = api_bad.len();
    let mut checks = n_api;
    let mut mal_rows = 0usize;
    let mut mal_err = 0usize;
    let mut reorder = 0usize;
    println!("{:<52} {:<24} {:>10} {:<10} {:>8} {:>8} {:>8} {:>9} {:>9} {:>8} {:>8}  {}",
             "input", "root", "bytes", "mode", "records", "events", "calls", "pull fwd", "fsm fwd", "pull rev", "fsm rev", "verdict");
    for inp in inputs(&only) {
        for &(mname, retain) in MODES {
            let (d, g) = binding::fsm_check_root(&inp.root, ctx.dec, &inp.bytes, retain, 0).expect("root");
            checks += 1;
            let ok = d.mismatch.is_none() && g.is_ok() && d.pull_rc >= 0 && d.records == d.events && d.calls == d.events;
            if !ok {
                fails += 1;
            }
            println!("{:<52} {:<24} {:>10} {:<10} {:>8} {:>8} {:>8} {:>9} {:>9} {:>8} {:>8}  {}{}",
                     inp.id, inp.root, inp.bytes.len(), mname, d.records, d.events, d.calls,
                     d.pull_fwd, d.fsm_fwd, d.pull_rev, d.fsm_rev,
                     if ok { "ok" } else { "FAIL " },
                     if ok { String::new() } else { format!("{:?} {:?} rc pull {} fsm {}", d.mismatch, g, d.pull_rc, d.fsm_rc) });
            // 2b: valid re-orderings.
            if let Some(recs) = records(&inp.bytes) {
                let mut spliced = Vec::new();
                for (i, r) in recs.iter().enumerate() {
                    if i > 0 { varint(&mut spliced, 9999 << 3); spliced.push(1); }
                    spliced.extend_from_slice(r);
                }
                let reversed: Vec<u8> = recs.iter().rev().flatten().copied().collect();
                for (what, v) in [("spliced", &spliced), ("reversed", &reversed)] {
                    let (d, g) = binding::fsm_check_root(&inp.root, ctx.dec, v, retain, 0).expect("root");
                    checks += 1;
                    reorder += 1;
                    if d.mismatch.is_some() || g.is_err() || d.pull_rc < 0 {
                        fails += 1;
                        println!("REORDER FAIL {} {what} {mname}: {:?} {:?} rc pull {} fsm {}", inp.id, d.mismatch, g, d.pull_rc, d.fsm_rc);
                    }
                }
            }
            if !malformed {
                continue;
            }
            // Malformed variants: truncations and one-byte mutations, deterministic.
            let b = &inp.bytes;
            let pts: Vec<usize> = if b.len() <= 48 { (0..b.len()).collect() } else { (0..48).map(|i| i * b.len() / 48 + (i * 7919) % 13).map(|x| x.min(b.len() - 1)).collect() };
            let mut variants: Vec<Vec<u8>> = Vec::new();
            for &p in &pts {
                variants.push(b[..p].to_vec());
                let mut m = b.clone();
                m[p] ^= [0xff, 0x80, 0x07, 0x40][p % 4];
                variants.push(m);
            }
            for sk in [0u64, u64::MAX] {
                for v in &variants {
                    let (d, g) = binding::fsm_check_root(&inp.root, ctx.dec, v, retain, sk).expect("root");
                    checks += 1;
                    mal_rows += 1;
                    if d.pull_rc < 0 {
                        mal_err += 1;
                    }
                    if d.mismatch.is_some() || g.is_err() {
                        fails += 1;
                        println!("MALFORMED FAIL {} {} sk={:#x} len {}: {:?} {:?} rc pull {} fsm {}",
                                 inp.id, mname, sk, v.len(), d.mismatch, g, d.pull_rc, d.fsm_rc);
                    }
                }
            }
        }
    }
    println!("# valid re-orderings (unknown field spliced between top-level records; records reversed): {reorder} compared");
    for (root, id, b) in synthetic() {
        for &(mname, retain) in MODES {
            let (d, g) = binding::fsm_check_root(root, ctx.dec, &b, retain, 0).expect("root");
            checks += 1;
            let ok = d.mismatch.is_none() && g.is_ok();
            if !ok {
                fails += 1;
            }
            println!("{:<52} {:<24} {:>10} {:<10} {:>8} {:>8} {:>8} {:>9} {:>9} {:>8} {:>8}  {}{}",
                     id, root, b.len(), mname, d.records, d.events, d.calls, d.pull_fwd, d.fsm_fwd, d.pull_rev, d.fsm_rev,
                     if ok { "ok" } else { "FAIL " }, if ok { String::new() } else { format!("{:?} {:?} rc pull {} fsm {}", d.mismatch, g, d.pull_rc, d.fsm_rc) });
            if malformed {
                for p in (0..b.len()).step_by(b.len() / 97 + 1) {
                    let (d, g) = binding::fsm_check_root(root, ctx.dec, &b[..p], retain, 0).expect("root");
                    checks += 1;
                    mal_rows += 1;
                    if d.pull_rc < 0 { mal_err += 1; }
                    if d.mismatch.is_some() || g.is_err() {
                        fails += 1;
                        println!("MALFORMED FAIL {id} {mname} len {p}: {:?} {:?}", d.mismatch, g);
                    }
                }
            }
        }
    }
    if malformed {
        println!("# malformed variants: {mal_rows} (input, mode, mask) rows, {mal_err} of them refused by pull; every one compared");
    }
    println!("# {checks} checks, {fails} failures");
    if fails == 0 {
        println!("FSM DIFFERENTIAL PASSED");
    } else {
        println!("FSM DIFFERENTIAL FAILED");
        std::process::exit(1);
    }
}
