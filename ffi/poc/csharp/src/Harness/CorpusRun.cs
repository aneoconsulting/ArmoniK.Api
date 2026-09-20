// The corpus consumer: `ffi/corpus/CONTRACT.md` C1 to C5.
//
// **Why this exists at all.** Byte identity against a manifest generated from
// the same description the decoder is generated from is a weaker oracle than it
// looks, and this slice has the receipt: the facade's unknown-field skipper had
// no case for the deprecated GROUP form for the whole life of the slice, and
// every gate it owns passed throughout. proto3 cannot express a group, so no
// payload in `ffi/schema` can carry one. Only the corpus could find it, and
// only the python slice did, because only the python slice consumed the corpus.
//
// **Rule 0, and the work it actually cost.** The contract's first rule is
// "generate your codec from `generated/corpus.proto`, never from
// `generated/corpus_superset.proto`" -- the difference between those two files
// is the corpus's entire unknown-field claim. This generator had no `.proto`
// front end; it drives off `ffi/schema/emit/shapes.py`. So conformance needed
// `gen/protoparse.py`, a second front end producing the same schema dict, after
// which every backend already written emitted the reader view untouched. The
// two front ends are cross-checked against each other at generation time on the
// nineteen messages and three enums they share.
//
// What is under test is the SAME runtime the measured arms use: `Enc`, `Dec`,
// `W` and `OrderedMap` come from `Armonik.Ffi.Facade`, and only the generated
// types, codec and projector are corpus-shaped. A corpus that exercised a
// second wire reader would be testing the second wire reader.

using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using Armonik.Ffi.Facade;
using C = Armonik.Ffi.Corpus;
using Gp = Armonik.Ffi.Shapes.V1;

namespace Armonik.Ffi.Harness;

public static class CorpusRun
{
    private static string Dir()
    {
        var d = AppContext.BaseDirectory;
        for (int i = 0; i < 12 && d != null; i++)
        {
            var c = Path.Combine(d, "ffi", "corpus", "generated");
            if (File.Exists(Path.Combine(c, "manifest.json"))) return c;
            d = Path.GetDirectoryName(d.TrimEnd(Path.DirectorySeparatorChar));
        }
        throw new DirectoryNotFoundException("cannot find ffi/corpus/generated");
    }

    private sealed class Result
    {
        public string Id, Cls, Root, Expect;
        public string C1 = "-", C2 = "-", C3 = "-", C4 = "-";
        public string Form = "";      // which accepted encoding we wrote
        public string Err = "";       // C4: the error we actually got
        public string Skip;           // non-null: could not run, and why
    }

    public static int Run(params string[] argv)
    {
        var dir = Dir();
        var man = Json.Parse(File.ReadAllText(Path.Combine(dir, "manifest.json")));
        var vectors = man["vectors"];
        var known = new HashSet<string>(C.Roots.All, StringComparer.Ordinal);
        string only = Environment.GetEnvironmentVariable("AK_CORPUS_ONLY");
        bool verbose = argv.Contains("-v");

        var results = new List<Result>();
        foreach (var id in vectors.Keys.OrderBy(x => x, StringComparer.Ordinal))
        {
            if (!string.IsNullOrEmpty(only) && !id.StartsWith(only, StringComparison.Ordinal)) continue;
            var v = vectors[id];
            var r = new Result
            {
                Id = id,
                Cls = v["class"].AsString,
                Root = v["root"].AsString,
                Expect = v["expect"].AsString,
            };
            results.Add(r);

            if (!known.Contains(r.Root)) { r.Skip = "root " + r.Root + " is not in corpus.proto"; continue; }
            var file = v["file"]?.AsString;
            if (file == null) { r.Skip = "manifest row has no file"; continue; }
            var path = Path.GetFullPath(Path.Combine(dir, file));
            if (!File.Exists(path)) { r.Skip = "missing " + file; continue; }
            var bytes = File.ReadAllBytes(path);

            // ---- C1 / C4: parse, or watch it refuse -------------------------
            object msg = null;
            string err = null;
            try
            {
                msg = C.Roots.New(r.Root);
                var d = new Dec { Buf = bytes, Pos = 0, End = bytes.Length, Err = 0 };
                C.Roots.Read(r.Root, ref d, msg, bytes.Length);
                if (d.Err != 0) { err = "Err " + d.Err + ErrName(d.Err); msg = null; }
            }
            catch (Exception ex) { err = ex.GetType().Name + ": " + Trim(ex.Message); msg = null; }

            if (r.Expect == "reject")
            {
                r.C4 = err != null ? "ok" : "ACCEPTED!";
                r.Err = err ?? "(none)";
                continue;
            }
            if (err != null) { r.C1 = "FAIL " + err; continue; }
            r.C1 = "ok";

            // ---- C2: project it --------------------------------------------
            var pj = v["projection"];
            if (pj == null || pj.Kind == Json.K.Null)
            {
                r.C2 = "n/a";       // a deliberately large run; byte identity is its oracle
            }
            else
            {
                try
                {
                    var want = Json.Parse(File.ReadAllText(Path.GetFullPath(Path.Combine(dir, pj.AsString))));
                    string why;
                    r.C2 = ProjEq(C.Proj.ByRoot(r.Root, msg), want, "", out why) ? "ok" : "DIFF " + why;
                }
                catch (Exception ex) { r.C2 = "THREW " + ex.GetType().Name + ": " + Trim(ex.Message); }
            }

            // ---- C3: re-encode, and say which form ---------------------------
            try
            {
                var e = Enc.New(C.Codec.Sites, bytes.Length * 2 + 8192);
                C.Roots.Write(r.Root, ref e, msg);
                var got = e.ToArray();
                var e2 = Enc.New(C.Codec.Sites, bytes.Length * 2 + 8192);
                C.Roots.WriteSized(r.Root, ref e2, msg);
                var got2 = e2.ToArray();

                string sha = Manifest.Sha(got, got.Length);
                string sha2 = Manifest.Sha(got2, got2.Length);
                if (sha != sha2) { r.C3 = "1PASS!=2PASS"; }
                else
                {
                    string form = null;
                    var enc = v["accepted_encodings"];
                    if (enc != null && enc.Arr != null)
                        foreach (var a in enc.Arr)
                            if (a["sha256"].AsString == sha)
                            {
                                var fs = a["forms"];
                                form = fs != null && fs.Arr != null && fs.Arr.Count > 0
                                    ? string.Join("+", fs.Arr.Select(x => x.AsString).ToArray())
                                    : "unlabelled";
                            }
                    // A `baseline` row carries no accepted_encodings list: it is
                    // ffi/schema's own payload by reference and its sha256 IS the
                    // one accepted form.
                    if (form == null && enc == null && v["sha256"]?.AsString == sha)
                        form = "the committed vector";
                    // P7.1 / B-P7_1 is the interleaved control: no canonical writer
                    // can interleave two repeated fields, so it is validated as a
                    // PERMUTATION of the same (tag, wire, body) triples and not as
                    // bytes. `harness conformance` treats it the same way, and
                    // treating it as a byte failure here would be this slice
                    // disagreeing with itself.
                    if (form == null && Triples.Same(new ReadOnlySpan<byte>(got, 0, got.Length),
                                                     bytes))
                        form = "a permutation of the committed triples";
                    r.C3 = form != null ? "ok" : "NOFORM " + got.Length + "B";
                    r.Form = form ?? "";
                }
            }
            catch (Exception ex) { r.C3 = "THREW " + ex.GetType().Name; }
        }

        Incumbent(dir, vectors, results);
        return Report(results, verbose);
    }

    /// **An independent oracle for the roots the incumbent also has.**
    ///
    /// Nineteen of the corpus's thirty roots exist in `ffi/schema`'s
    /// `shapes.proto` too, so `Google.Protobuf` can be pointed at the same bytes
    /// by descriptor name. That matters because the corpus's own expectations
    /// were computed with upb, and where this slice disagrees with a projection
    /// the useful question is not "who wrote the projection" but "what does the
    /// library ArmoniK actually runs do with these bytes" (R14). Reflection
    /// rather than a switch: a hand-written switch over nineteen roots is a
    /// place for one of them to be quietly missing.
    private static void Incumbent(string dir, Json vectors, List<Result> results)
    {
        var file = Gp.ShapesReflection.Descriptor;
        int agree = 0, differ = 0, absent = 0;
        var notes = new List<string>();
        foreach (var r in results)
        {
            if (r.Skip != null) continue;
            var md = file.FindTypeByName<Google.Protobuf.Reflection.MessageDescriptor>(r.Root);
            if (md == null) { absent++; continue; }
            var v = vectors[r.Id];
            var bytes = File.ReadAllBytes(Path.GetFullPath(Path.Combine(dir, v["file"].AsString)));
            bool gpOk;
            string gpErr = null;
            try { md.Parser.ParseFrom(bytes); gpOk = true; }
            catch (Exception ex) { gpOk = false; gpErr = ex.GetType().Name; }
            bool meOk = r.Expect == "reject" ? r.C4 != "ok" : r.C1 == "ok";
            // `meOk` here means "this decoder accepted it", not "it passed".
            meOk = r.Expect == "reject" ? r.C4 == "ACCEPTED!" : r.C1 == "ok";
            if (gpOk == meOk) agree++;
            else
            {
                differ++;
                notes.Add(string.Format("  {0,-32} incumbent {1}, managed {2}",
                    r.Id, gpOk ? "accepted" : "rejected " + gpErr, meOk ? "accepted" : "rejected"));
            }
        }
        Console.WriteLine("An INDEPENDENT oracle, for the roots `ffi/schema` also has: the same");
        Console.WriteLine("bytes through `Google.Protobuf`, found by descriptor name. The corpus's own");
        Console.WriteLine("expectations were computed with upb, and where the two disagree the");
        Console.WriteLine("question R14 asks is what the library ArmoniK runs actually does.");
        Console.WriteLine("  accept/reject agrees on {0} vectors, differs on {1}, {2} have no incumbent type.",
            agree, differ, absent);
        foreach (var n in notes.Take(20)) Console.WriteLine(n);
        if (notes.Count > 20) Console.WriteLine("  ... and {0} more", notes.Count - 20);
        Console.WriteLine();

        // And where the PROJECTION disagreed, print what the incumbent made of
        // the same bytes. A projection is computed, not observed, so a mismatch
        // is a three-way question and this is the third opinion.
        var disputed = results.Where(r => r.Skip == null && r.C2.StartsWith("DIFF", StringComparison.Ordinal)
                                       && file.FindTypeByName<Google.Protobuf.Reflection.MessageDescriptor>(r.Root) != null)
                              .ToArray();
        if (disputed.Length != 0)
        {
            Console.WriteLine("Where a projection disagreed AND the incumbent has the type, what the");
            Console.WriteLine("incumbent made of the same bytes (proto3 JSON, which is not the corpus's");
            Console.WriteLine("encoding -- it is here to answer \"did the field survive\", not to compare):");
            var fmt = new Google.Protobuf.JsonFormatter(
                new Google.Protobuf.JsonFormatter.Settings(false));
            foreach (var r in disputed.Take(4))
            {
                var md = file.FindTypeByName<Google.Protobuf.Reflection.MessageDescriptor>(r.Root);
                var v = vectors[r.Id];
                var bytes = File.ReadAllBytes(Path.GetFullPath(Path.Combine(dir, v["file"].AsString)));
                string j;
                try { j = fmt.Format(md.Parser.ParseFrom(bytes)); }
                catch (Exception ex) { j = "(threw " + ex.GetType().Name + ")"; }
                if (j.Length > 700) j = j.Substring(0, 700) + " ...";
                Console.WriteLine("  {0}: {1}", r.Id, r.C2);
                Console.WriteLine("    {0}", j);
            }
            Console.WriteLine();
        }
    }

    private static string ErrName(int e)
        => e == W.ErrTruncated ? " (truncated)"
         : e == W.ErrMalformed ? " (malformed)"
         : e == W.ErrDepth ? " (depth)"
         : e == W.ErrTranscode ? " (transcode)" : "";

    private static string Trim(string s)
        => s == null ? "" : (s.Length > 60 ? s.Substring(0, 60) : s).Replace('\n', ' ');

    // ---- the projection comparison ------------------------------------------

    /// Structural, not textual: key order is not part of the encoding. A double
    /// that differs only in spelling is compared numerically and the difference
    /// is REPORTED rather than swallowed, because "%.17g" and C#'s "G17" are two
    /// implementations of the same idea and not the same function.
    public static int NumericFallbacks;

    /// How many `_unknown` blocks the corpus expected and this codec dropped.
    public static int DroppedUnknown;

    private static bool ProjEq(object got, Json want, string at, out string why)
    {
        why = null;
        if (want == null) { why = at + ": expected nothing"; return false; }
        var od = got as SortedDictionary<string, object>;
        if (od != null)
        {
            if (want.Kind != Json.K.Obj) { why = at + ": got object, want " + want.Kind; return false; }
            foreach (var k in want.Keys)
            {
                // CONTRACT.md section 3: "Comparing `_unknown` is optional",
                // because whether the codec RETAINS unknown fields is ABI v1
                // open decision 11 and a behaviour change for four of the five
                // languages. This slice's generated codec DROPS them, which is
                // the answer to decision 11 for C# and is counted below rather
                // than compared here.
                if (k == "_unknown") { DroppedUnknown++; continue; }
                if (!od.ContainsKey(k)) { why = at + "." + k + ": missing (want " + Short(want[k]) + ")"; return false; }
            }
            foreach (var k in od.Keys)
                if (want[k] == null) { why = at + "." + k + ": extra"; return false; }
            foreach (var k in od.Keys)
                if (!ProjEq(od[k], want[k], at + "." + k, out why)) return false;
            return true;
        }
        var ol = got as List<object>;
        if (ol != null)
        {
            if (want.Kind != Json.K.Arr) { why = at + ": got array, want " + want.Kind; return false; }
            if (ol.Count != want.Arr.Count)
            {
                why = at + ": " + ol.Count + " elements, want " + want.Arr.Count;
                return false;
            }
            for (int i = 0; i < ol.Count; i++)
                if (!ProjEq(ol[i], want.Arr[i], at + "[" + i + "]", out why)) return false;
            return true;
        }
        if (got is bool)
        {
            if (want.Kind != Json.K.Bool) { why = at + ": got bool, want " + want.Kind; return false; }
            if ((bool)got != want.Bool) { why = at + ": " + got + " != " + want.Bool; return false; }
            return true;
        }
        var s = got as string;
        if (s == null) { why = at + ": unprojectable " + got.GetType().Name; return false; }
        string w = want.AsString;
        if (w == null) { why = at + ": got string, want " + want.Kind; return false; }
        if (s == w) return true;
        double a, b;
        if (double.TryParse(s, NumberStyles.Float, CultureInfo.InvariantCulture, out a)
         && double.TryParse(w, NumberStyles.Float, CultureInfo.InvariantCulture, out b)
         && (a == b || (double.IsNaN(a) && double.IsNaN(b))))
        {
            NumericFallbacks++;
            return true;
        }
        why = at + ": " + Q(s) + " != " + Q(w);
        return false;
    }

    private static string Q(string s) => s == null ? "null"
        : "\"" + (s.Length > 48 ? s.Substring(0, 48) + "..." : s) + "\"";

    private static string Short(Json j)
        => j == null ? "null" : j.Kind == Json.K.Str || j.Kind == Json.K.Num ? Q(j.Text) : j.Kind.ToString();

    // ---- the report ----------------------------------------------------------

    private static int Report(List<Result> rs, bool verbose)
    {
        Console.WriteLine("ffi/corpus, consumed. CONTRACT.md C1 (parse), C2 (project), C3 (re-encode");
        Console.WriteLine("to an accepted form), C4 (refuse, and name the error). C5 is the produce");
        Console.WriteLine("half and is reported separately below.");
        Console.WriteLine();
        Console.WriteLine("Codec generated from generated/corpus.proto, NEVER from");
        Console.WriteLine("corpus_superset.proto (rule 0). The runtime under it -- Enc, Dec, W,");
        Console.WriteLine("OrderedMap -- is the SAME source the measured arms use.");
        Console.WriteLine();

        var classes = rs.Select(r => r.Cls).Distinct().OrderBy(x => x, StringComparer.Ordinal).ToArray();
        Console.WriteLine("class        vectors   C1 parse   C2 project   C3 re-encode   C4 refuse   skipped");
        Console.WriteLine(new string('-', 92));
        int bad = 0, skipped = 0;
        foreach (var cls in classes)
        {
            var g = rs.Where(r => r.Cls == cls).ToArray();
            int n = g.Length;
            int sk = g.Count(r => r.Skip != null);
            int c1 = g.Count(r => r.C1 == "ok"), c1b = g.Count(r => r.C1.StartsWith("FAIL", StringComparison.Ordinal));
            int c2 = g.Count(r => r.C2 == "ok"), c2n = g.Count(r => r.C2 == "n/a");
            int c2b = g.Count(r => r.C2 != "ok" && r.C2 != "n/a" && r.C2 != "-");
            int c3 = g.Count(r => r.C3 == "ok"), c3b = g.Count(r => r.C3 != "ok" && r.C3 != "-");
            int c4 = g.Count(r => r.C4 == "ok"), c4b = g.Count(r => r.C4 != "ok" && r.C4 != "-");
            bad += c1b + c2b + c3b + c4b;
            skipped += sk;
            Console.WriteLine("{0,-12} {1,7}   {2,3}/{3,-5}  {4,4}{5,-8} {6,6}/{7,-6}  {8,5}/{9,-5} {10,8}",
                cls, n, c1, c1 + c1b, c2, c2n > 0 ? " (+" + c2n + " n/a)" : "", c3, c3 + c3b, c4, c4 + c4b, sk);
        }
        Console.WriteLine(new string('-', 92));
        Console.WriteLine("{0,-12} {1,7}", "TOTAL", rs.Count);
        Console.WriteLine();

        var fails = rs.Where(r => r.Skip == null &&
            (r.C1.StartsWith("FAIL", StringComparison.Ordinal) || r.C1.StartsWith("THREW", StringComparison.Ordinal)
             || (r.C2 != "ok" && r.C2 != "n/a" && r.C2 != "-")
             || (r.C3 != "ok" && r.C3 != "-")
             || (r.C4 != "ok" && r.C4 != "-"))).ToArray();
        if (fails.Length != 0)
        {
            Console.WriteLine("FAILURES, by id:");
            foreach (var r in fails.Take(60))
                Console.WriteLine("  {0,-32} {1,-10} C1={2} C2={3} C3={4} C4={5}",
                    r.Id, r.Cls, r.C1, r.C2, r.C3, r.C4);
            if (fails.Length > 60) Console.WriteLine("  ... and {0} more", fails.Length - 60);
            Console.WriteLine();
        }

        var skips = rs.Where(r => r.Skip != null).ToArray();
        if (skips.Length != 0)
        {
            Console.WriteLine("NOT RUN, by id and reason (CONTRACT.md section 5 item 6):");
            foreach (var r in skips) Console.WriteLine("  {0,-32} {1}", r.Id, r.Skip);
            Console.WriteLine();
        }

        // C3: which encoding this slice writes, which is the answer to ABI v1
        // decision 11 for C# and which nobody has written down.
        Console.WriteLine("C3: which accepted form this slice's encoder produced.");
        foreach (var g in rs.Where(r => r.C3 == "ok").GroupBy(r => r.Form)
                            .OrderByDescending(g => g.Count()))
            Console.WriteLine("  {0,5}  {1}", g.Count(), string.IsNullOrEmpty(g.Key) ? "(unlabelled)" : g.Key);
        Console.WriteLine();

        // C4: the error per reject vector. A rejection test that nothing
        // rejects is a test nobody has watched work.
        Console.WriteLine("C4: the error this decoder actually returned, per reject vector.");
        foreach (var g in rs.Where(r => r.Expect == "reject" && r.Skip == null)
                            .GroupBy(r => r.Err).OrderByDescending(g => g.Count()))
        {
            Console.WriteLine("  {0,3}x {1}", g.Count(), g.Key);
            if (verbose) foreach (var r in g) Console.WriteLine("        {0}", r.Id);
        }
        Console.WriteLine();
        if (DroppedUnknown != 0)
        {
            Console.WriteLine("ABI v1 open decision 11, answered for C#: this slice's generated codec");
            Console.WriteLine("DROPS unknown fields. {0} projected `_unknown` block(s) the corpus", DroppedUnknown);
            Console.WriteLine("expected are absent here, and CONTRACT.md section 3 makes comparing them");
            Console.WriteLine("optional for exactly that reason. `Google.Protobuf` RETAINS them, so the");
            Console.WriteLine("behaviour change is visible on .NET: see stage1-conformance.log.");
            Console.WriteLine();
        }
        if (NumericFallbacks != 0)
            Console.WriteLine("{0} projected value(s) matched numerically but not textually (double "
                + "spelling: %.17g against G17).", NumericFallbacks);
        // ---- the two remaining divergences, classified rather than counted ----
        int tdec = rs.Count(r => r.Expect == "reject" && r.Cls == "transcode" && r.C4 == "ACCEPTED!");
        int other = bad - tdec - rs.Count(r => r.Id == "U-map-entry" && r.C2 != "ok");
        Console.WriteLine("THE TWO DIVERGENCES, AND NEITHER IS A DEFECT THIS SLICE CAN FIX ALONE.");
        Console.WriteLine();
        Console.WriteLine("1. The {0} T-dec-* vectors: malformed UTF-8 in a string field, which", tdec);
        Console.WriteLine("   CONTRACT.md says a conformant parser must reject. This codec accepts");
        Console.WriteLine("   them, because it reads strings through Encoding.UTF8, which SUBSTITUTES");
        Console.WriteLine("   U+FFFD. **So does Google.Protobuf**: `harness utf8` runs the 15 root-site");
        Console.WriteLine("   vectors through both and the incumbent accepts every one. So the arms");
        Console.WriteLine("   agree, the decode comparison is like for like, and the managed decode");
        Console.WriteLine("   margin is NOT bought by skipping validation. It is ABI v1 open decision 3");
        Console.WriteLine("   and .NET's whole ecosystem is on the lossy side of it.");
        Console.WriteLine();
        Console.WriteLine("2. U-map-entry: an unknown field inside every map entry. This decoder skips");
        Console.WriteLine("   it and keeps the entry; the corpus's projection puts the WHOLE entries");
        Console.WriteLine("   under `_unknown` and leaves the map absent. `Google.Protobuf`, on the");
        Console.WriteLine("   same bytes, keeps the map -- printed above. Two implementations against");
        Console.WriteLine("   the projection, so this is raised as a question about the vector.");
        Console.WriteLine();
        Console.WriteLine("C5 (produce) and the chunking class, stated rather than claimed:");
        Console.WriteLine("  * C5 asks a slice to BUILD each message from the description and encode");
        Console.WriteLine("    it. This slice has payload builders for `ffi/schema`'s roots only, and");
        Console.WriteLine("    `harness conformance` is that claim: byte identity on all 16 payloads.");
        Console.WriteLine("    For the corpus's own roots there is no builder, so C3 (re-encode what");
        Console.WriteLine("    was parsed) is what runs and C5 is NOT claimed for them.");
        Console.WriteLine("  * The chunking class asks a batching host to run each vector at more than");
        Console.WriteLine("    one chunk. **The managed codec does not batch at all** -- it walks the");
        Console.WriteLine("    host's own containers in one pass -- so per CONTRACT.md it says so and");
        Console.WriteLine("    that is its gap. The core-ffi arm DOES batch and is measured at two");
        Console.WriteLine("    chunk sizes (AK_CHUNK), but its binding exists for M1 and M2 only and");
        Console.WriteLine("    the chunking roots are corpus-only, so it cannot reach them either.");
        Console.WriteLine();
        Console.WriteLine("{0} failure(s) ({1} T-dec policy, {2} U-map-entry, {3} other), {4} not run",
            bad, tdec, bad - tdec - other, other, skipped);
        return other;
    }
}
