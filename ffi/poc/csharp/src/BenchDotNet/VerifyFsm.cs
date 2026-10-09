// D23 (owner, 2026-10-09): the FSM decode family's checks against push and pull, in process.
//
//   BenchDotNet --verify-fsm [--events FILE] [--rust-events FILE] [--variants N]
//
// Inputs: every payload of the codec suite with its content sets, and every accepted,
// non-disputed U-* row (Cases.UnknownRows); modes drop and retain (the full build), drop (the
// no-unknown build). Per input and mode:
//   graph   push (ak_decode_*), pull (ak_parse_* + replay) and FSM (ak_fsm_begin/next) decode
//           it; the three graphs re-encoded by the MANAGED codec (host-gen, retained form where
//           the build has bags: a value comparison that does not go through the core) must be
//           byte-identical, and the read pass (Touch) must give the same value;
//   counts  forward / reverse crossings per family (host tally): the FSM's forward count is its
//           calls = its events (begin + next); written to --events FILE and, with --rust-events,
//           compared with the Rust slice's events per decode (logs/rust/opt/d23-fsm/checks/
//           events-counting.txt, column `events`) where the input is in both lists.
// Malformed variants (--variants N per input, default 48; 0 = none): N truncations and N byte
// flips (x ^ 0xFF) at evenly spaced positions of each payload (ASCII set) and U-* row: the FSM
// must return the code pull returns; push's code is tallied (a disagreement with pull is
// reported, not failed: push is not the reference here). Where all three accept, their graphs
// must be equal as above. Exit 0 only if nothing failed.
//
// A planted defect is caught by this program (gen/s10_checks.sh edits the generated FSM
// consumer, rebuilds, and requires a failure).

using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using Armonik.Ffi.Campaign;
using Armonik.Ffi.Facade;

namespace Armonik.Ffi.Bdn;

public static class VerifyFsm
{
#if AK_NO_UNKNOWN_FIELDS
    private static readonly bool[] Modes = { false };
#else
    private static readonly bool[] Modes = { false, true };
#endif

    private static int _bad;
    private static void Fail(string what) { _bad++; if (_bad <= 40) Console.WriteLine("  FAIL " + what); }

    public static int Run(string eventsOut, string rustEvents, int variants)
    {
        var rows = new List<string>();
        var rust = ReadRust(rustEvents);
        int inputs = 0, graphs = 0, rustCompared = 0, rustMissing = 0;
        long malformed = 0, refused = 0, pushDiffers = 0, accepted = 0;
        var inputsList = new List<(string Name, RootOps Ops, byte[] Wire, bool Malform)>();
        foreach (var pid in OpsTable.Payloads)
            foreach (var cs in Cases.SetsOf(pid))
            {
                Values.ContentSet = cs;
                var ops = OpsTable.ForPayload(pid);
                Values.ContentSet = Values.Ascii;
                inputsList.Add((cs == Values.Ascii ? pid : pid + "/" + Values.SetNames[cs], ops, Cases.DecodeWire(pid, ops), cs == Values.Ascii));
            }
        foreach (var id in Cases.UnknownRows())
        {
            var (ops, b) = Cases.Row(id);
            inputsList.Add((id, ops, b, true));
        }
        foreach (var (name, ops, wire, malform) in inputsList)
        {
            inputs++;
            foreach (var retain in Modes)
            {
                string mode = retain ? "retain" : "drop";
                long[] fwd = new long[3], rev = new long[3];
                object[] g = new object[3];
                for (int fam = 0; fam < 3; fam++)
                {
                    ops.FfiCallsReset();
                    try
                    {
                        g[fam] = fam == 0 ? ops.DecFfiGraph(wire, wire.Length, retain)
                               : fam == 1 ? ops.DecFfiPullGraph(wire, wire.Length, retain)
                               : ops.DecFfiFsmGraph(wire, wire.Length, retain);
                    }
                    catch (Exception e) { Fail(name + " " + mode + " family " + fam + ": " + e.Message); }
                    fwd[fam] = ops.FfiForward(); rev[fam] = ops.FfiReverse();
                }
                string verdict = "ok";
                if (g.All(x => x != null))
                {
                    if (!SameGraphs(ops, g, retain, out var why)) { verdict = why; Fail(name + " " + mode + ": " + why); }
                    graphs++;
                }
                else verdict = "refused";
                string rk = name + "|" + mode, rcmp = "-";
                if (rust.TryGetValue(rk, out var ev))
                {
                    rustCompared++;
                    rcmp = ev.ToString(CultureInfo.InvariantCulture);
                    if (ev != fwd[2]) { Fail(name + " " + mode + ": FSM calls " + fwd[2] + ", the Rust slice's events " + ev); verdict = "EVENTS"; }
                }
                else rustMissing++;
                // The FSM makes no reverse call but grow (retain); pull likewise.
                if (rev[2] != rev[1]) { Fail(name + " " + mode + ": reverse calls FSM " + rev[2] + " != pull " + rev[1]); verdict = "REV"; }
                rows.Add(string.Format(CultureInfo.InvariantCulture, "{0,-52} {1,8} {2,-6} {3,8} {4,5} {5,8} {6,5} {7,8} {8,5} {9,8}  {10}",
                    name, wire.Length, mode, fwd[0], rev[0], fwd[1], rev[1], fwd[2], rev[2], rcmp, verdict));
            }
            if (!malform || variants <= 0) continue;
            // ---- malformed variants
            foreach (var v in Variants(wire, variants))
                foreach (var retain in Modes)
                {
                    malformed++;
                    int rp = Rc(() => ops.TryPushRc(v.B, v.B.Length, retain));
                    int rl = Rc(() => ops.TryPullRc(v.B, v.B.Length, retain));
                    int rf = Rc(() => ops.TryFsmRc(v.B, v.B.Length, retain));
                    if (rf != rl) Fail(name + " " + v.What + (retain ? " retain" : " drop") + ": FSM " + rf + " != pull " + rl + " (push " + rp + ")");
                    if (rp != rl) pushDiffers++;
                    if (rl < 0) { refused++; continue; }
                    if (rf < 0 || rp < 0) continue;
                    accepted++;
                    var g = new object[3];
                    try
                    {
                        g[0] = ops.DecFfiGraph(v.B, v.B.Length, retain);
                        g[1] = ops.DecFfiPullGraph(v.B, v.B.Length, retain);
                        g[2] = ops.DecFfiFsmGraph(v.B, v.B.Length, retain);
                    }
                    catch (Exception e) { Fail(name + " " + v.What + ": " + e.Message); continue; }
                    if (!SameGraphs(ops, g, retain, out var why)) Fail(name + " " + v.What + (retain ? " retain" : " drop") + ": " + why);
                }
        }
        var head = new List<string>
        {
            "# D23 --verify-fsm (" + Armonik.Ffi.Harness.AbiVariant.Name + " build): per input and mode, forward / reverse crossings per decode family (host tally, one decode).",
            "# push fwd = ak_decode_* (+ elements), pull fwd = ak_parse_* + ak_bdr_ptr, fsm fwd = begin + next = events; rust = the Rust slice's FSM events for this input (- = not in its list).",
            string.Format(CultureInfo.InvariantCulture, "{0,-52} {1,8} {2,-6} {3,8} {4,5} {5,8} {6,5} {7,8} {8,5} {9,8}  {10}",
                "input", "bytes", "mode", "push fwd", "rev", "pull fwd", "rev", "fsm fwd", "rev", "rust", "verdict"),
        };
        if (eventsOut != null) File.WriteAllLines(eventsOut, head.Concat(rows));
        Console.WriteLine("verify-fsm ({0}): {1} inputs, {2} input x mode graphs compared (push = pull = FSM, managed re-encoding and read pass); FSM events vs the Rust slice: {3} compared, {4} not in its list",
            Armonik.Ffi.Harness.AbiVariant.Name, inputs, graphs, rustCompared, rustMissing);
        Console.WriteLine("malformed variants: {0} decodes per family ({1} refused by pull, FSM code == pull code required; {2} accepted by all, graphs compared); push's code differs from pull's on {3}",
            malformed, refused, accepted, pushDiffers);
        Console.WriteLine("verify-fsm: {0} failure(s)", _bad);
        return _bad == 0 ? 0 : 1;
    }

    private static int Rc(Func<int> f)
    {
        try { return f(); }
        catch (Exception e) { return -100000 - (e.GetType().Name.Length); }   // an exception is a distinct, failing code
    }

    private static bool SameGraphs(RootOps ops, object[] g, bool retain, out string why)
    {
        var e0 = ops.ReEncHost(g[0], retain);
        var e1 = ops.ReEncHost(g[1], retain);
        var e2 = ops.ReEncHost(g[2], retain);
        long t0 = ops.TouchF(g[0]), t1 = ops.TouchF(g[1]), t2 = ops.TouchF(g[2]);
        why = !e2.AsSpan().SequenceEqual(e0) ? "FSM graph != push graph (managed re-encoding)"
            : !e2.AsSpan().SequenceEqual(e1) ? "FSM graph != pull graph (managed re-encoding)"
            : t2 != t0 || t2 != t1 ? "read pass differs (push " + t0 + ", pull " + t1 + ", FSM " + t2 + ")"
            : null;
        return why == null;
    }

    private static IEnumerable<(string What, byte[] B)> Variants(byte[] w, int n)
    {
        if (w.Length == 0) yield break;
        var seen = new HashSet<int>();
        for (int k = 0; k < n; k++)
        {
            int at = (int)((long)k * w.Length / n);
            if (!seen.Add(at)) continue;
            yield return ("truncated at " + at, w.AsSpan(0, at).ToArray());
            var f = (byte[])w.Clone(); f[at] ^= 0xFF;
            yield return ("byte " + at + " flipped", f);
        }
        // the last byte, and a tail: the ends of the input
        yield return ("truncated by 1", w.AsSpan(0, w.Length - 1).ToArray());
        var g = (byte[])w.Clone(); g[w.Length - 1] ^= 0xFF;
        yield return ("last byte flipped", g);
    }

    /// The Rust slice's FSM events per (input, mode): lines `input root bytes mode records events ...`.
    private static Dictionary<string, long> ReadRust(string path)
    {
        var d = new Dictionary<string, long>(StringComparer.Ordinal);
        if (string.IsNullOrEmpty(path)) return d;
        foreach (var l in File.ReadLines(path))
        {
            var f = l.Split(' ', StringSplitOptions.RemoveEmptyEntries);
            if (f.Length < 7) continue;
#if AK_NO_UNKNOWN_FIELDS
            if (f[3] != "no-unknown") continue;   // the Rust slice's no-unknown build = this build's drop
            string mode = "drop";
#else
            if (f[3] != "drop" && f[3] != "retain") continue;
            string mode = f[3];
#endif
            if (!long.TryParse(f[5], NumberStyles.Integer, CultureInfo.InvariantCulture, out var ev)) continue;
            d[f[0] + "|" + mode] = ev;
        }
        return d;
    }
}
