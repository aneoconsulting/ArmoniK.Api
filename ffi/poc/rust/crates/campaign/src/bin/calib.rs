//! CAMPAIGN.md requirement 20: the crossing benchmark, forward and reverse reported
//! separately.
//!
//!   calib --launch L --rounds R --iters N --out FILE      the suite (JSON lines)
//!   calib --only forward|forward-reverse --iters N        one arm, for `perf stat`
//!
//! `forward`: `ak_noop(x)`, one host -> core crossing and nothing else. `forward-reverse`:
//! `ak_noop_reverse(f, x)`, which crosses into the core and calls back into the host once.
//! A reverse crossing cannot be made without the forward one that carries it, so the
//! reverse cost is `forward-reverse` minus `forward`, row by row in the same round. Both
//! arms go through the dynamic linker (the core is a cdylib), as every core-ffi arm does.
//! Process CPU (CLOCK_PROCESS_CPUTIME_ID, requirement 21 as amended) per round; the order of the two arms rotates by
//! launch (requirement 22).

use campaign::process_clock_ns;
use std::io::Write;

extern "C" fn host_cb(x: u64) -> u64 {
    x.wrapping_add(1)
}

#[inline(never)]
fn forward(n: u64) -> u64 {
    let mut x = 0u64;
    for _ in 0..n {
        x = unsafe { ak_abi::ak_noop(std::hint::black_box(x)) };
    }
    x
}

#[inline(never)]
fn forward_reverse(n: u64) -> u64 {
    let mut x = 0u64;
    for _ in 0..n {
        x = unsafe { ak_abi::ak_noop_reverse(host_cb, std::hint::black_box(x)) };
    }
    x
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let arg = |k: &str| a.iter().position(|x| x == k).map(|i| a[i + 1].clone());
    let iters: u64 = arg("--iters").map(|v| v.parse().unwrap()).unwrap_or(10_000_000);
    assert!(harness::generated::binding::ak_init_once() >= 0);
    if let Some(one) = arg("--only") {
        let r = match one.as_str() {
            "forward" => forward(iters),
            "forward-reverse" => forward_reverse(iters),
            o => panic!("--only {o}"),
        };
        std::hint::black_box(r);
        return;
    }
    let launch: usize = arg("--launch").map(|v| v.parse().unwrap()).unwrap_or(1);
    let rounds: usize = arg("--rounds").map(|v| v.parse().unwrap()).unwrap_or(5);
    let out = arg("--out").expect("--out");
    let arms: [(&str, fn(u64) -> u64); 2] = [("forward", forward), ("forward-reverse", forward_reverse)];
    let order: Vec<usize> = if launch % 2 == 1 { vec![0, 1] } else { vec![1, 0] };
    // Warm-up, identical for both arms (requirement 24).
    for &i in &order {
        std::hint::black_box((arms[i].1)(iters / 10));
    }
    let mut lines = Vec::new();
    for r in 1..=rounds {
        for &i in &order {
            let t0 = process_clock_ns();
            std::hint::black_box((arms[i].1)(iters));
            let cpu = process_clock_ns() - t0;
            lines.push(serde_json::json!({
                "slice": "rust", "suite": "calib", "arm": arms[i].0, "launch": launch,
                "round": r, "cpu_ns": cpu, "iters": iters,
            }).to_string());
        }
    }
    let mut f = std::fs::File::create(&out).unwrap();
    for h in campaign::header("calib", &[
        ("arms", "forward = ak_noop; forward-reverse = ak_noop_reverse (reverse = the difference)".into()),
        ("launch", launch.to_string()),
        ("order", order.iter().map(|&i| arms[i].0).collect::<Vec<_>>().join(",")),
        ("rounds", rounds.to_string()),
        ("iterations per round", iters.to_string()),
        ("warm-up", format!("{} iterations of each arm", iters / 10)),
        ("clock", "CLOCK_PROCESS_CPUTIME_ID (process CPU, requirement 21 as amended 2026-09-26)".into()),
        ("threads", "1 measuring thread, no runtime".into()),
    ]) {
        writeln!(f, "{h}").unwrap();
    }
    for l in lines {
        writeln!(f, "{l}").unwrap();
    }
}
