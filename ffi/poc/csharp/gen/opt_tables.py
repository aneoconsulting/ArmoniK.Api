#!/usr/bin/env python3
"""Tables of one gen/opt_bench.sh run (container instrumentation, never a result).

    gen/opt_tables.py DIR      reads DIR/codec-*.jsonl and DIR/rpc-*.jsonl, writes DIR/tables.md,
                               DIR/codec.tsv and DIR/rpc.tsv

Per case, every exported BDN actual iteration (round >= 1) gives one per-operation value
(cpu_ns / iters); a table entry is the median over the rounds with [min-max]. Allocation and
GC figures come from the MemoryDiagnoser fields of each case's first row (AK_BDN_MEMORY=1).
Crossing counts are read from the committed count files (gen/counts*.txt, gen/rpc-counts.txt);
none is measured here. No ratio is formed.
"""
import json
import os
import statistics
import sys

DIR = sys.argv[1]
GEN = os.path.dirname(os.path.abspath(__file__))


def rows(path):
    hdr, out = [], []
    for line in open(path):
        if line.startswith('{'):
            d = json.loads(line)
            if d.get('round', 0) >= 1 and 'cpu_ns' in d:
                out.append(d)
        elif line.startswith('#'):
            hdr.append(line.rstrip('\n'))
    return hdr, out


def counts(path):
    m = {}
    if not os.path.exists(path):
        return m
    for line in open(path):
        if line.startswith('#') or '|' not in line:
            continue
        key, mid = line.split('|')[0].strip(), line.split('|')[1].strip()
        f = mid.split()
        m[key] = '/'.join(f[i] for i in (1, 3, 5, 7))   # fwd/rev/grow/reset
    return m


def fmt(x):
    if x >= 1000:
        return f'{x:.0f}'
    if x >= 100:
        return f'{x:.1f}'
    if x >= 10:
        return f'{x:.2f}'
    return f'{x:.3f}'


def stat(vals):
    return statistics.median(vals), min(vals), max(vals)


def cell_txt(vals, flag_first=True):
    med, lo, hi = stat(vals)
    s = f'{fmt(med)} [{fmt(lo)}-{fmt(hi)}]'
    # flag: the spread exceeds 25 % of the median, or round 1 is the slowest by > 10 %
    marks = ''
    if med > 0 and (hi - lo) / med > 0.25:
        marks += ' *'
    if flag_first and len(vals) > 1 and vals[0] == hi and med > 0 and vals[0] / med > 1.10:
        marks += ' ^'
    return s + marks


md = []
P = lambda *a: md.append(' '.join(str(x) for x in a))
hdr = open(os.path.join(DIR, 'header.txt')).read().rstrip('\n').splitlines()
timing = open(os.path.join(DIR, 'timing.txt')).read().rstrip('\n').splitlines() if os.path.exists(os.path.join(DIR, 'timing.txt')) else []

P('# C# baseline (optimisation pass): core grid, absolute times per case')
P('')
for h in hdr:
    if h.startswith('# VOID'):
        P('**' + h[2:] + '**')
        P('')
P('CONTAINER INSTRUMENTATION, not a result (README 1.1); NOT GATED (gen/gate.sh not run; every codec '
  'process ran Cases.Verify, byte identity of every encode arm and variant and every U-* row, before '
  'timing; every RPC call checked). One `gen/opt_bench.sh` run, one launch, short BenchmarkDotNet settings '
  '(header below). No ratio is formed here.')
P('')
P('Notation: each entry is the **median over the BDN actual iterations (rounds) of one case** with '
  '**[min-max]** across them; one round = one BDN iteration; per-op value = that iteration\'s CPU (or '
  'wall) / its operations. ` *` = (max - min) > 25 % of the median; ` ^` = round 1 is the slowest and > 10 % '
  'above the median (a warm-up tail). Every case ran in its own child process (BDN default toolchain). '
  'Allocation and GC: BenchmarkDotNet MemoryDiagnoser, one extra workload iteration per case after the '
  'actual stage, GC.GetTotalAllocatedBytes(precise) over every thread / its operations; Gen0/1/2 = '
  'collections during that iteration per 1,000 operations (with the operation count beside it). Crossing '
  'counts fwd/rev/grow/reset per operation are from the committed count files (`gen/counts.txt`, '
  '`gen/counts-nounk.txt`, `gen/rpc-counts.txt`, gated; none measured in this run).')
P('')
P('```')
for h in hdr:
    P(h)
for t in timing:
    P(t)
P('```')
P('')

# ---------------------------------------------------------------- codec
cod = []
for b in ('full', 'nounk'):
    p = os.path.join(DIR, f'codec-{b}.jsonl')
    if os.path.exists(p):
        cod += rows(p)[1]
if cod:
    cf = counts(os.path.join(GEN, 'counts.txt'))
    cn = counts(os.path.join(GEN, 'counts-nounk.txt'))
    UNITS = [('incumbent-prod', 'full'), ('core-ffi', 'retain'), ('core-ffi', 'no-unknown'),
             ('host-gen', 'retain'), ('host-gen', 'no-unknown')]
    UL = ['incumbent-prod', 'core-ffi retain', 'core-ffi no-unknown', 'host-gen retain', 'host-gen no-unknown']

    def unit_of(d):
        if d['arm'] == 'incumbent-prod':
            return ('incumbent-prod', 'full')
        return (d['arm'], d['unknown_mode'])

    by = {}
    for d in cod:
        k = (d['payload'], d['content'], d['dir'], unit_of(d))
        by.setdefault(k, []).append(d)
    for v in by.values():
        v.sort(key=lambda d: d['round'])
    cases = sorted({(p, c, dr) for (p, c, dr, _) in by},
                   key=lambda t: (t[0].startswith('U-'), t[0], {'ascii': 0, 'latin1': 1, 'wide': 2}.get(t[1], 3)))
    modes = sorted({(d['arm'], d['unknown_mode'], d['build']) for d in cod})
    P('## Codec suite')
    P('')
    P('Units and the labels the rows carry (arm, unknown_mode, build): ' + '; '.join(f'`{a} {m} {b}`' for a, m, b in modes) + '. '
      '`incumbent-prod` runs in the full build only (section 4.0). Payloads: SHAPES.md; `/latin1`, `/wide` = content sets on P2.2; '
      'U-* = corpus rows (content `corpus`). encode = `encode-transport-hot` (end state ii, the form the arm hands its transport, one hot graph); '
      'decode = `decode-read` (decode, then read every field).')
    P('')
    for dr, title in (('encode-transport-hot', 'encode (encode-transport-hot)'), ('decode-read', 'decode-read')):
        for metric, mt in (('cpu', 'process CPU per op, microseconds'), ('wall', 'wall per op, microseconds')):
            P(f'### {title}: {mt}')
            P('')
            P('| payload | ' + ' | '.join(UL) + ' |')
            P('|---|' + '---:|' * len(UL))
            for (p, c, d2) in cases:
                if d2 != dr:
                    continue
                name = p + ('' if c in ('ascii', 'corpus') else '/' + c)
                cells = []
                for u in UNITS:
                    rs = by.get((p, c, dr, u))
                    if not rs:
                        cells.append('')
                        continue
                    key = 'cpu_ns' if metric == 'cpu' else 'wall_ns'
                    cells.append(cell_txt([r[key] / r['iters'] / 1000.0 for r in rs]))
                P(f'| {name} | ' + ' | '.join(cells) + ' |')
            P('')
        P(f'### {title}: allocated bytes per op, and Gen0/Gen1/Gen2 per 1,000 ops (MemoryDiagnoser iteration: ops)')
        P('')
        P('| payload | ' + ' | '.join(UL) + ' |')
        P('|---|' + '---:|' * len(UL))
        for (p, c, d2) in cases:
            if d2 != dr:
                continue
            name = p + ('' if c in ('ascii', 'corpus') else '/' + c)
            cells = []
            for u in UNITS:
                rs = by.get((p, c, dr, u))
                if not rs:
                    cells.append('')
                    continue
                r = rs[0]
                if 'mem_alloc_bytes_per_op' not in r:
                    cells.append('n/a')
                    continue
                ops = r['mem_ops']
                g = r['mem_gen']
                gg = '/'.join(f'{1000.0 * x / ops:.2f}'.rstrip('0').rstrip('.') if x else '0' for x in g)
                cells.append(f'{r["mem_alloc_bytes_per_op"]:,} B; {gg} ({ops} ops)')
            P(f'| {name} | ' + ' | '.join(cells) + ' |')
        P('')
        P(f'### {title}: core-ffi crossings per op, fwd/rev/grow/reset (committed count files)')
        P('')
        P('| payload | core-ffi retain (gen/counts.txt) | core-ffi no-unknown (gen/counts-nounk.txt) |')
        P('|---|---:|---:|')
        for (p, c, d2) in cases:
            if d2 != dr:
                continue
            name = p + ('' if c in ('ascii', 'corpus') else '/' + c)
            a = cf.get(f'{p} {c} core-ffi {dr} retain', 'MISSING')
            b = cn.get(f'{p} {c} core-ffi {dr} no-unknown', 'MISSING')
            P(f'| {name} | {a} | {b} |')
        P('')
    with open(os.path.join(DIR, 'codec.tsv'), 'w') as t:
        t.write('payload\tcontent\tdir\tarm\tunknown_mode\tbuild\trounds\tcpu_ns_per_op_median\tcpu_min\tcpu_max\twall_ns_per_op_median\twall_min\twall_max\titers_per_round\tminflt_per_op_median\talloc_bytes_per_op\tgen0\tgen1\tgen2\tmem_ops\tcpu_ns_per_op_by_round\n')
        for (p, c, dr, u), rs in sorted(by.items()):
            cpu = [r['cpu_ns'] / r['iters'] for r in rs]
            wall = [r['wall_ns'] / r['iters'] for r in rs]
            mf = [r['minflt'] / r['iters'] for r in rs]
            r0 = rs[0]
            g = r0.get('mem_gen', ['', '', ''])
            t.write('\t'.join(str(x) for x in [p, c, dr, r0['arm'], r0['unknown_mode'], r0['build'], len(rs),
                    f'{statistics.median(cpu):.1f}', f'{min(cpu):.1f}', f'{max(cpu):.1f}',
                    f'{statistics.median(wall):.1f}', f'{min(wall):.1f}', f'{max(wall):.1f}',
                    ','.join(str(r['iters']) for r in rs), f'{statistics.median(mf):.3f}',
                    r0.get('mem_alloc_bytes_per_op', ''), g[0], g[1], g[2], r0.get('mem_ops', ''),
                    ','.join(f'{x:.1f}' for x in cpu)]) + '\n')

# ---------------------------------------------------------------- rpc
rpc = []
for name in ('rpc-stock.jsonl', 'rpc-h2-batch.jsonl', 'rpc-stock.alloc-pinned.jsonl'):
    p = os.path.join(DIR, name)
    if os.path.exists(p):
        rpc += rows(p)[1]
if rpc:
    rc = counts(os.path.join(GEN, 'rpc-counts.txt'))
    by = {}
    for d in rpc:
        k = (d['cell'], d['dir'], d['payload'], d['inflight'], d['h2'], d['alloc'])
        by.setdefault(k, []).append(d)
    for v in by.values():
        v.sort(key=lambda d: d['round'])
    DIRS = [('a+read', 'P2.2'), ('b', 'P2.2'), ('c', 'P5.4'), ('d', 'stream-16MiB')]
    ORDER = ['A', 'B', 'Bf', 'C-retain', 'Cf-retain', 'E-retain', 'Ef-retain']
    P('## RPC grid')
    P('')
    P('Per call: the client process\'s CPU (`cpu_ns` = perf task-clock of the whole process, CAMPAIGN req 21 as amended; '
      '`proc_cpu_ns` = CLOCK_PROCESS_CPUTIME_ID beside it) or wall of one BDN iteration divided by its calls, in **microseconds**; '
      'k = calls in flight (one invocation = k calls). Transport `armonik` (cell A: packages/csharp GrpcChannelFactory; the core cells: the core\'s '
      'shipped client configuration), TCP 127.0.0.1, the one Rust server. Direction a+read has an empty request, so the framed cells\' a+read row '
      'is the reference cell\'s (B, C-retain, E-retain), run in the same BDN run under its own name, as the runner does. '
      'Crossings: per call from `gen/rpc-counts.txt` (A has none: Grpc.Net). Rows: stock h2 and the default allocator unless labelled.')
    P('')
    for dr, pl in DIRS:
        P(f'### direction {dr} ({pl})')
        P('')
        P('| cell | h2 / alloc | k | task-clock CPU / call | process CPU / call | wall / call | alloc B / call; Gen0/1/2 per 1k calls | minflt / call | crossings fwd/rev/grow/reset | client softirq ticks |')
        P('|---|---|---:|---:|---:|---:|---:|---:|---:|---:|')
        ks = sorted(by.items(), key=lambda kv: (ORDER.index(kv[0][0]) if kv[0][0] in ORDER else 99, kv[0][4], kv[0][5], kv[0][3]))
        for (cell, d2, p2, k, h2, al), rs in ks:
            if d2 != dr:
                continue
            tc = [r['cpu_ns'] / r['iters'] / 1000.0 for r in rs]
            pc = [r['proc_cpu_ns'] / r['iters'] / 1000.0 for r in rs]
            wl = [r['wall_ns'] / r['iters'] / 1000.0 for r in rs]
            mf = statistics.median([r['minflt'] / r['iters'] for r in rs])
            r0 = rs[0]
            if 'mem_alloc_bytes_per_op' in r0:
                ops = r0['mem_ops']
                gg = '/'.join(f'{1000.0 * x / ops:.1f}'.rstrip('0').rstrip('.') if x else '0' for x in r0['mem_gen'])
                mem = f'{r0["mem_alloc_bytes_per_op"]:,}; {gg} ({ops})'
            else:
                mem = 'n/a'
            mode = r0['unknown_mode']
            cnt = rc.get(f'RPC {cell} {dr} {p2} {mode}', '-' if cell == 'A' else 'MISSING')
            lab = ('stock' if h2 == 'stock' else '**h2-batch**') + ' / ' + ('default' if al == 'default' else '**pinned**')
            sirq = r0.get('client_softirq_ticks', '')
            P(f'| {cell} | {lab} | {k} | {cell_txt(tc)} | {cell_txt(pc)} | {cell_txt(wl)} | {mem} | {mf:.1f} | {cnt} | {sirq} |')
        P('')
    with open(os.path.join(DIR, 'rpc.tsv'), 'w') as t:
        t.write('cell\tdir\tpayload\tinflight\th2\talloc\trounds\ttaskclock_ns_per_call_median\tmin\tmax\tproc_cpu_ns_per_call_median\twall_ns_per_call_median\twall_min\twall_max\tcalls_per_round\tminflt_per_call_median\talloc_bytes_per_call\tgen0\tgen1\tgen2\tmem_calls\tclient_softirq_ticks\ttaskclock_ns_per_call_by_round\n')
        for (cell, dr, p2, k, h2, al), rs in sorted(by.items(), key=lambda kv: str(kv[0])):
            tc = [r['cpu_ns'] / r['iters'] for r in rs]
            pc = [r['proc_cpu_ns'] / r['iters'] for r in rs]
            wl = [r['wall_ns'] / r['iters'] for r in rs]
            mf = [r['minflt'] / r['iters'] for r in rs]
            r0 = rs[0]
            g = r0.get('mem_gen', ['', '', ''])
            t.write('\t'.join(str(x) for x in [cell, dr, p2, k, h2, al, len(rs),
                    f'{statistics.median(tc):.0f}', f'{min(tc):.0f}', f'{max(tc):.0f}', f'{statistics.median(pc):.0f}',
                    f'{statistics.median(wl):.0f}', f'{min(wl):.0f}', f'{max(wl):.0f}',
                    ','.join(str(r['iters']) for r in rs), f'{statistics.median(mf):.2f}',
                    r0.get('mem_alloc_bytes_per_op', ''), g[0], g[1], g[2], r0.get('mem_ops', ''),
                    r0.get('client_softirq_ticks', ''), ','.join(f'{x:.0f}' for x in tc)]) + '\n')

open(os.path.join(DIR, 'tables.md'), 'w').write('\n'.join(md) + '\n')
print(f'{len(cod)} codec rows, {len(rpc)} rpc rows -> {DIR}/tables.md')
