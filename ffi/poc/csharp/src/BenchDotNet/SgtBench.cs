// s12 (owner, 2026-10-09): [SuppressGCTransition] on the FSM's begin / next. Two parts, one
// process, CONTAINER INSTRUMENTATION.
//
//   BenchDotNet --sgtbench FILE [--rounds R] [--block-ms MS] [--calls K]
//
// 1. Crossing microbench: the raw FSM call loop (begin, then next until the root APPLY; no
//    dispatch, nothing read) through the generated imports, plain and [SuppressGCTransition]
//    twins (`_sgt`), on two probes: P7.1 (DualResponse, the committed interleaved vector: 7 calls)
//    and an empty-element probe (ListResultsResponse `0A 00`, one empty element: ADD then APPLY,
//    one begin + next pair). Drop mode (the context made with no options: no grow can happen).
//    Interleaved blocks of MS ms, R rounds, the variant order alternated per round; process CPU
//    per decode (ProcCpu), median and range over rounds; ns per call = per decode / calls.
// 2. The longest single call: every row of the core decode grid (16 shapes, P2.2 Latin-1 and
//    wide, the 7 U-* rows), drop (no-unknown in its build): each begin and next timed alone
//    (Stopwatch ticks around the call, plain import through the export's address), K decodes;
//    per event the MINIMUM over the K decodes (what the call costs without an interrupt), then
//    the row's longest such event, the number of events whose minimum exceeds 1 us, and the
//    events per decode. The attribute's rule is "under 1 us"; this says where that holds.
//
// Allocation: the attributed calls are made only after a plain begin on the same context, which
// is where the FSM state (DecCtxImpl.fsm: the Box, the 32 KB arena, the group words) is
// allocated; ak-core src/fsm.rs allocates nowhere else (fsm_of on the first begin, grp.resize
// only when a root's group needs more words than the context's first decode sized, and a
// context is bound to one root), and drop mode calls no grow.

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using Armonik.Ffi.Campaign;
using Armonik.Ffi.Facade;
using Armonik.Ffi.Harness;

namespace Armonik.Ffi.Bdn;

public static unsafe class SgtBench
{
    private static readonly byte[] Empty = { 0x0A, 0x00 };

    private static IntPtr NewCtxDual()
    {
#if AK_NO_UNKNOWN_FIELDS
        var c = Abi.ak_dec_ctx_new_DualResponse();
#else
        var c = Abi.ak_dec_ctx_new_DualResponse(null);
#endif
        var p = (ak_pvt_DualResponse*)NativeMemory.AllocZeroed((nuint)sizeof(ak_pvt_DualResponse));
        p->utf8_skip = AkUtf8Skip.DualResponse_ALL;
        if (Abi.ak_fsm_set_pvt_DualResponse(c, p) != 0) throw new InvalidOperationException("set_pvt");
        return c;
    }

    private static IntPtr NewCtxLrr()
    {
#if AK_NO_UNKNOWN_FIELDS
        var c = Abi.ak_dec_ctx_new_ListResultsResponse();
#else
        var c = Abi.ak_dec_ctx_new_ListResultsResponse(null);
#endif
        var p = (ak_pvt_ListResultsResponse*)NativeMemory.AllocZeroed((nuint)sizeof(ak_pvt_ListResultsResponse));
        p->utf8_skip = AkUtf8Skip.ListResultsResponse_ALL;
        if (Abi.ak_fsm_set_pvt_ListResultsResponse(c, p) != 0) throw new InvalidOperationException("set_pvt");
        return c;
    }

    // One decode, raw: the calls it took (> 0) or the error.
    private static int DualPlain(IntPtr c, byte* b, nuint n)
    {
        ak_fsm_ev ev; int k = 1;
        int op = Abi.ak_fsm_begin_DualResponse(c, b, n, &ev);
        while (op > 0 && op != (int)Abi.AK_BDR_APPLY) { op = Abi.ak_fsm_next_DualResponse(c, &ev); k++; }
        return op < 0 ? op : k;
    }
    private static int DualSgt(IntPtr c, byte* b, nuint n)
    {
        ak_fsm_ev ev; int k = 1;
        int op = Abi.ak_fsm_begin_DualResponse_sgt(c, b, n, &ev);
        while (op > 0 && op != (int)Abi.AK_BDR_APPLY) { op = Abi.ak_fsm_next_DualResponse_sgt(c, &ev); k++; }
        return op < 0 ? op : k;
    }
    private static int LrrPlain(IntPtr c, byte* b, nuint n)
    {
        ak_fsm_ev ev; int k = 1;
        int op = Abi.ak_fsm_begin_ListResultsResponse(c, b, n, &ev);
        while (op > 0 && op != (int)Abi.AK_BDR_APPLY) { op = Abi.ak_fsm_next_ListResultsResponse(c, &ev); k++; }
        return op < 0 ? op : k;
    }
    private static int LrrSgt(IntPtr c, byte* b, nuint n)
    {
        ak_fsm_ev ev; int k = 1;
        int op = Abi.ak_fsm_begin_ListResultsResponse_sgt(c, b, n, &ev);
        while (op > 0 && op != (int)Abi.AK_BDR_APPLY) { op = Abi.ak_fsm_next_ListResultsResponse_sgt(c, &ev); k++; }
        return op < 0 ? op : k;
    }

    public static int Run(string outp, int rounds, double blockMs, int calls)
    {
        AbiInit.Ensure();
        var lines = new List<string>
        {
            "# s12 --sgtbench (" + AbiVariant.Name + " build), CONTAINER INSTRUMENTATION; " + RuntimeInformation.FrameworkDescription
                + "; affinity 0x" + Process.GetCurrentProcess().ProcessorAffinity.ToString("x") + "; rounds " + rounds + " x " + blockMs + " ms blocks, order alternated",
        };
        // ---------------------------------------------------------------- 1. microbench
        var p71 = Cases.DecodeWire("P7.1", OpsTable.ForPayload("P7.1"));
        var probes = new (string Name, byte[] Wire, Func<IntPtr> Ctx, Func<IntPtr, IntPtr, nuint, int> Plain, Func<IntPtr, IntPtr, nuint, int> Sgt)[]
        {
            ("P7.1 (DualResponse, committed vector)", p71, NewCtxDual, (c, b, n) => DualPlain(c, (byte*)b, n), (c, b, n) => DualSgt(c, (byte*)b, n)),
            ("empty element (ListResultsResponse 0A 00)", Empty, NewCtxLrr, (c, b, n) => LrrPlain(c, (byte*)b, n), (c, b, n) => LrrSgt(c, (byte*)b, n)),
        };
        lines.Add("");
        lines.Add("## 1. crossing microbench: raw begin + next loop, no dispatch; process CPU per decode (ns), median [min-max] over rounds; per call = per decode / calls");
        lines.Add("| probe | calls per decode | plain ns/decode | sgt ns/decode | plain ns/call | sgt ns/call |");
        lines.Add("|---|---:|---:|---:|---:|---:|");
        foreach (var pr in probes)
        {
            var ctx = pr.Ctx();
            var h = GCHandle.Alloc(pr.Wire, GCHandleType.Pinned);
            try
            {
                IntPtr b = h.AddrOfPinnedObject(); nuint n = (nuint)pr.Wire.Length;
                int k = pr.Plain(ctx, b, n);   // the context's first begin: plain (allocates the FSM state)
                if (k <= 0 || pr.Sgt(ctx, b, n) != k) throw new InvalidOperationException(pr.Name + ": plain " + k + ", sgt differs");
                // calibrate
                long iters = 1000;
                for (; ; iters *= 2)
                {
                    long t0 = Stopwatch.GetTimestamp();
                    for (long i = 0; i < iters; i++) pr.Plain(ctx, b, n);
                    if ((Stopwatch.GetTimestamp() - t0) * 1000.0 / Stopwatch.Frequency >= blockMs) break;
                }
                for (int w = 0; w < 5; w++) { for (long i = 0; i < iters; i++) { pr.Plain(ctx, b, n); pr.Sgt(ctx, b, n); } }
                var res = new List<double>[] { new List<double>(), new List<double>() };
                for (int r = 0; r < rounds; r++)
                    foreach (int v in (r % 2 == 0) ? new[] { 0, 1 } : new[] { 1, 0 })
                    {
                        var f = v == 0 ? pr.Plain : pr.Sgt;
                        long c0 = ProcCpu.Ns();
                        for (long i = 0; i < iters; i++) f(ctx, b, n);
                        res[v].Add((ProcCpu.Ns() - c0) / (double)iters);
                    }
                string F(List<double> x) => string.Format(CultureInfo.InvariantCulture, "{0:F1} [{1:F1}-{2:F1}]", Med(x), x.Min(), x.Max());
                lines.Add(string.Format(CultureInfo.InvariantCulture, "| {0} | {1} | {2} | {3} | {4:F1} | {5:F1} |", pr.Name, k, F(res[0]), F(res[1]), Med(res[0]) / k, Med(res[1]) / k));
            }
            finally { h.Free(); Abi.ak_dec_ctx_free(ctx); }
        }

        // ---------------------------------------------------------------- 2. longest single call
        lines.Add("");
        lines.Add("## 2. the longest single call per row (plain import through the export's address, " + (AbiVariant.UnknownCompiledOut ? "no-unknown" : "drop") + " mode): per event the minimum over " + calls + " decodes (ns), then the row's longest; events > 1 us = events whose minimum exceeds 1,000 ns");
        lines.Add("| row | bytes | calls per decode | longest call (ns) | at call # (op) | median call (ns) | calls > 1 us |");
        lines.Add("|---|---:|---:|---:|---|---:|---:|");
        var lib = NativeLibrary.Load(Path.Combine(AppContext.BaseDirectory, "libak_core.so"));
        var rows = new List<(string, string, byte[])>();
        foreach (var pid in OpsTable.Payloads)
            foreach (var cs in pid == "P2.2" ? new[] { Values.Ascii, Values.Latin1, Values.Wide } : new[] { Values.Ascii })
            {
                Values.ContentSet = cs;
                var ops = OpsTable.ForPayload(pid);
                Values.ContentSet = Values.Ascii;
                rows.Add((cs == Values.Ascii ? pid : pid + "/" + Values.SetNames[cs], RootOf(pid), Cases.DecodeWire(pid, ops)));
            }
        var urows = Cases.UnknownRows();
        foreach (var id in Cases.CoreURows) if (urows.Contains(id)) rows.Add((id, ManifestRoot(id), Cases.Row(id).Item2));
        double tickNs = 1e9 / Stopwatch.Frequency;
        foreach (var (name, root, wire) in rows)
        {
            var begin = (delegate* unmanaged[Cdecl]<IntPtr, byte*, nuint, ak_fsm_ev*, int>)NativeLibrary.GetExport(lib, "ak_fsm_begin_" + root);
            var next = (delegate* unmanaged[Cdecl]<IntPtr, ak_fsm_ev*, int>)NativeLibrary.GetExport(lib, "ak_fsm_next_" + root);
#if AK_NO_UNKNOWN_FIELDS
            var ctx = ((delegate* unmanaged[Cdecl]<IntPtr>)NativeLibrary.GetExport(lib, "ak_dec_ctx_new_" + root))();
#else
            var ctx = ((delegate* unmanaged[Cdecl]<void*, IntPtr>)NativeLibrary.GetExport(lib, "ak_dec_ctx_new_" + root))(null);
#endif
            var pvt = (ulong*)NativeMemory.AllocZeroed(512);
            pvt[0] = (ulong)typeof(AkUtf8Skip).GetField(root + "_ALL").GetValue(null);
            ((delegate* unmanaged[Cdecl]<IntPtr, void*, int>)NativeLibrary.GetExport(lib, "ak_fsm_set_pvt_" + root))(ctx, pvt);
            long[] best = null; int[] opAt = null; int nev = 0;
            fixed (byte* b0 = wire)
            {
                byte* b = wire.Length == 0 ? (byte*)pvt : b0;
                for (int rep = 0; rep < calls + 3; rep++)
                {
                    var t = new List<long>(); var ops = new List<int>();
                    ak_fsm_ev ev;
                    long s = Stopwatch.GetTimestamp();
                    int op = begin(ctx, b, (nuint)wire.Length, &ev);
                    t.Add(Stopwatch.GetTimestamp() - s); ops.Add(op);
                    while (op > 0 && op != (int)Abi.AK_BDR_APPLY)
                    {
                        s = Stopwatch.GetTimestamp();
                        op = next(ctx, &ev);
                        t.Add(Stopwatch.GetTimestamp() - s); ops.Add(op);
                    }
                    if (op < 0) throw new InvalidOperationException(name + ": FSM " + op);
                    if (rep < 3) continue;   // warm (the first begin allocates the context's FSM state)
                    if (best == null) { best = t.ToArray(); opAt = ops.ToArray(); nev = t.Count; }
                    else for (int i = 0; i < nev; i++) best[i] = Math.Min(best[i], t[i]);
                }
            }
            NativeMemory.Free(pvt);
            Abi.ak_dec_ctx_free(ctx);
            int arg = 0; for (int i = 1; i < nev; i++) if (best[i] > best[arg]) arg = i;
            var sorted = best.OrderBy(x => x).ToArray();
            lines.Add(string.Format(CultureInfo.InvariantCulture, "| {0} | {1} | {2} | {3:F0} | {4} ({5}) | {6:F0} | {7} |", name, wire.Length, nev,
                best[arg] * tickNs, arg, OpName(opAt[arg]), sorted[nev / 2] * tickNs, best.Count(x => x * tickNs > 1000)));
        }
        lines.Add("");
        lines.Add("# timer: Stopwatch (" + Stopwatch.Frequency + " Hz); each call's figure includes two timestamp reads (about 20-40 ns here)");
        File.WriteAllLines(outp, lines);
        foreach (var l in lines) Console.WriteLine(l);
        return 0;
    }

    private static string OpName(int op) => op == 1 ? "APPLY" : op == 2 ? "ADD" : op == 3 ? "NEW" : op == 4 ? "APPLY_ELEM" : op.ToString(CultureInfo.InvariantCulture);
    private static double Med(List<double> x) { var s = x.OrderBy(v => v).ToArray(); return s.Length % 2 == 1 ? s[s.Length / 2] : (s[s.Length / 2 - 1] + s[s.Length / 2]) / 2; }
    private static string RootOf(string pid) => OpsTable.ForPayload(pid).GetType().Name.Substring("Ops_".Length);
    private static string ManifestRoot(string id) => Cases.Row(id).Item1.GetType().Name.Substring("Ops_".Length);
}
