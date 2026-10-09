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
    /// Step 9b (owner, 2026-10-09), AK_BDN_S9=1 (full build, decode rows): the owner's six arms:
    /// the incumbent retaining (its default parser) and discarding unknown fields
    /// (WithDiscardUnknownFields), core push retain / drop, core pull retain / drop; nothing else.
    /// D23 (owner, 2026-10-09), AK_BDN_S10=1: the same eight-arm set plus the FSM family
    /// (core-ffi-fsm retain / drop); implies S9's restrictions (decode rows, these arms only).
    public static readonly bool S10 = Environment.GetEnvironmentVariable("AK_BDN_S10") == "1";
    public static readonly bool S9 = Environment.GetEnvironmentVariable("AK_BDN_S9") == "1" || S10;

    /// The arms in their launch-1 order; launch n rotates it by n - 1 (requirement 22).
    public static readonly string[] Arms = { "incumbent-prod", "incumbent-best", "host-gen", "core-ffi", "core-ffi-pull", "core-ffi-fsm" };

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
    private static readonly string[] S9Arms = S10
        ? new[] { "incumbent-prod:default", "incumbent-prod:discard", "core-ffi:retain", "core-ffi:drop", "core-ffi-pull:retain", "core-ffi-pull:drop", "core-ffi-fsm:retain", "core-ffi-fsm:drop" }
        : new[] { "incumbent-prod:default", "incumbent-prod:discard", "core-ffi:retain", "core-ffi:drop", "core-ffi-pull:retain", "core-ffi-pull:drop" };
    private static readonly string[] DecArms = S9 ? S9Arms : new[] { "incumbent-prod:default", "incumbent-best:default", "host-gen:drop", "host-gen:retain", "core-ffi:drop", "core-ffi:retain", "core-ffi-pull:drop" };
    private static readonly string[] UnkArms = S9 ? S9Arms : new[] { "incumbent-prod:default", "incumbent-best:default", "host-gen:drop", "host-gen:retain", "core-ffi:drop", "core-ffi:retain" };
#endif

    /// CAMPAIGN section 4.0 (D18, owner 2026-10-03): AK_CAMPAIGN_GRID=core runs the campaign
    /// grid: arms incumbent-prod (full build only), core-ffi and host-gen (retain in the full
    /// build, no-unknown in its build); encode at end state (ii) with a hot input
    /// (encode-transport-hot) and decode-read; the 16 shapes (P7.1 decode only), Latin-1 and
    /// wide on P2.2 only; 7 U-* rows. Unset or `full`: today's grid (every extra).
    public static readonly bool CoreGrid = Environment.GetEnvironmentVariable("AK_CAMPAIGN_GRID") == "core";
#if AK_NO_UNKNOWN_FIELDS
    private static readonly string[] CoreArms = { "host-gen:no-unknown", "core-ffi:no-unknown" };
#else
    /// AK_BDN_DROP=1 (optimisation runs, a labelled extra): the drop units beside retain.
    private static readonly string[] CoreArms = Environment.GetEnvironmentVariable("AK_BDN_DROP") == "1"
        ? new[] { "incumbent-prod:default", "host-gen:retain", "core-ffi:retain", "host-gen:drop", "core-ffi:drop" }
        : new[] { "incumbent-prod:default", "host-gen:retain", "core-ffi:retain" };
#endif
    public static readonly string[] CoreURows = { "U-nested-before", "U-deep-u-repeated", "U-oneof-u-repeated",
        "U-wire-ListTaskSummaryResponse-tasks-as-wt5", "U-wire-UploadResultDataMessage-upload-as-wt5",
        "U-wire-ListMetricsResponse-batches-as-wt0", "U-wire-DualResponse-left-as-wt5" };
    /// AK_BDN_ARMS (comma list of arm names; narrowed exploration runs only): keep these arms.
    private static readonly string[] ArmsOnly = string.IsNullOrWhiteSpace(Environment.GetEnvironmentVariable("AK_BDN_ARMS")) ? null : Environment.GetEnvironmentVariable("AK_BDN_ARMS").Split(',');
    private static string[] G(string[] arms) => (CoreGrid && !S9 ? arms.Where(a => CoreArms.Contains(a)) : arms).Where(a => ArmsOnly == null || ArmsOnly.Contains(a.Split(':')[0])).ToArray();
    /// Section 4.0's encode row per arm (owner decision D1, 2026-10-04, optimisation step 1):
    /// end state (ii) is the form the arm's OWN transport cell receives. incumbent-prod: the
    /// Grpc.Net frame (cell A, encode-transport-hot); core-ffi: its encode left in the core's
    /// context, as cell Cf hands it to ak_call_unary_enc (encode-core-hot, no take, no frame);
    /// host-gen: its Enc buffer as cell Ef hands it to ak_call_unary (encode-core-hot). The
    /// Grpc.Net frame form of core-ffi and host-gen stays a labelled extra (the full grid, or
    /// AK_BDN_DIRS).
    private static string CoreEncDir(string arm) => arm == "core-ffi" || arm == "host-gen" ? "encode-core-hot" : "encode-transport-hot";
    /// AK_BDN_DIRS (comma list; narrowed exploration runs only): the directions to generate,
    /// replacing the grid's, each where it applies to the arm.
    private static readonly string[] DirsOverride = string.IsNullOrWhiteSpace(Environment.GetEnvironmentVariable("AK_BDN_DIRS")) ? null : Environment.GetEnvironmentVariable("AK_BDN_DIRS").Split(',');
    private static IEnumerable<string> EncDirsFor(string arm, string[] full)
    {
        var ds = DirsOverride != null ? DirsOverride.Where(IsEnc) : CoreGrid ? new[] { CoreEncDir(arm) } : full;
        return ds.Where(d => (!IsTransport(d) || HasTransport(arm)) && (!IsCoreForm(d) || HasCoreForm(arm)));
    }
    private static readonly string[] DecDirsG = DirsOverride != null ? DirsOverride.Where(d => !IsEnc(d)).ToArray() : CoreGrid ? new[] { "decode-read" } : new[] { "decode", "decode-read" };
    private static int[] SetsOfG(string pid) => CoreGrid ? (pid == "P2.2" ? new[] { 0, 1, 2 } : new[] { 0 }) : SetsOf(pid);

    /// CAMPAIGN req 7 (R-H26): the payloads that carry the Latin-1 and wide content sets.
    public static readonly string[] ContentPayloads = { "P1.2", "P2.2", "P2.4" };
    public static int[] SetsOf(string pid) => Array.IndexOf(ContentPayloads, pid) >= 0 ? new[] { 0, 1, 2 } : new[] { 0 };

    /// CAMPAIGN req 11 (R-H29): the encode variants of a shapes payload, as directions.
    /// end state: buf = the bytes in a reused buffer (incumbent: the BufWriter; host-gen: its
    /// Enc; core-ffi: the core's encode buffer); transport = the form the arm's Grpc.Net
    /// marshaller hands to the call (GrpcFrame.cs). input: pool = a pool of distinct graphs,
    /// larger than the last-level cache (PoolBytes), round robin; hot = one graph.
    /// encode-core / encode-core-hot (D1, 2026-10-04): end state (ii) for the CORE's transport,
    /// core-ffi and host-gen only (CoreEncDir).
    public static readonly string[] EncDirs = { "encode", "encode-hot", "encode-transport", "encode-transport-hot", "encode-core", "encode-core-hot" };
    public static bool IsEnc(string dir) => dir.StartsWith("encode", StringComparison.Ordinal);
    public static bool IsPool(string dir) => dir == "encode" || dir == "encode-transport" || dir == "encode-core";
    public static bool IsTransport(string dir) => dir.StartsWith("encode-transport", StringComparison.Ordinal);
    public static bool IsCoreForm(string dir) => dir.StartsWith("encode-core", StringComparison.Ordinal);
    private static bool HasCoreForm(string arm) => arm == "core-ffi" || arm == "host-gen";
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
        var all = (S9 ? G(DecArms) : G(EncArms).Concat(G(DecArms)).Concat(G(UnkArms))).Distinct().OrderBy(x => x, StringComparer.Ordinal).ToList();
        var rng = new Random(UnitSeed(launch));
        return all.OrderBy(_ => rng.Next()).ToList();
    }

    /// The counted cases (CountRun, the gate's req-19 check): every case of the grid in force,
    /// and, on the full grid, also the core grid's U-* rows at end state (ii)
    /// (encode-transport-hot, CAMPAIGN 4.0), which the full grid does not time, so the gate
    /// (run with AK_CAMPAIGN_GRID unset) counts every core-ffi row either grid times (owner,
    /// 2026-10-03).
    public static IEnumerable<string> CountKeys()
    {
        foreach (var k in All()) yield return k;
        if (CoreGrid) yield break;
#if AK_NO_UNKNOWN_FIELDS
        var ams = new[] { "core-ffi:no-unknown" };
#else
        var ams = new[] { "core-ffi:retain", "core-ffi:drop" };   // drop: the optimisation runs' labelled extra (AK_BDN_DROP)
#endif
        foreach (var am in ams)
        {
            if (!InUnit(am)) continue;
            var f = am.Split(':');
            if (f[1] != "drop")
                foreach (var id in CoreURows) yield return string.Join("|", f[0], "encode-transport-hot", id, "corpus", f[1]);
            foreach (var id in CoreURows) yield return string.Join("|", f[0], "encode-core-hot", id, "corpus", f[1]);   // D1 (2026-10-04)
        }
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
        // BDN's default toolchain runs each case from its generated project under the artifacts
        // directory, which may sit outside the repository: the host passes the path it found.
        var env = Environment.GetEnvironmentVariable("AK_CORPUS_DIR");
        if (!string.IsNullOrEmpty(env) && File.Exists(Path.Combine(env, "manifest.json"))) return env;
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
            foreach (var cs in SetsOfG(pid))
            {
                foreach (var dir in (CoreGrid && pid == "P7.1") ? Array.Empty<string>() : EncDirs)   // section 4.0: P7.1 decode only
                    foreach (var am in G(EncArms).Where(InUnit))
                    {
                        var f = am.Split(':');
                        if (!EncDirsFor(f[0], EncDirs).Contains(dir)) continue;
                        yield return string.Join("|", f[0], dir, pid, Values.SetNames[cs], f[1]);
                    }
                foreach (var dir in DecDirsG)
                    foreach (var am in G(DecArms).Where(InUnit)) { var f = am.Split(':'); yield return string.Join("|", f[0], dir, pid, Values.SetNames[cs], f[1]); }
            }
        }
        if (Environment.GetEnvironmentVariable("AK_BDN_NO_UNKNOWN") == "1") yield break;
        var ul = Environment.GetEnvironmentVariable("AK_BDN_UROWS");   // smoke: keep this many rows, spread evenly
        var rows = UnknownRows();
        if (CoreGrid)
        {
            var missing = CoreURows.Where(r => !rows.Contains(r)).ToList();
            if (missing.Count > 0) throw new InvalidOperationException("core grid U-* rows not in the accepted, non-disputed list: " + string.Join(", ", missing));
            rows = CoreURows.ToList();
        }
        else if (!string.IsNullOrEmpty(ul) && int.TryParse(ul, out int lim) && lim > 0 && lim < rows.Count)
            rows = Enumerable.Range(0, lim).Select(k => rows[k * rows.Count / lim]).ToList();
        foreach (var id in rows)
        {
            if (keep != null && !keep.Contains(id)) continue;
            foreach (var dir in UnkDirs.Concat(EncDirs).Distinct())
                foreach (var am in G(UnkArms).Where(InUnit))
                {
                    var f = am.Split(':');
                    if (IsEnc(dir) ? !EncDirsFor(f[0], new[] { "encode-hot" }).Contains(dir) : !DecDirsG.Concat(CoreGrid || DirsOverride != null ? Array.Empty<string>() : UnkDirs).Contains(dir)) continue;
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
                    Same(o.EncFfiCoreBytes(true), wire, what + " core-ffi retain (encode-core: the bytes left in the core's context)");
                    n++;
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
                    Same(o.EncFfiCoreBytes(false), wire, what + " core-ffi drop/no-unknown (encode-core: the bytes left in the core's context)");
                    var e1 = Enc.New(Armonik.Ffi.Facade.Codec.Sites, 1 << 16);
                    o.EncHost(ref e1, false);
                    Same(e1.ToArray(), wire, what + " host-gen drop/no-unknown (encode-core: Ef's Enc)");
                    n += 13;
                }
            }
        // D21: every string encode path gives the same bytes, whatever path this process times
        // (AK_STR_ENC): E0, E1 (pinned UTF-16, ak_tc_utf16), E2 (the C# transcoder), and a
        // threshold split, on every payload and content set (Latin-1, wide; astral by the sweep).
        // D21 step 7: E3, E3L (the core's ak_utf16_to_utf8 from the callback), E1R (fixed frames,
        // chunks of K elements: K 3 so every chunk boundary and the recursion's unwinding are
        // crossed, and the default), E1C (GCHandles per chunk), and the threshold and ASCII splits.
        var mode0 = Armonik.Ffi.Harness.Stage.Mode; var th0 = Armonik.Ffi.Harness.Stage.Threshold;
        var na0 = Armonik.Ffi.Harness.Stage.NonAsciiOnly; var k0 = Armonik.Ffi.Harness.Stage.PinK;
        try
        {
            foreach (var spec in new[] { "E0", "E1", "E2", "ETH:16", "E3", "E3L", "E1R", "E1R:0:x3", "E1C:0:x3", "E1R:16", "E1R:16:na", "E3:16:na", "E1R:128", "E1C:16:x3" })
            {
                var sp = spec.Replace(":x3", "");
                Armonik.Ffi.Harness.Stage.Mode = Armonik.Ffi.Harness.Stage.ParseMode(sp, out Armonik.Ffi.Harness.Stage.Threshold, out Armonik.Ffi.Harness.Stage.NonAsciiOnly);
                Armonik.Ffi.Harness.Stage.PinK = spec.EndsWith(":x3", StringComparison.Ordinal) ? 3 : 64;
                var m = spec;
                foreach (var pid in OpsTable.Payloads)
                    foreach (var cs in SetsOf(pid))
                    {
                        Values.ContentSet = cs;
                        var ops = OpsTable.ForPayload(pid);
                        Values.ContentSet = Values.Ascii;
                        var wire = ops.IncumbentBytes();
                        var what = pid + "/" + Values.SetNames[cs] + " string path " + m;
                        Same(ops.EncFfiBytes(false), wire, what + " core-ffi drop/no-unknown");
                        Same(ops.EncFfiCoreBytes(false), wire, what + " core-ffi drop/no-unknown (encode-core)");
#if !AK_NO_UNKNOWN_FIELDS
                        Same(ops.EncFfiBytes(true), wire, what + " core-ffi retain");
                        Same(ops.EncFfiCoreBytes(true), wire, what + " core-ffi retain (encode-core)");
                        n += 2;
#endif
                        n += 2;
                    }
                foreach (var id in UnknownRows())
                {
                    var (ops, b) = Row(id);
                    var w = new BufWriter(b.Length * 2 + 4096);
                    ops.FromWire(b, 0).EncIncProd(w);
                    var inc = w.WrittenSpan.ToArray();
#if AK_NO_UNKNOWN_FIELDS
                    var e = Enc.New(Armonik.Ffi.Facade.Codec.Sites, b.Length + 4096); ops.FromWire(b, 1).EncHost(ref e, false);
                    Same(ops.FromWire(b, 3).EncFfiCoreBytes(false), e.ToArray(), id + " string path " + m + " core-ffi no-unknown (encode-core)");
#else
                    Same(ops.FromWire(b, 4).EncFfiCoreBytes(true), inc, id + " string path " + m + " core-ffi retain (encode-core)");
#endif
                    n++;
                }
            }
        }
        finally { Armonik.Ffi.Harness.Stage.Mode = mode0; Armonik.Ffi.Harness.Stage.Threshold = th0; Armonik.Ffi.Harness.Stage.NonAsciiOnly = na0; Armonik.Ffi.Harness.Stage.PinK = k0; }
        {
            // P7.1's decode rows decode the committed (interleaved) vector: every decode arm of
            // this build gives the graph the incumbent's bytes encode (managed re-encoding).
            var ops = OpsTable.ForPayload("P7.1");
            var v = DecodeWire("P7.1", ops);
            var want = ops.IncumbentBytes();
            Same(ops.RtIncBytes(v), want, "P7.1 committed vector, incumbent");
            foreach (var retain in Modes)
            {
                Same(ops.RtHost(v, v.Length, retain), want, "P7.1 committed vector, host-gen " + retain);
                Same(ops.ReEncHost(ops.DecFfiGraph(v, v.Length, retain), retain), want, "P7.1 committed vector, core push " + retain);
                Same(ops.ReEncHost(ops.DecFfiPullGraph(v, v.Length, retain), retain), want, "P7.1 committed vector, core pull " + retain);
                Same(ops.ReEncHost(ops.DecFfiFsmGraph(v, v.Length, retain), retain), want, "P7.1 committed vector, core FSM " + retain);
                n += 4;
            }
            n++;
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
            // Section 4.0 (D18): the U-* rows' encode at end state (ii), the transport form.
            var dropped = HostEnc(ops.FromWire(b, 1), false);
            ops.FromWire(b, 3).EncFfiTransport(false, fr); SameFrame(id + " core-ffi no-unknown", dropped);
            ops.FromWire(b, 1).EncHostTransport(false, fr); SameFrame(id + " host-gen no-unknown", dropped);
            Same(ops.FromWire(b, 3).EncFfiCoreBytes(false), dropped, id + " core-ffi no-unknown (encode-core)");
            n += 9;
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
            // Section 4.0 (D18): the U-* rows' encode at end state (ii), the transport form.
            ops.FromWire(b, 0).EncIncTransport(fr); SameFrame(id + " incumbent-prod", inc);
            ops.FromWire(b, 4).EncFfiTransport(true, fr); SameFrame(id + " core-ffi retain", inc);
            ops.FromWire(b, 2).EncHostTransport(true, fr); SameFrame(id + " host-gen retain", inc);
            Same(ops.FromWire(b, 4).EncFfiCoreBytes(true), inc, id + " core-ffi retain (encode-core)");
            n += 9;
#endif
        }
        return n;
    }

    /// The bytes a payload's DECODE rows decode. P7.1 (SHAPES.md: two repeated fields
    /// INTERLEAVED, which no canonical writer produces) decodes its committed vector
    /// (schema/generated/payloads/P7_1.bin); until D23 every arm here decoded the incumbent's
    /// contiguous re-encoding of the graph instead (a permutation of the same triples: 3 FSM
    /// events where the committed vector gives 7, found by comparing the FSM's events with the
    /// Rust slice's). Every other payload: the incumbent's bytes (byte-identical to the vector).
    public static byte[] DecodeWire(string pid, RootOps ops)
    {
        var w = ops.IncumbentBytes();
        if (pid != "P7.1") return w;
        var v = File.ReadAllBytes(Path.Combine(CorpusDir(), "..", "..", "schema", "generated", "payloads", "P7_1.bin"));
        if (v.AsSpan().SequenceEqual(w) || !Armonik.Ffi.Harness.Triples.Same(v, w))
            throw new InvalidOperationException("P7.1: the committed vector is not an interleaved permutation of the incumbent's bytes");
        return v;
    }

#if AK_NO_UNKNOWN_FIELDS
    private static readonly bool[] Modes = { false };
#else
    private static readonly bool[] Modes = { false, true };
#endif

    internal static (RootOps, byte[]) Row(string id)
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
            wire = IsEnc(c.Dir) ? ops.IncumbentBytes() : DecodeWire(c.Payload, ops);
        }
        int len = wire.Length;
        var seq = new ReadOnlySequence<byte>(wire);
        bool read = c.Dir == "decode-read";
        var w = new BufWriter(len * 2 + 4096);
        var fr = new GrpcFrame();
        string d = IsEnc(c.Dir) ? (IsTransport(c.Dir) ? "encode-transport" : IsCoreForm(c.Dir) ? "encode-core" : "encode") : c.Dir;
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
            // D1: the form the core's transport receives (cells Cf / Ef): core-ffi's encode left
            // in the core's context (EncodeInto, no take); host-gen's Enc buffer, sized as Ef's.
            case "core-ffi:encode-core": return () => { ops.Next(); return ops.EncFfiCore(retain); };
            case "host-gen:encode-core": { var box = new EncBox(-1); return () => { ops.Next(); return box.Run(ops, retain); }; }
            case "incumbent-prod:decode": case "incumbent-prod:decode-read":
                if (c.Mode == "discard") return () => ops.DecIncProdDiscard(seq, read);
                return () => ops.DecIncProd(seq, read);
            case "incumbent-best:decode": case "incumbent-best:decode-read": return () => ops.DecIncBest(wire, len, read);
            case "host-gen:decode": case "host-gen:decode-read": return () => ops.DecHost(wire, len, retain, read);
            case "core-ffi:decode": case "core-ffi:decode-read": return () => ops.DecFfi(wire, len, retain, read);
            case "core-ffi-pull:decode": case "core-ffi-pull:decode-read":
                if (c.Mode == "retain") return () => ops.DecFfiPullR(wire, len, true, read);
                return () => ops.DecFfiPull(wire, len, read);
            case "core-ffi-fsm:decode": case "core-ffi-fsm:decode-read": return () => ops.DecFfiFsm(wire, len, retain, read);
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
        /// len < 0: the RPC grid's Enc for cell Ef (64 KiB initial, grown by Enc itself).
        public EncBox(int len) { _e = Enc.New(Armonik.Ffi.Facade.Codec.Sites, len < 0 ? 1 << 16 : len + 4096); }
        public long Run(RootOps ops, bool retain) => ops.EncHost(ref _e, retain);
    }
}
