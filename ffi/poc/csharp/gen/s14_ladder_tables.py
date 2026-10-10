#!/usr/bin/env python3
"""gen/s14_ladder_tables.py DIR: s14 part 2's ladder tables. CONTAINER INSTRUMENTATION; absolute
times only.
DIR/ladder-sweep/*.tsv (gen/s14_ladder_sweep.sh, process kinds simd / noguard / generic / stub /
stubng, configurations pgo0 / dflt, reps r1..) -> DIR/ladder-sweep/table.md: ns per string
(process CPU per encode of UploadResultDataMessage{upload.session_id = the string}), the median of
the per-process medians, and per process (reference section) with every process's own E0.
Rungs: R0 = E0 (simd processes); R1 = E0 on the tc-measure-generic core; R2 = E1R on the
tc-measure-u16stub core (bytes NOT checked); R2g = R2 with AK_STR_NOGUARD=1; R3 = E1R (simd);
R3g = E1R with AK_STR_NOGUARD=1.
DIR/grid/*.jsonl (gen/s14_grid.sh, variants r0/r1/r2/r3/r3g x pgo0/dflt) -> DIR/grid/table.md:
us per op, median over every round of both reps, plus each rung's step divided by the strings
per encode (census, JOURNAL: P2.2 17,167; P2.4 26,267; U-deep-u-repeated 31)."""
import glob, json, os, re, statistics, sys
D = sys.argv[1]
f2 = lambda x: '-' if x is None else (('%.4g' % x) if abs(x) >= 1 else ('%.3f' % x))
RUNGS = [('R0', 'simd', 'E0'), ('R1', 'generic', 'E0'), ('R2', 'stub', 'E1R'), ('R2g', 'stubng', 'E1R'), ('R3', 'simd', 'E1R'), ('R3g', 'noguard', 'E1R')]
CFGS = [('pgo0', 'DOTNET_TieredPGO=0'), ('dflt', 'default JIT configuration, fast-mode processes')]
# ---------------------------------------------------------------- sweep
sw = os.path.join(D, 'ladder-sweep')
if os.path.isdir(sw):
    by = {}   # (cfg, content, len) -> (kind, path) -> rep -> median
    for f in glob.glob(os.path.join(sw, '*-r*.tsv')):
        m = re.match(r'(\w+)-(\w+)-r(\d+)\.tsv$', os.path.basename(f))
        if not m: continue
        kind, cfg, rep = m.group(1), m.group(2), int(m.group(3))
        v = {}
        for l in open(f):
            p = l.rstrip('\n').split('\t')
            if p[0] == 'content': continue
            v.setdefault((p[0], int(p[1]), p[3]), []).append(float(p[5]))
        for (c, n, path), x in v.items():
            by.setdefault((cfg, c, n), {}).setdefault((kind, path), {})[rep] = statistics.median(x)
    def med(k, kind, path):
        d = by.get(k, {}).get((kind, path))
        return statistics.median(d.values()) if d else None
    t = ['# s14 ladder, one-string sweep: ns per string, median of the per-process medians (n processes per cell in reference.md). CONTAINER INSTRUMENTATION.', '',
         'R0 E0; R1 E0 through the generic transcoder path (identity copy); R2 E1R with the UTF-16 stub (bytes NOT checked); R2g R2 without the guard; R3 E1R (simdutf); R3g R3 without the guard. Steps: R1-R0 the generic path; R2-R1 E1R\'s frame, mark, patch and element calls against E0\'s staging (both through the generic path); R3-R2 the UTF-16 transcoder over the stub; R3-R3g and R2-R2g the guard.', '']
    for cfg, cn in CFGS:
        t += ['## ' + cn, '', '| content | units | ' + ' | '.join(r for r, _, _ in RUNGS) + ' | R3-R0 | R1-R0 | R2-R1 | R3-R2 | R3-R3g | R2-R2g |', '|---|---:|' + '---:|' * (len(RUNGS) + 6)]
        for c in ('ascii', 'latin1'):
            for n in (40, 48):
                k = (cfg, c, n)
                if k not in by: continue
                v = {r: med(k, kind, path) for r, kind, path in RUNGS}
                d = lambda a, b: None if v[a] is None or v[b] is None else v[a] - v[b]
                t.append('| %s | %d | %s | %s |' % (c, n, ' | '.join(f2(v[r]) for r, _, _ in RUNGS), ' | '.join(f2(d(a, b)) for a, b in (('R3', 'R0'), ('R1', 'R0'), ('R2', 'R1'), ('R3', 'R2'), ('R3', 'R3g'), ('R2', 'R2g')))))
        t.append('')
    # in-process differences: every process carries E0 (the R0 code on every core but the generic
    # one, whose E0 is R1) and E1R (R3 on the default and generic cores, R3g / R2 / R2g on the
    # others), so E1R - E0 inside one process is free of the process-to-process drift of both
    def dd(k, kind):
        a, b = by.get(k, {}).get((kind, 'E1R'), {}), by.get(k, {}).get((kind, 'E0'), {})
        x = [a[q] - b[q] for q in a if q in b]
        return statistics.median(x) if x else None
    t += ['## In-process differences (E1R - E0 inside each process, median over processes), ns per string', '',
          'simd: R3-R0. noguard: R3g-R0 (NOGUARD also drops Go\'s end check on E0). stub: R2-R0. stubng: R2g-R0. generic: R3-R1. Derived: R1-R0 = simd - generic; R2-R1 = stub - (R1-R0); R3-R2 = simd - stub; guard = simd - noguard (and stub - stubng).', '']
    for cfg, cn in CFGS:
        t += ['### ' + cn, '', '| content | units | simd R3-R0 | noguard R3g-R0 | generic R3-R1 | stub R2-R0 | stubng R2g-R0 | R1-R0 | R2-R1 | R3-R2 | guard (R3) | guard (R2) |', '|---|---:|' + '---:|' * 10]
        for c in ('ascii', 'latin1'):
            for n in (40, 48):
                k = (cfg, c, n)
                if k not in by: continue
                v = {kd: dd(k, kd) for kd in ('simd', 'noguard', 'generic', 'stub', 'stubng')}
                sub = lambda a, b: None if a is None or b is None else a - b
                r10 = sub(v['simd'], v['generic'])
                t.append('| %s | %d | %s |' % (c, n, ' | '.join(f2(x) for x in (v['simd'], v['noguard'], v['generic'], v['stub'], v['stubng'], r10, sub(v['stub'], r10), sub(v['simd'], v['stub']), sub(v['simd'], v['noguard']), sub(v['stub'], v['stubng'])))))
        t.append('')
    r = ['# s14 ladder sweep reference: per process (r1, r2, ...) median ns per string; E0 and E1R of every process kind (each process carries its own E0)', '']
    KINDS = ['simd', 'noguard', 'generic', 'stub', 'stubng']
    for cfg, cn in CFGS:
        r += ['## ' + cn, '', '| content | units | ' + ' | '.join('%s %s' % (kd, p) for kd in KINDS for p in ('E0', 'E1R')) + ' |', '|---|---:|' + '---|' * (2 * len(KINDS))]
        for c in ('ascii', 'latin1'):
            for n in (40, 48):
                k = (cfg, c, n)
                if k not in by: continue
                cells = []
                for kd in KINDS:
                    for p in ('E0', 'E1R'):
                        d = by[k].get((kd, p), {})
                        cells.append(', '.join(f2(d[x]) for x in sorted(d)))
                r.append('| %s | %d | %s |' % (c, n, ' | '.join(cells)))
        r.append('')
    open(os.path.join(sw, 'table.md'), 'w').write('\n'.join(t) + '\n')
    open(os.path.join(sw, 'reference.md'), 'w').write('\n'.join(r) + '\n')
    print('\n'.join(t))
# ---------------------------------------------------------------- grid
gd = os.path.join(D, 'grid')
NSTR = {'P2.2': 17167, 'P2.4': 26267, 'U-deep-u-repeated': 31}
if os.path.isdir(gd) and glob.glob(os.path.join(gd, '*.jsonl')):
    by = {}
    for f in glob.glob(os.path.join(gd, '*-full-r*.jsonl')):
        m = re.match(r'(.+)-full-r(\d+)\.jsonl$', os.path.basename(f)); var, rep = m.group(1), int(m.group(2))
        for l in open(f):
            if not l.startswith('{'): continue
            x = json.loads(l)
            if x.get('round', 0) < 1 or 'cpu_ns' not in x or x.get('arm') != 'core-ffi': continue
            k = (x['payload'], x['content'], x['unknown_mode'])
            by.setdefault(k, {}).setdefault(var, {}).setdefault(rep, []).append(x['cpu_ns'] / x['iters'] / 1000)
    def key(k):
        m = re.match(r'P(\d+)\.(\d+)$', k[0])
        return ((0, int(m.group(1)), int(m.group(2))) if m else (1, k[0], 0), k[1], k[2])
    rows = sorted(by, key=key)
    R = ['r0', 'r1', 'r2', 'r3', 'r3g']
    t = ['# s14 ladder grid: encode-core-hot, core-ffi, process CPU per op (us), median over every round of both reps; per-string steps in ns = step / strings per encode. CONTAINER INSTRUMENTATION.', '',
         'r0 E0; r1 E0 on the generic transcoder path; r2 E1R with the UTF-16 stub (bytes NOT checked); r3 E1R; r3g E1R without the guard.', '']
    rf = ['# s14 ladder grid reference: median [min-max] (per-rep medians), us per op', '']
    for cfg, cn in CFGS:
        t += ['## ' + cn, '', '| payload | content | mode | strings | R0 | R1 | R2 | R3 | R3g | R3-R0 ns/str | R1-R0 | R2-R1 | R3-R2 | R3-R3g |', '|---|---|---|---:|' + '---:|' * 10]
        rf += ['## ' + cn, '', '| payload | content | mode | ' + ' | '.join(R) + ' |', '|---|---|---|' + '---|' * len(R)]
        for k in rows:
            v, cr = {}, []
            for q in R:
                e = by[k].get('%s-%s' % (q, cfg))
                if not e: v[q] = None; cr.append('-'); continue
                a = [y for z in e.values() for y in z]
                v[q] = statistics.median(a)
                cr.append('%s [%s-%s] (%s)' % (f2(v[q]), f2(min(a)), f2(max(a)), ' '.join(f2(statistics.median(e[p])) for p in sorted(e))))
            ns = NSTR.get(k[0])
            ps = lambda a, b: None if v[a] is None or v[b] is None or not ns else (v[a] - v[b]) * 1000 / ns
            t.append('| %s | %s | %s | %s | %s | %s |' % (k[0], k[1], k[2], ns or '?', ' | '.join(f2(v[q]) for q in R), ' | '.join(f2(ps(a, b)) for a, b in (('r3', 'r0'), ('r1', 'r0'), ('r2', 'r1'), ('r3', 'r2'), ('r3', 'r3g')))))
            rf.append('| %s | %s | %s | %s |' % (k[0], k[1], k[2], ' | '.join(cr)))
        t.append(''); rf.append('')
    open(os.path.join(gd, 'table.md'), 'w').write('\n'.join(t) + '\n')
    open(os.path.join(gd, 'reference.md'), 'w').write('\n'.join(rf) + '\n')
    print('\n'.join(t))
