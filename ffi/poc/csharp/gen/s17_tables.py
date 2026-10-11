#!/usr/bin/env python3
"""gen/s17_tables.py DIR: s17 part 2's tables, today's (eager) encode render against the lazy
nested render (gen/s17_grid.sh). CONTAINER INSTRUMENTATION; absolute times only.
DIR/full/*.jsonl and DIR/small/*.jsonl -> <set>/table.md and <set>/reference.md: per core-ffi
encode row (payload, content, direction, mode) and string path (E0, E1R:128), process CPU per op:
the median over every round of every rep per render, the lazy - eager difference, and the
per-rep medians (each rep is one BDN host process per variant; within a rep the four variants
ran back to back on the same machine state, so the difference is also given per rep: min and
max of lazy_r - eager_r over the reps)."""
import glob, json, os, re, statistics, sys

D = sys.argv[1]
PATHS = (('e0', 'E0'), ('e1r128', 'E1R:128'))


def fmt(x):
    if x is None:
        return '-'
    return ('%.4g' % x) if abs(x) >= 1 else ('%.3f' % x)


def load(gd):
    by = {}
    for f in glob.glob(os.path.join(gd, '*-full-r*.jsonl')):
        m = re.match(r'(eager|lazy)-(e0|e1r128)-full-r(\d+)\.jsonl$', os.path.basename(f))
        if not m:
            continue
        rend, path, rep = m.group(1), m.group(2), int(m.group(3))
        for l in open(f):
            if not l.startswith('{'):
                continue
            x = json.loads(l)
            if x.get('round', 0) < 1 or 'cpu_ns' not in x or x.get('arm') != 'core-ffi':
                continue
            k = (x['payload'], x['content'], x['dir'], x['unknown_mode'])
            by.setdefault(k, {}).setdefault((rend, path), {}).setdefault(rep, []).append(x['cpu_ns'] / x['iters'])
    return by


def key(k):
    m = re.match(r'P(\d+)\.(\d+)$', k[0])
    return ((0, int(m.group(1)), int(m.group(2))) if m else (1, k[0], 0), k[1], k[2], k[3])


def unit(v):
    """ns below 10 us, us above."""
    return (v, 'ns') if v < 10000 else (v / 1000, 'us')


for s in ('full', 'small'):
    gd = os.path.join(D, s)
    if not glob.glob(os.path.join(gd, '*.jsonl')):
        continue
    by = load(gd)
    reps = sorted({r for v in by.values() for e in v.values() for r in e})
    t = ['# s17 %s set: core-ffi encode rows, eager (today) vs lazy nested render, process CPU per op, median over every round of every rep (%d reps = %d BDN host processes per variant, interleaved). Lazy - eager: the difference of those medians, and [min, max] of the per-rep differences. ns, or us where marked. CONTAINER INSTRUMENTATION.' % (s, len(reps), len(reps)), '']
    rf = ['# s17 %s set reference: per variant, median [min-max of per-rep medians] (per-rep medians in rep order), ns per op' % s, '']
    for pk, pn in PATHS:
        t += ['## %s' % pn, '', '| payload | content | dir | mode | eager | lazy | lazy - eager | per-rep diff [min, max] | unit |', '|---|---|---|---|---:|---:|---:|---|---|']
        rf += ['## %s' % pn, '', '| payload | content | dir | mode | eager | lazy |', '|---|---|---|---|---|---|']
        for k in sorted(by, key=key):
            e, z = by[k].get(('eager', pk)), by[k].get(('lazy', pk))
            if not e or not z:
                continue
            me = statistics.median([y for a in e.values() for y in a])
            mz = statistics.median([y for a in z.values() for y in a])
            pr = [statistics.median(z[r]) - statistics.median(e[r]) for r in reps if r in e and r in z]
            sc = 1 if me < 10000 else 1000
            u = 'ns' if sc == 1 else 'us'
            t.append('| %s | %s | %s | %s | %s | %s | %s | [%s, %s] | %s |' % (k[0], k[1], k[2], k[3], fmt(me / sc), fmt(mz / sc), fmt((mz - me) / sc), fmt(min(pr) / sc), fmt(max(pr) / sc), u))
            cell = lambda d: '%s [%s-%s] (%s)' % (fmt(statistics.median([y for a in d.values() for y in a])), fmt(min(statistics.median(a) for a in d.values())), fmt(max(statistics.median(a) for a in d.values())), ' '.join(fmt(statistics.median(d[r])) for r in sorted(d)))
            rf.append('| %s | %s | %s | %s | %s | %s |' % (k[0], k[1], k[2], k[3], cell(e), cell(z)))
        t.append('')
        rf.append('')
    open(os.path.join(gd, 'table.md'), 'w').write('\n'.join(t) + '\n')
    open(os.path.join(gd, 'reference.md'), 'w').write('\n'.join(rf) + '\n')
    print('\n'.join(t))
