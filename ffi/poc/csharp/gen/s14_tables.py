#!/usr/bin/env python3
"""gen/s14_tables.py DIR: s14 part 1's distribution table from DIR/dist/*.tsv (gen/s14_dist.sh):
per JIT configuration, content and path, the per-PROCESS medians (ns per encode of the one-string
message, 48 units): min, quartiles, max, and how many processes fall in each mode (fast: under
300 ns; slow: 300 ns or more). CONTAINER INSTRUMENTATION."""
import collections, glob, os, re, statistics, sys
D = os.path.join(sys.argv[1], 'dist')
d = collections.defaultdict(list)
for f in glob.glob(os.path.join(D, '*-*.tsv')):
    cfg = re.match(r'(.+)-\d+\.tsv$', os.path.basename(f)).group(1)
    v = collections.defaultdict(list)
    for l in open(f):
        p = l.rstrip('\n').split('\t')
        if p[0] == 'content': continue
        v[(p[0], p[3])].append(float(p[5]))
    for k, x in v.items(): d[(cfg,) + k].append(statistics.median(x))
CF = [('default', 'default'), ('pgo0', 'DOTNET_TieredPGO=0'), ('tc0', 'DOTNET_TieredCompilation=0'), ('osr0', 'DOTNET_TC_OnStackReplacement=0'), ('r2r0', 'DOTNET_ReadyToRun=0')]
t = ['# s14 part 1: per-process median ns per encode (UploadResultDataMessage, one 48-unit string), one-string sweep, CPUs 0,1; per configuration and path: processes, min, q1, median, q3, max, processes under 300 ns (fast) / at or above (slow)', '',
     '| configuration | content | path | processes | min | q1 | median | q3 | max | fast | slow |', '|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|']
for c, cn in CF:
    for content in ('ascii', 'latin1'):
        for path in ('E0', 'E1R'):
            x = sorted(d.get((c, content, path), []))
            if not x: continue
            q = statistics.quantiles(x, n=4)
            t.append('| %s | %s | %s | %d | %.0f | %.0f | %.0f | %.0f | %.0f | %d | %d |' % (cn, content, path, len(x), x[0], q[0], q[1], q[2], x[-1], sum(1 for y in x if y < 300), sum(1 for y in x if y >= 300)))
open(os.path.join(D, 'summary.md'), 'w').write('\n'.join(t) + '\n')
print('\n'.join(t))
