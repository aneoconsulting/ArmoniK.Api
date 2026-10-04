// D21 step 7 item 4 (owner, 2026-10-04): what pinning N strings costs, apart from the codec.
//
//   BenchDotNet --pinbench OUT.tsv [--rounds R] [--block-ms T]
//
// One process. For N distinct 36-character strings (a GUID's length, the grid's strings), per
// string: (a) `live`: N GCHandle.Alloc(Pinned), then N Free (E1: every handle of an encode
// live until the call returns); (b) `chunk64`: Alloc 64, Free 64, repeated (E1C: one chunk's
// handles live); (c) `fixed64`: a recursion of one frame per string, each `fixed` and storing its
// pointer, 64 deep, unwound (E1R's frames without the codec). Each op covers the N strings;
// reported per string (process CPU ns), with minor faults and GC collections per op. Paths
// interleaved, the order rotated per round. Container instrumentation.

using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;

namespace Armonik.Ffi.Bdn;

public static unsafe class PinBench
{
    private static readonly IntPtr[] Sink = new IntPtr[64];

    private static int Rec(string[] s, int off, int k, int j)
    {
        if (j == k) return k;
        fixed (char* p = s[off + j])
        {
            Sink[j] = (IntPtr)p;
            return Rec(s, off, k, j + 1);
        }
    }

    public static int Run(string outp, int rounds, double blockMs)
    {
        var names = new[] { "live", "chunk64", "fixed64" };
        var lines = new List<string> { "n\tpath\tround\tcpu_ns_per_string\tminflt_per_op\tgc0_per_op\tgc1_per_op\tgc2_per_op\tops" };
        var md = new List<string>
        {
            "# D21 step 7: pinning N strings apart from the codec, process CPU ns per string, median [min-max] over " + rounds + " rounds; minflt and GC collections per op (N strings)",
            "",
            "CONTAINER INSTRUMENTATION. `live` = N GCHandle.Alloc(Pinned) then N Free (E1); `chunk64` = Alloc/Free 64 at a time (E1C); `fixed64` = one recursive frame per string with `fixed`, 64 deep (E1R's frames alone).",
            "",
            "| N | live | chunk64 | fixed64 | live minflt/op | live gen0/op |",
            "|---:|---:|---:|---:|---:|---:|",
        };
        foreach (int n in new[] { 1, 16, 64, 256, 1024, 5000, 17167, 26267 })
        {
            var s = new string[n];
            for (int i = 0; i < n; i++) s[i] = Guid.NewGuid().ToString();
            var h = new GCHandle[n];
            void Op(int path)
            {
                if (path == 0)
                {
                    for (int i = 0; i < n; i++) h[i] = GCHandle.Alloc(s[i], GCHandleType.Pinned);
                    for (int i = 0; i < n; i++) h[i].Free();
                }
                else if (path == 1)
                {
                    for (int off = 0; off < n; off += 64)
                    {
                        int k = Math.Min(64, n - off);
                        for (int i = 0; i < k; i++) h[off + i] = GCHandle.Alloc(s[off + i], GCHandleType.Pinned);
                        for (int i = 0; i < k; i++) h[off + i].Free();
                    }
                }
                else
                {
                    for (int off = 0; off < n; off += 64) Rec(s, off, Math.Min(64, n - off), 0);
                }
            }
            long Iters(int path)
            {
                long it = 1; double t;
                while (true)
                {
                    long w0 = ProcCpu.Wall();
                    for (long i = 0; i < it; i++) Op(path);
                    t = (ProcCpu.Wall() - w0) / 1e6;
                    if (t > blockMs / 4 || it > 1 << 26) break;
                    it *= 2;
                }
                return Math.Max(1, (long)(it * blockMs / Math.Max(t, 1e-3)));
            }
            var iters = new long[3];
            long ws = ProcCpu.Wall();
            while ((ProcCpu.Wall() - ws) / 1e6 < 1000) for (int q = 0; q < 3; q++) iters[q] = Iters(q);
            var per = new[] { new List<double>(), new List<double>(), new List<double>() };
            var mf = new[] { new List<double>(), new List<double>(), new List<double>() };
            var g0 = new[] { new List<double>(), new List<double>(), new List<double>() };
            for (int r = 0; r < rounds; r++)
                for (int j = 0; j < 3; j++)
                {
                    int q = (j + r) % 3;
                    int c0 = GC.CollectionCount(0), c1 = GC.CollectionCount(1), c2 = GC.CollectionCount(2);
                    long f0 = ProcCpu.MinFlt(), t0 = ProcCpu.Ns();
                    for (long i = 0; i < iters[q]; i++) Op(q);
                    long t1 = ProcCpu.Ns(), f1 = ProcCpu.MinFlt();
                    double ns = (double)(t1 - t0) / iters[q] / n, f = (double)(f1 - f0) / iters[q];
                    double d0 = (double)(GC.CollectionCount(0) - c0) / iters[q], d1 = (double)(GC.CollectionCount(1) - c1) / iters[q], d2 = (double)(GC.CollectionCount(2) - c2) / iters[q];
                    per[q].Add(ns); mf[q].Add(f); g0[q].Add(d0);
                    lines.Add(string.Join("\t", n, names[q], r + 1, ns.ToString("F2", CultureInfo.InvariantCulture), f.ToString("G4", CultureInfo.InvariantCulture),
                        d0.ToString("G4", CultureInfo.InvariantCulture), d1.ToString("G4", CultureInfo.InvariantCulture), d2.ToString("G4", CultureInfo.InvariantCulture), iters[q]));
                }
            string Fm(List<double> v) { var a = v.OrderBy(x => x).ToList(); return string.Format(CultureInfo.InvariantCulture, "{0:F1} [{1:F1}-{2:F1}]", a[a.Count / 2], a[0], a[^1]); }
            double Med(List<double> v) { var a = v.OrderBy(x => x).ToList(); return a[a.Count / 2]; }
            md.Add(string.Format(CultureInfo.InvariantCulture, "| {0} | {1} | {2} | {3} | {4:G3} | {5:G3} |", n, Fm(per[0]), Fm(per[1]), Fm(per[2]), Med(mf[0]), Med(g0[0])));
        }
        File.WriteAllLines(outp, lines);
        File.WriteAllLines(Path.ChangeExtension(outp, ".md"), md);
        foreach (var l in md) Console.WriteLine(l);
        return 0;
    }
}
