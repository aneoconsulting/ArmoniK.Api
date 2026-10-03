#!/usr/bin/env python3
"""CAMPAIGN.md requirements 22a and 28: Google Benchmark's JSON output (every repetition,
run_type "iteration") converted to section 7's one-object-per-sample lines. Glue: no value
is computed but the unit conversion. Aggregate rows (mean/median/stddev) are dropped: the
raw repetitions are the samples.

  gbench_to_jsonl.py GBENCH.json LAUNCH [BUILD [SUITE]]   >> codec-launchN.jsonl
  BUILD: "full" (default) or "no-unknown" (WP5 step 10), on every sample
  SUITE: "codec" (default) or "rpc" (WP9: the RPC grid on Google Benchmark). rpc names are
  "cell|payload|-|dir|unknown_mode|inflight=k,transport=..,send_path=..,build=..";
  iters = iterations x k (one iteration is a batch of k calls), order_pos = the position in
  Google Benchmark's output (random interleaving), socket "uds".

Any benchmark with error_occurred makes the whole file refused (exit 2): no sample from a
run in which a check failed.

name = "arm|payload|content|dir|unknown_mode"; round = repetition_index;
cpu_ns = cpu_time x iterations (Google Benchmark's default CPU timer: the benchmark
thread); wall_ns = real_time x iterations; iters = iterations.
"""
import json
import os
import sys

# req. 25 (D9 as amended 2026-10-03): the allocator mode of the run, on every sample: "default"
# (the main figures) or "pinned" (the GLIBC_TUNABLES diagnostic pass), set by the runner.
ALLOC = os.environ.get("AK_ALLOC_MODE", "default")
# D18: the h2 variant of the core the client loaded ("stock" or "h2-batch"), set by the runner.
H2 = os.environ.get("AK_H2_VARIANT", "stock")


def rpc_core_grid(cell, direction, payload, k, transport, build):
    """CAMPAIGN section 4.0 (D18): cells A, Bf, Cf, Ef (full build, retain, the core cells
    blocking on the framed path), a+read and b on P2.2, c on P5.4, d on 16 MiB, k = 1 and 8,
    one transport (armonik: ArmoniK's client configuration, CAMPAIGN 4.0 as amended); stock h2 everywhere, plus Cf on h2-batch for c and d."""
    if k not in (1, 8) or transport != "armonik" or build != "full":
        return False
    if (direction, payload) not in (("a+read", "P2.2"), ("b", "P2.2"), ("c", "P5.4"), ("d", "16MiB")):
        return False
    if H2 == "h2-batch":
        return cell == "Cf-retain" and direction in ("c", "d")
    return cell in ("A", "Bf", "Cf-retain", "Ef-retain")

SCALE = {"ns": 1.0, "us": 1e3, "ms": 1e6, "s": 1e9}


def main(path, launch, build="full", suite="codec"):
    d = json.load(open(path))
    ctx = d.get("context", {})
    print("# " + json.dumps({"google_benchmark_context": {k: ctx.get(k) for k in (
        "library_version", "library_build_type", "num_cpus", "mhz_per_cpu",
        "cpu_scaling_enabled", "caches", "executable", "date")}}, sort_keys=True))
    n = 0
    if any(b.get("error_occurred") for b in d.get("benchmarks", [])):
        print("# " + json.dumps({"refused": "a benchmark reported error_occurred"}))
        return 2
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
        if suite == "rpc":
            k = int(tags.pop("inflight"))
            tags.pop("build", None)
            # getrusage(RUSAGE_SELF) deltas around the repetition's timed loop (repetition totals),
            # present when the client records them.
            ru = {c: round(b[c]) for c in ("ru_nvcsw", "ru_nivcsw", "ru_minflt", "ru_majflt") if c in b}
            if "ru_minflt" in ru:
                ru["minflt_per_call"] = round(ru["ru_minflt"] / float(it * k), 3)
            ru["alloc"] = ALLOC
            ru["h2"] = H2
            ru["grid"] = "core" if rpc_core_grid(arm, direction, payload, k, tags.get("transport"), build) else "extra"
            print(json.dumps({"slice": "cpp", "suite": "rpc", "build": build, "cell": arm, "unknown_mode": mode,
                              "payload": payload, "dir": direction, "socket": "uds", "inflight": k,
                              "launch": int(launch), "round": int(b.get("repetition_index", 0)), "order_pos": n,
                              "cpu_ns": round(b["cpu_time"] * sc * it), "cpu_clock": "process",
                              "wall_ns": round(b["real_time"] * sc * it), "iters": it * k,
                              "sampler": "google-benchmark", **ru, **tags}, separators=(",", ":")))
            n += 1
            continue
        cru = {c: round(b[c]) for c in ("ru_minflt", "ru_majflt") if c in b}
        if "ru_minflt" in cru:
            cru["minflt_per_op"] = round(cru["ru_minflt"] / float(it), 4)
        print(json.dumps({"slice": "cpp", "suite": "codec", "arm": arm, "payload": payload,
                          "content": content, "dir": direction, "unknown_mode": mode,
                          "build": build, "launch": int(launch), "round": int(b.get("repetition_index", 0)),
                          "cpu_ns": round(b["cpu_time"] * sc * it), "cpu_clock": "process", "wall_ns": round(b["real_time"] * sc * it),
                          "iters": it, "alloc": ALLOC, **cru, **tags}, separators=(",", ":")))
        n += 1
    print("# " + json.dumps({"samples_converted": n}))
    return 0 if n else 1


if __name__ == "__main__":
    sys.exit(main(*sys.argv[1:5]))
