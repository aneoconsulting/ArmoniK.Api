#!/usr/bin/env python3
"""gen/s15_tables.py DIR: s15's tables (D26, no thread-local storage). CONTAINER
INSTRUMENTATION; absolute times only.
DIR/dist/*.tsv (gen/s15_dist.sh) -> dist/summary.md: per configuration (new-default, new-pgo0,
base-default), content and path, the per-PROCESS medians (ns per encode of the one-string
message, 48 units): min, quartiles, max, processes under 300 ns (fast) / at or above (slow).
DIR/ladder/*.jsonl (gen/s15_grid.sh, S15_SET=ladder) -> ladder/table.md, reference.md: us per
op, median over every round of every rep, per variant; E1R - E0 per string (P2.2 17,167
strings, P2.4 26,267, U-deep-u-repeated 31) for the D26 code and the base.
DIR/codec/*.jsonl (S15_SET=codec) -> codec/table.md, reference.md: us per op, new and base, per
payload, content, direction and mode."""
import collections, glob, json, os, re, statistics, sys
D = sys.argv[1]
f2 = lambda x: '-' if x is None else (('%.4g' % x) if abs(x) >= 1 else ('%.3f' % x))
# ---------------------------------------------------------------- dist
dd = os.path.join(D, 'dist')
if os.path.isdir(dd):
    d = collections.defaultdict(list)
    for f in glob.glob(os.path.join(dd, '*-*.tsv')):
        m = re.match(r'(.+)-(\d+)\.tsv$', os.path.basename(f))
        if not m: continue
        v = collections.defaultdict(list)
        for l in open(f):
            p = l.rstrip('\n').split('\t')
            if p[0] == 'content': continue
            v[(p[0], p[3])].append(float(p[5]))
        for k, x in v.items(): d[(m.group(1),) + k].append(statistics.median(x))
    CF = [('new-default', 'D26 code, default JIT configuration'), ('new-pgo0', 'D26 code, DOTNET_TieredPGO=0'), ('base-default', 'code before D26 (control), default JIT configuration')]
    t = ['# s15: per-process median ns per encode (UploadResultDataMessage, one 48-unit string), one-string sweep, CPUs 0,1, interleaved; per configuration and path: processes, min, q1, median, q3, max, processes under 300 ns (fast) / at or above (slow). CONTAINER INSTRUMENTATION.', '',
         '| configuration | content | path | processes | min | q1 | median | q3 | max | fast | slow |', '|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|']
    for c, cn in CF:
        for content in ('ascii', 'latin1'):
            for path in ('E0', 'E1R'):
                x = sorted(d.get((c, content, path), []))
                if not x: continue
                q = statistics.quantiles(x, n=4) if len(x) > 1 else [x[0]] * 3
                t.append('| %s | %s | %s | %d | %.0f | %.0f | %.0f | %.0f | %.0f | %d | %d |' % (cn, content, path, len(x), x[0], q[0], q[1], q[2], x[-1], sum(1 for y in x if y < 300), sum(1 for y in x if y >= 300)))
    open(os.path.join(dd, 'summary.md'), 'w').write('\n'.join(t) + '\n')
    print('\n'.join(t))


def load(gd):
    by = {}
    for f in glob.glob(os.path.join(gd, '*-full-r*.jsonl')):
        m = re.match(r'(.+)-full-r(\d+)\.jsonl$', os.path.basename(f)); var, rep = m.group(1), int(m.group(2))
        for l in open(f):
            if not l.startswith('{'): continue
            x = json.loads(l)
            if x.get('round', 0) < 1 or 'cpu_ns' not in x or x.get('arm') != 'core-ffi': continue
            k = (x['payload'], x['content'], x['dir'], x['unknown_mode'])
            by.setdefault(k, {}).setdefault(var, {}).setdefault(rep, []).append(x['cpu_ns'] / x['iters'] / 1000)
    return by


def key(k):
    m = re.match(r'P(\d+)\.(\d+)$', k[0])
    return ((0, int(m.group(1)), int(m.group(2))) if m else (1, k[0], 0), k[1], k[2], k[3])


def cell(e):
    a = [y for z in e.values() for y in z]
    return statistics.median(a), '%s [%s-%s] (%s)' % (f2(statistics.median(a)), f2(min(a)), f2(max(a)), ' '.join(f2(statistics.median(e[p])) for p in sorted(e)))


# ---------------------------------------------------------------- ladder
NSTR = {'P2.2': 17167, 'P2.4': 26267, 'U-deep-u-repeated': 31}
gd = os.path.join(D, 'ladder')
if os.path.isdir(gd) and glob.glob(os.path.join(gd, '*.jsonl')):
    by = load(gd)
    R = ['new-r0', 'new-r3', 'new-r3g', 'base-r0', 'base-r3']
    t = ['# s15 ladder rows: encode-core-hot, core-ffi, process CPU per op (us), median over every round of every rep (3 BDN host processes per variant); E1R - E0 per string in ns (step / strings per encode). new = D26 code, base = the code before D26 (same session). CONTAINER INSTRUMENTATION.', '']
    rf = ['# s15 ladder reference: median [min-max] (per-rep medians), us per op', '']
    for cfg, cn in (('pgo0', 'DOTNET_TieredPGO=0'), ('dflt', 'default JIT configuration (every BDN child kept)')):
        t += ['## ' + cn, '', '| payload | content | mode | new R0 | new R3 | new R3g | base R0 | base R3 | new R3-R0 ns/str | base R3-R0 ns/str | new R3-R3g ns/str |', '|---|---|---|' + '---:|' * 8]
        rf += ['## ' + cn, '', '| payload | content | mode | ' + ' | '.join(R) + ' |', '|---|---|---|' + '---|' * len(R)]
        for k in sorted(by, key=key):
            v, cr = {}, []
            for q in R:
                e = by[k].get('%s-%s' % (q, cfg))
                if not e: v[q] = None; cr.append('-'); continue
                v[q], c = cell(e); cr.append(c)
            ns = NSTR.get(k[0])
            ps = lambda a, b: None if v[a] is None or v[b] is None or not ns else (v[a] - v[b]) * 1000 / ns
            t.append('| %s | %s | %s | %s | %s |' % (k[0], k[1], k[3], ' | '.join(f2(v[q]) for q in R), ' | '.join(f2(x) for x in (ps('new-r3', 'new-r0'), ps('base-r3', 'base-r0'), ps('new-r3', 'new-r3g')))))
            rf.append('| %s | %s | %s | %s |' % (k[0], k[1], k[3], ' | '.join(cr)))
        t.append(''); rf.append('')
    open(os.path.join(gd, 'table.md'), 'w').write('\n'.join(t) + '\n')
    open(os.path.join(gd, 'reference.md'), 'w').write('\n'.join(rf) + '\n')
    print('\n'.join(t))
# ---------------------------------------------------------------- codec
for gd in (os.path.join(D, 'codec'), os.path.join(D, 'codec-p22'), os.path.join(D, 'codec-small')):
  if not (os.path.isdir(gd) and glob.glob(os.path.join(gd, '*.jsonl'))): continue
  if True:
    by = load(gd)
    t = ['# s15 core-ffi codec grid (the core grid core-ffi directions, encode-core-hot and decode-read; drop and retain; E0 and the FSM as today): process CPU per op (us), median over every round of every rep (3 BDN host processes per variant), new = D26 code, base = the code before D26 (same session, interleaved). CONTAINER INSTRUMENTATION.', '',
         '| payload | content | dir | mode | new | base |', '|---|---|---|---|---:|---:|']
    rf = ['# s15 codec grid reference: median [min-max] (per-rep medians), us per op', '', '| payload | content | dir | mode | new | base |', '|---|---|---|---|---|---|']
    for k in sorted(by, key=key):
        v, cr = {}, []
        for q in ('new', 'base'):
            e = by[k].get(q)
            if not e: v[q] = None; cr.append('-'); continue
            v[q], c = cell(e); cr.append(c)
        t.append('| %s | %s | %s | %s | %s | %s |' % (k[0], k[1], k[2], k[3], f2(v['new']), f2(v['base'])))
        rf.append('| %s | %s | %s | %s | %s |' % (k[0], k[1], k[2], k[3], ' | '.join(cr)))
    open(os.path.join(gd, 'table.md'), 'w').write('\n'.join(t) + '\n')
    open(os.path.join(gd, 'reference.md'), 'w').write('\n'.join(rf) + '\n')
    print('\n'.join(t))
