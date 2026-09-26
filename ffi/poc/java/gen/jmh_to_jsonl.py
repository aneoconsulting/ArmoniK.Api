#!/usr/bin/env python3
"""JMH's JSON result -> the section 7 JSON lines (design/CAMPAIGN.md req 22a, 28).

Every raw per-iteration measurement JMH recorded (primaryMetric.rawData, one list per fork)
becomes one sample line; nothing is summarised or dropped. CPU time comes from the side file
`ak.CodecJmh` writes (thread CPU per iteration, tagged warm-up or measurement), joined by
(cell, measurement-iteration index); a sample whose CPU is missing is refused, not guessed.
A meta line per benchmark records what JMH ran with: warm-up and measurement iterations,
the mode, forks, the JVM, its arguments.

  gen/jmh_to_jsonl.py <jmh.json> <cpu.tsv> <launch> <coder> [<build>] >> <log.jsonl>

`build` (WP5 step 10): full (default) or no-unknown, written on every sample.
"""
import json
import sys


def main():
    res = json.load(open(sys.argv[1]))
    launch, coder = int(sys.argv[3]), sys.argv[4]
    build = sys.argv[5] if len(sys.argv) > 5 else "full"
    cpu = {}
    for line in open(sys.argv[2]):
        f = line.rstrip("\n").split("\t")
        cell, kind, idx, c, n = f[:5]
        jit = int(f[5]) if len(f) > 5 else None     # ms of JIT compilation during the iteration
        if kind == "m":
            cpu[(cell, int(idx))] = (int(c), int(n), jit)
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
        for fork in pm["rawData"]:
            for i, wall in enumerate(fork):
                if (cell, i) not in cpu:
                    raise SystemExit("no CPU sample for %s measurement %d" % (cell, i))
                c, n, jit = cpu[(cell, i)]
                extra = {}
                cont = content
                if content.startswith("corpus:"):
                    cont, extra = "corpus", {"root": content[7:]}
                rec = {"slice": "java", "suite": "codec", "arm": arm, "payload": payload,
                    "content": cont, "dir": d, "unknown_mode": mode, "build": build, "coder": coder,
                    "engine": "jmh", "launch": launch, "round": i + 1, "cpu_ns": c,
                    "wall_ns": int(round(wall)), "iters": n}
                if jit is not None:
                    rec["jit_ms_during"] = jit
                rec.update(extra)
                out.append(json.dumps(rec, separators=(",", ":")))
    print("\n".join(out))


if __name__ == "__main__":
    main()
