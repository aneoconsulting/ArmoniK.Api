#!/usr/bin/env python3
"""Did the GROUP-skip fix (C24) move any timing?

The fix changes `ak::Dec::skip`'s SIGNATURE, so every emission site in the generated
decoder changed and every decode function was recompiled. The skip itself is on the
unknown-field path, which no payload in `shapes.json` reaches -- but "it should not move"
is a prediction, and a prediction is not a measurement.

This compares a fresh `bench_a17_shared` run against the PUBLISHED log, row by row, on
the lo end of each arm's ratio to `pb`. It is an ACROSS-BUILD comparison and therefore
carries R4's drift bar from `logs/cpp/drift.log`: the same source built twice with a
neutral layout perturbation moves a ratio by up to 0.240, so anything under that says
only that nothing larger than the bar was found.

Usage: c24_timing.py <published log> <fresh log>
"""
import re
import statistics
import sys

ROW = re.compile(r"^(P\d\.\d)\s+(enc|dec)\s+(\S+)\s+([\d.]+)\s+([\d.]+)\s+"
                 r"([\d.]+)\s+([\d.]+)\s+([\d.]+)\s+[\d.,]+$")
DRIFT = 0.240


def rows(path):
    out = {}
    for line in open(path):
        m = ROW.match(line)
        if m:
            out[(m.group(1), m.group(2), m.group(3))] = (float(m.group(6)),
                                                         float(m.group(7)))
    return out


def main():
    a, b = rows(sys.argv[1]), rows(sys.argv[2])
    keys = sorted(set(a) & set(b))
    print("# C24: did the GROUP-skip fix move any timing?")
    print("#   published  %s" % sys.argv[1])
    print("#   fresh      %s" % sys.argv[2])
    print("#   compared   %d ratio rows present in both" % len(keys))
    print("#   bar        R4's across-build drift, %.3f (logs/cpp/drift.log)" % DRIFT)
    print("#   NOT a within-process comparison. Two binaries, two runs: the bar applies.")
    print()
    d = sorted(((abs(b[k][0] - a[k][0]), k, a[k][0], b[k][0]) for k in keys),
               reverse=True)
    print("the 15 largest moves, on the lo end of each arm's ratio to pb")
    for mv, k, x, y in d[:15]:
        print("  %.3f  %-6s %-4s %-12s  published %.3f  fresh %.3f%s"
              % (mv, k[0], k[1], k[2], x, y, "   OVER THE BAR" if mv > DRIFT else ""))
    over = [x for x in d if x[0] > DRIFT]
    print()
    print("worst move %.3f, median %.4f, %d row(s) over the %.3f bar"
          % (d[0][0], statistics.median(x[0] for x in d), len(over), DRIFT))
    print()
    if over:
        print("VERDICT: %d row(s) moved by more than the across-build bar. Named above."
              % len(over))
        return 1
    print("VERDICT: nothing moved by more than the across-build drift bar, so the")
    print("published timing tables stand and are NOT re-taken. The largest movers are")
    print("P5.2-P5.4, whose own `pb` denominator has an 11-32 percent round-to-round")
    print("spread (STATE.md), so they are where a re-run moves most whatever changed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
