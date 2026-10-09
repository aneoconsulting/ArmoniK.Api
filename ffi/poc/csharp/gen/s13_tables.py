#!/usr/bin/env python3
"""gen/s13_tables.py DIR: s13's tables (scalar UTF-16 transcoders). CONTAINER INSTRUMENTATION.
DIR/sweep/*.tsv (gen/s13_sweep.sh) -> DIR/sweep/table.md: ns per string (process CPU per encode of
a one-string message), median of the per-rep medians, and reference.md (each rep's median and
the in-process E0 control of every process).
DIR/grid/*.jsonl (gen/opt_ab.sh, variants <path>-<core>) -> DIR/grid/table.md: us per op, median
over every round of both reps; reference.md: [min-max], per-rep medians, B/op."""
import glob, json, os, re, statistics, sys
D = sys.argv[1]
CORES = [('target-core', 'simdutf (default)'), ('target-core-tcnaive', 'scalar naive'), ('target-core-tcword', 'scalar word')]
f2 = lambda x: ('%.4g' % x) if x >= 1 else ('%.3f' % x)
# ---------------------------------------------------------------- sweep
sw = os.path.join(D, 'sweep')
if os.path.isdir(sw):
    by = {}   # (content,len,bytes) -> (core,path) -> rep -> [cpu]
    for f in glob.glob(os.path.join(sw, '*-r*.tsv')):
        m = re.match(r'(.+)-r(\d+)\.tsv$', os.path.basename(f)); core, rep = m.group(1), int(m.group(2))
        for l in open(f):
            p = l.rstrip('\n').split('\t')
            if p[0] == 'content': continue
            by.setdefault((p[0], int(p[1]), int(p[2])), {}).setdefault((core, p[3]), {}).setdefault(rep, []).append(float(p[5]))
    order = {'ascii': 0, 'latin1': 1, 'wide': 2, 'astral': 3}
    rows = sorted(by, key=lambda k: (order.get(k[0], 9), k[1]))
    def med(c, k, path): 
        d = by[k].get((c, path)); 
        return None if not d else statistics.median([statistics.median(v) for v in d.values()])
    t = ['# s13 sweep: ns per string (process CPU per encode of UploadResultDataMessage{upload.session_id = the string}), median of the two processes\' medians; DOTNET_TieredPGO=0 (see header.txt)', '',
         '| content | chars | UTF-8 bytes | E0 | E1R, simdutf | E1R, scalar naive | E1R, scalar word |', '|---|---:|---:|---:|---:|---:|---:|']
    r = ['# s13 sweep reference: per process (rep) median [ns]; E0 is each process\'s in-process control', '',
         '| content | chars | ' + ' | '.join('%s E0 (r1, r2) | %s E1R (r1, r2)' % (n, n) for _, n in CORES) + ' |', '|---|---:|' + '---|' * (2 * len(CORES))]
    for k in rows:
        t.append('| %s | %d | %d | %s | %s |' % (k[0], k[1], k[2], f2(med('target-core', k, 'E0')), ' | '.join(f2(med(c, k, 'E1R')) for c, _ in CORES)))
        cells = []
        for c, _ in CORES:
            for path in ('E0', 'E1R'):
                d = by[k].get((c, path), {})
                cells.append(', '.join(f2(statistics.median(d[x])) for x in sorted(d)))
        r.append('| %s | %d | %s |' % (k[0], k[1], ' | '.join(cells)))
    open(os.path.join(sw, 'table.md'), 'w').write('\n'.join(t) + '\n')
    open(os.path.join(sw, 'reference.md'), 'w').write('\n'.join(r) + '\n')
    print('\n'.join(t))
# ---------------------------------------------------------------- grid
gd = os.path.join(D, 'grid')
if os.path.isdir(gd) and glob.glob(os.path.join(gd, '*.jsonl')):
    by = {}
    for f in glob.glob(os.path.join(gd, '*-full-r*.jsonl')):
        m = re.match(r'(.+)-full-r(\d+)\.jsonl$', os.path.basename(f)); var, rep = m.group(1), int(m.group(2))
        for l in open(f):
            if not l.startswith('{'): continue
            x = json.loads(l)
            if x.get('round', 0) < 1 or 'cpu_ns' not in x: continue
            k = (x['payload'] + ('' if x['content'] in ('ascii', 'corpus') else '/' + x['content']), x['unknown_mode'])
            e = by.setdefault(k, {}).setdefault(var, {'cpu': {}, 'mem': []})
            e['cpu'].setdefault(rep, []).append(x['cpu_ns'] / x['iters'] / 1000)
            if 'mem_alloc_bytes_per_op' in x: e['mem'].append(x['mem_alloc_bytes_per_op'])
    VARS = [(p + '-' + c, '%s, %s' % (P, n)) for c, n in (('simd', 'simdutf'), ('naive', 'scalar naive'), ('word', 'scalar word')) for p, P in (('e0', 'E0'), ('e1r', 'E1R'), ('e1r128', 'E1R:128'))]
    def key(k):
        m = re.match(r'P(\d+)\.(\d+)(/(\w+))?$', k[0])
        return ((0, int(m.group(1)), int(m.group(2)), m.group(4) or '') if m else (1, k[0], 0, ''), k[1])
    rows = sorted(by, key=key)
    t = ['# s13 grid: encode-core-hot, core-ffi, process CPU per op (us), median over every round of both reps', '',
         '| payload | mode | ' + ' | '.join(n for _, n in VARS) + ' |', '|---|---|' + '---:|' * len(VARS)]
    r = ['# s13 grid reference: median [min-max] (per-rep medians); B/op', '', '| payload | mode | ' + ' | '.join(n for _, n in VARS) + ' |', '|---|---|' + '---|' * len(VARS)]
    for k in rows:
        cs, cr = [], []
        for v, _ in VARS:
            e = by[k].get(v)
            if not e: cs.append('-'); cr.append('-'); continue
            a = [y for z in e['cpu'].values() for y in z]
            cs.append(f2(statistics.median(a)))
            cr.append('%s [%s-%s] (%s); %s B' % (f2(statistics.median(a)), f2(min(a)), f2(max(a)), ' '.join(f2(statistics.median(e['cpu'][q])) for q in sorted(e['cpu'])), ('%.4g' % statistics.median(e['mem'])) if e['mem'] else '-'))
        t.append('| %s | %s | %s |' % (k[0], k[1], ' | '.join(cs))); r.append('| %s | %s | %s |' % (k[0], k[1], ' | '.join(cr)))
    open(os.path.join(gd, 'table.md'), 'w').write('\n'.join(t) + '\n')
    open(os.path.join(gd, 'reference.md'), 'w').write('\n'.join(r) + '\n')
    print('\n'.join(t))
