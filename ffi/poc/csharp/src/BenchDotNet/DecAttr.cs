// D21 s8 (owner, 2026-10-08): ATTRIBUTION of the core-ffi decode, no optimisation.
//
//   BenchDotNet --decattr OUT.tsv [--rounds R] [--block-ms T] [--only ID,...]
//   BenchDotNet --decattr-counts OUT.txt        (the counting build: crossings per arm)
//
// One process; per row (payload / content set, or a corpus U-* row) and mode (retain and
// drop in the full build, no-unknown in the no-unknown build), every arm below is timed in
// rounds, interleaved and rotated, after a warm-up of >= 1 s, so their ratios share the process:
//   ref        core-ffi push decode-read (HEAD), the reference
//   skip       ref with G.SkipStrings (the existing ceiling: every string "" , nothing else changed)
//   noop       the push decode through VtNoop: every callback returns at once (new_: token 0)
//   parse      the push decode through VtParse: every callback null but new_ (the core skips a
//              non-leaf element whose new_ is null): the core's own share, utf8_skip all bits
//   parsev     parse with utf8_skip 0 (the core validates every string)
//   pull       core-ffi-pull decode-read (the pull family, D20 pvt bits set; drop only: the
//              existing arm's form)
//   pparse     the pull family's parse alone (ak_parse_<Root>, no replay)
//   touch      the read pass alone (Touch.F_<Root>) over a graph decoded beforehand
//   strs       the payload's strings alone: Strict.GetString over their UTF-8 (the decoded graph's
//              non-empty strings, in walk order, from one buffer)
//   host       host-gen decode-read (control)
//   inc        incumbent-prod decode-read (control; ParseFrom(ReadOnlySequence) as the grid's row;
//              mode-independent, timed in every mode)
// Per arm: process CPU ns per op, bytes allocated per op (this thread), gen0/1/2 per op, minflt
// per op. Per row: the census of the decoded graphs (facade: objects, non-empty strings and their
// UTF-16 units, lists, maps, byte[]; incumbent: messages, strings, repeated, maps, ByteString).
// Container instrumentation.

using System;
using System.Collections;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Text;
using Armonik.Ffi.Campaign;
using Armonik.Ffi.Facade;
using Google.Protobuf;
using Google.Protobuf.Reflection;

namespace Armonik.Ffi.Bdn;

public static class DecAttr
{
#if AK_NO_UNKNOWN_FIELDS
    private static readonly string[] Modes = { "no-unknown" };
#else
    private static readonly string[] Modes = { "retain", "drop" };
#endif
    private static readonly string[] Arms = { "ref", "skip", "noop", "parse", "parsev", "pull", "pparse", "touch", "strs", "host", "inc" };

    private static readonly UTF8Encoding Strict = new UTF8Encoding(false, true);

    private sealed class Row { public string Id, Content; public RootOps Ops; public byte[] Wire; }

    private static List<Row> Rows(string only)
    {
        var keep = string.IsNullOrEmpty(only) ? null : new HashSet<string>(only.Split(','));
        var rows = new List<Row>();
        foreach (var pid in new[] { "P1.2", "P2.2", "P2.3", "P2.4", "P2.5", "P4.1" })
            foreach (var cs in pid == "P2.2" ? new[] { 0, 1, 2 } : new[] { 0 })
            {
                if (keep != null && !keep.Contains(pid)) continue;
                Values.ContentSet = cs;
                RootOps ops;
                try { ops = OpsTable.ForPayload(pid); } finally { Values.ContentSet = Values.Ascii; }
                rows.Add(new Row { Id = pid, Content = Values.SetNames[cs], Ops = ops, Wire = ops.IncumbentBytes() });
            }
        foreach (var id in Cases.CoreURows)
        {
            if (keep != null && !keep.Contains(id)) continue;
            var (ops, b) = Cases.Row(id);
            rows.Add(new Row { Id = id, Content = "corpus", Ops = ops, Wire = b });
        }
        return rows;
    }

    private static Func<long> Op(Row r, string arm, bool retain, object graph, (byte[] buf, int[] off, int[] len) strs)
    {
        var o = r.Ops; var b = r.Wire; int n = b.Length; var seq = new System.Buffers.ReadOnlySequence<byte>(b);
        return arm switch
        {
            "ref" => () => o.DecFfi(b, n, retain, true),
            "skip" => () => { Armonik.Ffi.Harness.G.SkipStrings = true; try { return o.DecFfi(b, n, retain, true); } finally { Armonik.Ffi.Harness.G.SkipStrings = false; } },
            "noop" => () => o.DecFfiVt(b, n, retain, 1),
            "parse" => () => o.DecFfiVt(b, n, retain, 0),
            "parsev" => () => o.DecFfiVt(b, n, retain, 2),
            "pull" => () => o.DecFfiPull(b, n, true),
            "pparse" => () => o.DecFfiParse(b, n, retain),
            "touch" => () => o.TouchF(graph),
            "strs" => () => { long t = 0; for (int i = 0; i < strs.off.Length; i++) t += Strict.GetString(strs.buf, strs.off[i], strs.len[i]).Length; return t; },
            "host" => () => o.DecHost(b, n, retain, true),
            "inc" => () => o.DecIncProd(seq, true),
            _ => throw new ArgumentException(arm),
        };
    }

    // ---------------------------------------------------------------- census
    public sealed class Census { public long Objects, Strings, Units, Lists, Maps, Bytes, Elements; public List<string> All = new List<string>();
        public override string ToString() => string.Format(CultureInfo.InvariantCulture, "objects {0} strings {1} (utf16 units {2}) lists {3} maps {4} byte[] {5} list/map elements {6}", Objects, Strings, Units, Lists, Maps, Bytes, Elements); }

    private static void Str(Census c, string s) { if (string.IsNullOrEmpty(s)) return; c.Strings++; c.Units += s.Length; c.All.Add(s); }

    public static void Walk(object x, Census c)
    {
        if (x == null) return;
        switch (x)
        {
            case string s: Str(c, s); return;
            case byte[] bb: if (bb.Length > 0) c.Bytes++; return;
            case ByteString bs: if (bs.Length > 0) c.Bytes++; return;
        }
        var t = x.GetType();
        if (x is IMessage m)
        {
            c.Objects++;
            foreach (var f in m.Descriptor.Fields.InFieldNumberOrder())
            {
                var v = f.Accessor.GetValue(m);
                if (f.IsMap) { var d = (IDictionary)v; if (d.Count > 0) c.Maps++; foreach (DictionaryEntry e in d) { c.Elements++; Walk(e.Key, c); Walk(e.Value, c); } }
                else if (f.IsRepeated) { var l = (IList)v; if (l.Count > 0) c.Lists++; foreach (var e in l) { c.Elements++; Walk(e, c); } }
                else if (f.FieldType == FieldType.Message || f.FieldType == FieldType.String || f.FieldType == FieldType.Bytes) Walk(v, c);
            }
            return;
        }
        if (t.Namespace == typeof(ListResultsResponse).Namespace && t.IsClass)
        {
            c.Objects++;
            foreach (var f in t.GetFields(BindingFlags.Public | BindingFlags.Instance))
            {
                var v = f.GetValue(x);
                if (v == null) continue;
                if (v is string || v is byte[]) { Walk(v, c); continue; }
                var vt = v.GetType();
                if (vt.IsGenericType && vt.GetGenericTypeDefinition() == typeof(List<>)) { var l = (IList)v; if (l.Count > 0) c.Lists++; foreach (var e in l) { c.Elements++; Walk(e, c); } continue; }
                if (vt.IsGenericType && vt.GetGenericTypeDefinition() == typeof(OrderedMap<,>))
                {
                    int k = 0;
                    foreach (var e in (IEnumerable)v) { k++; c.Elements++; var et = e.GetType(); Walk(et.GetProperty("Key").GetValue(e), c); Walk(et.GetProperty("Value").GetValue(e), c); }
                    if (k > 0) c.Maps++;
                    continue;
                }
                if (vt.IsClass && vt.Namespace == t.Namespace) Walk(v, c);
            }
        }
    }

    // ---------------------------------------------------------------- timing
    public static int Run(string outp, int rounds, double blockMs, string only)
    {
        var lines = new List<string> { "row\tcontent\tmode\tarm\tround\tcpu_ns_per_op\talloc_bytes_per_op\tgen0_per_op\tgen1_per_op\tgen2_per_op\tminflt_per_op\tops" };
        var census = new List<string> { "# D21 s8: census of the decoded graphs (one decode, the mode's)" };
        var md = new List<string>();
        foreach (var r in Rows(only))
            foreach (var mode in Modes)
            {
                bool retain = mode == "retain";
                object graph = r.Ops.DecFfiGraph(r.Wire, r.Wire.Length, retain);
                var cf = new Census(); Walk(graph, cf);
                var ci = new Census(); Walk(r.Ops.DecIncGraph(r.Wire, r.Wire.Length), ci);
                census.Add(r.Id + " " + r.Content + " " + mode + " | facade: " + cf + " | incumbent: " + ci);
                // the strings alone: the facade graph's strings, UTF-8, one buffer
                var ms = new MemoryStream(); var offs = new List<int>(); var lens = new List<int>();
                foreach (var s in cf.All) { var u = Strict.GetBytes(s); offs.Add((int)ms.Position); lens.Add(u.Length); ms.Write(u, 0, u.Length); }
                var strs = (ms.ToArray(), offs.ToArray(), lens.ToArray());
                var arms = Arms.Where(a => !(a == "pull" && retain)).ToArray();
                var ops = arms.Select(a => Op(r, a, retain, graph, strs)).ToArray();
                foreach (var f in ops) f();   // every arm runs once (errors surface here)
                long Iters(int k)
                {
                    long it = 1; double t;
                    while (true)
                    {
                        long w0 = ProcCpu.Wall();
                        for (long i = 0; i < it; i++) ops[k]();
                        t = (ProcCpu.Wall() - w0) / 1e6;
                        if (t > blockMs / 4 || it > 1 << 24) break;
                        it *= 2;
                    }
                    return Math.Max(1, (long)(it * blockMs / Math.Max(t, 1e-3)));
                }
                var iters = new long[ops.Length];
                long ws = ProcCpu.Wall();
                while ((ProcCpu.Wall() - ws) / 1e6 < 1000) for (int k = 0; k < ops.Length; k++) iters[k] = Iters(k);
                var per = arms.Select(_ => new List<double[]>()).ToArray();
                for (int round = 0; round < rounds; round++)
                    for (int j = 0; j < ops.Length; j++)
                    {
                        int k = (j + round) % ops.Length;
                        int g0 = GC.CollectionCount(0), g1 = GC.CollectionCount(1), g2 = GC.CollectionCount(2);
                        long a0 = GC.GetAllocatedBytesForCurrentThread(), f0 = ProcCpu.MinFlt(), c0 = ProcCpu.Ns();
                        for (long i = 0; i < iters[k]; i++) ops[k]();
                        long c1 = ProcCpu.Ns(), f1 = ProcCpu.MinFlt(), a1 = GC.GetAllocatedBytesForCurrentThread();
                        double it = iters[k];
                        var v = new[] { (c1 - c0) / it, (a1 - a0) / it, (GC.CollectionCount(0) - g0) / it, (GC.CollectionCount(1) - g1) / it, (GC.CollectionCount(2) - g2) / it, (f1 - f0) / it };
                        per[k].Add(v);
                        lines.Add(string.Join("\t", r.Id, r.Content, mode, arms[k], round + 1, string.Join("\t", v.Select(x => x.ToString("G6", CultureInfo.InvariantCulture))), iters[k]));
                    }
                string Fm(List<double[]> v) { var a = v.Select(x => x[0] / 1000).OrderBy(x => x).ToList(); return string.Format(CultureInfo.InvariantCulture, "{0:G4} [{1:G4}-{2:G4}]", a[a.Count / 2], a[0], a[^1]); }
                double Med(List<double[]> v, int i) { var a = v.Select(x => x[i]).OrderBy(x => x).ToList(); return a[a.Count / 2]; }
                md.Add(string.Format(CultureInfo.InvariantCulture, "| {0} | {1} | {2} | {3} |", r.Id, r.Content, mode,
                    string.Join(" | ", Arms.Select(a => { int k = Array.IndexOf(arms, a); return k < 0 ? "-" : Fm(per[k]) + string.Format(CultureInfo.InvariantCulture, "; {0:G4} B; g0 {1:G3}; f {2:G3}", Med(per[k], 1), Med(per[k], 2), Med(per[k], 5)); }))));
                Console.WriteLine(md[^1]);
            }
        File.WriteAllLines(outp, lines);
        File.WriteAllLines(Path.ChangeExtension(outp, ".census.txt"), census);
        var head = new List<string>
        {
            "# D21 s8 decode attribution: process CPU us per op, median [min-max] over " + rounds + " rounds (one process, arms interleaved and rotated); bytes allocated per op; gen0 collections per op; minor faults per op",
            "",
            "CONTAINER INSTRUMENTATION. Arms: see DecAttr.cs's header. Build: " + Armonik.Ffi.Harness.AbiVariant.Name + ".",
            "",
            "| row | content | mode | " + string.Join(" | ", Arms) + " |",
            "|---|---|---|" + string.Concat(Arms.Select(_ => "---|")),
        };
        File.WriteAllLines(Path.ChangeExtension(outp, ".md"), head.Concat(md));
        return 0;
    }

    // ---------------------------------------------------------------- crossings (counting build)
    public static int Counts(string outp)
    {
#if !AK_HOST_COUNT
        Console.Error.WriteLine("--decattr-counts needs the counting build");
        return 2;
#else
        var o = new List<string> { "# D21 s8: crossings per attribution arm, one call each (counting build, " + Armonik.Ffi.Harness.AbiVariant.Name + "): fwd = every exported entry point called (resets included), rev = the generated callbacks' tally + G.AttrCalls (noop/new-zero) + grow calls" };
        foreach (var r in Rows(null))
            foreach (var mode in Modes)
            {
                bool retain = mode == "retain";
                object graph = r.Ops.DecFfiGraph(r.Wire, r.Wire.Length, retain);
                foreach (var arm in new[] { "ref", "noop", "parse", "parsev", "pull", "pparse" })
                {
                    if (arm == "pull" && retain) continue;
                    var f = Op(r, arm, retain, graph, (Array.Empty<byte>(), Array.Empty<int>(), Array.Empty<int>()));
                    f();
                    Armonik.Ffi.Harness.Abi.EntryReset(); r.Ops.FfiCallsReset(); Armonik.Ffi.Harness.G.AttrCalls = 0;
#if !AK_NO_UNKNOWN_FIELDS
                    Armonik.Ffi.Harness.UnkHost.Grows = 0;
                    long Gr() => Armonik.Ffi.Harness.UnkHost.Grows;
#else
                    long Gr() => 0;
#endif
                    f();
                    var e = Armonik.Ffi.Harness.Abi.EntryCounts().OrderBy(x => x.Name, StringComparer.Ordinal).ToList();
                    o.Add(string.Format(CultureInfo.InvariantCulture, "{0} {1} {2} {3} | fwd {4} rev {5} grow {6} | {7}", r.Id, r.Content, mode, arm, e.Sum(x => x.Calls),
                        r.Ops.FfiReverse() + Armonik.Ffi.Harness.G.AttrCalls, Gr(), string.Join(" ", e.Select(x => x.Name + "=" + x.Calls))));
                }
            }
        File.WriteAllLines(outp, o);
        foreach (var l in o) Console.WriteLine(l);
        return 0;
#endif
    }
}
