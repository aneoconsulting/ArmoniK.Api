"""Collapse a multi-process log into the ranges a finding is allowed to quote.

`ffi/README.md` asks for ranges and spreads, never a single digit dressed up as
precision, and `ffi/CLAUDE.md` asks every figure to name its log.  This reads the
log back and prints, per row, the spread of the per-process medians and of the
per-process ratios -- so a number in `STATE.md` is a thing that can be
regenerated from the file beside it rather than a number someone typed.

Usage:  python summarise.py ffi/logs/python/30-mechanism-py3.11.log [...]
"""

import collections
import re
import sys

ROW = re.compile(r"^(\S.*?)\s{2,}(\d+)\s+([\d.]+)\s+([\d.]+)\s+([\d.]+)"
                 r"(?:\s+([\d.]+))?\s*$")


def parse(path):
    """{(block, group, arm): [(median, ratio|None), ...]}"""
    out = collections.defaultdict(list)
    block = group = None
    for ln in open(path):
        if ln.startswith("##########"):
            block = ln.strip("# \n")
            continue
        if ln.startswith("## "):
            group = ln[3:].strip()
            continue
        if ln.lstrip().startswith(("arm ", "#")) or not group:
            continue
        m = ROW.match(ln.rstrip())
        if not m:
            continue
        arm = m.group(1).strip()
        if arm == "arm":
            continue
        out[(block, group, arm)].append(
            (float(m.group(3)), float(m.group(6)) if m.group(6) else None))
    return out


def fmt(lo, hi, w=8, p=2):
    if abs(hi - lo) < 10 ** -p:
        return "%*.*f" % (w * 2 + 3, p, lo)
    return "%*.*f - %*.*f" % (w, p, lo, w, p, hi)


def main(paths):
    for path in paths:
        rows = parse(path)
        blocks = []
        for (b, _, _) in rows:
            if b not in blocks:
                blocks.append(b)
        print("\n=== %s  (%d block%s)" % (path, len(blocks),
                                          "" if len(blocks) == 1 else "s"))
        merged = collections.defaultdict(list)
        for (b, g, a), v in rows.items():
            merged[(g, a)] += v
        seen = []
        for (g, _a) in merged:
            if g not in seen:
                seen.append(g)
        for g in seen:
            print("\n## %s" % g)
            for (gg, a), v in merged.items():
                if gg != g:
                    continue
                meds = [x[0] for x in v]
                rats = [x[1] for x in v if x[1] is not None]
                line = "  %-44s n=%d  %s ns" % (a[:44], len(v),
                                                fmt(min(meds), max(meds)))
                if rats:
                    line += "   ratio %s" % fmt(min(rats), max(rats), 6, 3)
                print(line)


if __name__ == "__main__":
    main(sys.argv[1:])
