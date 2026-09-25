#!/usr/bin/env python3
"""design/CAMPAIGN.md requirement 30: the only summaries a slice may produce, from the raw
per-sample JSON lines the runner committed (never from anything else).

Per key -- (suite, build, arm or cell, payload, content, dir, unknown_mode, transport, inflight) --
over every round of every launch: the median, minimum and maximum of CPU and of wall time
PER ITERATION; and the PER-ROUND RATIO to the reference taken in the same launch and round
(the same build's `incumbent-prod` for the codec suite, same payload/content/dir; cell `A` for the RPC suite,
same dir/transport/inflight), with its median and range. No significance claims, no verdict
words: interpretation is the aggregating session's, the decision the owner's.

  campaign_summary.py FILE.jsonl [FILE.jsonl ...]   > summary.json
"""
import json
import statistics
import sys


def load(paths):
    out = []
    for p in paths:
        for line in open(p):
            line = line.strip()
            if line.startswith("{"):
                out.append(json.loads(line))
    return out


def key(s):
    return (s.get("suite"), s.get("build", "full"), s.get("arm") or s.get("cell"), s.get("payload"),
            s.get("content"), s.get("dir"), s.get("unknown_mode"), s.get("transport"), s.get("inflight"))


def ref_key(s):
    """The reference sample for s: same launch, round and case."""
    if s.get("suite") == "codec":
        return ("codec", s.get("build", "full"), "incumbent-prod", s.get("payload"), s.get("content"),
                s.get("dir"), s.get("launch"), s.get("round"))
    if s.get("suite") == "rpc":
        return ("rpc", s.get("build", "full"), "A", s.get("payload"), s.get("dir"), s.get("transport"),
                s.get("inflight"), s.get("launch"), s.get("round"))
    return None


def own_ref_key(s):
    if s.get("suite") == "codec":
        return ("codec", s.get("build", "full"), s.get("arm"), s.get("payload"), s.get("content"),
                s.get("dir"), s.get("launch"), s.get("round"))
    return ("rpc", s.get("build", "full"), s.get("cell"), s.get("payload"), s.get("dir"), s.get("transport"),
            s.get("inflight"), s.get("launch"), s.get("round"))


def stats(v):
    return {"median": statistics.median(v), "min": min(v), "max": max(v), "n": len(v)} if v else None


def main(paths):
    samples = load(paths)
    refs = {}
    for s in samples:
        if (s.get("suite") == "codec" and s.get("arm") == "incumbent-prod") or \
           (s.get("suite") == "rpc" and s.get("cell") == "A"):
            refs[own_ref_key(s)] = s
    groups = {}
    for s in samples:
        g = groups.setdefault(key(s), {"cpu": [], "wall": [], "ratio_cpu": [], "ratio_wall": []})
        it = float(s.get("iters") or 1)
        g["cpu"].append(s["cpu_ns"] / it)
        if "wall_ns" in s:
            g["wall"].append(s["wall_ns"] / it)
        rk = ref_key(s)
        r = refs.get(rk) if rk else None
        if r is not None:
            g["ratio_cpu"].append((s["cpu_ns"] / it) / (r["cpu_ns"] / float(r.get("iters") or 1)))
            if "wall_ns" in s and "wall_ns" in r:
                g["ratio_wall"].append((s["wall_ns"] / it) / (r["wall_ns"] / float(r.get("iters") or 1)))
    out = []
    names = ["suite", "build", "arm_or_cell", "payload", "content", "dir", "unknown_mode", "transport", "inflight"]
    for k, g in sorted(groups.items(), key=lambda kv: tuple(str(x) for x in kv[0])):
        row = {n: v for n, v in zip(names, k) if v is not None}
        row.update({"cpu_ns_per_iter": stats(g["cpu"]), "wall_ns_per_iter": stats(g["wall"]),
                    "ratio_cpu_to_ref": stats(g["ratio_cpu"]), "ratio_wall_to_ref": stats(g["ratio_wall"])})
        out.append(row)
    json.dump(out, sys.stdout, indent=1, sort_keys=True)
    print()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
