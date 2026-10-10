#!/usr/bin/env python3
"""gen/s16_tables.py DIR: s16's reset-on-entry tables from DIR/bench/*.tsv (gen/s16_bench.sh).
CONTAINER INSTRUMENTATION; absolute times only (ns per op of process CPU).
Per case (payload, content, direction, mode): E (the explicit-reset path, the default binding,
the core's switch off on its contexts), R (the binding rendered with reset_on_entry=True) and
E2 (E again, the A/A control), each the median over every process of the process's median over
its rounds; and the paired differences R - E and E2 - E (per process: the median over rounds of
the same round's difference; then the median and min-max over processes). The resets each path
makes per call. -> bench/table.md, bench/reference.md (per-process values), bench/resets.md."""
import collections, glob, os, re, statistics, sys
D = os.path.join(sys.argv[1], 'bench')
f1 = lambda x: '-' if x is None else ('%.1f' % x if abs(x) < 1000 else '%.0f' % x)
by = collections.defaultdict(lambda: collections.defaultdict(dict))   # case -> proc -> path -> {round: ns}
rpc = {}
for f in sorted(glob.glob(os.path.join(D, '*.tsv'))):
    if f.endswith('.resets.tsv'):
        continue
    proc = os.path.basename(f)[:-4]
    for l in open(f):
        p = l.rstrip('\n').split('\t')
        if p[0] == 'payload':
            continue
        case = tuple(p[:4])
        by[case][proc].setdefault(p[4], {})[int(p[5])] = float(p[6])
        rpc[(case, p[4])] = p[8]
med = statistics.median
order = {'encode-core-hot': 0, 'decode-read': 1}


def key(c):
    m = re.match(r'P(\d+)\.(\d+)$', c[0])
    return (order[c[2]], (0, int(m.group(1)), int(m.group(2))) if m else (1, c[0], 0), c[1], c[3])


t = ['# s16 reset-on-entry, one process per run, both paths interleaved (gen/s16_bench.sh): ns per op, process CPU. E = explicit resets (the default binding; the core switch off on its contexts), R = reset-on-entry binding, E2 = E again (A/A). Values: the median over processes of each process\'s median over its rounds; R-E and E2-E: the median [min-max] over processes of each process\'s median paired (same-round) difference. CONTAINER INSTRUMENTATION.', '',
     '| dir | payload | content | mode | processes | E | R | E2 | R - E | E2 - E | resets/call E, R |', '|---|---|---|---|---:|---:|---:|---:|---|---|---|']
ref = ['# s16 reference: per process (sorted by name), median E / R / E2 and the paired median R - E, ns per op', '']
for c in sorted(by, key=key):
    procs = by[c]
    me = {q: [med(procs[pr][q].values()) for pr in sorted(procs) if q in procs[pr]] for q in ('E', 'R', 'E2')}
    def pdiff(a):
        out = []
        for pr in sorted(procs):
            x, y = procs[pr].get(a), procs[pr].get('E')
            if not x or not y:
                continue
            out.append(med([x[r] - y[r] for r in x if r in y]))
        return out
    dr, da = pdiff('R'), pdiff('E2')
    rng = lambda v: '%s [%s, %s]' % (f1(med(v)), f1(min(v)), f1(max(v))) if v else '-'
    t.append('| %s | %s | %s | %s | %d | %s | %s | %s | %s | %s | %s, %s |' % (c[2], c[0], c[1], c[3], len(procs),
             f1(med(me['E'])), f1(med(me['R'])), f1(med(me['E2'])), rng(dr), rng(da), rpc.get((c, 'E'), '?'), rpc.get((c, 'R'), '?')))
    ref.append('- %s %s %s %s: ' % (c[2], c[0], c[1], c[3]) + '; '.join(
        '%s %s/%s/%s (%s)' % (pr, f1(med(procs[pr]['E'].values())), f1(med(procs[pr]['R'].values())), f1(med(procs[pr]['E2'].values())),
                              f1(med([procs[pr]['R'][r] - procs[pr]['E'][r] for r in procs[pr]['R'] if r in procs[pr]['E']])))
        for pr in sorted(procs) if all(q in procs[pr] for q in ('E', 'R', 'E2'))))
open(os.path.join(D, 'table.md'), 'w').write('\n'.join(t) + '\n')
open(os.path.join(D, 'reference.md'), 'w').write('\n'.join(ref) + '\n')
print('\n'.join(t))
# the reset calls alone
rs = collections.defaultdict(lambda: collections.defaultdict(list))   # (call, opts) -> proc -> [ns]
for f in sorted(glob.glob(os.path.join(D, '*.resets.tsv'))):
    proc = os.path.basename(f)[:-len('.tsv.resets.tsv')]
    for l in open(f):
        p = l.rstrip('\n').split('\t')
        if p[0] == 'call':
            continue
        rs[(p[0], p[2])][proc].append(float(p[4]))
r = ['# s16: one reset call alone from C# (the default P/Invoke binding), back to back, ns per call: the median over processes of each process\'s median over its rounds [min-max of the process medians]. CONTAINER INSTRUMENTATION.', '',
     '| call | options | processes | ns per call |', '|---|---|---:|---|']
for (call, opts) in sorted(rs):
    v = [med(x) for x in rs[(call, opts)].values()]
    r.append('| %s | %s | %d | %.1f [%.1f, %.1f] |' % (call, opts, len(v), med(v), min(v), max(v)))
open(os.path.join(D, 'resets.md'), 'w').write('\n'.join(r) + '\n')
print('\n'.join(r))
