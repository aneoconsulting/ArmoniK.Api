#!/usr/bin/env bash
# R4's across-build control, which this slice did not have and needed.
#
# Two binaries, the SAME source, differing only by a semantically neutral layout
# perturbation. Every row is identical code, so every difference between the two is
# across-build drift and nothing else. Three published conclusions in the first version of
# this slice were smaller than the drift observed between two of its logs, so any claim
# formed by comparing two binaries now carries this bar or is withdrawn.
set -u
cd "$(dirname "$0")/.." || exit 2
R=${1:-9}
a=$(mktemp); b=$(mktemp)
./build/bench_a17_shared  "$R" > "$a" 2>&1
./build/bench_a17_perturb "$R" > "$b" 2>&1
python3 - "$a" "$b" <<'PY'
import re, sys
ARMS = ("pb", "pb-det", "pb-arena", "memcpy", "native", "ffi", "ffi-valtc",
        "ffi-zeroed", "ffi-nobat", "ffi-hosttc", "groupfill", "groupfill-ind")
def rows(p):
    """Only the final report table. The delta tables above it have the same leading
    columns and a different meaning, and parsing both is how the first run of this script
    reported a 78-point ratio drift that was really a units mismatch."""
    out = {}
    seen_header = False
    for ln in open(p):
        if ln.startswith("payload dir  arm") or (" arm " in ln and "ns/op med" in ln):
            seen_header = True
            continue
        if not seen_header:
            continue
        m = re.match(r'^(P\d\.\d)\s+(enc|dec|gfill)\s+(\S+)\s+([\d.]+)\s+([\d.]+)\s+([\d.]+)\s+([\d.]+)', ln)
        if m and m.group(3) in ARMS:
            out[(m.group(1), m.group(2), m.group(3))] = (float(m.group(5)), float(m.group(6)), float(m.group(7)))
    return out
A, B = rows(sys.argv[1]), rows(sys.argv[2])
keys = sorted(set(A) & set(B))
print("%-6s %-5s %-12s %12s %12s %9s %9s %9s" % ("payload","dir","arm","min A ns","min B ns","abs drift","ratio A","ratio B"))
worst_abs = worst_ratio = 0.0
for k in keys:
    a_min, a_lo, a_hi = A[k]; b_min, b_lo, b_hi = B[k]
    d = abs(b_min - a_min) / a_min * 100.0
    dr = abs(((b_lo + b_hi) / 2) - ((a_lo + a_hi) / 2))
    worst_abs = max(worst_abs, d); worst_ratio = max(worst_ratio, dr)
    print("%-6s %-5s %-12s %12.1f %12.1f %8.2f%% %9.3f %9.3f"
          % (k[0], k[1], k[2], a_min, b_min, d, (a_lo+a_hi)/2, (b_lo+b_hi)/2))
print()
print("ACROSS-BUILD CONTROL: worst absolute drift %.2f%%, worst RATIO drift %.3f" % (worst_abs, worst_ratio))
print("Any claim formed by comparing two BINARIES must be larger than the ratio figure.")
print("A ratio formed INSIDE one binary is not exposed to this and is the reason R4 says so.")
PY
rm -f "$a" "$b"
