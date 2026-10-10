"""reset-on-entry: the tables of gen/roe_bench.sh's launches. Absolute times only: for each
(input, mode, op), the explicit-reset path and the reset-on-entry path, each as the median over
launches of the per-launch median, with the range of the per-launch medians; then the
isolated reset calls. Container instrumentation.

usage: python3 gen/roe_tables.py BENCH_DIR > tables.md
"""
import glob, re, statistics, sys

UNIT = {"ns": 1.0, "us": 1e3, "ms": 1e6}


def ns(s):
    v, u = s.split()
    return float(v) * UNIT[u]


def fmt(x):
    if x >= 1e6:
        return "%.3f ms" % (x / 1e6)
    if x >= 1e3:
        return "%.2f us" % (x / 1e3)
    return "%.1f ns" % x


d = sys.argv[1]
runs = sorted(glob.glob(d + "/launch-*.txt"))
data = {}  # (input, mode, arm) -> [medians per launch]
order = []
for f in runs:
    for line in open(f):
        if line.startswith("#") or line.startswith("input"):
            continue
        m = re.match(r"(\S+)\s+(\S+)\s+(.+?)\s{2,}(\S+ \S+)\s+(\S+ \S+)\s+(\S+ \S+)\s*$", line)
        if not m:
            continue
        k = (m.group(1), m.group(2), m.group(3).strip())
        if k not in data:
            data[k] = []
            order.append(k)
        data[k].append(ns(m.group(4)))


def cell(k):
    v = data.get(k)
    if not v:
        return "-"
    return "%s (%s - %s)" % (fmt(statistics.median(v)), fmt(min(v)), fmt(max(v)))


print("# reset-on-entry: explicit reset vs reset on entry (container instrumentation, absolute times)\n")
print("%d launches (`%s`); each cell: the median over launches of the per-launch median, (the range of the per-launch medians). One process per launch, both paths interleaved in it on the reset-on-entry core (the explicit path with the core's switch off on its contexts)." % (len(runs), d))
print()
rows = []
for k in order:
    if k[2].endswith(" explicit"):
        rows.append((k[0], k[1], k[2][: -len(" explicit")]))
for op, title in [("dec-rd", "FSM decode-read"), ("enc-rb", "encode, end state reused-buffer (= transport-ready-core's op)"), ("enc-tt", "encode, end state transport-ready-tonic")]:
    print("## %s\n" % title)
    print("| input | mode | explicit reset | reset on entry |")
    print("|---|---|---|---|")
    for (i, m, o) in rows:
        if o != op:
            continue
        print("| %s | %s | %s | %s |" % (i, m, cell((i, m, o + " explicit")), cell((i, m, o + " roe"))))
    print()
print("## the reset calls alone\n")
print("| call | context | time |")
print("|---|---|---|")
enc = [x for k in order if k[2] == "ak_enc_reset" for x in data[k]]
if enc:
    print("| ak_enc_reset | every encode context of the rows above (%d cases x launches) | median %s (%s - %s) |" % (len([k for k in order if k[2] == 'ak_enc_reset']), fmt(statistics.median(enc)), fmt(min(enc)), fmt(max(enc))))
for k in order:
    if k[2].startswith("ak_dec_reset"):
        print("| ak_dec_reset_%s (the binding's options) | %s | %s |" % (k[0].strip("[]"), k[1], cell(k)))
