#!/usr/bin/env python3
"""FIX-PLAN WP5 step 2's byte audit: the C++ arms BEFORE the port against the C++ arms
AFTER it, row by row, on every corpus row the old harness could root (the seven shapes
roots). "Any byte change is a rule fix: list it with its corpus row."

BEFORE is the retired harness (`src/corpus.cpp` at the commit named on the command line,
native = `cpp_core.py` over `rust_core.py`'s walk, ffi = the old binding over the shapes
core), built out of tree; AFTER is `build/corpus_all_a17` (native = `cpp_native.py`, ffi =
the shared binding over the corpus core). Each row runs in its own process under a
timeout on both sides. For each (row, arm) the outcome is compared: refused vs accepted,
the refusal code, and the re-encoding's bytes (by SHA-256 and length: the old
harness prints no hex for a large re-encoding). Glue, no wire rule.

  wp5_bytes.py OLD_BINARY NEW_BINARY
"""
import hashlib
import json
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
FFI = os.path.dirname(os.path.dirname(os.path.dirname(HERE)))
CORPUS = os.path.join(FFI, "corpus", "generated")
ROOTS = ["ListResultsResponse", "ListTasksDetailedResponse", "ListProbeResponse",
         "ListTaskSummaryResponse", "UploadResultDataMessage", "ListMetricsResponse",
         "DualResponse"]


def old_row(binary, rid, row, tmp):
    t = os.path.join(tmp, "t.tsv")
    open(t, "w").write("msg\t%s\t%s\t%s\n" % (rid, row["root"], os.path.join(CORPUS, row["file"])))
    try:
        p = subprocess.run([binary, t], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=10)
    except subprocess.TimeoutExpired:
        return {"native": ("timeout",), "ffi": ("timeout",)}
    out = {}
    hexes = {}
    for line in p.stdout.decode("utf-8", "replace").splitlines():
        f = line.split("\t")
        if f[0] == "R" and len(f) >= 7:
            out[f[2]] = (f[3], int(f[4]))
            hexes[f[2]] = "sha256 %s, %s B" % (f[5], f[6])
    res = {}
    for arm in ("native", "ffi"):
        if arm not in out:
            res[arm] = ("no result (exit %d)" % p.returncode,)
        elif out[arm][0] == "accept":
            res[arm] = ("accept", hexes.get(arm))
        else:
            res[arm] = (out[arm][0], out[arm][1])
    return res


def new_row(binary, row):
    try:
        p = subprocess.run([binary, os.path.join(CORPUS, row["file"]), row["root"]],
                           stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=10)
    except subprocess.TimeoutExpired:
        return {"native": ("timeout",), "ffi": ("timeout",)}
    j = json.loads(p.stdout)
    res = {}
    for arm, key in (("native", "native-drop"), ("ffi", "ffi-drop")):
        r = j[key]
        if r.get("ok"):
            bb = bytes.fromhex(r.get("hex", ""))
            res[arm] = ("accept", "sha256 %s, %d B" % (hashlib.sha256(bb).hexdigest(), len(bb)))
        else:
            res[arm] = ("reject", r.get("err"))
    return res


def main(old_bin, new_bin):
    man = json.load(open(os.path.join(CORPUS, "manifest.json")))["vectors"]
    rows = {k: r for k, r in man.items() if r.get("root") in ROOTS}
    print("# WP5 step 2 byte audit: C++ arms before vs after the port, drop mode")
    print("#   before  %s" % old_bin)
    print("#   after   %s" % new_bin)
    print("#   rows    %d corpus rows rooted at the seven shapes roots (of %d)" % (len(rows), len(man)))
    print()
    same = {"native": 0, "ffi": 0}
    diffs = []
    with tempfile.TemporaryDirectory() as tmp:
        for rid in sorted(rows):
            row = rows[rid]
            a = old_row(old_bin, rid, row, tmp)
            b = new_row(new_bin, row)
            for arm in ("native", "ffi"):
                if a[arm] == b[arm]:
                    same[arm] += 1
                else:
                    diffs.append((rid, arm, row["class"], row["expect"], a[arm], b[arm]))
    for arm in ("native", "ffi"):
        print("  %-6s identical outcome and bytes on %d of %d rows" % (arm, same[arm], len(rows)))
    print()
    print("## rows whose outcome or bytes CHANGED (each a rule fix, or a defect)")
    if not diffs:
        print("  none")
    for rid, arm, cls, exp, a, b in diffs:
        def fmt(x):
            if x[0] == "accept":
                return "accept %s" % x[1]
            return " ".join(str(v) for v in x)
        print("  %-44s %-6s class=%-9s expect=%-6s before: %s" % (rid, arm, cls, exp, fmt(a)))
        print("  %-44s %-6s %-9s        %-6s after:  %s" % ("", "", "", "", fmt(b)))
    print()
    print("changed: %d (row, arm) pairs" % len(diffs))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1], sys.argv[2]))
