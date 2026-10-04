#!/usr/bin/env python3
"""gen/opt_ab_table.py DIR: per case and variant of gen/opt_ab.sh, process CPU per op (us):
median over every round of every rep [min-max], and each rep's median; allocated bytes per op
and minflt per op (median). Container instrumentation."""
import glob, json, os, re, statistics, sys
D = sys.argv[1]
by = {}
for f in sorted(glob.glob(os.path.join(D, '*.jsonl'))):
    m = re.match(r'(.+)-(full|nounk)-r(\d+)\.jsonl$', os.path.basename(f))
    if not m:
        continue
    var, rep = m.group(1), int(m.group(3))
    for l in open(f):
        if l.startswith('{'):
            r = json.loads(l)
            if r.get('round', 0) >= 1 and 'cpu_ns' in r:
                k = (r['payload'] + ('' if r['content'] in ('ascii', 'corpus') else '/' + r['content']), r['dir'], r['arm'], r['unknown_mode'])
                e = by.setdefault(k, {}).setdefault(var, {'cpu': {}, 'mem': [], 'mf': []})
                e['cpu'].setdefault(rep, []).append(r['cpu_ns'] / r['iters'] / 1000)
                e['mf'].append(r['minflt'] / r['iters'])
                if 'mem_alloc_bytes_per_op' in r:
                    e['mem'].append(r['mem_alloc_bytes_per_op'])
vars_ = sorted({v for e in by.values() for v in e})
print('| payload | dir | arm | mode | ' + ' | '.join(f'{v}: CPU us/op median [min-max] (per-rep medians); B/op; minflt/op' for v in vars_) + ' |')
print('|---|---|---|---|' + '---|' * len(vars_))
fm = lambda x: f'{x:.4g}'
for k in sorted(by):
    cells = []
    for v in vars_:
        e = by[k].get(v)
        if not e:
            cells.append('')
            continue
        allv = [x for rr in e['cpu'].values() for x in rr]
        reps = ' '.join(fm(statistics.median(e['cpu'][r])) for r in sorted(e['cpu']))
        mem = fm(statistics.median(e['mem'])) if e['mem'] else 'n/a'
        cells.append(f'{fm(statistics.median(allv))} [{fm(min(allv))}-{fm(max(allv))}] ({reps}); {mem}; {statistics.median(e["mf"]):.3g}')
    print(f'| {k[0]} | {k[1]} | {k[2]} | {k[3]} | ' + ' | '.join(cells) + ' |')
