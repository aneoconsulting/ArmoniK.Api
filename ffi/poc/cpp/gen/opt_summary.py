#!/usr/bin/env python3
"""Summaries of ONE gen/opt_bench.sh run, from its raw samples (no comparison, no ratio).

  gen/opt_summary.py RUN_DIR

Reads RUN_DIR/codec-*.jsonl and RUN_DIR/rpc-*.jsonl (section 7's lines, converted from
Google Benchmark's per-repetition JSON by gen/gbench_to_jsonl.py; the raw JSON is kept beside
them as *.gbench.json.gz) and writes:

  summary-codec.tsv   per case: build, file, input, payload, content, arm, dir, mode, encode
                      variant, samples, median/min/max/q25/q75 ns per operation (process CPU),
                      spread = (max - min) / median, wall median, the fewest iterations
                      in one repetition (min_iters), note
  variants-codec.tsv  absolute medians, one row per (input, dir, variant), one column per arm
                      and mode; the full build's process and the no-unknown build's (@nounk
                      columns) are two processes
  summary-rpc.tsv     per (build, transport, cell, dir, payload, k): client process CPU and wall
                      per call, median and min/max over the repetitions, and the fewest
                      batches (Google Benchmark iterations) in one repetition (min_batches)
  tables-codec.md, tables-rpc.md   the same absolutes as markdown, in microseconds

Per operation: codec = cpu_ns / iters (one repetition); rpc = cpu_ns / iters, iters being the
calls of the repetition (Google Benchmark iterations x k in flight), so a k > 1 row is the
process CPU (or wall) of the batch divided by its calls.
Every figure is CONTAINER INSTRUMENTATION.

Harness defect H-1 (not fixed in the baseline): every input=pool encode row encodes pool[0]
only; those rows are labelled.
"""
import glob
import json
import os
import statistics
import sys

H1 = "H-1: pool[0] only"
FULL_ARMS = ["incumbent-prod", "incumbent-best", "incumbent-arena", "host-gen-drop", "host-gen-retain",
             "core-ffi-drop", "core-ffi-retain", "core-ffi-borrow-drop", "core-ffi-borrow-retain"]
NOUNK_ARMS = ["incumbent-prod@nounk", "incumbent-best@nounk", "incumbent-arena@nounk", "host-gen-nounk",
              "core-ffi-nounk", "core-ffi-borrow-nounk"]
KNOWN_TAGS = ("end", "input", "row", "set")  # every other tag of a sample is part of its variant
VARIANTS = ["reused/hot", "reused/pool", "transport/hot", "transport/pool"]
DIRS = ["encode", "decode", "decode_read"]
RPC_JOBS = [("a", "P2.2"), ("a+read", "P2.2"), ("b", "P2.2"), ("c", "P5.3"), ("c", "P5.4"), ("d", "4MiB"),
            ("d", "16MiB")]
JOB_TEXT = {"a": "Fetch, the P2.2 response decoded", "a+read": "Fetch, the P2.2 response decoded and every field read",
            "b": "Push, the P2.2 request encoded (the server decodes it)", "c": "Upload, the request encoded, empty response",
            "d": "UploadStream in 2 MiB M5 chunks, the server's count checked"}
FULL_CELLS = ["A", "B", "Bf", "C-drop", "Cf-drop", "C-retain", "Cf-retain", "D-drop", "D-retain", "E-drop",
              "Ef-drop", "E-retain", "Ef-retain", "F-drop", "F-retain"]
NOUNK_CELLS = ["A", "B", "Bf", "C-nounk", "Cf-nounk", "D-nounk", "E-nounk", "Ef-nounk", "F-nounk"]


def samples(path):
    with open(path) as f:
        for l in f:
            if l.startswith("{"):
                yield json.loads(l)


def header_lines(path):
    """A short version of the run's header: commit, machine, CPU sets, versions, settings."""
    out = []
    with open(path) as f:
        for l in f:
            if not l.startswith("#"):
                break
            if l.startswith("# {") and '"slice"' in l and not out:
                h = json.loads(l[2:])
                m, v = h.get("machine", {}), h.get("versions", {})
                out.append(f"# commit {h.get('commit', '?')[:12]}{' DIRTY' if h.get('dirty') else ''}; "
                           f"{m.get('cpu_model')}, {m.get('nproc')} CPUs; client CPUs {m.get('cpu_client')}, "
                           f"server CPUs {m.get('cpu_server')}; {v.get('cxx')}; protobuf {v.get('protobuf_pkgconfig')}, "
                           f"grpc++ {v.get('grpcpp')}; {v.get('rustc')}")
            elif l.startswith("# settings") or l.startswith("# CONTAINER") or l.startswith("# this file"):
                out.append(l.rstrip("\n"))
    return out


def q(v, p):
    if len(v) == 1:
        return v[0]
    return statistics.quantiles(v, n=4, method="inclusive")[{25: 0, 75: 2}[p]]


def stats(v):
    v = sorted(v)
    med = statistics.median(v)
    return {"n": len(v), "median": med, "min": v[0], "max": v[-1], "q25": q(v, 25), "q75": q(v, 75),
            "spread": (v[-1] - v[0]) / med if med else 0.0}


def fmt(x):
    return f"{x:.1f}"


def us(ns):
    if ns is None or ns == "":
        return ""
    v = float(ns) / 1000.0
    d = 3 if v < 10 else 2 if v < 1000 else 1
    return f"{v:.{d}f}"


def arm_col(s):
    arm, mode, build = s["arm"], s["unknown_mode"], s["build"]
    if arm.startswith("incumbent"):
        return arm + ("@nounk" if build == "no-unknown" else "")
    return f"{arm}-{'nounk' if mode == 'no-unknown' else mode}"  # host-gen-drop/-retain/-nounk, core-ffi-...


BASE_FIELDS = {"slice", "suite", "arm", "payload", "content", "dir", "unknown_mode", "build", "launch", "round",
               "cpu_ns", "cpu_clock", "wall_ns", "iters", "figures"}


def TAG_FIELDS_OTHER(s):
    return [k for k in s if k not in BASE_FIELDS and k not in KNOWN_TAGS]


def codec(run):
    cases = {}
    for path in sorted(glob.glob(os.path.join(run, "codec-*.jsonl"))):
        tag = os.path.basename(path)[:-len(".jsonl")]
        for s in samples(path):
            extra = "/".join(f"{k}={s[k]}" for k in sorted(s) if k in TAG_FIELDS_OTHER(s))
            if s["dir"] == "encode":
                variant = f"{s['end']}/{s['input']}" + (f"/{extra}" if extra else "")
            else:
                variant = extra or "-"
            inp = s["payload"] if s["content"] == "ascii" else f"{s['payload']}/{s['content']}"
            k = (s["build"], tag, inp, s["payload"], s["content"], s["arm"], s["dir"], s["unknown_mode"], variant)
            c = cases.setdefault(k, {"cpu": [], "wall": [], "it": [], "col": arm_col(s), "row": s.get("row", ""),
                                     "set": s.get("set", "")})
            c["it"].append(s["iters"])
            c["cpu"].append(s["cpu_ns"] / s["iters"])
            c["wall"].append(s["wall_ns"] / s["iters"])
    if not cases:
        return None
    with open(os.path.join(run, "summary-codec.tsv"), "w") as f:
        f.write("# CONTAINER INSTRUMENTATION (gen/opt_bench.sh, not gated); ns per operation = process CPU "
                "of one Google Benchmark repetition / its iterations; spread = (max - min) / median\n")
        f.write("build\tfile\tinput\tpayload\tcontent\tarm\tdir\tmode\tvariant\tsamples\tmedian_ns\tmin_ns\tmax_ns"
                "\tq25_ns\tq75_ns\tspread\twall_median_ns\tmin_iters\tnote\n")
        for k in sorted(cases):
            c = cases[k]
            st = stats(c["cpu"])
            note = ""
            f.write("\t".join(list(k) + [str(st["n"]), fmt(st["median"]), fmt(st["min"]), fmt(st["max"]),
                                          fmt(st["q25"]), fmt(st["q75"]), f"{st['spread']:.3f}",
                                          fmt(statistics.median(c["wall"])), str(min(c["it"])), note]) + "\n")
    # variants: (input, dir, variant) x arm column; the U rows after the payloads
    table = {}
    kind = {}
    for k, c in cases.items():
        inp, d, variant = k[2], k[6], k[8]
        table.setdefault((inp, d, variant), {})[c["col"]] = statistics.median(c["cpu"])
        kind[inp] = "U" if c["row"] == "U" else "P"
    present = {c["col"] for c in cases.values()}
    cols = [c for c in FULL_ARMS if c in present] + sorted(c for c in present if c not in FULL_ARMS + NOUNK_ARMS
                                                         and "@nounk" not in c and "-nounk" not in c)
    cols += [c for c in NOUNK_ARMS if c in present] + sorted(c for c in present if c not in FULL_ARMS + NOUNK_ARMS
                                                           and ("@nounk" in c or "-nounk" in c))

    def order(key):
        inp, d, variant = key
        return (kind[inp] == "U", inp, DIRS.index(d), VARIANTS.index(variant) if variant in VARIANTS else 99, variant)
    keys = sorted(table, key=order)
    with open(os.path.join(run, "variants-codec.tsv"), "w") as f:
        f.write("# CONTAINER INSTRUMENTATION (gen/opt_bench.sh): median ns per operation (process CPU), ABSOLUTE; "
                "the full-build columns and the @nounk / -nounk columns are two processes\n")
        f.write("input\tdir\tvariant\t" + "\t".join(cols) + "\tnote\n")
        for key in keys:
            row = table[key]
            f.write("\t".join(list(key) + [fmt(row[c]) if c in row else "" for c in cols] + [""]) + "\n")
    # markdown
    first = sorted(glob.glob(os.path.join(run, "codec-*.jsonl")))[0]
    h1_fixed = "H-1: input=pool walks the pool" in open(first).read(200000)
    with open(os.path.join(run, "tables-codec.md"), "w") as f:
        f.write("# Codec: absolute medians per variant, one run\n\n")
        f.write("CONTAINER INSTRUMENTATION, not a result (README 1.1), not gated (the codec process's own "
                "byte-identity pre-check was on). One `gen/opt_bench.sh` run. Median process CPU per operation "
                "in **microseconds** over the Google Benchmark repetitions of one process. The columns up to "
                "`core-ffi-retain` are the full build's process; the `@nounk` and `-nounk` columns are the "
                "no-unknown build's process, so a comparison across the two groups crosses processes "
                "(`incumbent-*` and `incumbent-*@nounk` are the same code in the two processes: the control). "
                "`incumbent-arena`, `core-ffi-borrow-*` (the borrowed-string facade) and the `from=bytebuffer` rows are "
                "labelled extras (payloads only). "
                "Empty cell: the arm has no such row (incumbent-prod encodes to the transport form only, "
                "incumbent-best to a reused string only; P7.1 is decode only). Source: `variants-codec.tsv` (ns); "
                "per-case spreads in `summary-codec.tsv`.\n\n")
        if not h1_fixed:
            f.write("**Harness defect H-1 (not fixed in this run):** every `*/pool` row encodes pool[0] only "
                    "(`campaign_codec.cpp` calls `run(1)` per iteration and `run` indexes `i % m` from 0), so it is "
                    "a second hot row over a copy of the graph, not req 11's beyond-cache variant. Those rows are "
                    "marked `pool[0]*`.\n\n")
        else:
            f.write("`*/pool` rows walk a pool of distinct copies of the graph (H-1 fixed), one graph per "
                    "iteration; its size is in the header (`pool_bytes`).\n\n")
        f.write("Variants: `reused` = bytes in a reused buffer; `transport` = the form handed to grpc++ "
                "(a `grpc::ByteBuffer`; core-ffi and host-gen move their bytes into it); `hot` = one graph.\n\n")
        for l in header_lines(first):
            f.write("    " + l + "\n")
        f.write("\n")
        for sect, title in (("P", "Payloads (SHAPES.md, content sets)"), ("U", "U-* rows (the 92 corpus rows, reduced settings)")):
            f.write(f"## {title}\n\n")
            f.write("| input | dir | variant | " + " | ".join(cols) + " |\n")
            f.write("|---|---|---|" + "---:|" * len(cols) + "\n")
            for key in keys:
                if kind[key[0]] != sect:
                    continue
                row = table[key]
                v = key[2] if h1_fixed else key[2].replace("/pool", "/pool[0]*")
                f.write(f"| {key[0]} | {key[1]} | {v} | " + " | ".join(us(row.get(c)) for c in cols) + " |\n")
            f.write("\n")
    return len(cases)


def rpc(run):
    cases = {}
    files = sorted(glob.glob(os.path.join(run, "rpc-*.jsonl")))
    for path in files:
        tag = os.path.basename(path)[:-len(".jsonl")]
        for s in samples(path):
            k = (s["build"], s["transport"], s["cell"], s["dir"], s["payload"], s["inflight"])
            c = cases.setdefault(k, {"cpu": [], "wall": [], "it": [], "file": tag, "send_path": s.get("send_path", "")})
            c["it"].append(s["iters"] // s["inflight"])
            c["cpu"].append(s["cpu_ns"] / s["iters"])
            c["wall"].append(s["wall_ns"] / s["iters"])
    if not cases:
        return None
    with open(os.path.join(run, "summary-rpc.tsv"), "w") as f:
        f.write("# CONTAINER INSTRUMENTATION (gen/opt_bench.sh, not gated; every call checked); ns per call = "
                "client process CPU (or wall) of one Google Benchmark repetition / its calls (iterations x k)\n")
        f.write("file\tbuild\ttransport\tcell\tsend_path\tdir\tpayload\tinflight\trounds\tcpu_median_ns\tcpu_min_ns"
                "\tcpu_max_ns\twall_median_ns\twall_min_ns\twall_max_ns\tmin_batches\n")
        for k in sorted(cases, key=lambda k: (k[0], k[1], k[3], k[4], k[2], k[5])):
            c = cases[k]
            a, w = stats(c["cpu"]), stats(c["wall"])
            f.write("\t".join([c["file"], k[0], k[1], k[2], c["send_path"], k[3], k[4], str(k[5]), str(a["n"]),
                               fmt(a["median"]), fmt(a["min"]), fmt(a["max"]), fmt(w["median"]), fmt(w["min"]),
                               fmt(w["max"]), str(min(c["it"]))]) + "\n")
    transports = sorted({k[1] for k in cases}, key=lambda t: (t != "pinned", t))
    ks = sorted({k[5] for k in cases})
    rounds = sorted({len(c["cpu"]) for c in cases.values()})
    with open(os.path.join(run, "tables-rpc.md"), "w") as f:
        f.write("# RPC grid: every cell and framed twin, one run\n\n")
        f.write("CONTAINER INSTRUMENTATION, not a result (README 1.1), not gated (every call checked, the "
                "client's pre-checks on). One `gen/opt_bench.sh` run, ONE server process (poc/rust's tonic "
                "`rpc_server` through `serve.sh`, pinned to AK_CPU_SERVER) for the whole grid. Per call: "
                "client process CPU (or wall) of one Google Benchmark repetition divided by its calls, in "
                "**microseconds**; each entry is the median over the repetitions with [min-max]. `k` = calls "
                "in flight (c and d run at 1 and 8 only). `pinned` and `shipped` are the two transport "
                "configurations; the full client (cells in drop and retain) and the no-unknown client "
                "(`-nounk` cells, and its own A and B) are separate processes. `f` = the core's framed send "
                "path (labelled extra cells), placed under its reference twin. `†` = at least one repetition was a "
                "single batch (one Google Benchmark iteration: the batch took longer than min_time), so that "
                "sample is one batch of k calls. Source: `summary-rpc.tsv` (ns).\n\n")
        f.write(f"Repetitions per entry: {', '.join(map(str, rounds))}.\n\n")
        if files:
            for l in header_lines(files[0]):
                f.write("    " + l + "\n")
            f.write("\n")
        for build, cells, name in (("full", FULL_CELLS, "full client"), ("no-unknown", NOUNK_CELLS, "no-unknown client")):
            for d, p in RPC_JOBS:
                present = [c for c in cells if any(k[0] == build and k[2] == c and k[3] == d and k[4] == p for k in cases)]
                present += sorted({k[2] for k in cases if k[0] == build and k[3] == d and k[4] == p} - set(present))
                if not present:
                    continue
                cols = [(t, k) for t in transports for k in ks
                        if any((build, t, c, d, p, k) in cases for c in present)]
                for clock, lab in (("cpu", "client CPU"), ("wall", "wall")):
                    f.write(f"## {name}, direction {d} ({p}): {JOB_TEXT[d]} -- {lab}\n\n")
                    f.write("| cell | " + " | ".join(f"{t} k={k}" for t, k in cols) + " |\n")
                    f.write("|---|" + "---:|" * len(cols) + "\n")
                    for c in present:
                        ent = []
                        for t, k in cols:
                            x = cases.get((build, t, c, d, p, k))
                            if not x:
                                ent.append("")
                                continue
                            st = stats(x[clock])
                            one = " †" if min(x["it"]) == 1 else ""
                            ent.append(f"{us(st['median'])} [{us(st['min'])}-{us(st['max'])}]{one}")
                        f.write(f"| {c} | " + " | ".join(ent) + " |\n")
                    f.write("\n")
    return len(cases)


def main(run):
    n = codec(run)
    m = rpc(run)
    print(f"opt_summary: {n or 0} codec cases, {m or 0} rpc cases -> summary-codec.tsv, variants-codec.tsv, "
          f"summary-rpc.tsv, tables-codec.md, tables-rpc.md")
    return 0 if (n or m) else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1]))
