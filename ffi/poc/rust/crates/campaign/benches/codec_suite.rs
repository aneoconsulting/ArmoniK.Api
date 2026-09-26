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
//!   AK_ORDER         `blocks` (default; requirement 22: blocks by arm, the arm order rotated
//!                    by launch) or `shuffle`: every case of the process in one seeded random
//!                    order (AK_SEED, default 1), so no arm is always first or last. Criterion
//!                    runs all samples of one case back to back (it cannot interleave samples
//!                    of different cases), so the order is interleaved per case, not per sample.
//!                    Or `interleave` (the optimisation benchmark, gen/opt_bench.sh): NOT
//!                    criterion. The cases are grouped in clusters (input, direction) -- every
//!                    arm and mode of one input and direction -- clusters in a seeded random
//!                    order; inside a cluster every case is warmed (AK_WARMUP_ITERS fixed
//!                    iterations, then AK_WARMUP_MS timed, which also sets its iterations per
//!                    sample so a sample lasts AK_MEASURE_MS / AK_SAMPLES), then the SAMPLES
//!                    are interleaved: round r times one sample of every case of the cluster,
//!                    the case order rotated by r. A ratio's numerator and denominator are then
//!                    timed milliseconds apart instead of seconds apart, which is what this
//!                    container needs: an A/A pair showed the machine's speed drifting over
//!                    seconds (lag-1 autocorrelation of per-case drift 0.5-0.8).
//!   AK_NRESAMPLES    criterion's bootstrap resamples for its console summary (default 100000,
//!                    criterion's own); it touches no exported sample, only analysis time
//!   AK_NODROP        comma-separated EXACT input ids that also get the labelled extra
//!                    `decode-nodrop` rows (the decoded graph dropped outside the timed region)
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
    nodrop: &'a [String],
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
        self.cases.extend(cases_for::<R>(self.ctx, self.inp, self.nodrop.iter().any(|x| *x == self.inp.id)));
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
    let order_mode: String = env("AK_ORDER", "blocks".to_string());
    let seed: u64 = env("AK_SEED", 1);
    let nresamples: usize = env("AK_NRESAMPLES", 100_000);
    let nodrop: Vec<String> = std::env::var("AK_NODROP").unwrap_or_default()
        .split(',').filter(|s| !s.is_empty()).map(String::from).collect();
    assert!(order_mode == "blocks" || order_mode == "shuffle" || order_mode == "interleave",
            "AK_ORDER: blocks | shuffle | interleave");
    let home = std::env::var("CRITERION_HOME").unwrap_or_default();
    assert!(order_mode == "interleave" || !home.is_empty(), "CRITERION_HOME (the runner sets it per launch)");

    let ctx: &'static _ = Box::leak(Box::new(harness::arms::core_ffi_arm::Ctx::new()));
    let inputs = inputs(&only);
    let mut cases = Vec::new();
    let (mut checks, mut fails, mut refused) = (0usize, Vec::new(), Vec::new());
    for inp in &inputs {
        let ok = generated::roots::with_root(&inp.root, &mut Collect {
            ctx, nodrop: &nodrop, inp: inp, cases: &mut cases, checks: &mut checks, fails: &mut fails, refused: &mut refused,
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
    if order_mode == "interleave" {
        let rows = interleaved(&mut cases, seed, samples, warm_iters, warm_ms, meas_ms, launch);
        let mut f = std::fs::File::create(&out_path).unwrap();
        for h in header("codec", &[
            ("engine", "INTERLEAVED SAMPLER (AK_ORDER=interleave), not criterion: measurement = thread CPU (CLOCK_THREAD_CPUTIME_ID) per sample, clusters (input, direction) in a seeded random order, samples of a cluster's cases interleaved round by round (case order rotated per round); criterion 0.5 cannot interleave samples of different benchmarks. Raw samples exported, none dropped".into()),
            ("build", build_line()),
            ("launch", launch.to_string()),
            ("arm order", format!("interleave: cluster order seed {seed} (splitmix64 Fisher-Yates); inside a cluster the cases rotate by round")),
            ("samples (rounds) per case", samples.to_string()),
            ("warm-up", format!("{warm_iters} fixed iterations per case, then {warm_ms} ms timed (sets iterations per sample for a {} ms sample); measurement {meas_ms} ms per case in {samples} samples", meas_ms as f64 / samples as f64)),
            ("wall", "not recorded for the codec suite (thread CPU is requirement 21's)".into()),
            ("unknown modes", modes_line()),
            ("decode-nodrop", nodrop_line(&nodrop)),
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
        for r in &rows {
            writeln!(f, "{r}").unwrap();
        }
        eprintln!("# wrote {} sample rows to {out_path}", rows.len());
        return;
    }
    let mut c = Criterion::default()
        .with_measurement(ThreadCpu)
        .sample_size(samples.max(10))
        .warm_up_time(Duration::from_millis(warm_ms))
        .measurement_time(Duration::from_millis(meas_ms))
        .nresamples(nresamples)
        .without_plots();
    // Blocks by arm, the arm order rotated by launch (requirement 22).
    let mut ran: Vec<(usize, &Case)> = Vec::new();
    let mut idx_of = Vec::new();
    if order_mode == "blocks" {
        for arm in &order {
            for (i, cs) in cases.iter().enumerate() {
                if cs.arm == *arm {
                    idx_of.push(i);
                }
            }
        }
    } else {
        // Seeded Fisher-Yates over every case (splitmix64).
        idx_of = (0..cases.len()).collect();
        let mut st = seed;
        let mut next = || {
            st = st.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = st;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        };
        for i in (1..idx_of.len()).rev() {
            let j = (next() % (i as u64 + 1)) as usize;
            idx_of.swap(i, j);
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
            match cs.bench.as_mut() {
                Some(bf) => g.bench_function(format!("{i:05}"), |b| bf(b)),
                None => g.bench_function(format!("{i:05}"), |b| b.iter(|| (cs.op)())),
            };
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
        ("build", build_line()),
        ("launch", launch.to_string()),
        ("arm order", if order_mode == "blocks" {
            format!("blocks by arm: {}", order.join(","))
        } else {
            format!("shuffle: every case in one seeded random order, seed {seed} (splitmix64 Fisher-Yates); criterion runs one case's samples back to back")
        }),
        ("criterion resamples", format!("{nresamples} (analysis only; no exported sample depends on it)")),
        ("decode-nodrop", nodrop_line(&nodrop)),
        ("samples (rounds) per case", samples.max(10).to_string()),
        ("warm-up", format!("{warm_iters} fixed iterations per case, then criterion warm-up {warm_ms} ms; measurement {meas_ms} ms")),
        ("wall", "not recorded for the codec suite (criterion measures one quantity; thread CPU is requirement 21's)".into()),
        ("unknown modes", modes_line()),
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

fn build_line() -> String {
    if cfg!(feature = "unknown-fields") {
        "unknown-fields: core-ffi / core-native / core-ffi-pull in modes drop and retain".to_string()
    } else {
        "NO-UNKNOWN (unknown-field support compiled out, CAMPAIGN.md req 10): core-ffi / core-native / core-ffi-pull in mode no-unknown; incumbent-prod and armonik as in-process controls".to_string()
    }
}

fn modes_line() -> String {
    format!("core-native, core-ffi, core-ffi-pull: {} (retain = every position armed); incumbent-prod and armonik: default (prost drops unknown fields). core-native's drop rendering has no unknown-field code in either build", MODES.iter().map(|m| m.0).collect::<Vec<_>>().join(", "))
}

fn nodrop_line(nodrop: &[String]) -> String {
    if nodrop.is_empty() {
        "none".to_string()
    } else {
        format!("labelled extra rows on {}: the decoded graph dropped outside the timed region (criterion: iter_with_large_drop, outputs kept until the sample ends; interleaved sampler: each operation timed alone, its output dropped after its clock stops), incumbent-prod, armonik, core-native, core-ffi; the headline decode rows keep the drop inside", nodrop.join(","))
    }
}

fn splitmix(st: &mut u64) -> u64 {
    *st = st.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *st;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// One sample of `n` operations of case `cs`, in thread CPU ns.
fn sample(cs: &mut Case, n: u64) -> u64 {
    if let Some(t) = cs.timed.as_mut() {
        return t(n);
    }
    let t0 = thread_cpu_ns();
    for _ in 0..n {
        black_box((cs.op)());
    }
    thread_cpu_ns() - t0
}

/// AK_ORDER=interleave (see the module comment). Returns the JSON lines.
fn interleaved(cases: &mut [Case], seed: u64, samples: usize, warm_iters: u64, warm_ms: u64, meas_ms: u64,
               launch: usize) -> Vec<String> {
    // Clusters (input, direction), in first-seen order, then shuffled.
    let mut keys: Vec<(String, &'static str)> = Vec::new();
    let mut members: Vec<Vec<usize>> = Vec::new();
    for (i, cs) in cases.iter().enumerate() {
        let k = (cs.payload.clone(), cs.dir);
        match keys.iter().position(|x| *x == k) {
            Some(j) => members[j].push(i),
            None => { keys.push(k); members.push(vec![i]); }
        }
    }
    let mut st = seed;
    for i in (1..members.len()).rev() {
        let j = (splitmix(&mut st) % (i as u64 + 1)) as usize;
        members.swap(i, j);
    }
    let per_sample_ns = (meas_ms as f64 * 1e6 / samples as f64).max(1.0);
    let mut rows = Vec::new();
    for m in &members {
        // Warm-up, identical for every case (requirement 24), and the iteration count.
        let mut iters = Vec::with_capacity(m.len());
        for &i in m {
            let cs = &mut cases[i];
            for _ in 0..warm_iters {
                black_box((cs.op)());
            }
            let (mut n, mut t) = (0u64, 0u64);
            let mut step = 1u64;
            while t < warm_ms * 1_000_000 {
                t += sample(cs, step);
                n += step;
                step = (step * 2).min(1 << 20);
            }
            let per_op = t as f64 / n.max(1) as f64;
            iters.push(((per_sample_ns / per_op.max(1.0)).round() as u64).max(1));
        }
        let mut out: Vec<Vec<(u64, u64)>> = vec![Vec::with_capacity(samples); m.len()];
        for r in 0..samples {
            for j in 0..m.len() {
                let k = (j + r) % m.len();
                let n = iters[k];
                let t = sample(&mut cases[m[k]], n);
                out[k].push((t, n));
            }
        }
        for (k, &i) in m.iter().enumerate() {
            let cs = &cases[i];
            for (r, &(t, n)) in out[k].iter().enumerate() {
                let mut o = serde_json::json!({
                    "slice": "rust", "suite": "codec", "arm": cs.arm, "payload": cs.payload,
                    "content": cs.content, "dir": cs.dir, "launch": launch, "round": r + 1,
                    "cpu_ns": t, "iters": n,
                });
                if cs.unknown_mode != "default" {
                    o["unknown_mode"] = cs.unknown_mode.into();
                }
                rows.push(o.to_string());
            }
        }
    }
    rows
}
