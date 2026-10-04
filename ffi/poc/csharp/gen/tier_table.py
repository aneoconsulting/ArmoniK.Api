#!/usr/bin/env python3
"""gen/tier_table.py DIR: per case and variant of gen/tier_check.sh, process CPU per op (us),
median [min-max] over the 6 rounds, and the rounds in order. Container instrumentation."""
import json, os, statistics, sys
D = sys.argv[1]
V = ['1cpu', '1cpu-delay0', '1cpu-tc0', '2cpu', '1cpu-grouped', '2cpu-grouped', '2cpu-w10x100', '2cpu-w10x40', '2cpu-cpus23', '2cpu-delay0', '2cpu-w25x40']
V = [v for v in V if os.path.exists(os.path.join(D, v + '.jsonl'))]
d = {}
for v in V:
    for l in open(os.path.join(D, v + '.jsonl')):
        if l.startswith('{'):
            r = json.loads(l)
            if r.get('round', 0) >= 1 and 'cpu_ns' in r:
                d.setdefault((r['payload'], r['dir'], r['arm']), {}).setdefault(v, []).append(r['cpu_ns'] / r['iters'] / 1000)
print('# tier check: process CPU per op, microseconds, median [min-max] over 6 BDN rounds, one case per child process unless grouped')
print()
print('CONTAINER INSTRUMENTATION. Variants: see header.txt (cpus, toolchain, warm-up count x iteration ms, environment).')
print()
for part in (V[:6], V[6:]):
    if not part:
        continue
    print('| payload | dir | arm | ' + ' | '.join(part) + ' |')
    print('|---|---|---|' + '---:|' * len(part))
    for k in sorted(d):
        c = []
        for v in part:
            x = d[k].get(v)
            c.append(f'{statistics.median(x):.3f} [{min(x):.3f}-{max(x):.3f}]' if x else '')
        print(f'| {k[0]} | {k[1]} | {k[2]} | ' + ' | '.join(c) + ' |')
    print()
print('## rounds in order (process CPU per op, us)')
print()
for k in sorted(d):
    print(f'- {k[0]} {k[1]} {k[2]}: ' + '; '.join(f'{v} ' + ' '.join(f'{x:.3g}' for x in d[k][v]) for v in V if v in d[k]))
