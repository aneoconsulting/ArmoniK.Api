#!/usr/bin/env python3
"""design/CAMPAIGN.md requirement 30: the only summaries a slice may produce, from the raw
per-sample JSON lines the runner committed (never from anything else).

Per key -- (suite, build, arm or cell, payload, content, dir, unknown_mode, transport, inflight,
and the row tags: end, input, set, row) -- over every round of every launch: the median,
minimum and maximum of CPU and of wall time PER ITERATION.

Ratios (amended by the owner 2026-09-26): formed from PER-LAUNCH MEDIANS. For each launch,
the key's median per iteration is divided by the reference's median per iteration in the
same launch; the summary gives the median and range of those per-launch ratios. The
reference is the same build's `incumbent-prod` for the codec suite (same payload, content,
dir, input and set/row tags; for encode, incumbent-prod's own end state, the transport form,
is the only one it has), and the same build's cell `A` for the RPC suite (same dir,
transport, inflight). No significance claims, no verdict words: interpretation is the
aggregating session's, the decision the owner's.

  campaign_summary.py FILE.jsonl [FILE.jsonl ...]   > summary.json
"""
import json
import statistics
import sys

TAGS = ("end", "input", "set", "row")


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
            s.get("content"), s.get("dir"), s.get("unknown_mode"), s.get("transport"), s.get("inflight")) + \
        tuple(s.get(t) for t in TAGS)


def ref_of(k):
    """The reference key for key k (without launch)."""
    suite, build, who, payload, content, d, _mode, transport, inflight, end, inp, st, row = k
    if suite == "codec":
        return ("codec", build, "incumbent-prod", payload, content, d, "default", transport, inflight,
                "transport" if d == "encode" else None, inp, st, row)
    if suite == "rpc":
        return ("rpc", build, "A", payload, content, d, "default", transport, inflight, end, inp, st, row)
    return None


def stats(v):
    return {"median": statistics.median(v), "min": min(v), "max": max(v), "n": len(v)} if v else None


def main(paths):
    samples = load(paths)
    groups = {}
    per_launch = {}   # (key, launch) -> {"cpu": [...], "wall": [...]}
    for s in samples:
        k = key(s)
        it = float(s.get("iters") or 1)
        if "cpu_ns" not in s:
            continue          # a smoke log with its figures stripped
        g = groups.setdefault(k, {"cpu": [], "wall": []})
        pl = per_launch.setdefault((k, s.get("launch")), {"cpu": [], "wall": []})
        g["cpu"].append(s["cpu_ns"] / it)
        pl["cpu"].append(s["cpu_ns"] / it)
        if "wall_ns" in s:
            g["wall"].append(s["wall_ns"] / it)
            pl["wall"].append(s["wall_ns"] / it)
    out = []
    names = ["suite", "build", "arm_or_cell", "payload", "content", "dir", "unknown_mode", "transport",
             "inflight"] + list(TAGS)
    for k, g in sorted(groups.items(), key=lambda kv: tuple(str(x) for x in kv[0])):
        rk = ref_of(k)
        rc, rw = [], []
        for (kk, launch), pl in per_launch.items():
            if kk != k or rk is None:
                continue
            ref = per_launch.get((rk, launch))
            if not ref:
                continue
            if pl["cpu"] and ref["cpu"]:
                rc.append(statistics.median(pl["cpu"]) / statistics.median(ref["cpu"]))
            if pl["wall"] and ref["wall"]:
                rw.append(statistics.median(pl["wall"]) / statistics.median(ref["wall"]))
        row = {n: v for n, v in zip(names, k) if v is not None}
        row.update({"cpu_ns_per_iter": stats(g["cpu"]), "wall_ns_per_iter": stats(g["wall"]),
                    "ratio_cpu_to_ref_per_launch_medians": stats(rc),
                    "ratio_wall_to_ref_per_launch_medians": stats(rw)})
        out.append(row)
    json.dump(out, sys.stdout, indent=1, sort_keys=True)
    print()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
