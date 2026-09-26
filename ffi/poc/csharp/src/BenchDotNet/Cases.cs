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
    // plan-generated too). Mode `no-unknown`; the incumbent arms run as their own units
    // (every unit is its own process, so they are cross-process controls, not in-process ones).
    private static readonly string[] EncArms = { "incumbent-prod:default", "incumbent-best:default", "host-gen:no-unknown", "core-ffi:no-unknown" };
    private static readonly string[] DecArms = { "incumbent-prod:default", "incumbent-best:default", "host-gen:no-unknown", "core-ffi:no-unknown", "core-ffi-pull:no-unknown" };
    private static readonly string[] UnkArms = { "incumbent-prod:default", "incumbent-best:default", "host-gen:no-unknown", "core-ffi:no-unknown" };
#else
    private static readonly string[] EncArms = { "incumbent-prod:default", "incumbent-best:default", "host-gen:drop", "host-gen:retain", "core-ffi:drop", "core-ffi:retain" };
    private static readonly string[] DecArms = { "incumbent-prod:default", "incumbent-best:default", "host-gen:drop", "host-gen:retain", "core-ffi:drop", "core-ffi:retain", "core-ffi-pull:drop" };
    private static readonly string[] UnkArms = { "incumbent-prod:default", "incumbent-best:default", "host-gen:drop", "host-gen:retain", "core-ffi:drop", "core-ffi:retain" };
#endif

    /// CAMPAIGN req 7 (R-H26): the payloads that carry the Latin-1 and wide content sets.
    public static readonly string[] ContentPayloads = { "P1.2", "P2.2", "P2.4" };
    public static int[] SetsOf(string pid) => Array.IndexOf(ContentPayloads, pid) >= 0 ? new[] { 0, 1, 2 } : new[] { 0 };

    /// CAMPAIGN req 11 (R-H29): the encode variants of a shapes payload, as directions.
    /// end state: buf = the bytes in a reused buffer (incumbent: the BufWriter; host-gen: its
    /// Enc; core-ffi: the core's encode buffer); transport = the form the arm's Grpc.Net
    /// marshaller hands to the call (GrpcFrame.cs). input: pool = a pool of distinct graphs,
    /// larger than the last-level cache (PoolBytes), round robin; hot = one graph.
    public static readonly string[] EncDirs = { "encode", "encode-hot", "encode-transport", "encode-transport-hot" };
    public static bool IsEnc(string dir) => dir.StartsWith("encode", StringComparison.Ordinal);
    public static bool IsPool(string dir) => dir == "encode" || dir == "encode-transport";
    public static bool IsTransport(string dir) => dir.StartsWith("encode-transport", StringComparison.Ordinal);
    /// incumbent-best has no gRPC path of its own, so it has no transport rows.
    private static bool HasTransport(string arm) => arm != "incumbent-best";

    /// The pool's size: its graphs' retained heap is at least this (2 x the last-level cache,
    /// AK_LLC_BYTES, default 13.75 MB = the reference i9-7900X's L3; AK_POOL_BYTES overrides).
    public static long PoolBytes
    {
        get
        {
            var pb = Environment.GetEnvironmentVariable("AK_POOL_BYTES");
            if (!string.IsNullOrEmpty(pb)) return long.Parse(pb, System.Globalization.CultureInfo.InvariantCulture);
            var llc = Environment.GetEnvironmentVariable("AK_LLC_BYTES");
            return 2 * (string.IsNullOrEmpty(llc) ? 14417920L : long.Parse(llc, System.Globalization.CultureInfo.InvariantCulture));
        }
    }
    /// The pre-warm builds pools of at most this many graphs (0 = the full pool).
    public static int PoolCap;
    /// Per case: (graphs, retained bytes) of its pool, for the export.
    public static readonly Dictionary<string, (int N, long Bytes)> Pools = new Dictionary<string, (int, long)>();

    /// The process unit (JOURNAL 51): one process per "arm:mode", so each process holds at
    /// most a few hundred cases. BDN keeps tens of kB per case alive for the whole run, and
    /// every forced GC it runs between iterations walks that live heap: with all 1,780 cases
    /// in one process the per-case overhead tripled. Null = every case (a single process).
    public static string Unit;

    /// The units of a launch, in its order: the arm order rotated by launch - 1
    /// (requirement 22), and within an arm the modes rotated by launch - 1 too.
    /// R-H23 (CAMPAIGN req 22 as amended 2026-09-26): the order is RANDOMISED where the
    /// framework allows it. The unit order of a launch is a seeded shuffle (seed = launch), so
    /// adjacency changes between launches; the seed and the order are in every header.
    public static int UnitSeed(int launch) => launch * 7919;
    public static List<string> Units(int launch)
    {
        var all = EncArms.Concat(DecArms).Concat(UnkArms).Distinct().OrderBy(x => x, StringComparer.Ordinal).ToList();
        var rng = new Random(UnitSeed(launch));
        return all.OrderBy(_ => rng.Next()).ToList();
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

    /// U-* row directions (CAMPAIGN req 7 as amended, R-H27): encode (a graph the arm itself
    /// decoded from the row, untimed, re-encoded into a reused buffer: `encode-hot`), decode
    /// and decode-read; decode-reencode stays as a labelled extra (no incumbent-best row).
    public static readonly string[] UnkDirs = { "encode-hot", "decode", "decode-read", "decode-reencode" };

    public static IEnumerable<string> All()
    {
        var only = Environment.GetEnvironmentVariable("AK_BDN_ONLY");   // payload ids, comma-separated
        var keep = string.IsNullOrWhiteSpace(only) ? null : new HashSet<string>(only.Split(','), StringComparer.Ordinal);
        foreach (var pid in OpsTable.Payloads)
        {
            if (keep != null && !keep.Contains(pid)) continue;
            foreach (var cs in SetsOf(pid))
            {
                foreach (var dir in EncDirs)
                    foreach (var am in EncArms.Where(InUnit))
                    {
                        var f = am.Split(':');
                        if (IsTransport(dir) && !HasTransport(f[0])) continue;
                        yield return string.Join("|", f[0], dir, pid, Values.SetNames[cs], f[1]);
                    }
                foreach (var dir in new[] { "decode", "decode-read" })
                    foreach (var am in DecArms.Where(InUnit)) { var f = am.Split(':'); yield return string.Join("|", f[0], dir, pid, Values.SetNames[cs], f[1]); }
            }
        }
        if (Environment.GetEnvironmentVariable("AK_BDN_NO_UNKNOWN") == "1") yield break;
        var ul = Environment.GetEnvironmentVariable("AK_BDN_UROWS");   // smoke: keep this many rows, spread evenly
        var rows = UnknownRows();
        if (!string.IsNullOrEmpty(ul) && int.TryParse(ul, out int lim) && lim > 0 && lim < rows.Count)
            rows = Enumerable.Range(0, lim).Select(k => rows[k * rows.Count / lim]).ToList();
        foreach (var id in rows)
        {
            if (keep != null && !keep.Contains(id)) continue;
            foreach (var dir in UnkDirs)
                foreach (var am in UnkArms.Where(InUnit))
                {
                    var f = am.Split(':');
                    if (dir == "decode-reencode" && f[0] == "incumbent-best") continue;
                    yield return string.Join("|", f[0], dir, id, "corpus", f[1]);
                }
        }
    }

    /// FromWire's `how` for an arm and mode (RootOps.FromWire).
    public static int How(string arm, string mode) =>
        arm.StartsWith("incumbent", StringComparison.Ordinal) ? 0
        : arm == "host-gen" ? (mode == "retain" ? 2 : 1)
        : (mode == "retain" ? 4 : 3);

    private static void Same(byte[] got, byte[] want, string what)
    {
        if (!got.AsSpan().SequenceEqual(want)) throw new InvalidOperationException("byte identity failed before timing: " + what);
    }

    /// Requirement 26 inside the timed process: every timed ENCODE arm and variant is
    /// byte-identical to the incumbent on every payload and content set (the transport form:
    /// the 5-byte gRPC header then the same bytes), every arm accepts every unknown row, and
    /// every arm's encode of a row it decoded gives the expected form, before BenchmarkDotNet
    /// runs anything. Throws otherwise.
    public static int Verify()
    {
        int n = 0;
        var fr = new GrpcFrame { Capture = true };
        void SameFrame(string what, byte[] wire)
        {
            var got = fr.LastCopy;
            if (got == null || got.Length != wire.Length + GrpcFrame.HeaderSize || got[0] != 0
                || System.Buffers.Binary.BinaryPrimitives.ReadUInt32BigEndian(got.AsSpan(1, 4)) != (uint)wire.Length
                || !got.AsSpan(GrpcFrame.HeaderSize).SequenceEqual(wire))
                throw new InvalidOperationException("byte identity failed before timing: " + what + " (transport form)");
            fr.LastCopy = null;
        }
        foreach (var pid in OpsTable.Payloads)
            foreach (var cs in SetsOf(pid))
            {
                Values.ContentSet = cs;
                var ops = OpsTable.ForPayload(pid);
                // the pool's graphs: a second, distinct graph of the same payload encodes the same
                var ops2 = OpsTable.ForPayload(pid);
                ops2.SetPool(new[] { OpsTable.BuildF(pid), OpsTable.BuildF(pid) }, new[] { OpsTable.BuildG(pid), OpsTable.BuildG(pid) });
                ops2.Next();
                Values.ContentSet = Values.Ascii;
                var wire = ops.IncumbentBytes();
                var what = pid + "/" + Values.SetNames[cs];
                foreach (var o in new[] { ops, ops2 })
                {
                    var e = Enc.New(Armonik.Ffi.Facade.Codec.Sites, wire.Length + 4096);
                    o.EncHost(ref e, false);
                    Same(e.ToArray(), wire, what + " host-gen drop");
#if !AK_NO_UNKNOWN_FIELDS
                    o.EncHost(ref e, true);
                    Same(e.ToArray(), wire, what + " host-gen retain");
                    Same(o.EncFfiBytes(true), wire, what + " core-ffi retain");
                    o.EncHostTransport(true, fr); SameFrame(what + " host-gen retain", wire);
                    o.EncFfiTransport(true, fr); SameFrame(what + " core-ffi retain", wire);
#endif
                    Same(o.EncFfiBytes(false), wire, what + " core-ffi drop");
                    var w = new BufWriter(wire.Length + 4096);
                    o.EncIncProd(w);
                    Same(w.WrittenSpan.ToArray(), wire, what + " incumbent-prod");
                    o.EncIncBest(w);
                    Same(w.WrittenSpan.ToArray(), wire, what + " incumbent-best");
                    o.EncIncTransport(fr); SameFrame(what + " incumbent-prod", wire);
                    o.EncHostTransport(false, fr); SameFrame(what + " host-gen drop", wire);
                    o.EncFfiTransport(false, fr); SameFrame(what + " core-ffi drop", wire);
                    n += 11;
                }
            }
        foreach (var id in UnknownRows())
        {
            var (ops, b) = Row(id);
            var w = new BufWriter(b.Length * 2 + 4096);
            byte[] IncEnc(RootOps o, bool best) { if (best) o.EncIncBest(w); else o.EncIncProd(w); return w.WrittenSpan.ToArray(); }
            byte[] HostEnc(RootOps o, bool retain) { var e = Enc.New(Armonik.Ffi.Facade.Codec.Sites, b.Length + 4096); o.EncHost(ref e, retain); return e.ToArray(); }
            var inc = IncEnc(ops.FromWire(b, 0), false);
            Same(IncEnc(ops.FromWire(b, 0), true), inc, id + " incumbent-best encode vs incumbent-prod");
#if AK_NO_UNKNOWN_FIELDS
            // The no-unknown build: every arm accepts the row, and host-gen and core-ffi both
            // re-encode it in the DROPPED form, the same bytes, by decode-reencode and by encode.
            ops.DecIncBest(b, b.Length, true); ops.DecHost(b, b.Length, false, true); ops.DecFfi(b, b.Length, false, true);
            Same(ops.RtFfi(b, b.Length, false), ops.RtHost(b, b.Length, false), id + " core-ffi no-unknown vs host-gen no-unknown (decode-reencode, dropped form)");
            Same(ops.FromWire(b, 3).EncFfiBytes(false), HostEnc(ops.FromWire(b, 1), false), id + " core-ffi no-unknown vs host-gen no-unknown (encode, dropped form)");
            n += 6;
#else
            ops.DecIncBest(b, b.Length, true); ops.DecHost(b, b.Length, false, true); ops.DecHost(b, b.Length, true, true);
            ops.DecFfi(b, b.Length, false, true); ops.DecFfi(b, b.Length, true, true);
            n += 6;
            // Requirement 10 (decision 11, WP5 step 9): the timed RETAIN arms keep the unknown
            // fields. host-gen retain and core-ffi retain re-encode to the same bytes, and to
            // the incumbent's (Google.Protobuf retains) wherever the row's unknowns are not
            // inside a map entry (the facade map has no bag: U-map-entry, which is disputed
            // and never reaches this list). Likewise the ENCODE direction (req 7, R-H27): each
            // arm encodes the graph it decoded from the row; drop arms give the dropped form.
            Same(ops.RtFfi(b, b.Length, true), ops.RtHost(b, b.Length, true), id + " core-ffi retain vs host-gen retain (decode-reencode)");
            Same(ops.RtFfi(b, b.Length, true), ops.RtIncBytes(b), id + " core-ffi retain vs incumbent (decode-reencode)");
            Same(ops.FromWire(b, 4).EncFfiBytes(true), inc, id + " core-ffi retain encode vs incumbent");
            Same(HostEnc(ops.FromWire(b, 2), true), inc, id + " host-gen retain encode vs incumbent");
            Same(ops.FromWire(b, 3).EncFfiBytes(false), HostEnc(ops.FromWire(b, 1), false), id + " core-ffi drop vs host-gen drop (encode, dropped form)");
            n += 5;
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

    /// A graph's retained heap: graphs are built, 8 then doubling, until the live heap grew by
    /// at least 1 MiB (a full GC before and after), so a small graph is not measured in the
    /// noise of the allocation contexts; at least 24 bytes (one object).
    private static long ProbeFootprint(Func<object> mk)
    {
        for (int k = 8; ; k *= 2)
        {
            long before = GC.GetTotalMemory(true);
            var probe = new object[k];
            for (int i = 0; i < k; i++) probe[i] = mk();
            long grew = GC.GetTotalMemory(true) - before;
            GC.KeepAlive(probe);
            if (grew >= 1 << 20 || k >= 1 << 16) return Math.Max(24, grew / k);
        }
    }

    /// The ops object behind the last Build (the counting run reads its host tally).
    public static RootOps LastOps;

    /// The operation a case times: one call, returning something the engine consumes. Graph
    /// construction (pools included) happens here, in the case's setup, never in the timed call.
    public static Func<long> Build(Case c)
    {
        RootOps ops;
        byte[] wire;
        bool retain = c.Mode == "retain";
        if (c.Content == "corpus")
        {
            (ops, wire) = Row(c.Payload);
            // req 7: an encode row encodes the graph THIS arm decoded from the row (untimed).
            if (IsEnc(c.Dir)) ops = ops.FromWire(wire, How(c.Arm, c.Mode));
        }
        else
        {
            int cs = c.Content == Prime ? Values.Ascii : Array.IndexOf(Values.SetNames, c.Content);
            Values.ContentSet = cs;
            try
            {
                ops = OpsTable.ForPayload(c.Payload);
                if (IsPool(c.Dir)) BuildPool(c, ops);
            }
            finally { Values.ContentSet = Values.Ascii; }
            wire = ops.IncumbentBytes();
        }
        int len = wire.Length;
        var seq = new ReadOnlySequence<byte>(wire);
        bool read = c.Dir == "decode-read";
        var w = new BufWriter(len * 2 + 4096);
        var fr = new GrpcFrame();
        string d = IsEnc(c.Dir) ? (IsTransport(c.Dir) ? "encode-transport" : "encode") : c.Dir;
        LastOps = ops;
        switch (c.Arm + ":" + d)
        {
            // Every encode row moves to the next graph first (a hot input is a pool of one).
            case "incumbent-prod:encode": return () => { ops.Next(); return ops.EncIncProd(w); };
            case "incumbent-best:encode": return () => { ops.Next(); return ops.EncIncBest(w); };
            case "host-gen:encode": { var box = new EncBox(len); return () => { ops.Next(); return box.Run(ops, retain); }; }
            case "core-ffi:encode": return () => { ops.Next(); return ops.EncFfi(retain); };
            case "incumbent-prod:encode-transport": return () => { ops.Next(); return ops.EncIncTransport(fr); };
            case "host-gen:encode-transport": return () => { ops.Next(); return ops.EncHostTransport(retain, fr); };
            case "core-ffi:encode-transport": return () => { ops.Next(); return ops.EncFfiTransport(retain, fr); };
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

    /// req 11's pool: distinct graphs of the payload whose RETAINED heap is at least PoolBytes
    /// (measured, not estimated: 8 graphs are built and the live heap read with a full GC, the
    /// count scaled from that; the pool is then built from fresh graphs and ITS retained bytes
    /// read the same way, before and after).
    private static void BuildPool(Case c, RootOps ops)
    {
        bool gp = c.Arm.StartsWith("incumbent", StringComparison.Ordinal);
        Func<object> mk = gp ? () => OpsTable.BuildG(c.Payload) : () => OpsTable.BuildF(c.Payload);
        long per = ProbeFootprint(mk);
        long want = PoolBytes;
        int n = (int)Math.Min(int.MaxValue / 2, Math.Max(2, (want + per - 1) / per));
        if (PoolCap > 0) n = Math.Min(n, PoolCap);
        long before = GC.GetTotalMemory(true);
        var list = new List<object>(n);
        for (int i = 0; i < n; i++) list.Add(mk());
        long bytes = GC.GetTotalMemory(true) - before;
        // topped up until the measured pool reaches the target (the probe is an estimate)
        while (PoolCap == 0 && bytes < want && list.Count < int.MaxValue / 2)
        {
            long more = (want - bytes + per - 1) / per + 1;
            for (long i = 0; i < more; i++) list.Add(mk());
            bytes = GC.GetTotalMemory(true) - before;
        }
        var pool = list.ToArray();
        n = pool.Length;
        if (gp) ops.SetPool(null, pool); else ops.SetPool(pool, null);
        Pools[c.Key] = (n, bytes);
    }

    /// `Enc` is a mutable struct used by ref; a box keeps one per case.
    private sealed class EncBox
    {
        private Enc _e;
        public EncBox(int len) { _e = Enc.New(Armonik.Ffi.Facade.Codec.Sites, len + 4096); }
        public long Run(RootOps ops, bool retain) => ops.EncHost(ref _e, retain);
    }
}
