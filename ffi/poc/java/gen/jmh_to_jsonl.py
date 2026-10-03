#!/usr/bin/env python3
"""JMH's JSON result -> the section 7 JSON lines (design/CAMPAIGN.md req 22a, 28).

Every raw per-iteration measurement JMH recorded (primaryMetric.rawData, one list per fork)
becomes one sample line; nothing is summarised or dropped. Process CPU, the operation count
and the JIT delta of each iteration are `ak.CodecJmh`'s @AuxCounters, which JMH exports in the
same JSON (secondaryMetrics cpuNs, iters, jitMs, rawData per fork and iteration); a sample
whose counter is missing is refused, not guessed.
A meta line per benchmark records what JMH ran with: warm-up and measurement iterations,
the mode, forks, the JVM, its arguments.

  gen/jmh_to_jsonl.py <jmh.json> <launch> <coder> [<build> [<alloc>]] >> <log.jsonl>

`build` (WP5 step 10): full (default) or no-unknown, written on every sample.
"""
import json
import sys


# CAMPAIGN req 7 as amended 2026-09-26 (R-H26, R-H27): what every slice must report; the
# rest are labelled extras.
REQUIRED_SETS = {"P1.2", "P2.2", "P2.4"}


def row_class(payload, content):
    if content.startswith("shapes:"):
        return "required"
    if content.startswith("corpus:"):
        return "extra"
    if content == "ascii" or payload in REQUIRED_SETS:
        return "required"
    return "extra"


def main():
    res = json.load(open(sys.argv[1]))
    launch, coder = int(sys.argv[2]), sys.argv[3]
    build = sys.argv[4] if len(sys.argv) > 4 else "full"
    alloc = sys.argv[5] if len(sys.argv) > 5 else "default"   # req 25 / D9 as amended
    out = []
    for b in res:
        cell = b["params"]["cell"]
        arm, mode, payload, content, d = cell.split("|")
        pm = b["primaryMetric"]
        if pm.get("scoreUnit") not in ("ns/op",):
            raise SystemExit("unexpected JMH unit %r for %s" % (pm.get("scoreUnit"), cell))
        out.append(json.dumps({"meta": {"suite": "codec", "engine": "jmh " + b.get("jmhVersion", "?"),
            "cell": cell, "mode": b["mode"], "forks": b["forks"],
            "warmup_iterations": b["warmupIterations"], "measurement_iterations": b["measurementIterations"],
            "warmup_batch": b.get("warmupBatchSize"), "measurement_batch": b.get("measurementBatchSize"),
            "jdk": b.get("jdkVersion"), "vm": b.get("vmName", "") + " " + b.get("vmVersion", ""),
            "jvm_args": b.get("jvmArgs")}}, separators=(",", ":")))
        sm = b.get("secondaryMetrics", {})
        for name in ("cpuNs", "iters", "jitMs"):
            if name not in sm:
                raise SystemExit("no %s counter for %s" % (name, cell))
        for f, fork in enumerate(pm["rawData"]):
            for i, wall in enumerate(fork):
                try:
                    c, n, jit = (int(round(sm[x]["rawData"][f][i])) for x in ("cpuNs", "iters", "jitMs"))
                except (IndexError, KeyError):
                    raise SystemExit("no counter sample for %s measurement %d" % (cell, i))
                extra = {}
                cont = content
                if content.startswith("corpus:"):
                    cont, extra = "corpus", {"root": content[7:], "core": "corpus"}
                elif content.startswith("shapes:"):
                    cont, extra = "corpus", {"root": content[7:], "core": "shapes"}
                # Req 11 (R-H29): the encode variant, split out of the cell's dir.
                dd = d
                if d.startswith("encode"):
                    dd = "encode"
                    extra["end"] = "transport" if d.startswith("encode-transport") else "buf"
                    extra["input"] = "hot" if d.endswith("-hot") else ("pool" if cont != "corpus" else "row")
                extra["row_class"] = row_class(payload, content)
                extra["alloc"] = alloc
                if "minflt" in sm:
                    extra["minflt"] = int(round(sm["minflt"]["rawData"][f][i]))
                extra["h2"] = "n/a"     # D11: codec cores carry no rpc feature, so no h2
                rec = {"slice": "java", "suite": "codec", "arm": arm, "payload": payload,
                    "content": cont, "dir": dd, "unknown_mode": mode, "build": build, "coder": coder,
                    "engine": "jmh", "launch": launch, "round": i + 1, "cpu_ns": c,
                    "wall_ns": int(round(wall)), "iters": n}
                rec["jit_ms_during"] = jit
                rec.update(extra)
                out.append(json.dumps(rec, separators=(",", ":")))
    print("\n".join(out))


if __name__ == "__main__":
    main()
