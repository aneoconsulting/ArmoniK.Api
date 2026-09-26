"""CAMPAIGN req 32: a smoke run's logs are committed with their figures STRIPPED (a container
timing is instrumentation, never a result). Every JSON-lines sample keeps its labels (arm,
cell, payload, content, dir, mode, variant, launch, round, iters, pool size) and loses
`cpu_ns` and `wall_ns`; criterion's console logs keep their header and lose the body.

  python3 gen/strip_figures.py DIR     (in place; idempotent)
"""
import json
import os
import sys


def strip_jsonl(p):
    out, n = [], 0
    for line in open(p):
        if line.startswith("#") or not line.strip():
            out.append(line)
            continue
        o = json.loads(line)
        for k in ("cpu_ns", "wall_ns"):
            if k in o:
                del o[k]
                n += 1
        o["figures"] = "stripped"
        out.append(json.dumps(o, sort_keys=True) + "\n")
    if not any(l.startswith("# FIGURES STRIPPED") for l in out):
        out.insert(0, "# FIGURES STRIPPED (CAMPAIGN req 32): cpu_ns and wall_ns removed from every sample; labels kept\n")
    open(p, "w").writelines(out)
    return n


def strip_criterion(p):
    keep = [l for l in open(p) if l.startswith("#")]
    keep.append("# criterion's console output removed: its figures are stripped (CAMPAIGN req 32)\n")
    open(p, "w").writelines(keep)


def main(d):
    for f in sorted(os.listdir(d)):
        p = os.path.join(d, f)
        if f.endswith(".jsonl"):
            print("%s: %d figures removed" % (f, strip_jsonl(p)))
        elif f.endswith(".criterion.log"):
            strip_criterion(p)
            print("%s: body removed" % f)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1]))
