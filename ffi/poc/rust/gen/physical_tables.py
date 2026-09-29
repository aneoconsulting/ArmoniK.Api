#!/usr/bin/env python3
"""Tables of a gen/physical_probe.sh session: absolute per-call client CPU (ms, median
[p10-p90]) per cell and workload, wall beside it, the in-process gap of each cell to A per pass
and its range, the spread of Df - A and A2 - A over the spread passes (the "notable"
threshold), getrusage context switches and minor faults per call, the thread counts per class,
and the attribution pass (per-thread-class CPU, allocations, split cells' per-chunk host work).
No ratios.

   gen/physical_tables.py SESSION_DIR               one session
   gen/physical_tables.py DIR1 DIR2 ...              + a side-by-side of the sessions' absolutes
"""
import glob, json, os, re, statistics, sys
from collections import defaultdict

CORDER = ["A", "A2", "Df", "Df-chan", "Cf", "Cf-cb", "C", "C-cb", "Cf-split", "Cf-cb-split", "C-split", "C-cb-split"]
WORDER = ["d/16MiB k1", "d/4MiB k1", "d/16MiB k8", "d/4MiB k8", "c/P5.4 k1", "c/P5.4 k8", "c/P5.3 k1", "c/P5.3 k8"]


def short(c):
    return re.sub(r"-(retain|drop|nounk)$", "", c)


def q(xs, f):
    xs = sorted(xs)
    return xs[min(len(xs) - 1, int(f * len(xs)))]


def ms(x):
    return f"{x / 1e6:.2f}"


def mpp(xs):
    return f"{ms(statistics.median(xs))} [{ms(q(xs, 0.1))}-{ms(q(xs, 0.9))}]" if xs else "--"


def corder(c):
    return (CORDER.index(c) if c in CORDER else 50, c)


def worder(w):
    return (WORDER.index(w) if w in WORDER else 50, w)


class Session:
    def __init__(self, d):
        self.d = d
        self.hdr = open(os.path.join(d, "header.txt")).read().rstrip()
        m = re.search(r"segment (\w+)", self.hdr)
        self.seg = m.group(1) if m else os.path.basename(d)
        m = re.search(r"AK_HOST_WORKERS=(\S+) .*AK_CORE_WORKERS=(\S+) ", self.hdr)
        sw = re.search(r"-> (\d+) tokio workers", self.hdr)
        self.label = f"{self.seg} (host {m.group(1) if m else '?'}, core {m.group(2) if m else '?'}, server {sw.group(1) if sw else '?'})"
        # (kind, pass, src, workload, cell) -> per-call CPU ns; wall, ru per round/sample
        self.cpu = defaultdict(list)
        self.wall = defaultdict(list)
        self.ru = defaultdict(lambda: defaultdict(list))
        self.one_batch = defaultdict(int)
        self.threads = {}
        self.attr = defaultdict(lambda: defaultdict(list))
        for p in sorted(glob.glob(os.path.join(d, "*.jsonl"))):
            b = os.path.basename(p)[:-6]
            m = re.match(r"(spread|main)-(probe|grid)-(\d+)$", b)
            if m:
                kind, src, n = m.group(1), m.group(2), int(m.group(3))
            elif b == "attr":
                kind, src, n = "attr", "probe", 0
            else:
                continue
            for l in open(p):
                if l.startswith("# threads by class"):
                    self.threads[b] = l[2:].strip()
                if l.startswith("#"):
                    continue
                r = json.loads(l)
                if src == "probe":
                    w, cell = f"d/{r['size']} k1", r["cell"]
                    key = (kind, n, src, w, short(cell))
                    if kind == "attr":
                        for k, v in r.items():
                            if isinstance(v, (int, float)) and k not in ("round", "calls"):
                                self.attr[(w, short(cell))][k].append(v)
                        continue
                    self.cpu[key].extend(r["cpu_calls"])
                    self.wall[key].append(r["wall_ns"])
                    for x in ("ru_nvcsw", "ru_nivcsw", "ru_minflt"):
                        self.ru[key][x].append(r[x])
                else:
                    w = f"{r['dir']}/{r['payload']} k{r['inflight']}"
                    key = (kind, n, src, w, short(r["cell"]))
                    it = r["iters"]
                    self.cpu[key].append(r["cpu_ns"] / it)
                    self.wall[key].append(r["wall_ns"] / it)
                    if r["batches"] == 1:
                        self.one_batch[key] += 1
                    for x in ("ru_nvcsw", "ru_nivcsw", "ru_minflt"):
                        if x in r:
                            self.ru[key][x].append(r[x] / it)

    def keys(self, kind, src):
        ws = sorted({k[3] for k in self.cpu if k[0] == kind and k[2] == src}, key=worder)
        cs = sorted({k[4] for k in self.cpu if k[0] == kind and k[2] == src}, key=corder)
        ps = sorted({k[1] for k in self.cpu if k[0] == kind and k[2] == src})
        return ws, cs, ps

    def pooled(self, kind, src, w, c, what="cpu"):
        m = self.cpu if what == "cpu" else self.wall
        out = []
        for k, v in m.items():
            if k[0] == kind and k[2] == src and k[3] == w and k[4] == c:
                out.extend(v)
        return out

    def pass_med(self, kind, src, n, w, c):
        xs = self.cpu.get((kind, n, src, w, c))
        return statistics.median(xs) if xs else None


def gap_row(s, kind, src, w, c, ref="A"):
    _, _, ps = s.keys(kind, src)
    g = []
    for n in ps:
        a, b = s.pass_med(kind, src, n, w, ref), s.pass_med(kind, src, n, w, c)
        if a is not None and b is not None:
            g.append(b - a)
    return g


def fmt_gaps(g):
    if not g:
        return "--", "--"
    return " ".join(f"{x / 1e6:+.2f}" for x in g), f"{(max(g) - min(g)) / 1e6:.2f}"


def one(s):
    print(s.hdr)
    if "ABORTED" in s.hdr:
        print("\n**The session ABORTED: no table is formed from it.**")
        return
    print(f"\nSession: {s.label}. Every figure below is per call, in ms (client process CPU unless marked wall).")

    # 1. the spread
    print("\n## 1. Run-to-run spread of an in-process gap (the notable threshold)\n")
    print("Per pass: the median of each cell's per-call CPU in that pass's process; the gap is the difference of two such medians in the same process. Range = max - min over the passes.\n")
    print("| source | workload | A per pass | Df - A per pass (spread passes) | range | A2 - A per pass | range | Df - A per pass (main passes) | range |")
    print("|---|---|---|---|---:|---|---:|---|---:|")
    for src in ("probe", "grid"):
        ws = sorted(set(s.keys("spread", src)[0]) | set(s.keys("main", src)[0]), key=worder)
        for w in ws:
            _, _, ps = s.keys("spread", src)
            a = [s.pass_med("spread", src, n, w, "A") for n in ps]
            a = " ".join(ms(x) for x in a if x is not None) or "--"
            g1, r1 = fmt_gaps(gap_row(s, "spread", src, w, "Df"))
            g2, r2 = fmt_gaps(gap_row(s, "spread", src, w, "A2"))
            g3, r3 = fmt_gaps(gap_row(s, "main", src, w, "Df"))
            print(f"| {src} | {w} | {a} | {g1} | {r1} | {g2} | {r2} | {g3} | {r3} |")

    # 2. absolutes, main passes
    for src, title in (("probe", "probe (bin stream_probe, block order): direction d, k = 1"),
                       ("grid", "grid (benches/rpc_suite on criterion): c/P5.4 and d, k = 1 and 8")):
        ws, cs, ps = s.keys("main", src)
        if not ws:
            continue
        print(f"\n## 2{'a' if src == 'probe' else 'b'}. Main passes, {title}: CPU median [p10-p90] pooled over {len(ps)} passes\n")
        print("| cell | " + " | ".join(ws) + " |")
        print("|---|" + "---:|" * len(ws))
        for c in cs:
            print(f"| {c} | " + " | ".join(mpp(s.pooled("main", src, w, c)) for w in ws) + " |")
        print(f"\nWall per call, median [p10-p90] ({'per round, calls back to back' if src == 'probe' else 'per sample / calls; at k = 8 the batch wall over 8 calls'}):\n")
        print("| cell | " + " | ".join(ws) + " |")
        print("|---|" + "---:|" * len(ws))
        for c in cs:
            print(f"| {c} | " + " | ".join(mpp(s.pooled("main", src, w, c, "wall")) for w in ws) + " |")
        print("\nIn-process gap to A per pass (cell median - A median, same process), and its range over the passes:\n")
        print("| cell | " + " | ".join(ws) + " |")
        print("|---|" + "---|" * len(ws))
        for c in cs:
            if c == "A":
                continue
            cells = []
            for w in ws:
                g, r = fmt_gaps(gap_row(s, "main", src, w, c))
                cells.append(f"{g} (range {r})" if g != "--" else "--")
            print(f"| {c} | " + " | ".join(cells) + " |")
        print("\nContext switches (voluntary / involuntary) and minor faults per call, medians over rounds or samples, main passes:\n")
        print("| cell | " + " | ".join(ws) + " |")
        print("|---|" + "---:|" * len(ws))
        for c in cs:
            cells = []
            for w in ws:
                rr = defaultdict(list)
                for k, v in s.ru.items():
                    if k[0] == "main" and k[2] == src and k[3] == w and k[4] == c:
                        for x, ys in v.items():
                            rr[x].extend(ys)
                cells.append(" / ".join(f"{statistics.median(rr[x]):.1f}" for x in ("ru_nvcsw", "ru_nivcsw", "ru_minflt")) if rr else "--")
            print(f"| {c} | " + " | ".join(cells) + " |")
        if src == "grid":
            ob = defaultdict(int)
            for k, v in s.one_batch.items():
                ob[(k[3], k[4])] += v
            if ob:
                print("\nGrid samples holding ONE batch (criterion sized one iteration per sample), main passes: " +
                      ", ".join(f"{c} {w}: {n}" for (w, c), n in sorted(ob.items(), key=lambda x: (worder(x[0][0]), corder(x[0][1])))))

    # 3. threads
    print("\n## 3. Client threads by class after the warm-up (each probe process)\n")
    for b, t in sorted(s.threads.items()):
        print(f"- `{b}`: {t}")

    # 4. attribution
    if s.attr:
        print("\n## 4. Attribution pass (not timed: /proc reads around every round, allocation shim loaded): medians over rounds, per call\n")
        cls = ("main", "caller", "cell-rt", "core-rt", "other")
        print("| workload | cell | CPU ms by class " + "/".join(cls) + " | vcs by class | ics by class | minflt (process) | allocs | allocs >= 1 MiB | host encode us/chunk | send CPU us/chunk | send wall us/chunk |")
        print("|---|---|---|---|---|---:|---:|---:|---:|---:|---:|")
        med = lambda v, k, f=1.0: (f"{statistics.median(v[k]) * f:.2f}" if k in v and v[k] else "-")
        for (w, c) in sorted(s.attr, key=lambda x: (worder(x[0]), corder(x[1]))):
            v = s.attr[(w, c)]
            n = 8 if "16MiB" in w else 2
            print(f"| {w} | {c} | " + "/".join(med(v, f"cpu_{k}", 1e-6) for k in cls) + " | " +
                  "/".join(med(v, f"vcs_{k}") for k in cls) + " | " + "/".join(med(v, f"ics_{k}") for k in cls) + " | " +
                  f"{med(v, 'ru_minflt')} | {med(v, 'allocs')} | {med(v, 'allocs_big')} | " +
                  f"{med(v, 'host_encode_cpu', 1e-3 / n)} | {med(v, 'host_send_cpu', 1e-3 / n)} | {med(v, 'host_send_wall', 1e-3 / n)} |")


def side_by_side(ss):
    print("\n# Sessions side by side: main passes, CPU median [p10-p90] per call, ms (each session its own processes and server launch; absolutes, no ratio)\n")
    for src in ("probe", "grid"):
        ws = sorted({w for s in ss for w in s.keys("main", src)[0]}, key=worder)
        cs = sorted({c for s in ss for c in s.keys("main", src)[1]}, key=corder)
        for w in ws:
            print(f"\n## {src}, {w}\n")
            print("| cell | " + " | ".join(s.label for s in ss) + " |")
            print("|---|" + "---:|" * len(ss))
            for c in cs:
                print(f"| {c} | " + " | ".join(mpp(s.pooled("main", src, w, c)) for s in ss) + " |")
            print("| spread: A per pass | " + " | ".join(" ".join(ms(x) for x in (s.pass_med("main", src, n, w, "A") for n in s.keys("main", src)[2]) if x is not None) or "--" for s in ss) + " |")


ss = [Session(d) for d in sys.argv[1:]]
for s in ss:
    one(s)
if len(ss) > 1:
    side_by_side(ss)
