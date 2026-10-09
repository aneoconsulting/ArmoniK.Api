#!/usr/bin/env python3
"""gen/s9_tables.py DIR: step 9b's tables from gen/opt_ab.sh's jsonl (AK_BDN_S9=1, decode-read).
table.md: ONLY times, one row per payload, the six arms' process CPU per op (median over every
round of every rep). reference.md: the same cells with [min-max] (per-rep medians), bytes
allocated per op, gen0 per op, minflt per op. Container instrumentation."""
import glob, json, os, re, statistics, sys
D = sys.argv[1]
by = {}
for f in sorted(glob.glob(os.path.join(D, '*-r*.jsonl'))):
    rep = int(re.search(r'-r(\d+)\.jsonl$', f).group(1))
    for l in open(f):
        if not l.startswith('{'): continue
        r = json.loads(l)
        if r.get('round', 0) < 1 or 'cpu_ns' not in r or r['dir'] != 'decode-read': continue
        k = r['payload'] + ('' if r['content'] in ('ascii', 'corpus') else '/' + r['content'])
        a = r['arm'] + ':' + r['unknown_mode']
        e = by.setdefault(k, {}).setdefault(a, {'cpu': {}, 'mem': [], 'g0': [], 'mf': []})
        e['cpu'].setdefault(rep, []).append(r['cpu_ns'] / r['iters'] / 1000)
        e['mf'].append(r['minflt'] / r['iters'])
        if 'mem_alloc_bytes_per_op' in r: e['mem'].append(r['mem_alloc_bytes_per_op'])
        if r.get('mem_ops'): e['g0'].append(1000.0 * r['mem_gen'][0] / r['mem_ops'])
cols = [('incumbent-prod:default', 'Google.Protobuf, unknown retained'), ('incumbent-prod:discard', 'Google.Protobuf, unknown discarded'),
        ('core-ffi:retain', 'core push, retain'), ('core-ffi:drop', 'core push, drop'),
        ('core-ffi-pull:retain', 'core pull, retain'), ('core-ffi-pull:drop', 'core pull, drop')]
def key(k):
    m = re.match(r'P(\d+)\.(\d+)(/(\w+))?$', k)
    if m: return (0, int(m.group(1)), int(m.group(2)), ['', 'latin1', 'wide'].index(m.group(4) or ''))
    return (1, k, 0, 0)
rows = sorted(by, key=key)
fmt = lambda x: ('%.4g' % x) if x >= 1 else ('%.3f' % x)
allv = lambda e: [x for v in e['cpu'].values() for x in v]
t = ['# Decode-read, process CPU per op (us), median', '', '| payload | ' + ' | '.join(c[1] for c in cols) + ' |', '|---|' + '---:|' * len(cols)]
r2 = ['# Step 9b reference: process CPU per op (us) median [min-max] (per-rep medians); B/op; gen0 per 1k ops; minflt/op. CONTAINER INSTRUMENTATION.', '',
      '| payload | ' + ' | '.join(c[1] for c in cols) + ' |', '|---|' + '---|' * len(cols)]
for k in rows:
    cs, cr = [], []
    for a, _ in cols:
        e = by[k].get(a)
        if not e: cs.append('-'); cr.append('-'); continue
        v = allv(e)
        cs.append(fmt(statistics.median(v)))
        cr.append('%s [%s-%s] (%s); %s B; %s; %s' % (fmt(statistics.median(v)), fmt(min(v)), fmt(max(v)), ' '.join(fmt(statistics.median(e['cpu'][r])) for r in sorted(e['cpu'])),
                  ('%.4g' % statistics.median(e['mem'])) if e['mem'] else '-', ('%.3g' % statistics.median(e['g0'])) if e['g0'] else '-', '%.3g' % statistics.median(e['mf'])))
    t.append('| %s | ' % k + ' | '.join(cs) + ' |'); r2.append('| %s | ' % k + ' | '.join(cr) + ' |')
open(os.path.join(D, 'table.md'), 'w').write('\n'.join(t) + '\n')
open(os.path.join(D, 'reference.md'), 'w').write('\n'.join(r2) + '\n')
print('\n'.join(t))
