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

    private static readonly string[] EncArms = { "incumbent-prod:default", "incumbent-best:default", "host-gen:drop", "host-gen:retain", "core-ffi:drop", "core-ffi:retain" };
    private static readonly string[] DecArms = { "incumbent-prod:default", "incumbent-best:default", "host-gen:drop", "host-gen:retain", "core-ffi:drop", "core-ffi:retain", "core-ffi-pull:drop" };
    private static readonly string[] UnkArms = { "incumbent-prod:default", "host-gen:drop", "host-gen:retain", "core-ffi:drop", "core-ffi:retain" };

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
    /// Requirement 7: every corpus U-* row whose root this slice implements, disputed rows
    /// excluded (accept rows only: a row that is refused has nothing to decode or re-encode).
    public static List<string> UnknownRows()
    {
        if (_unk != null) return _unk;
        var man = Json.Parse(File.ReadAllText(Path.Combine(CorpusDir(), "manifest.json")))["vectors"];
        _unk = new List<string>();
        foreach (var id in man.Keys.OrderBy(x => x, StringComparer.Ordinal))
        {
            var v = man[id];
            if (!id.StartsWith("U-", StringComparison.Ordinal) || v["verdict"].AsString == "disputed" || v["expect"].AsString != "accept") continue;
            if (OpsTable.ForRoot(v["root"].AsString) == null) continue;
            _unk.Add(id);
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
                foreach (var am in EncArms) { var f = am.Split(':'); yield return string.Join("|", f[0], "encode", pid, Values.SetNames[cs], f[1]); }
                foreach (var dir in new[] { "decode", "decode-read" })
                    foreach (var am in DecArms) { var f = am.Split(':'); yield return string.Join("|", f[0], dir, pid, Values.SetNames[cs], f[1]); }
            }
        }
        if (Environment.GetEnvironmentVariable("AK_BDN_NO_UNKNOWN") == "1") yield break;
        foreach (var id in UnknownRows())
        {
            if (keep != null && !keep.Contains(id)) continue;
            foreach (var dir in new[] { "decode", "decode-read", "decode-reencode" })
                foreach (var am in UnkArms) { var f = am.Split(':'); yield return string.Join("|", f[0], dir, id, "corpus", f[1]); }
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
                Same(ops.EncFfiBytes(true), wire, pid + " core-ffi retain");
                var w = new BufWriter(wire.Length + 4096);
                ops.EncIncProd(w);
                Same(w.WrittenSpan.ToArray(), wire, pid + " incumbent-prod");
                n += 4;
            }
        foreach (var id in UnknownRows())
        {
            var (ops, b) = Row(id);
            ops.DecIncBest(b, b.Length, true); ops.DecHost(b, b.Length, false, true); ops.DecHost(b, b.Length, true, true);
            ops.DecFfi(b, b.Length, false, true); ops.DecFfi(b, b.Length, true, true);
            n += 5;
        }
        return n;
    }

    private static (RootOps, byte[]) Row(string id)
    {
        var dir = CorpusDir();
        var v = Json.Parse(File.ReadAllText(Path.Combine(dir, "manifest.json")))["vectors"][id];
        return (OpsTable.ForRoot(v["root"].AsString), File.ReadAllBytes(Path.Combine(dir, v["file"].AsString)));
    }

    /// The operation a case times: one call, returning something the engine consumes.
    public static Func<long> Build(Case c)
    {
        RootOps ops;
        byte[] wire;
        if (c.Content == "corpus") (ops, wire) = Row(c.Payload);
        else
        {
            Values.ContentSet = Array.IndexOf(Values.SetNames, c.Content);
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
