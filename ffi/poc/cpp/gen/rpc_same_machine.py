#!/usr/bin/env python3
"""summary-rpc.tsv and tables-rpc.md of one gen/rpc_same_machine.sh run (no comparison).

  gen/rpc_same_machine.py OUT_DIR

Laid out as poc/rust/gen/rpc_same_machine.py lays out its run, so the two tables sit side by
side: per (file, transport, cell, dir, k): repetitions (samples), iterations (batches) per
repetition, client CPU and wall per call (repetition value / calls in it), median, min, max.
`*` = one batch per repetition. CONTAINER INSTRUMENTATION.
"""
import json
import os
import statistics
import sys
from collections import defaultdict

DIRS = ["a", "a+read", "b", "c/P5.3", "c/P5.4", "d/4MiB", "d/16MiB"]
DIR_TEXT = {  # the Rust run's wording
    "a": "direction a: Fetch, the P2.2 response decoded",
    "a+read": "direction a+read: Fetch, decoded, every field read",
    "b": "direction b: P2.2 encoded and sent (Push)",
    "c/P5.3": "direction c (labelled extra, U1-unary): a unary upload of P5.3",
    "c/P5.4": "direction c (labelled extra, U1-unary): a unary upload of P5.4",
    "d/4MiB": "direction d (labelled extra, U2-stream): a client-streamed upload of 4 MiB in 2 MiB chunks",
    "d/16MiB": "direction d (labelled extra, U2-stream): a client-streamed upload of 16 MiB in 2 MiB chunks",
}


def us(ns):
    v = float(ns) / 1000.0
    d = 3 if v < 10 else 2 if v < 1000 else 1
    return f"{v:.{d}f}"


def cell_key(c):
    base, _, mode = c.partition("-")
    return (base[0], {"": 0, "drop": 1, "retain": 2, "nounk": 3}[mode], {"": 0, "f": 1, "p": 2}[base[1:]])


def main():
    out = sys.argv[1]
    acc, order = defaultdict(lambda: {"cpu": [], "wall": [], "b": []}), []
    for fn in sorted(os.listdir(out)):
        if not (fn.startswith("rpc-") and fn.endswith(".jsonl")):
            continue
        tag = fn[:-len(".jsonl")]
        for line in open(os.path.join(out, fn)):
            if not line.startswith("{"):
                continue
            o = json.loads(line)
            d = o["dir"] if o["dir"] not in ("c", "d") else o["dir"] + "/" + o["payload"]
            k = (tag, o["transport"], o["cell"], d, o["inflight"])
            if k not in acc:
                order.append(k)
            acc[k]["cpu"].append(o["cpu_ns"] / o["iters"])
            acc[k]["wall"].append(o["wall_ns"] / o["iters"])
            acc[k]["b"].append(o["iters"] // o["inflight"])
    rows = []
    with open(os.path.join(out, "summary-rpc.tsv"), "w") as f:
        f.write("# CONTAINER INSTRUMENTATION (gen/rpc_same_machine.sh); ns per call = client process CPU (or wall) per "
                "Google Benchmark repetition / calls in the repetition; batches = iterations per repetition (one "
                "iteration = one batch of k calls); samples = repetitions\n")
        f.write("file\ttransport\tcell\tdir\tinflight\tsamples\tbatches_min\tbatches_max\tcpu_median_ns\tcpu_min_ns\t"
                "cpu_max_ns\twall_median_ns\twall_min_ns\twall_max_ns\n")
        for k in order:
            a = acc[k]
            c, w = sorted(a["cpu"]), sorted(a["wall"])
            r = dict(zip(["file", "transport", "cell", "dir", "inflight"], map(str, k)))
            r.update(samples=len(c), bmin=min(a["b"]), bmax=max(a["b"]),
                     cpu=(statistics.median(c), c[0], c[-1]), wall=(statistics.median(w), w[0], w[-1]))
            rows.append(r)
            f.write("\t".join(map(str, k)) + "\t%d\t%d\t%d\t%.1f\t%.1f\t%.1f\t%.1f\t%.1f\t%.1f\n" % (
                len(c), r["bmin"], r["bmax"], *r["cpu"], *r["wall"]))
    single = [r for r in rows if r["bmin"] == 1]
    few = [r for r in rows if r["samples"] < 10]
    with open(os.path.join(out, "tables-rpc.md"), "w") as f:
        w = f.write
        w("# RPC grid, same-machine run (cpp slice): every cell and twin\n\n")
        w("CONTAINER INSTRUMENTATION, not a result (README 1.1). One `gen/rpc_same_machine.sh` run, laid out as "
          "the Rust slice's `logs/rust/opt/rpc-same-machine/tables-rpc.md` for a side-by-side; no comparison is "
          "made here. Per call: client process CPU (or wall) of a Google Benchmark repetition divided by the calls "
          "in it, in **microseconds**; each entry is the median over the 10 repetitions with [min-max]. `k` = calls "
          "in flight. The full client (cells in drop and retain) and the no-unknown client (`-nounk` cells, and its "
          "own A, B, Bf) are separate processes; so are the three direction groups (a/a+read/b, c, d) of each "
          "(transport, client); every cell of one direction shares a process. `f` = the framed send path "
          "(labelled extra cells), `p` = the pull decode twins (labelled extra, a and a+read only). **`*`** = one "
          "batch per repetition (one batch of k calls already took longer than the repetition's min time). The "
          "header's `vs rust` line lists what differs from the Rust run. Source: `summary-rpc.tsv` (ns).\n\n")
        for l in open(os.path.join(out, "header.txt")):
            w(f"    {l.rstrip()}\n")
        w(f"\nEntries: {len(rows)}; samples per entry {min(r['samples'] for r in rows)}-{max(r['samples'] for r in rows)}; "
          f"batches per sample {min(r['bmin'] for r in rows)}-{max(r['bmax'] for r in rows)}; "
          f"entries with ONE batch per sample: {len(single)}"
          + (f" ({', '.join(sorted({r['dir'] + ' k=' + r['inflight'] for r in single}))})" if single else "")
          + (f"; entries with fewer than 10 samples: {len(few)}" if few else "") + ".\n\n")
        for client, nounk in (("full client", False), ("no-unknown client", True)):
            for dr in DIRS:
                sel = [r for r in rows if r["dir"] == dr and r["file"].endswith("-nounk") == nounk]
                if not sel:
                    continue
                ks = sorted({int(r["inflight"]) for r in sel})
                cells = sorted({r["cell"] for r in sel}, key=cell_key)
                idx = {(r["cell"], r["transport"], int(r["inflight"])): r for r in sel}
                for metric, label in (("cpu", "client CPU"), ("wall", "wall")):
                    w(f"## {client}, {DIR_TEXT[dr]} -- {label} per call (us)\n\n")
                    hdr = [f"{t} k={k}" for t in ("shipped", "pinned") for k in ks]
                    w("| cell | " + " | ".join(hdr) + " |\n")
                    w("|---|" + "---:|" * len(hdr) + "\n")
                    for c in cells:
                        ent = []
                        for t in ("shipped", "pinned"):
                            for k in ks:
                                r = idx.get((c, t, k))
                                if r is None:
                                    ent.append("")
                                    continue
                                m, lo, hi = r[metric]
                                ent.append(f"{us(m)} [{us(lo)}-{us(hi)}]" + (" *" if r["bmin"] == 1 else ""))
                        w(f"| {c} | " + " | ".join(ent) + " |\n")
                    w("\n")
        w("## Batches per repetition\n\n| dir | k | client | min | median | max |\n|---|---:|---|---:|---:|---:|\n")
        for dr in DIRS:
            for k in sorted({int(r["inflight"]) for r in rows if r["dir"] == dr}):
                for nounk in (False, True):
                    b = sorted(r["bmin"] for r in rows if r["dir"] == dr and int(r["inflight"]) == k
                               and r["file"].endswith("-nounk") == nounk)
                    if b:
                        w(f"| {dr} | {k} | {'no-unknown' if nounk else 'full'} | {b[0]} | {b[len(b) // 2]} | {b[-1]} |\n")
    print(f"summary-rpc.tsv: {len(rows)} entries; one batch per repetition: {len(single)}; wrote tables-rpc.md")


if __name__ == "__main__":
    main()
