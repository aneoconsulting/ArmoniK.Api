#!/usr/bin/env python3
"""gen/opt_ab_rpc_table.py DIR: per RPC case and variant of gen/opt_ab_rpc.sh, per call (us):
task-clock, process CPU, wall (median over every round of every rep [min-max], per-rep medians
of task-clock); allocated bytes per call; minflt per call. Container instrumentation."""
import glob, json, os, re, statistics, sys
D = sys.argv[1]
by = {}
for f in sorted(glob.glob(os.path.join(D, '*.jsonl'))):
    m = re.match(r'(.+)-r(\d+)\.jsonl$', os.path.basename(f))
    if not m:
        continue
    var, rep = m.group(1), int(m.group(2))
    for l in open(f):
        if l.startswith('{'):
            r = json.loads(l)
            if r.get('round', 0) >= 1:
                k = (r['cell'], r['dir'], r['payload'], r['inflight'])
                e = by.setdefault(k, {}).setdefault(var, {'tc': {}, 'pc': [], 'wl': [], 'mem': [], 'mf': []})
                e['tc'].setdefault(rep, []).append(r['cpu_ns'] / r['iters'] / 1000)
                e['pc'].append(r['proc_cpu_ns'] / r['iters'] / 1000)
                e['wl'].append(r['wall_ns'] / r['iters'] / 1000)
                e['mf'].append(r['minflt'] / r['iters'])
                if 'mem_alloc_bytes_per_op' in r:
                    e['mem'].append(r['mem_alloc_bytes_per_op'])
fm = lambda x: f'{x:.4g}'
md = lambda v: f'{fm(statistics.median(v))} [{fm(min(v))}-{fm(max(v))}]'
print('| cell | dir | payload | k | variant | task-clock us/call (per-rep medians) | process CPU | wall | B/call | minflt/call |')
print('|---|---|---|---:|---|---|---|---|---:|---:|')
for k in sorted(by):
    for v in sorted(by[k]):
        e = by[k][v]
        tc = [x for rr in e['tc'].values() for x in rr]
        reps = ' '.join(fm(statistics.median(e['tc'][r])) for r in sorted(e['tc']))
        mem = fm(statistics.median(e['mem'])) if e['mem'] else 'n/a'
        print(f'| {k[0]} | {k[1]} | {k[2]} | {k[3]} | {v} | {md(tc)} ({reps}) | {md(e["pc"])} | {md(e["wl"])} | {mem} | {statistics.median(e["mf"]):.3g} |')
