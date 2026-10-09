//! CAMPAIGN.md 4.1, the codec suite, on **criterion** (requirement 22a).
//!
//! One process per launch (the runner pins it to `AK_CPU_CLIENT` with `taskset` and starts
//! it three times). Configuration from the environment, recorded in the header:
//!   AK_LAUNCH        1..=3; the seed of the launch's random arm and case order (req 22)
//!   AK_OUT           the JSON-lines file to write (section 7)
//!   AK_ONLY          comma-separated input id prefixes (smoke runs), empty = everything
//!   AK_SAMPLES       criterion samples per case = rounds (requirement 23; >= 10, criterion's floor)
//!   AK_WARMUP_MS     criterion's own warm-up time per case
//!   AK_MEASURE_MS    criterion's measurement time per case
//!   AK_LLC_BYTES     the last-level cache (default 13.75 MiB, the reference i9-7900X)
//!   AK_POOL_BYTES    requirement 11's pool input: wire bytes of distinct graphs (default
//!                    2 x AK_LLC_BYTES)
//!   AK_NRESAMPLES    criterion's bootstrap resamples for its console summary (default
//!                    100000, criterion's own); analysis only, no exported sample depends on it
//!   AK_ZC            comma-separated input id PREFIXES that get the labelled extra arm
//!                    `core-ffi-zc` (optimisation Z1: `bytes` fields share the input buffer);
//!                    labelled extra arms run after the arm blocks
//!
//! Criterion measures with `campaign::ProcessCpu` (CLOCK_PROCESS_CPUTIME_ID, requirement 21
//! as amended 2026-09-26)
//! in `SamplingMode::Flat`, so every sample (round) of a case has the same iteration count.
//! Every raw sample criterion saved (`<CRITERION_HOME>/codec/<case>/new/sample.json`) is
//! converted to one JSON line; criterion's outlier classification touches only its console
//! summary and no sample is dropped from the output.

use campaign::*;
use criterion::{Criterion, SamplingMode};
use std::io::Write;
use std::time::Duration;

fn env<T: std::str::FromStr>(k: &str, d: T) -> T {
    std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d)
}

struct Collect<'a> {
    ctx: &'static harness::arms::core_ffi_arm::Ctx,
    inp: &'a Input,
    zc: &'a [String],
    cases: &'a mut Vec<Case>,
    checks: &'a mut usize,
    fails: &'a mut Vec<String>,
    refused: &'a mut Vec<String>,
}

impl Visit for Collect<'_> {
    fn visit<R: Ops>(&mut self) {
        let (n, f, r) = precheck::<R>(self.ctx, self.inp);
        *self.checks += n;
        self.fails.extend(f);
        self.refused.extend(r);
        self.cases.extend(cases_for::<R>(self.ctx, self.inp, self.zc.iter().any(|x| self.inp.id.starts_with(x.as_str()))));
    }
}

fn main() {
    // CAMPAIGN req 25 (D9): the allocator mode, checked before anything is timed.
    let (alloc, alloc_read) = alloc_check();
    let launch: usize = env("AK_LAUNCH", 1);
    let out_path: String = env("AK_OUT", "codec.jsonl".to_string());
    let only: Vec<String> = std::env::var("AK_ONLY").unwrap_or_default()
        .split(',').filter(|s| !s.is_empty()).map(String::from).collect();
    let samples: usize = env("AK_SAMPLES", 10);
    let warm_ms: u64 = env("AK_WARMUP_MS", 500);
    let meas_ms: u64 = env("AK_MEASURE_MS", 2000);
    let nresamples: usize = env("AK_NRESAMPLES", 100_000);
    let zc: Vec<String> = std::env::var("AK_ZC").unwrap_or_default()
        .split(',').filter(|s| !s.is_empty()).map(String::from).collect();
    let home = std::env::var("CRITERION_HOME").expect("CRITERION_HOME (the runner sets it per launch)");

    let ctx: &'static _ = Box::leak(Box::new(harness::arms::core_ffi_arm::Ctx::new()));
    // CAMPAIGN section 4.0 (D18): the core grid's inputs only (the pre-check below then covers
    // every arm on every TIMED input; the gate covers the rest).
    let grid_sel = campaign::campaign_grid();
    let inputs: Vec<_> = inputs(&only).into_iter().filter(|i| grid_sel == "full" || campaign::core_codec_input(&i.id)).collect();
    let mut cases = Vec::new();
    let (mut checks, mut fails, mut refused) = (0usize, Vec::new(), Vec::new());
    for inp in &inputs {
        let ok = generated::roots::with_root(&inp.root, &mut Collect {
            ctx, inp: inp, zc: &zc, cases: &mut cases, checks: &mut checks, fails: &mut fails, refused: &mut refused,
        });
        assert!(ok, "no root {}", inp.root);
    }
    // Narrowed runs (not the campaign's; every filter empty = every case, as before):
    // AK_CASE_ARMS, AK_CASE_DIRS, AK_CASE_END (end states), AK_CASE_INPUT (hot | pool), each
    // a comma-separated list. Applied AFTER the per-input pre-check above, which still
    // covers every arm on every input; the variant check below covers the cases kept.
    let flt = |k: &str| -> Vec<String> {
        std::env::var(k).unwrap_or_default().split(',').filter(|s| !s.is_empty()).map(String::from).collect()
    };
    let (f_arm, f_dir, f_end, f_inp) = (flt("AK_CASE_ARMS"), flt("AK_CASE_DIRS"), flt("AK_CASE_END"), flt("AK_CASE_INPUT"));
    let keep = |v: &[String], x: &str| v.is_empty() || v.iter().any(|s| s == x);
    let narrowed = !(f_arm.is_empty() && f_dir.is_empty() && f_end.is_empty() && f_inp.is_empty());
    cases.retain(|c| keep(&f_arm, c.arm) && keep(&f_dir, c.dir)
        && (c.end_state.is_empty() || (keep(&f_end, c.end_state) && keep(&f_inp, c.input)))
        && (f_end.is_empty() || !c.end_state.is_empty())
        && (grid_sel == "full" || campaign::core_codec_case(c.arm, c.dir, c.unknown_mode, c.end_state, c.input)));
    if narrowed {
        eprintln!("# narrowed: arms [{}] dirs [{}] end states [{}] inputs [{}]: {} cases kept",
                  f_arm.join(","), f_dir.join(","), f_end.join(","), f_inp.join(","), cases.len());
    }
    // Requirement 11's variants write the same bytes: every (arm, input, mode) returns the
    // same length from each of its end-state x input variants (the pool built and freed).
    {
        let mut want: std::collections::HashMap<(String, &str, &str, &str), u64> = Default::default();
        for cs in cases.iter_mut().filter(|c| !c.end_state.is_empty()) {
            if let Some(p) = cs.prep.as_mut() { p(); }
            let (a, b) = ((cs.op)(), (cs.op)());
            if let Some(d) = cs.done.as_mut() { d(); }
            let key = (cs.payload.clone(), cs.content, cs.arm, cs.unknown_mode);
            let w = *want.entry(key).or_insert(a);
            checks += 1;
            if a != w || b != w {
                fails.push(format!("{} {} {} {}/{}: {} B, {} B where the hot reused-buffer row wrote {} B",
                                   cs.payload, cs.arm, cs.unknown_mode, cs.end_state, cs.input, a, b, w));
            }
        }
    }
    // Requirement 26: nothing is timed if any timed arm is wrong on any input.
    eprintln!("# precheck: {checks} checks, {} failures, {} inputs, {} cases", fails.len(), inputs.len(), cases.len());
    if !fails.is_empty() {
        for f in &fails {
            eprintln!("PRECHECK FAIL {f}");
        }
        std::process::exit(2);
    }
    if std::env::var_os("AK_PRECHECK_ONLY").is_some() {
        println!("PRECHECK PASSED: {checks} checks, {} inputs, {} cases; incumbent refusals on U-* rows (not timed): {}", inputs.len(), cases.len(), refused.len());
        for r in &refused {
            println!("  refused: {r}");
        }
        return;
    }

    let order = arm_order(launch);
    let mut c = Criterion::default()
        .with_measurement(ProcessCpu)
        .sample_size(samples.max(10))
        .warm_up_time(Duration::from_millis(warm_ms))
        .measurement_time(Duration::from_millis(meas_ms))
        .nresamples(nresamples)
        .without_plots();
    // Blocks by arm (requirement 22), in a seeded random order per launch, and the cases
    // inside a block in a seeded random order too (R-H23): criterion runs benchmarks in
    // registration order, so the order is decided here.
    let mut ran: Vec<(usize, &Case)> = Vec::new();
    let mut idx_of = Vec::new();
    for (b, arm) in order.iter().enumerate() {
        let mut block: Vec<usize> = cases.iter().enumerate().filter(|(_, cs)| cs.arm == *arm).map(|(i, _)| i).collect();
        campaign::shuffle(&mut block, (launch as u64) << 8 | b as u64);
        idx_of.extend(block);
    }
    // Labelled extra arms (core-ffi-zc) are not in the requirement-22 blocks: after them.
    for (i, cs) in cases.iter().enumerate() {
        if !order.contains(&cs.arm) {
            idx_of.push(i);
        }
    }
    let faults: Vec<std::rc::Rc<std::cell::RefCell<Vec<(u64, u64, u64)>>>> =
        cases.iter().map(|_| Default::default()).collect();
    {
        let mut g = c.benchmark_group("codec");
        g.sampling_mode(SamplingMode::Flat);
        for &i in &idx_of {
            let cs = &mut cases[i];
            // Requirement 11: a pool input's graphs are built here, before the warm-up and
            // outside every timed window, and freed after the case.
            if let Some(p) = cs.prep.as_mut() {
                p();
            }
            // The routine reads the minor faults around its iterations, outside criterion's
            // clock (req 25, D9), and the export matches them to criterion's samples.
            let fl = faults[i].clone();
            g.bench_function(format!("{i:05}"), |b| b.iter_custom(|iters| {
                let f0 = minflt();
                let c0 = process_clock_ns();
                for _ in 0..iters {
                    criterion::black_box((cs.op)());
                }
                let cpu = process_clock_ns() - c0;
                fl.borrow_mut().push((iters, cpu, minflt() - f0));
                cpu
            }));
            if let Some(d) = cs.done.as_mut() {
                d();
            }
        }
        g.finish();
    }
    for &i in &idx_of {
        ran.push((i, &cases[i]));
    }

    // Section 7: one JSON line per raw criterion sample.
    let mut f = std::fs::File::create(&out_path).unwrap();
    let mut hx = vec![
        ("grid", format!("AK_CAMPAIGN_GRID={grid_sel} (CAMPAIGN section 4.0, D18; core | full). core: the 16 shapes (P7.1 decode only), Latin-1 and wide on P2.2 only, the 7 U-* rows {}; arms incumbent-prod (full build only, once), core-ffi (push) and host-gen, which in Rust is core-native (the codec the shared generator writes into Rust, no C ABI boundary); encode at end state (ii) (incumbent-prod transport-ready-tonic, cell A's form; core-ffi and core-native transport-ready-core, cells Cf and Ef) on the hot input, and decode-read; retain in the full build, no-unknown in the no-unknown build. Every row is labelled row = core | extra", campaign::CORE_U_ROWS.join(", "))),
        ("extras left out", if grid_sel == "core" { campaign::CODEC_EXTRAS.to_string() } else { "none (full grid: extras run, labelled row = extra)".to_string() }),
        ("alloc", alloc_header(alloc, alloc_read)),
        ("engine", "criterion 0.5, measurement = PROCESS CPU (CLOCK_PROCESS_CPUTIME_ID, requirement 21 as amended), SamplingMode::Flat, raw samples exported, none dropped".into()),
        ("threads", "1 measuring thread (criterion, in-process); no runtime, no worker pool in the codec suite".into()),
        ("encode variants", format!("every encode arm x mode in 4 rows, core-native and core-ffi in 6 (requirement 11): end_state reused-buffer | transport-ready-tonic | transport-ready-core (core arms only) ({}) x input hot (one graph) | pool (distinct graphs cloned until the heap they hold, measured with glibc mallinfo2, reaches AK_POOL_BYTES; at least 2, at most 2^20; built before the case's warm-up and freed after; each pool row records pool_graphs and pool_heap_bytes; AK_POOL_BYTES = {}, AK_LLC_BYTES = {})", TRANSPORT_FORMS, pool_bytes(), llc_bytes())),
        ("build", if cfg!(feature = "unknown-fields") {
            "unknown-fields: core-ffi / core-native / core-ffi-pull in modes drop and retain".to_string()
        } else {
            "NO-UNKNOWN (unknown-field support compiled out, CAMPAIGN.md req 10): core-ffi / core-native / core-ffi-pull in mode no-unknown; incumbent-prod and armonik as in-process controls".to_string()
        }),
        ("launch", launch.to_string()),
        ("arm order", format!("{} (arm blocks and the cases inside each block in a seeded random order, seed = launch; criterion runs them in this registration order)", order.join(","))),
        ("samples (rounds) per case", samples.max(10).to_string()),
        ("criterion resamples", format!("{nresamples} (analysis only; no exported sample depends on it)")),
        ("core-ffi encode fill", FFI_ENCODE_FILL.into()),
        ("core-ffi-zc", if zc.is_empty() { "none".to_string() } else { format!("labelled extra arm on inputs {}*: the core-ffi decode with every bytes field a slice of the input Bytes (optimisation Z1; not ABI v1 decision 13's copy semantics, which core-ffi keeps); run after the arm blocks", zc.join("*,")) }),
        ("warm-up", format!("criterion's own warm-up, {warm_ms} ms per case (AK_WARMUP_MS; FIX-PLAN WP9: no hand-written warm-up loop beside it); measurement {meas_ms} ms")),
        ("wall", "not recorded for the codec suite (criterion measures one quantity; process CPU is requirement 21's)".into()),
        ("unknown modes", format!("core-native, core-ffi, core-ffi-pull: {} (retain = every position armed); incumbent-prod and armonik: default (prost drops unknown fields). core-native's drop rendering has no unknown-field code in either build", MODES.iter().map(|m| m.0).collect::<Vec<_>>().join(", "))),
        ("precheck", format!("{checks} checks passed")),
        ("inputs", inputs.len().to_string()),
        ("cases", cases.len().to_string()),
        ("narrowed", if narrowed { format!("arms [{}] dirs [{}] end states [{}] inputs [{}] (AK_CASE_*; not a campaign run)", f_arm.join(","), f_dir.join(","), f_end.join(","), f_inp.join(",")) } else { "no".to_string() }),
        ("refusals", format!("{} (row, arm) pairs the incumbent's prost refuses and that are therefore not timed; listed below", refused.len())),
    ];
    if campaign::fsm_arm_on() {
        hx.insert(0, ("core-ffi-fsm",
        "labelled extra arm (AK_FSM=1, FIX-PLAN D23): the FSM decode family, ak_fsm_begin_<Root> then ak_fsm_next_<Root> to the end event, each event fed to the push vtable's host functions as it arrives (binding fsm_with_<root>, _unk in retain); in the arm blocks; pre-checked against core-ffi's value and pull's log".to_string()));
    }
    for h in header("codec", &hx) {
        writeln!(f, "{h}").unwrap();
    }
    for r in &refused {
        writeln!(f, "# refused: {r}").unwrap();
    }
    let mut rows = 0usize;
    for (i, cs) in ran {
        let p = std::path::Path::new(&home).join("codec").join(format!("{i:05}")).join("new/sample.json");
        let s: serde_json::Value = serde_json::from_slice(&std::fs::read(&p)
            .unwrap_or_else(|e| panic!("criterion sample file {}: {e}", p.display()))).unwrap();
        let iters = s["iters"].as_array().unwrap();
        let times = s["times"].as_array().unwrap();
        // The measured samples are the routine's LAST n calls; each must match criterion's.
        let fl = faults[i].borrow();
        let n = iters.len();
        assert!(fl.len() >= n, "case {i}: {} routine calls for {n} samples", fl.len());
        let tail = &fl[fl.len() - n..];
        for (r, ((it, t), &(fi, fc, fm))) in iters.iter().zip(times).zip(tail).enumerate() {
            let (it, t) = (it.as_f64().unwrap() as u64, t.as_f64().unwrap() as u64);
            assert!(it == fi && t == fc, "case {i}: sample {r} (iters {it}, cpu {t}) does not match the routine's ({fi}, {fc})");
            let mut o = serde_json::json!({
                "slice": "rust", "suite": "codec", "arm": cs.arm, "payload": cs.payload,
                "content": cs.content, "dir": cs.dir, "launch": launch, "round": r + 1,
                "cpu_ns": t, "iters": it, "minflt": fm, "alloc": alloc, "grid": grid_sel,
                "row": if campaign::core_codec_input(&cs.payload) && campaign::core_codec_case(cs.arm, cs.dir, cs.unknown_mode, cs.end_state, cs.input) { "core" } else { "extra" },
            });
            if cs.unknown_mode != "default" {
                o["unknown_mode"] = cs.unknown_mode.into();
            }
            if !cs.end_state.is_empty() {
                o["end_state"] = cs.end_state.into();
                o["input"] = cs.input.into();
            }
            if let Some(pi) = &cs.pool_info {
                let (n, held) = pi.get();
                o["pool_graphs"] = n.into();
                o["pool_heap_bytes"] = held.into();
            }
            writeln!(f, "{o}").unwrap();
            rows += 1;
        }
    }
    eprintln!("# wrote {rows} sample rows to {out_path}");
}
