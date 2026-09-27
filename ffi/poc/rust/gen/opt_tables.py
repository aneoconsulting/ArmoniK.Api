#!/usr/bin/env python3
"""Tables of ONE opt_bench run (no comparison): tables-codec.md and tables-rpc.md.

  gen/opt_tables.py RUN_DIR

Reads RUN_DIR/variants-codec.tsv, unknown-retain-vs-drop.tsv and summary-rpc.tsv (written by
gen/opt_summary.py) and writes RUN_DIR/tables-codec.md and RUN_DIR/tables-rpc.md.
Every figure is CONTAINER INSTRUMENTATION: absolutes of one process per build, not results.
"""
import csv
import os
import subprocess
import sys


def rows(path):
    with open(path) as f:
        lines = [l for l in f if not l.startswith("#")]
    return list(csv.DictReader(lines, delimiter="\t"))


def head_of(path):
    with open(path) as f:
        return f.readline().strip().lstrip("# ")


def us(ns, digits=None):
    if ns in ("", None):
        return ""
    v = float(ns) / 1000.0
    if digits is None:
        digits = 3 if v < 10 else 2 if v < 1000 else 1
    return f"{v:.{digits}f}"


def run_header(d):
    p = os.path.join(d, "header.txt")
    if not os.path.exists(p):
        return []
    return [l.rstrip("\n") for l in open(p) if l.strip()][:8]


def codec(d, out):
    vpath = os.path.join(d, "variants-codec.tsv")
    vr = rows(vpath)
    cols = [c for c in csv.DictReader([l for l in open(vpath) if not l.startswith("#")],
                                      delimiter="\t").fieldnames if c not in ("input", "dir", "variant")]
    w = out.write
    w("# Codec: absolute medians per variant, one run\n\n")
    w("CONTAINER INSTRUMENTATION, not a result (README 1.1). One `gen/opt_bench.sh` run "
      f"(`{os.path.basename(d)}`), no comparison with any other run. Median CPU time per operation "
      "in **microseconds** (criterion, process CPU). The columns up to `ffi-zc-retain` are the full "
      "build's process; the `@nounk` columns and `native-nounk`, `ffi-nounk`, `pull-nounk`, "
      "`ffi-zc-nounk` are the no-unknown build's process, so a ratio across the two groups crosses "
      "processes. `prost` and `prost@nounk` are the same code in the two processes (the control). "
      "Empty cell: the arm has no such row (pull and zc arms decode only; `transport-ready-core` "
      "exists for the core arms only; `ffi-zc` is the zero-copy arm, P5.* only). Source: "
      "`variants-codec.tsv` (ns).\n\n")
    for l in run_header(d):
        w(f"    {l}\n")
    w("\n")
    w("| input | dir | variant | " + " | ".join(cols) + " |\n")
    w("|---|---|---|" + "---:|" * len(cols) + "\n")
    for r in vr:
        w(f"| {r['input']} | {r['dir']} | {r['variant']} | " + " | ".join(us(r[c]) for c in cols) + " |\n")
    upath = os.path.join(d, "unknown-retain-vs-drop.tsv")
    ur = rows(upath)
    w("\n## U-* rows: retain against drop\n\n")
    w("The 92 accepted `U-*` corpus rows (unknown fields). `gmean_retain/drop`, min and max are "
      "over the rows, retain and drop in the same process (full build); the three absolute "
      "columns are the medians over the rows in microseconds, the no-unknown one from the other "
      f"process. Source: `unknown-retain-vs-drop.tsv` ({head_of(upath)}).\n\n")
    w("| arm | dir | variant | rows | gmean retain/drop | min | max | median drop (us) | median retain (us) | median no-unknown (us) |\n")
    w("|---|---|---|---:|---:|---:|---:|---:|---:|---:|\n")
    for r in ur:
        w(f"| {r['arm']} | {r['dir']} | {r['variant']} | {r['rows']} | {float(r['gmean_retain/drop']):.3f} | "
          f"{float(r['min']):.3f} | {float(r['max']):.3f} | {us(r['median_abs_drop_ns'])} | "
          f"{us(r['median_abs_retain_ns'])} | {us(r['median_abs_nounk_ns'])} |\n")


DIRS = ["a", "a+read", "b", "c/P5.3", "c/P5.4", "d/4MiB", "d/16MiB"]
DIR_TEXT = {
    "a": "direction a: Fetch, the P2.2 response decoded",
    "a+read": "direction a+read: Fetch, decoded, every field read",
    "b": "direction b: P2.2 encoded and sent (Push)",
    "c/P5.3": "direction c (labelled extra, U1-unary): a unary upload of P5.3",
    "c/P5.4": "direction c (labelled extra, U1-unary): a unary upload of P5.4",
    "d/4MiB": "direction d (labelled extra, U2-stream): a client-streamed upload of 4 MiB in 2 MiB chunks",
    "d/16MiB": "direction d (labelled extra, U2-stream): a client-streamed upload of 16 MiB in 2 MiB chunks",
}


def cell_key(c):
    base, _, mode = c.partition("-")
    letter, framed = base[0], base[1:] == "f"
    order_mode = {"": 0, "drop": 1, "retain": 2, "nounk": 3}[mode]
    return (letter, order_mode, framed)


def rpc(d, out):
    spath = os.path.join(d, "summary-rpc.tsv")
    sr = rows(spath)
    w = out.write
    w("# RPC grid: every cell and framed twin, one run\n\n")
    w("CONTAINER INSTRUMENTATION, not a result (README 1.1). One `gen/opt_bench.sh` run "
      f"(`{os.path.basename(d)}`), no comparison with any other run. Per call: client process "
      "CPU (or wall) of a round divided by the calls in the round, in **microseconds**; each "
      "entry is the median over the rounds with [min-max]. `k` = calls in flight. `pinned` and "
      "`shipped` are the two transport configurations (CAMPAIGN.md); the full client (cells in "
      "drop and retain) and the no-unknown client (`-nounk` cells, and its own A and B) are two "
      "processes each. `f` = the framed send path (labelled extra cells), placed under its "
      "reference twin. Source: `summary-rpc.tsv` (ns).\n\n")
    for l in run_header(d):
        w(f"    {l}\n")
    w("\n")
    for client, fsuffix in (("full client", ""), ("no-unknown client", "-nounk")):
        for dr in DIRS:
            sel = [r for r in sr if r["dir"] == dr and r["file"].endswith(fsuffix)
                   and (fsuffix or not r["file"].endswith("-nounk"))]
            if not sel:
                continue
            ks = sorted({int(r["inflight"]) for r in sel})
            cells = sorted({r["cell"] for r in sel}, key=cell_key)
            idx = {(r["cell"], r["transport"], int(r["inflight"])): r for r in sel}
            rounds = sorted({r["rounds"] for r in sel})
            for metric, label in (("cpu", "client CPU"), ("wall", "wall")):
                w(f"## {client}, {DIR_TEXT[dr]} -- {label}\n\n")
                w(f"Rounds per entry: {', '.join(rounds)}.\n\n")
                hdr = [f"{t} k={k}" for t in ("pinned", "shipped") for k in ks]
                w("| cell | " + " | ".join(hdr) + " |\n")
                w("|---|" + "---:|" * len(hdr) + "\n")
                for c in cells:
                    ent = []
                    for t in ("pinned", "shipped"):
                        for k in ks:
                            r = idx.get((c, t, k))
                            if r is None:
                                ent.append("")
                                continue
                            ent.append(f"{us(r[metric + '_median_ns'])} [{us(r[metric + '_min_ns'])}-{us(r[metric + '_max_ns'])}]")
                    w(f"| {c} | " + " | ".join(ent) + " |\n")
                w("\n")


def main():
    d = sys.argv[1]
    with open(os.path.join(d, "tables-codec.md"), "w") as f:
        codec(d, f)
    with open(os.path.join(d, "tables-rpc.md"), "w") as f:
        rpc(d, f)
    print(f"wrote {d}/tables-codec.md and {d}/tables-rpc.md")


if __name__ == "__main__":
    main()
