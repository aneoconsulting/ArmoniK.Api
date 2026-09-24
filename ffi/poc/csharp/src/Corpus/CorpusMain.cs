// The conformance corpus (ffi/corpus/CONTRACT.md), four arms, every row in a CHILD PROCESS
// under a timeout. FIX-PLAN WP5 step 4; the model is poc/rust/corpus.
//
//   managed-drop    the managed codec (poc/codec/gen/cs_managed.py, rendered from the corpus
//                   READER plan), unknown fields skipped
//   managed-retain  the same codec with `Dec.Retain`: unknown runs captured and re-emitted
//   ffi-drop        core-ffi against the core generated for the corpus reader schema
//                   (`ak-core --features corpus,init-guard`), `ak_decode_*` / `ak_encode_*`
//   ffi-retain      the same, with the capture callbacks and `ak_uencode_*`
//
// Obligations: C1 (parse every accept row), C2 (project it; `_unknown` not compared, as the
// contract allows), C3 (re-encode to one of `accepted_encodings`, or a re-ordering where
// `permutation_accepted`; the managed arms' one-pass and two-pass encoders must agree; the
// FORM written is recorded), C4 (refuse every reject row with an error CODE -- an exception
// is not a refusal). C5 (produce) is not claimed. A disputed row is excluded from pass/fail
// and the reading is reported. A row that hangs or kills its child is a row result.
//
//   corpus [--only P1,P2] [--timeout-ms N]     the whole corpus: a verdict, exit 0/1
//   corpus --row ID                             one row, one line per arm (the child)
//
// Controls (gen/gate.sh runs each and requires it to FAIL):
//   AK_CORPUS_PLANT=proj    a planted key in every projection      -> C2 must fail
//   AK_CORPUS_PLANT=reenc   a byte appended to every re-encoding   -> C3 must fail
//   AK_CORPUS_PLANT=accept  every refusal read as an acceptance    -> C4 must fail
//   AK_GATE_PLANT_NO_INIT=1 the binding skips ak_init (init-guard) -> the ffi arms must fail

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text;
using Armonik.Ffi.Facade;
using H = Armonik.Ffi.Harness;
using Gp = Armonik.Ffi.Shapes.V1;

namespace Armonik.Ffi.Corpus;

public static class Program
{
    private static readonly string[] Arms = { "managed-drop", "managed-retain", "ffi-drop", "ffi-retain" };

    /// `--manifest PATH`: a manifest in the corpus's format elsewhere (e.g. the oracle probe
    /// rows of poc/rust/gen/probe_corpus.py); children get the same.
    private static string ManifestArg;

    private static string Dir()
    {
        if (ManifestArg != null) return Path.GetDirectoryName(Path.GetFullPath(ManifestArg));
        var d = AppContext.BaseDirectory;
        for (int i = 0; i < 12 && d != null; i++)
        {
            var c = Path.Combine(d, "ffi", "corpus", "generated");
            if (File.Exists(Path.Combine(c, "manifest.json"))) return c;
            d = Path.GetDirectoryName(d.TrimEnd(Path.DirectorySeparatorChar));
        }
        throw new DirectoryNotFoundException("cannot find ffi/corpus/generated");
    }

    public static int Main(string[] argv)
    {
        string row = null, only = null;
        int timeout = 20000;
        for (int i = 0; i < argv.Length; i++)
        {
            if (argv[i] == "--row") row = argv[++i];
            else if (argv[i] == "--only") only = argv[++i];
            else if (argv[i] == "--manifest") ManifestArg = argv[++i];
            else if (argv[i] == "--timeout-ms") timeout = int.Parse(argv[++i], CultureInfo.InvariantCulture);
            else if (argv[i] == "--layout")
            {
                // The corpus binding's structs against the corpus build's Rust declaration.
                var path = argv[++i];
                var probe = H.Json.Parse(File.ReadAllText(path));
                var mine = AbiLayout.Table().Select(s => (s.Name, s.Size, s.Fields, s.F)).ToList();
                return H.LayoutCompare.Compare(probe, mine, path, AbiLayout.Facts, AbiLayout.FactCount,
                    () => "ak_abi_version() = " + Abi.ak_abi_version() + "; ak_init() returned " + AbiInit.Code);
            }
            else { Console.Error.WriteLine("unknown argument " + argv[i]); return 2; }
        }
        return row != null ? Child(row) : Parent(only, timeout);
    }

    // ============================================================== the child

    private sealed class R
    {
        public string Verdict = "fail";   // pass | fail | disputed | notinabi
        public bool Accepted;
        public string Err = "";
        public string Form = "";
        public string Detail = "";
    }

    private static int Child(string id)
    {
        var dir = Dir();
        var man = H.Json.Parse(File.ReadAllText(Path.Combine(dir, "manifest.json")));
        var v = man["vectors"][id];
        var bytes = File.ReadAllBytes(Path.Combine(dir, v["file"].AsString));
        var root = v["root"].AsString;
        foreach (var arm in Arms)
        {
            R r;
            try { r = RunArm(arm, dir, v, root, bytes); }
            catch (Exception ex) { r = new R { Detail = "harness THREW " + ex.GetType().Name + ": " + Clean(ex.Message) }; }
            Console.WriteLine(string.Join("\t", "ARM", arm, r.Verdict, r.Accepted ? "1" : "0", Clean(r.Err), Clean(r.Form), Clean(r.Detail)));
        }
        Console.Out.Flush();
        return 0;
    }

    private static string Clean(string s) => (s ?? "").Replace('\t', ' ').Replace('\n', ' ').Replace('\r', ' ');

    private static R RunArm(string arm, string dir, H.Json v, string root, byte[] bytes)
    {
        var r = new R();
        bool retain = arm.EndsWith("retain", StringComparison.Ordinal);
        bool ffi = arm.StartsWith("ffi", StringComparison.Ordinal);
        string plant = Environment.GetEnvironmentVariable("AK_CORPUS_PLANT") ?? "";

        // ---- decode
        object msg = null;
        string err = null;
        if (ffi)
        {
            if (!Ffi.Roots.Contains(root)) { r.Verdict = "notinabi"; r.Detail = "root " + root + " is refused by the generator"; return r; }
            int rc;
            try { rc = Ffi.Decode(root, bytes, retain, out msg); }
            catch (Exception ex) { rc = int.MinValue; err = "THREW " + ex.GetType().Name + ": " + ex.Message; }
            if (err == null && rc < 0) err = "core " + rc + CoreName(rc);
        }
        else
        {
            try
            {
                msg = Roots.New(root);
                var d = new Dec { Buf = bytes, Pos = 0, End = bytes.Length, Err = 0, Retain = retain };
                Roots.Read(root, ref d, msg);
                if (d.Err != 0) err = "managed " + d.Err + ManagedName(d.Err);
            }
            catch (Exception ex) { err = "THREW " + ex.GetType().Name + ": " + ex.Message; }
        }
        r.Accepted = err == null;
        r.Err = err ?? "";

        // ---- disputed: excluded, reading reported
        if (v["verdict"]?.AsString == "disputed")
        {
            r.Verdict = "disputed";
            r.Detail = err != null ? "refused (" + err + ")" : Reading(dir, v, root, msg);
            return r;
        }

        // ---- C4
        if (v["expect"].AsString == "reject")
        {
            bool refused = err != null && !err.StartsWith("THREW", StringComparison.Ordinal);
            if (plant == "accept" && refused) { r.Verdict = "fail"; r.Detail = "C4: PLANTED acceptance"; return r; }
            if (err == null) { r.Verdict = "fail"; r.Detail = "C4: accepted a reject row"; return r; }
            if (!refused) { r.Verdict = "fail"; r.Detail = "C4: refused by an EXCEPTION, not an error code: " + err; return r; }
            r.Verdict = "pass";
            return r;
        }
        if (err != null) { r.Detail = "C1: refused: " + err; return r; }

        // ---- C2
        var pj = v["projection"];
        if (pj != null && pj.Kind != H.Json.K.Null)
        {
            var want = H.Json.Parse(File.ReadAllText(Path.Combine(dir, pj.AsString)));
            var got = Proj.ByRoot(root, msg);
            if (plant == "proj" && got is SortedDictionary<string, object> gd) gd["__planted"] = "x";
            if (!ProjEq(got, want, "", out var why)) { r.Detail = "C2: projection differs: " + why; return r; }
        }

        // ---- C3
        byte[] re;
        if (ffi)
        {
            int rc = Ffi.Encode(root, msg, retain, out re);
            if (rc != 0) { r.Detail = "C3: re-encode failed: core " + rc; return r; }
        }
        else
        {
            var e = Enc.New(Codec.Sites, bytes.Length * 2 + 8192);
            Roots.Write(root, ref e, msg);
            var e2 = Enc.New(Codec.Sites, bytes.Length * 2 + 8192);
            Roots.WriteSized(root, ref e2, msg);
            if (e.Err != 0 || e2.Err != 0) { r.Detail = "C3: re-encode refused: " + e.Err + "/" + e2.Err; return r; }
            re = e.ToArray();
            var re2 = e2.ToArray();
            if (!re.AsSpan().SequenceEqual(re2)) { r.Detail = "C3: one-pass and two-pass encodings differ"; return r; }
        }
        if (plant == "reenc") re = re.Concat(new byte[] { 0 }).ToArray();
        string sha = H.Manifest.Sha(re, re.Length);
        string form = null;
        var enc = v["accepted_encodings"];
        if (enc != null && enc.Arr != null)
            foreach (var a in enc.Arr)
                if (a["sha256"].AsString == sha)
                    form = a["forms"]?.Arr != null && a["forms"].Arr.Count > 0
                        ? string.Join(" / ", a["forms"].Arr.Select(x => x.AsString)) : "unlabelled";
        // A baseline row carries no list: its own sha256 is the one accepted form.
        if (form == null && enc == null && v["sha256"]?.AsString == sha) form = "the committed vector";
        if (form == null && v["permutation_accepted"]?.Kind == H.Json.K.Bool && v["permutation_accepted"].Bool
            && H.Triples.Same(re, bytes))
            form = "a re-ordering of the committed form (permutation_accepted)";
        if (form == null) { r.Detail = "C3: re-encoding " + sha.Substring(0, 12) + " (" + re.Length + " B) is not an accepted form"; return r; }
        r.Form = form;
        r.Verdict = "pass";
        return r;
    }

    private static string CoreName(int e) => e switch
    {
        -2 => " (malformed)", -3 => " (truncated)", -4 => " (depth)", -6 => " (transcode)",
        -10 => " (uninitialized)", -11 => " (abi)", -1 => " (host)", _ => "",
    };

    private static string ManagedName(int e)
        => e == W.ErrTruncated ? " (truncated)" : e == W.ErrMalformed ? " (malformed)"
         : e == W.ErrDepth ? " (depth)" : e == W.ErrTranscode ? " (transcode)" : "";

    /// A disputed row's reading: which of the listed readings this arm's projection matches.
    private static string Reading(string dir, H.Json v, string root, object msg)
    {
        var readings = v["dispute"]?["readings"];
        if (readings == null || readings.Arr == null) return "accepted";
        var hit = new List<string>();
        foreach (var rd in readings.Arr)
        {
            try
            {
                var want = H.Json.Parse(File.ReadAllText(Path.Combine(dir, rd["projection"].AsString)));
                if (ProjEq(Proj.ByRoot(root, msg), want, "", out _))
                    hit.Add(string.Join(", ", rd["read_by"].Arr.Select(x => x.AsString)));
            }
            catch (Exception) { }
        }
        return hit.Count == 0 ? "accepted, matches NO listed reading" : "accepted, reads as: " + string.Join(" | ", hit);
    }

    /// Structural, not textual (key order is not part of the encoding). `_unknown` is not
    /// compared (CONTRACT.md section 3 makes it optional); a double that differs only in
    /// spelling is compared numerically.
    private static bool ProjEq(object got, H.Json want, string at, out string why)
    {
        why = null;
        if (want == null) { why = at + ": expected nothing"; return false; }
        if (got is SortedDictionary<string, object> od)
        {
            if (want.Kind != H.Json.K.Obj) { why = at + ": got object, want " + want.Kind; return false; }
            foreach (var k in want.Keys)
            {
                if (k == "_unknown") continue;
                if (!od.ContainsKey(k)) { why = at + "." + k + ": missing"; return false; }
            }
            foreach (var k in od.Keys)
                if (want[k] == null) { why = at + "." + k + ": extra"; return false; }
            foreach (var k in od.Keys)
                if (!ProjEq(od[k], want[k], at + "." + k, out why)) return false;
            return true;
        }
        if (got is List<object> ol)
        {
            if (want.Kind != H.Json.K.Arr) { why = at + ": got array, want " + want.Kind; return false; }
            if (ol.Count != want.Arr.Count) { why = at + ": " + ol.Count + " elements, want " + want.Arr.Count; return false; }
            for (int i = 0; i < ol.Count; i++)
                if (!ProjEq(ol[i], want.Arr[i], at + "[" + i + "]", out why)) return false;
            return true;
        }
        if (got is bool gb)
        {
            if (want.Kind != H.Json.K.Bool || gb != want.Bool) { why = at + ": bool mismatch"; return false; }
            return true;
        }
        var s = got as string;
        string w = want.AsString;
        if (s == null || w == null) { why = at + ": unprojectable"; return false; }
        if (s == w) return true;
        if (double.TryParse(s, NumberStyles.Float, CultureInfo.InvariantCulture, out var a)
         && double.TryParse(w, NumberStyles.Float, CultureInfo.InvariantCulture, out var b)
         && (a == b || (double.IsNaN(a) && double.IsNaN(b))))
            return true;
        why = at + ": \"" + (s.Length > 40 ? s.Substring(0, 40) : s) + "\" != \"" + (w.Length > 40 ? w.Substring(0, 40) : w) + "\"";
        return false;
    }

    // ============================================================== the parent

    private sealed class Tally
    {
        public int Pass, Fail, Disputed, NotInAbi;
        public SortedDictionary<string, int[]> ByClass = new SortedDictionary<string, int[]>(StringComparer.Ordinal);
        public SortedDictionary<string, int> Forms = new SortedDictionary<string, int>(StringComparer.Ordinal);
        public SortedDictionary<string, int> Errs = new SortedDictionary<string, int>(StringComparer.Ordinal);
        public List<string> Fails = new List<string>(), Disputes = new List<string>(), RetainGap = new List<string>();
    }

    private static int Parent(string only, int timeout)
    {
        var dir = Dir();
        var man = H.Json.Parse(File.ReadAllText(Path.Combine(dir, "manifest.json")));
        var vectors = man["vectors"];
        var ids = vectors.Keys.OrderBy(x => x, StringComparer.Ordinal).ToList();
        if (!string.IsNullOrEmpty(only))
        {
            var pre = only.Split(',');
            ids = ids.Where(i => pre.Any(p => i.StartsWith(p, StringComparison.Ordinal))).ToList();
        }
        Console.WriteLine("# the conformance corpus through the C# codecs, unknown fields dropped and retained");
        Console.WriteLine("#   manifest   {0} ({1} rows run of {2})", Path.Combine(dir, "manifest.json"), ids.Count, vectors.Keys.Count());
        Console.WriteLine("#   runtime    {0}", System.Runtime.InteropServices.RuntimeInformation.FrameworkDescription);
        Console.WriteLine("#   managed    poc/codec/gen/cs_managed.py over the corpus READER plan; utf8={0} unknown={1} limit={2}",
            Codec.Utf8Policy, Codec.UnknownMode, Codec.Limit);
        Console.WriteLine("#   core-ffi   poc/codec/gen/cs_binding.py + cs_host.py, against libak_core.so built with");
        Console.WriteLine("#              --features corpus,init-guard (the core generated for the reader schema)");
        foreach (var n in Ffi.NotInAbi) Console.WriteLine("#   NOT IN THE C ABI: {0}: {1}", n[0], n[1]);
        Console.WriteLine("#   each row in a child process, timeout {0} ms", timeout);
        var plant = Environment.GetEnvironmentVariable("AK_CORPUS_PLANT");
        if (!string.IsNullOrEmpty(plant)) Console.WriteLine("#   PLANTED DEFECT: {0} (a control: it MUST fail)", plant);
        if (Environment.GetEnvironmentVariable("AK_GATE_PLANT_NO_INIT") == "1")
            Console.WriteLine("#   PLANTED DEFECT: ak_init skipped (a control: the ffi arms MUST fail)");
        Console.WriteLine();

        var tallies = Arms.ToDictionary(a => a, a => new Tally());
        var accepted = new Dictionary<string, bool>();
        int hard = 0;
        foreach (var id in ids)
        {
            var v = vectors[id];
            var cls = v["class"].AsString;
            var lines = RunChild(id, timeout, out var hardWhy);
            if (hardWhy != null) { hard++; Console.WriteLine("!! {0}: {1}", id, hardWhy); }
            foreach (var arm in Arms)
            {
                var t = tallies[arm];
                if (!t.ByClass.ContainsKey(cls)) t.ByClass[cls] = new int[2];
                string[] f = lines.TryGetValue(arm, out var l) ? l : null;
                if (f == null)
                {
                    t.Fail++; t.ByClass[cls][1]++; t.Fails.Add(id + ": " + (hardWhy ?? "no result"));
                    continue;
                }
                string verdict = f[2], err = f[4], form = f[5], detail = f[6];
                if (arm == "managed-drop") accepted[id] = f[3] == "1";
                switch (verdict)
                {
                    case "pass":
                        t.Pass++; t.ByClass[cls][0]++;
                        if (form.Length != 0) t.Forms[form] = (t.Forms.TryGetValue(form, out var c) ? c : 0) + 1;
                        if (err.Length != 0) { var k = System.Text.RegularExpressions.Regex.Replace(err, "^(\\w+ -?\\d+( \\(\\w+\\))?).*$", "$1"); t.Errs[k] = (t.Errs.TryGetValue(k, out var e) ? e : 0) + 1; }
                        if (arm.EndsWith("retain", StringComparison.Ordinal) && cls == "unknown" && form.Contains("dropped"))
                            t.RetainGap.Add(id);
                        break;
                    case "disputed": t.Disputed++; t.Disputes.Add(id + ": " + detail); break;
                    case "notinabi": t.NotInAbi++; break;
                    default: t.Fail++; t.ByClass[cls][1]++; t.Fails.Add(id + ": " + detail); break;
                }
            }
        }

        int total = 0;
        foreach (var arm in Arms)
        {
            var t = tallies[arm];
            total += t.Fail;
            Console.WriteLine("## {0}", arm);
            Console.WriteLine("   pass {0}  fail {1}  disputed (excluded) {2}  not in the C ABI {3}", t.Pass, t.Fail, t.Disputed, t.NotInAbi);
            foreach (var kv in t.ByClass) Console.WriteLine("     {0,-10} pass {1,4}  fail {2,3}", kv.Key, kv.Value[0], kv.Value[1]);
            Console.WriteLine("   forms written (C3):");
            foreach (var kv in t.Forms) Console.WriteLine("     {0,4}  {1}", kv.Value, kv.Key);
            Console.WriteLine("   refusals by code (C4):");
            foreach (var kv in t.Errs) Console.WriteLine("     {0,4}  {1}", kv.Value, kv.Key);
            foreach (var d in t.Disputes) Console.WriteLine("   disputed: {0}", d);
            if (t.RetainGap.Count != 0)
            {
                Console.WriteLine("   retain mode wrote the DROPPED form on {0} unknown-class row(s) (accepted by the contract; a retention gap):", t.RetainGap.Count);
                Console.WriteLine("     {0}", string.Join(", ", t.RetainGap));
            }
            foreach (var f in t.Fails.Take(80)) Console.WriteLine("   FAIL {0}", f);
            if (t.Fails.Count > 80) Console.WriteLine("   ... and {0} more", t.Fails.Count - 80);
            Console.WriteLine();
        }
        Incumbent(dir, vectors, accepted);
        Console.WriteLine("# rows that hung or crashed a child: {0}", hard);
        if (total == 0 && hard == 0) { Console.WriteLine("CORPUS PASSES on all four arms"); return 0; }
        Console.WriteLine("CORPUS FAILS: {0} arm-row failure(s), {1} hang/crash row(s)", total, hard);
        return 1;
    }

    private static Dictionary<string, string[]> RunChild(string id, int timeout, out string hard)
    {
        hard = null;
        var exe = Environment.ProcessPath;
        var psi = new ProcessStartInfo { FileName = exe, UseShellExecute = false, RedirectStandardOutput = true, RedirectStandardError = true };
        // `dotnet corpus.dll`: the host is dotnet, so hand it the assembly again.
        if (Path.GetFileNameWithoutExtension(exe) == "dotnet") psi.ArgumentList.Add(typeof(Program).Assembly.Location);
        if (ManifestArg != null) { psi.ArgumentList.Add("--manifest"); psi.ArgumentList.Add(Path.GetFullPath(ManifestArg)); }
        psi.ArgumentList.Add("--row");
        psi.ArgumentList.Add(id);
        var res = new Dictionary<string, string[]>();
        using var p = Process.Start(psi);
        var so = p.StandardOutput.ReadToEndAsync();
        var se = p.StandardError.ReadToEndAsync();
        if (!p.WaitForExit(timeout))
        {
            try { p.Kill(true); } catch { }
            p.WaitForExit();
            hard = "TIMEOUT after " + timeout + " ms";
            return res;
        }
        p.WaitForExit();
        foreach (var line in so.Result.Split('\n'))
        {
            var f = line.TrimEnd('\r').Split('\t');
            if (f.Length == 7 && f[0] == "ARM") res[f[1]] = f;
        }
        if (p.ExitCode != 0 || res.Count != Arms.Length)
            hard = "CHILD EXIT " + p.ExitCode + " with " + res.Count + " arm line(s): "
                 + Clean(se.Result.Length > 300 ? se.Result.Substring(0, 300) : se.Result);
        return res;
    }

    /// The incumbent as an INDEPENDENT accept/reject oracle for the roots ffi/schema also
    /// has (R14): the same bytes through Google.Protobuf, by descriptor name, against the
    /// managed-drop arm's acceptance.
    private static void Incumbent(string dir, H.Json vectors, Dictionary<string, bool> accepted)
    {
        var file = Gp.ShapesReflection.Descriptor;
        int agree = 0, differ = 0, absent = 0;
        var notes = new List<string>();
        foreach (var kv in accepted)
        {
            var v = vectors[kv.Key];
            var md = file.FindTypeByName<Google.Protobuf.Reflection.MessageDescriptor>(v["root"].AsString);
            if (md == null) { absent++; continue; }
            bool gpOk;
            try { md.Parser.ParseFrom(File.ReadAllBytes(Path.Combine(dir, v["file"].AsString))); gpOk = true; }
            catch (Exception) { gpOk = false; }
            if (gpOk == kv.Value) agree++;
            else { differ++; notes.Add(string.Format("  {0,-32} incumbent {1}, managed {2} (expect {3})", kv.Key,
                gpOk ? "accepts" : "refuses", kv.Value ? "accepts" : "refuses", v["expect"].AsString)); }
        }
        Console.WriteLine("Google.Protobuf as a second oracle (accept/reject, managed-drop arm): agrees on {0}, differs on {1}, {2} rows have no incumbent root.",
            agree, differ, absent);
        foreach (var n in notes.Take(40)) Console.WriteLine(n);
        if (notes.Count > 40) Console.WriteLine("  ... and {0} more", notes.Count - 40);
        Console.WriteLine();
    }
}
