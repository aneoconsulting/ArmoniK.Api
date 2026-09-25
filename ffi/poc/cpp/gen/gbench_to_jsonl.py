#!/usr/bin/env python3
"""CAMPAIGN.md requirements 22a and 28: Google Benchmark's JSON output (every repetition,
run_type "iteration") converted to section 7's one-object-per-sample lines. Glue: no value
is computed but the unit conversion. Aggregate rows (mean/median/stddev) are dropped: the
raw repetitions are the samples.

  gbench_to_jsonl.py GBENCH.json LAUNCH   >> codec-launchN.jsonl

name = "arm|payload|content|dir|unknown_mode"; round = repetition_index;
cpu_ns = cpu_time x iterations (Google Benchmark's default CPU timer: the benchmark
thread); wall_ns = real_time x iterations; iters = iterations.
"""
import json
import sys

SCALE = {"ns": 1.0, "us": 1e3, "ms": 1e6, "s": 1e9}


def main(path, launch):
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
        arm, payload, content, direction, mode = b["run_name"].split("/")[0].split("|")
        it = int(b["iterations"])
        sc = SCALE[b.get("time_unit", "ns")]
        print(json.dumps({"slice": "cpp", "suite": "codec", "arm": arm, "payload": payload,
                          "content": content, "dir": direction, "unknown_mode": mode,
                          "launch": int(launch), "round": int(b.get("repetition_index", 0)),
                          "cpu_ns": round(b["cpu_time"] * sc * it), "wall_ns": round(b["real_time"] * sc * it),
                          "iters": it}, separators=(",", ":")))
        n += 1
    print("# " + json.dumps({"samples_converted": n}))
    return 0 if n else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1], sys.argv[2]))
