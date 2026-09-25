//! CAMPAIGN.md 4.1, the codec suite, on **criterion** (requirement 22a).
//!
//! One process per launch (the runner pins it to `AK_CPU_CLIENT` with `taskset` and starts
//! it three times). Configuration from the environment, recorded in the header:
//!   AK_LAUNCH        1..=3; the arm order is rotated by launch (requirement 22)
//!   AK_OUT           the JSON-lines file to write (section 7)
//!   AK_ONLY          comma-separated input id prefixes (smoke runs), empty = everything
//!   AK_SAMPLES       criterion samples per case = rounds (requirement 23; >= 10, criterion's floor)
//!   AK_WARMUP_ITERS  a FIXED number of iterations of every case before criterion starts
//!                    (requirement 24, identical for every arm; also warms the allocator, 25)
//!   AK_WARMUP_MS     criterion's own warm-up time per case
//!   AK_MEASURE_MS    criterion's measurement time per case
//!
//! Criterion measures with `campaign::ThreadCpu` (CLOCK_THREAD_CPUTIME_ID, requirement 21)
//! in `SamplingMode::Flat`, so every sample (round) of a case has the same iteration count.
//! Every raw sample criterion saved (`<CRITERION_HOME>/codec/<case>/new/sample.json`) is
//! converted to one JSON line; criterion's outlier classification touches only its console
//! summary and no sample is dropped from the output.

use campaign::*;
use criterion::{black_box, Criterion, SamplingMode};
use std::io::Write;
use std::time::Duration;

fn env<T: std::str::FromStr>(k: &str, d: T) -> T {
    std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d)
}

struct Collect<'a> {
    ctx: &'static harness::arms::core_ffi_arm::Ctx,
    inp: &'a Input,
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
        self.cases.extend(cases_for::<R>(self.ctx, self.inp));
    }
}

fn main() {
    let launch: usize = env("AK_LAUNCH", 1);
    let out_path: String = env("AK_OUT", "codec.jsonl".to_string());
    let only: Vec<String> = std::env::var("AK_ONLY").unwrap_or_default()
        .split(',').filter(|s| !s.is_empty()).map(String::from).collect();
    let samples: usize = env("AK_SAMPLES", 10);
    let warm_iters: u64 = env("AK_WARMUP_ITERS", 100);
    let warm_ms: u64 = env("AK_WARMUP_MS", 500);
    let meas_ms: u64 = env("AK_MEASURE_MS", 2000);
    let home = std::env::var("CRITERION_HOME").expect("CRITERION_HOME (the runner sets it per launch)");

    let ctx: &'static _ = Box::leak(Box::new(harness::arms::core_ffi_arm::Ctx::new()));
    let inputs = inputs(&only);
    let mut cases = Vec::new();
    let (mut checks, mut fails, mut refused) = (0usize, Vec::new(), Vec::new());
    for inp in &inputs {
        let ok = generated::roots::with_root(&inp.root, &mut Collect {
            ctx, inp: inp, cases: &mut cases, checks: &mut checks, fails: &mut fails, refused: &mut refused,
        });
        assert!(ok, "no root {}", inp.root);
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
        .with_measurement(ThreadCpu)
        .sample_size(samples.max(10))
        .warm_up_time(Duration::from_millis(warm_ms))
        .measurement_time(Duration::from_millis(meas_ms))
        .without_plots();
    // Blocks by arm, the arm order rotated by launch (requirement 22).
    let mut ran: Vec<(usize, &Case)> = Vec::new();
    let mut idx_of = Vec::new();
    for arm in &order {
        for (i, cs) in cases.iter().enumerate() {
            if cs.arm == *arm {
                idx_of.push(i);
            }
        }
    }
    {
        let mut g = c.benchmark_group("codec");
        g.sampling_mode(SamplingMode::Flat);
        for &i in &idx_of {
            let cs = &mut cases[i];
            // Requirement 24: a fixed number of iterations of THIS case before criterion's
            // own warm-up, identical for every arm.
            for _ in 0..warm_iters {
                black_box((cs.op)());
            }
            g.bench_function(format!("{i:05}"), |b| b.iter(|| (cs.op)()));
        }
        g.finish();
    }
    for &i in &idx_of {
        ran.push((i, &cases[i]));
    }

    // Section 7: one JSON line per raw criterion sample.
    let mut f = std::fs::File::create(&out_path).unwrap();
    for h in header("codec", &[
        ("engine", "criterion 0.5, measurement = thread CPU (CLOCK_THREAD_CPUTIME_ID), SamplingMode::Flat, raw samples exported, none dropped".into()),
        ("build", if cfg!(feature = "unknown-fields") {
            "unknown-fields: core-ffi / core-native / core-ffi-pull in modes drop and retain".to_string()
        } else {
            "NO-UNKNOWN (unknown-field support compiled out, CAMPAIGN.md req 10): core-ffi / core-native / core-ffi-pull in mode no-unknown; incumbent-prod and armonik as in-process controls".to_string()
        }),
        ("launch", launch.to_string()),
        ("arm order", order.join(",")),
        ("samples (rounds) per case", samples.max(10).to_string()),
        ("warm-up", format!("{warm_iters} fixed iterations per case, then criterion warm-up {warm_ms} ms; measurement {meas_ms} ms")),
        ("wall", "not recorded for the codec suite (criterion measures one quantity; thread CPU is requirement 21's)".into()),
        ("unknown modes", format!("core-native, core-ffi, core-ffi-pull: {} (retain = every position armed); incumbent-prod and armonik: default (prost drops unknown fields). core-native's drop rendering has no unknown-field code in either build", MODES.iter().map(|m| m.0).collect::<Vec<_>>().join(", "))),
        ("precheck", format!("{checks} checks passed")),
        ("inputs", inputs.len().to_string()),
        ("cases", cases.len().to_string()),
        ("refusals", format!("{} (row, arm) pairs the incumbent's prost refuses and that are therefore not timed; listed below", refused.len())),
    ]) {
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
        for (r, (it, t)) in iters.iter().zip(times).enumerate() {
            let mut o = serde_json::json!({
                "slice": "rust", "suite": "codec", "arm": cs.arm, "payload": cs.payload,
                "content": cs.content, "dir": cs.dir, "launch": launch, "round": r + 1,
                "cpu_ns": t.as_f64().unwrap() as u64, "iters": it.as_f64().unwrap() as u64,
            });
            if cs.unknown_mode != "default" {
                o["unknown_mode"] = cs.unknown_mode.into();
            }
            writeln!(f, "{o}").unwrap();
            rows += 1;
        }
    }
    eprintln!("# wrote {rows} sample rows to {out_path}");
}
