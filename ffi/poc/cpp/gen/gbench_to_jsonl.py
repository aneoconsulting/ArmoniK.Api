#!/usr/bin/env python3
"""CAMPAIGN.md requirements 22a and 28: Google Benchmark's JSON output (every repetition,
run_type "iteration") converted to section 7's one-object-per-sample lines. Glue: no value
is computed but the unit conversion. Aggregate rows (mean/median/stddev) are dropped: the
raw repetitions are the samples.

  gbench_to_jsonl.py GBENCH.json LAUNCH [BUILD]   >> codec-launchN.jsonl
  BUILD: "full" (default) or "no-unknown" (WP5 step 10), on every sample

name = "arm|payload|content|dir|unknown_mode"; round = repetition_index;
cpu_ns = cpu_time x iterations (Google Benchmark's default CPU timer: the benchmark
thread); wall_ns = real_time x iterations; iters = iterations.
"""
import json
import sys

SCALE = {"ns": 1.0, "us": 1e3, "ms": 1e6, "s": 1e9}


def main(path, launch, build="full"):
    d = json.load(open(path))
    ctx = d.get("context", {})
    print("# " + json.dumps({"google_benchmark_context": {k: ctx.get(k) for k in (
        "library_version", "library_build_type", "num_cpus", "mhz_per_cpu",
        "cpu_scaling_enabled", "caches", "executable", "date")}}, sort_keys=True))
    n = 0
    for b in d.get("benchmarks", []):
        if b.get("run_type") != "iteration":
            continue
        # run_name carries Google Benchmark's own suffixes ("/iterations:N/repeats:R").
        parts = b["run_name"].split("/")[0].split("|")
        arm, payload, content, direction, mode = parts[:5]
        # WP7: a sixth field carries the row's tags, "k=v,k=v" (req. 11's end= and input=,
        # req. 7's set= and row=), each written as its own field.
        tags = dict(kv.split("=", 1) for kv in parts[5].split(",") if "=" in kv) if len(parts) > 5 else {}
        it = int(b["iterations"])
        sc = SCALE[b.get("time_unit", "ns")]
        print(json.dumps({"slice": "cpp", "suite": "codec", "arm": arm, "payload": payload,
                          "content": content, "dir": direction, "unknown_mode": mode,
                          "build": build, "launch": int(launch), "round": int(b.get("repetition_index", 0)),
                          "cpu_ns": round(b["cpu_time"] * sc * it), "cpu_clock": "process", "wall_ns": round(b["real_time"] * sc * it),
                          "iters": it, **tags}, separators=(",", ":")))
        n += 1
    print("# " + json.dumps({"samples_converted": n}))
    return 0 if n else 1


if __name__ == "__main__":
    sys.exit(main(*sys.argv[1:4]))
