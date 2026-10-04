#!/usr/bin/env python3
"""D19 tables from gen/d19_bench.sh's runs (container instrumentation).

Per cell (content set x length in UTF-16 code units) and arm: the per-process median ns per
string of the three processes, shown as the range across processes [min..max] and their
middle value, and GB/s of UTF-16 input from that middle value; then the new/pre-D19 ratio
(each process's median of per-round ratios, range across processes).
"""
import re
import sys
from pathlib import Path

out = Path(sys.argv[1])
runs = sorted(out.glob("run-*.txt"))
cells = {}  # (set, n) -> arm -> [ns medians per run]
ratios = {}  # (set, n, label) -> [ratio per run]
outb = {}
order = []
for r in runs:
    for line in r.read_text().splitlines():
        if line.startswith("#") or " | " not in line:
            continue
        f = [x.strip() for x in line.split("|")]
        key = (f[0], int(f[1]))
        if key not in order:
            order.append(key)
        if f[2] == "":
            ratios.setdefault((f[0], int(f[1]), f[4]), []).append(float(f[5]))
            continue
        outb[key] = int(f[3])
        m = re.match(r"([\d.]+) \[", f[5])
        cells.setdefault(key, {}).setdefault(f[4], []).append(float(m.group(1)))

arms = ["pre-D19-core", "old-scalar", "new-simdutf", "export"]
print("# D19: the UTF-16 transcoder alone, old against new (CONTAINER INSTRUMENTATION)")
print()
print((out / "header.txt").read_text().strip().replace("\n", "  \n"))
print()
print(f"{len(runs)} processes, one after the other; in each, every cell runs 7 rounds of the four arms "
      "(order rotated per round, ~40 ms per measurement) over a pool of distinct strings "
      "(16 to 1024 strings, at most 4 MiB of UTF-16), capacity 3 bytes per unit (no grow). "
      "ns/string: the middle of the three per-process medians, [min..max] across processes. "
      "GB/s: UTF-16 input bytes (2 per unit) over the middle ns. Arms: `pre-D19-core` = the "
      "commit before D19's own ak_tc_utf16 (loaded RTLD_LOCAL in the same process); "
      "`old-scalar` = the D19 core's ak_tc_utf16_scalar (the same source, kept as the oracle); "
      "`new` = the D19 ak_tc_utf16 (simdutf); `export` = ak_utf16_to_utf8.")
print()
print("Content sets: ascii (U+0000-007F), latin1 (U+0000-00FF uniform), bmp-wide (CJK mostly, "
      "3 B/unit), astral (pairs, emoji mostly), mixed (the four classes uniformly at random per "
      "code point: a branch-hostile mix for any scalar loop), lone-surrogates (the mixed set with "
      "1-4 lone surrogates at start/middle/end: the replacing path, labelled extra).")
print()
print("| set | units | out B/str | pre-D19 core ns | old-scalar ns | new (simdutf) ns | export ns | pre-D19 GB/s | new GB/s | new/pre-D19 per process |")
print("|---|---:|---:|---:|---:|---:|---:|---:|---:|---|")


def mid(v):
    s = sorted(v)
    return s[len(s) // 2]


def cell(v):
    if not v:
        return "-"
    if len(v) == 1:
        return f"{v[0]:.1f}"
    return f"{mid(v):.1f} [{min(v):.1f}..{max(v):.1f}]"


for key in order:
    c = cells[key]
    n = key[1]
    gb = lambda arm: f"{2 * n / mid(c[arm]):.2f}" if arm in c else "-"
    rr = ratios.get((key[0], key[1], "new/pre-D19-core (median of per-round ratios)"), [])
    rtxt = f"{min(rr):.3f}..{max(rr):.3f}" if rr else "-"
    print(f"| {key[0]} | {n} | {outb[key]} | {cell(c.get('pre-D19-core', []))} | {cell(c.get('old-scalar', []))} | "
          f"{cell(c.get('new-simdutf', []))} | {cell(c.get('export', []))} | {gb('pre-D19-core')} | {gb('new-simdutf')} | {rtxt} |")
print()
print("Control (`pre-D19-core` / `old-scalar`, per-process medians of per-round ratios, range across processes):")
print()
print("| set | units | pre-D19 core / old-scalar |")
print("|---|---:|---|")
for key in order:
    rr = ratios.get((key[0], key[1], "pre-D19-core/old-scalar (control)"), [])
    if rr:
        print(f"| {key[0]} | {key[1]} | {min(rr):.3f}..{max(rr):.3f} |")
