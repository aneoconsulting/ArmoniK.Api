#!/usr/bin/env python3
"""gen/s8_tables.py DIR: the D21 s8 decode attribution tables from BenchDotNet --decattr's
.tsv files (full-r*.tsv, nounk-r*.tsv). Per row and mode: each arm's process CPU per op (us),
median over every round of every rep [min-max] (per-rep medians), and the split of the
core-ffi decode into buckets (from the medians): parse (core), cross (noop - parse: reverse
calls and the hand-off of groups and runs), build (skip - noop: managed objects, lists and the
read pass, no strings), strings (ref - skip); pull: pparse (core parse into the record stream)
and replay (pull - pparse); valid (parsev - parse: the core's UTF-8 check, not in ref).
Container instrumentation."""
import csv, glob, os, re, statistics, sys
D = sys.argv[1]
data = {}
for f in sorted(glob.glob(os.path.join(D, '*-r*.tsv'))):
    rep = int(re.search(r'-r(\d+)\.tsv$', f).group(1))
    for r in csv.DictReader(open(f), delimiter='\t'):
        k = (r['row'], r['content'], r['mode'])
        e = data.setdefault(k, {}).setdefault(r['arm'], {'cpu': {}, 'alloc': [], 'g0': [], 'mf': [], 'gc': []})
        e['cpu'].setdefault(rep, []).append(float(r['cpu_ns_per_op']) / 1000)
        e['alloc'].append(float(r['alloc_bytes_per_op'])); e['g0'].append(float(r['gen0_per_op'])); e['mf'].append(float(r['minflt_per_op'])); e['gc'].append(float(r.get('gc_pause_ns_per_op') or 'nan') / 1000)
med = lambda v: statistics.median(v)
def allv(e): return [x for v in e['cpu'].values() for x in v]
def m(k, a):
    e = data[k].get(a)
    return med(allv(e)) if e else None
def cell(k, a):
    e = data[k].get(a)
    if not e: return '-'
    v = allv(e)
    return '%.4g [%.4g-%.4g] (%s)' % (med(v), min(v), max(v), ' '.join('%.4g' % med(e['cpu'][r]) for r in sorted(e['cpu'])))
order = lambda k: (0 if k[0].startswith('P') else 1, k[0], ['ascii', 'latin1', 'wide', 'corpus'].index(k[1]), ['retain', 'drop', 'no-unknown'].index(k[2]))
keys = sorted(data, key=order)
arms = ['ref', 'skip', 'noop', 'parse', 'parsev', 'pull', 'pparse', 'touch', 'strs', 'host', 'hskip', 'inc']
out = ['# D21 s8: core-ffi decode attribution, process CPU us per op; one process per build and rep (BenchDotNet --decattr), arms interleaved; median [min-max] over 2 reps x 6 rounds (per-rep medians). CONTAINER INSTRUMENTATION.', '',
       '## 1. Arms', '', '| row | content | mode | ' + ' | '.join(arms) + ' |', '|---|---|---|' + '---|' * len(arms)]
for k in keys: out.append('| %s | %s | %s | ' % k + ' | '.join(cell(k, a) for a in arms) + ' |')
out += ['', '## 2. Split of the core-ffi decode (push, the reference) and of pull, from the medians (us per op; % of ref)', '',
        '| row | content | mode | ref | parse (core) | cross (noop-parse) | build (skip-noop) | strings (ref-skip) | strs alone | GC pause ref / skip | touch (read pass, in build) | valid (parsev-parse) | pull | pparse | replay (pull-pparse) | host | host parse+build (hskip) | host strings (host-hskip) | inc | ref - host | ref - inc |',
        '|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|']
def pc(x, ref): return '%.4g (%.0f %%)' % (x, 100 * x / ref)
for k in keys:
    ref, sk, no, pa = m(k, 'ref'), m(k, 'skip'), m(k, 'noop'), m(k, 'parse')
    pv, pu, pp, to, st, ho, hs, ic = (m(k, a) for a in ('parsev', 'pull', 'pparse', 'touch', 'strs', 'host', 'hskip', 'inc'))
    gcr, gcs = med(data[k]['ref']['gc']), med(data[k]['skip']['gc'])
    out.append('| %s | %s | %s | %.4g | %s | %s | %s | %s | %.4g | %.3g / %.3g | %.4g | %.4g | %s | %.4g | %s | %.4g | %s | %s | %.4g | %+.4g | %+.4g |' % (
        k + (ref, pc(pa, ref), pc(no - pa, ref), pc(sk - no, ref), pc(ref - sk, ref), st, gcr, gcs, to, pv - pa,
             '%.4g' % pu if pu is not None else '-', pp, ('%.4g' % (pu - pp)) if pu is not None else '-', ho,
             ('%.4g' % hs) if hs is not None else '-', ('%.4g' % (ho - hs)) if hs is not None else '-', ic, ref - ho, ref - ic)))
out += ['', '## 3. Allocated bytes, gen0 collections and minor faults per op (medians)', '',
        '| row | content | mode | ' + ' | '.join(a + ' B/op; gen0; minflt; GC pause us' for a in arms) + ' |', '|---|---|---|' + '---|' * len(arms)]
for k in keys:
    cs = []
    for a in arms:
        e = data[k].get(a)
        cs.append('-' if not e else '%.4g; %.3g; %.3g; %.3g' % (med(e['alloc']), med(e['g0']), med(e['mf']), med(e['gc'])))
    out.append('| %s | %s | %s | ' % k + ' | '.join(cs) + ' |')
open(os.path.join(D, 'tables.md'), 'w').write('\n'.join(out) + '\n')
print('\n'.join(out[:3]))
