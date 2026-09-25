// The codec suite's cases (design/CAMPAIGN.md requirements 7, 8, 9, 10), one BenchmarkDotNet
// benchmark case each. A case is "arm|dir|payload|content|mode"; the calls behind it are the
// generated per-root operations (Generated/CampaignOps.cs, gen/cs_campaign.py).

using System;
using System.Buffers;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Armonik.Ffi.Campaign;
using Armonik.Ffi.Facade;
using Armonik.Ffi.Harness;

namespace Armonik.Ffi.Bdn;

public sealed class Case
{
    public string Arm, Dir, Payload, Content, Mode;
    public string Key => string.Join("|", Arm, Dir, Payload, Content, Mode);
    public static Case Parse(string k) { var f = k.Split('|'); return new Case { Arm = f[0], Dir = f[1], Payload = f[2], Content = f[3], Mode = f[4] }; }
}

public static class Cases
{
    /// The arms in their launch-1 order; launch n rotates it by n - 1 (requirement 22).
    public static readonly string[] Arms = { "incumbent-prod", "incumbent-best", "host-gen", "core-ffi", "core-ffi-pull" };

#if AK_NO_UNKNOWN_FIELDS
    // WP5 step 10: the NO-UNKNOWN build (unknown fields compiled out of the core, the
    // binding and the managed codec, which is rendered from the drop plan: host-gen here is
    // plan-generated too). Mode `no-unknown`; the incumbent arms are the in-process controls.
    private static readonly string[] EncArms = { "incumbent-prod:default", "incumbent-best:default", "host-gen:no-unknown", "core-ffi:no-unknown" };
    private static readonly string[] DecArms = { "incumbent-prod:default", "incumbent-best:default", "host-gen:no-unknown", "core-ffi:no-unknown", "core-ffi-pull:no-unknown" };
    private static readonly string[] UnkArms = { "incumbent-prod:default", "host-gen:no-unknown", "core-ffi:no-unknown" };
#else
    private static readonly string[] EncArms = { "incumbent-prod:default", "incumbent-best:default", "host-gen:drop", "host-gen:retain", "core-ffi:drop", "core-ffi:retain" };
    private static readonly string[] DecArms = { "incumbent-prod:default", "incumbent-best:default", "host-gen:drop", "host-gen:retain", "core-ffi:drop", "core-ffi:retain", "core-ffi-pull:drop" };
    private static readonly string[] UnkArms = { "incumbent-prod:default", "host-gen:drop", "host-gen:retain", "core-ffi:drop", "core-ffi:retain" };
#endif

    /// The process unit (JOURNAL 51): one process per "arm:mode", so each process holds at
    /// most a few hundred cases. BDN keeps tens of kB per case alive for the whole run, and
    /// every forced GC it runs between iterations walks that live heap: with all 1,780 cases
    /// in one process the per-case overhead tripled. Null = every case (a single process).
    public static string Unit;

    /// The units of a launch, in its order: the arm order rotated by launch - 1
    /// (requirement 22), and within an arm the modes rotated by launch - 1 too.
    public static List<string> Units(int launch)
    {
        int k = (launch - 1) % Arms.Length;
        var arms = Arms.Skip(k).Concat(Arms.Take(k));
        var all = EncArms.Concat(DecArms).Concat(UnkArms).Distinct().ToList();
        var o = new List<string>();
        foreach (var a in arms)
        {
            var modes = all.Where(x => x.StartsWith(a + ":", StringComparison.Ordinal)).ToList();
            int r = (launch - 1) % modes.Count;
            o.AddRange(modes.Skip(r).Concat(modes.Take(r)));
        }
        return o;
    }

    /// Two sacrificial cases run first in every process: copies of its first two cases, with
    /// content "prime". They are timed by BDN but never exported. They absorb the tier-up of
    /// runtime helpers that BDN's own engine touches first (cast cache, span fill): without
    /// them the first two cases of each process read back hot code at tier 0 (JOURNAL 51).
    public const string Prime = "prime";
    public static IEnumerable<string> PrimeCases() =>
        All().Take(2).Select(k => { var c = Case.Parse(k); c.Content = Prime; return c.Key; });

    private static bool InUnit(string am) => Unit == null || am == Unit;

    public static string CorpusDir()
    {
        var d = AppContext.BaseDirectory;
        for (int i = 0; i < 12 && d != null; i++)
        {
            var c = Path.Combine(d, "ffi", "corpus", "generated");
            if (File.Exists(Path.Combine(c, "manifest.json"))) return c;
            d = Path.GetDirectoryName(d.TrimEnd(Path.DirectorySeparatorChar));
        }
        throw new DirectoryNotFoundException("ffi/corpus/generated");
    }

    private static List<string> _unk;
    private static Dictionary<string, (string Root, string File)> _rows;
    /// Requirement 7: every corpus U-* row whose root this slice implements, disputed rows
    /// excluded (accept rows only: a row that is refused has nothing to decode or re-encode).
    /// Only (root, file) per row is kept: the parsed manifest (about 40 MB of JSON tree) is
    /// dropped, because every forced GC BenchmarkDotNet runs between iterations walks the
    /// live heap, and its size set the per-case overhead (JOURNAL 51).
    public static List<string> UnknownRows()
    {
        if (_unk != null) return _unk;
        var man = Json.Parse(File.ReadAllText(Path.Combine(CorpusDir(), "manifest.json")))["vectors"];
        _unk = new List<string>();
        _rows = new Dictionary<string, (string, string)>(StringComparer.Ordinal);
        foreach (var id in man.Keys.OrderBy(x => x, StringComparer.Ordinal))
        {
            var v = man[id];
            if (!id.StartsWith("U-", StringComparison.Ordinal) || v["verdict"].AsString == "disputed" || v["expect"].AsString != "accept") continue;
            if (OpsTable.ForRoot(v["root"].AsString) == null) continue;
            _unk.Add(id);
            _rows[id] = (v["root"].AsString, v["file"].AsString);
        }
        return _unk;
    }

    public static IEnumerable<string> All()
    {
        var only = Environment.GetEnvironmentVariable("AK_BDN_ONLY");   // payload ids, comma-separated
        var keep = string.IsNullOrWhiteSpace(only) ? null : new HashSet<string>(only.Split(','), StringComparer.Ordinal);
        foreach (var pid in OpsTable.Payloads)
        {
            if (keep != null && !keep.Contains(pid)) continue;
            var sets = pid == "P1.2" || pid == "P2.2" ? new[] { 0, 1, 2 } : new[] { 0 };
            foreach (var cs in sets)
            {
                foreach (var am in EncArms.Where(InUnit)) { var f = am.Split(':'); yield return string.Join("|", f[0], "encode", pid, Values.SetNames[cs], f[1]); }
                foreach (var dir in new[] { "decode", "decode-read" })
                    foreach (var am in DecArms.Where(InUnit)) { var f = am.Split(':'); yield return string.Join("|", f[0], dir, pid, Values.SetNames[cs], f[1]); }
            }
        }
        if (Environment.GetEnvironmentVariable("AK_BDN_NO_UNKNOWN") == "1") yield break;
        foreach (var id in UnknownRows())
        {
            if (keep != null && !keep.Contains(id)) continue;
            foreach (var dir in new[] { "decode", "decode-read", "decode-reencode" })
                foreach (var am in UnkArms.Where(InUnit)) { var f = am.Split(':'); yield return string.Join("|", f[0], dir, id, "corpus", f[1]); }
        }
    }

    private static void Same(byte[] got, byte[] want, string what)
    {
        if (!got.AsSpan().SequenceEqual(want)) throw new InvalidOperationException("byte identity failed before timing: " + what);
    }

    /// Requirement 26 inside the timed process: every timed ENCODE arm is byte-identical to
    /// the incumbent on every payload and content set, and every arm accepts every unknown
    /// row, before BenchmarkDotNet runs anything. Throws otherwise.
    public static int Verify()
    {
        int n = 0;
        foreach (var pid in OpsTable.Payloads)
            foreach (var cs in pid == "P1.2" || pid == "P2.2" ? new[] { 0, 1, 2 } : new[] { 0 })
            {
                Values.ContentSet = cs;
                var ops = OpsTable.ForPayload(pid);
                Values.ContentSet = Values.Ascii;
                var wire = ops.IncumbentBytes();
                var e = Enc.New(Armonik.Ffi.Facade.Codec.Sites, wire.Length + 4096);
                ops.EncHost(ref e);
                Same(e.ToArray(), wire, pid + " host-gen");
                Same(ops.EncFfiBytes(false), wire, pid + " core-ffi drop");
#if !AK_NO_UNKNOWN_FIELDS
                Same(ops.EncFfiBytes(true), wire, pid + " core-ffi retain");
#endif
                var w = new BufWriter(wire.Length + 4096);
                ops.EncIncProd(w);
                Same(w.WrittenSpan.ToArray(), wire, pid + " incumbent-prod");
                ops.EncIncBest(w);
                Same(w.WrittenSpan.ToArray(), wire, pid + " incumbent-best");
                n += 5;
            }
        foreach (var id in UnknownRows())
        {
            var (ops, b) = Row(id);
#if AK_NO_UNKNOWN_FIELDS
            // The no-unknown build: every arm accepts the row, and host-gen and core-ffi both
            // re-encode it in the DROPPED form, the same bytes.
            ops.DecIncBest(b, b.Length, true); ops.DecHost(b, b.Length, false, true); ops.DecFfi(b, b.Length, false, true);
            Same(ops.RtFfi(b, b.Length, false), ops.RtHost(b, b.Length, false), id + " core-ffi no-unknown vs host-gen no-unknown (decode-reencode, dropped form)");
            n += 4;
#else
            ops.DecIncBest(b, b.Length, true); ops.DecHost(b, b.Length, false, true); ops.DecHost(b, b.Length, true, true);
            ops.DecFfi(b, b.Length, false, true); ops.DecFfi(b, b.Length, true, true);
            n += 5;
            // Requirement 10 (decision 11, WP5 step 9): the timed RETAIN arms keep the unknown
            // fields. host-gen retain and core-ffi retain re-encode to the same bytes, and to
            // the incumbent's (Google.Protobuf retains) wherever the row's unknowns are not
            // inside a map entry (the facade map has no bag: U-map-entry, which is disputed
            // and never reaches this list).
            var inc = ops.RtIncBytes(b);
            Same(ops.RtFfi(b, b.Length, true), ops.RtHost(b, b.Length, true), id + " core-ffi retain vs host-gen retain (decode-reencode)");
            Same(ops.RtFfi(b, b.Length, true), inc, id + " core-ffi retain vs incumbent (decode-reencode)");
            n += 2;
#endif
        }
        return n;
    }

    private static (RootOps, byte[]) Row(string id)
    {
        UnknownRows();
        var (root, file) = _rows[id];
        return (OpsTable.ForRoot(root), File.ReadAllBytes(Path.Combine(CorpusDir(), file)));
    }

    /// The operation a case times: one call, returning something the engine consumes.
    public static Func<long> Build(Case c)
    {
        RootOps ops;
        byte[] wire;
        if (c.Content == "corpus") (ops, wire) = Row(c.Payload);
        else
        {
            Values.ContentSet = c.Content == Prime ? Values.Ascii : Array.IndexOf(Values.SetNames, c.Content);
            ops = OpsTable.ForPayload(c.Payload);
            Values.ContentSet = Values.Ascii;
            wire = ops.IncumbentBytes();
        }
        int len = wire.Length;
        var seq = new ReadOnlySequence<byte>(wire);
        bool retain = c.Mode == "retain";
        bool read = c.Dir == "decode-read";
        var w = new BufWriter(len * 2 + 4096);
        switch (c.Arm + ":" + c.Dir)
        {
            case "incumbent-prod:encode": return () => ops.EncIncProd(w);
            case "incumbent-best:encode": return () => ops.EncIncBest(w);
            case "host-gen:encode": { var box = new EncBox(len); return () => box.Run(ops); }
            case "core-ffi:encode": return () => ops.EncFfi(retain);
            case "incumbent-prod:decode": case "incumbent-prod:decode-read": return () => ops.DecIncProd(seq, read);
            case "incumbent-best:decode": case "incumbent-best:decode-read": return () => ops.DecIncBest(wire, len, read);
            case "host-gen:decode": case "host-gen:decode-read": return () => ops.DecHost(wire, len, retain, read);
            case "core-ffi:decode": case "core-ffi:decode-read": return () => ops.DecFfi(wire, len, retain, read);
            case "core-ffi-pull:decode": case "core-ffi-pull:decode-read": return () => ops.DecFfiPull(wire, len, read);
            case "incumbent-prod:decode-reencode": return () => ops.RtIncProd(seq, w);
            case "host-gen:decode-reencode": return () => ops.RtHost(wire, len, retain).Length;
            case "core-ffi:decode-reencode": return () => ops.RtFfi(wire, len, retain).Length;
            default: throw new ArgumentException("no case " + c.Key);
        }
    }

    /// `Enc` is a mutable struct used by ref; a box keeps one per case.
    private sealed class EncBox
    {
        private Enc _e;
        public EncBox(int len) { _e = Enc.New(Armonik.Ffi.Facade.Codec.Sites, len + 4096); }
        public long Run(RootOps ops) => ops.EncHost(ref _e);
    }
}
