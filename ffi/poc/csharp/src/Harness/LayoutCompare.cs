// The layout comparison (by name, both ways) shared by the harness, akrpc and the corpus
// runner; see LayoutCheck.cs for what is compared and why (FIX-PLAN WP5 step 4, R-E6).

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.RegularExpressions;

namespace Armonik.Ffi.Harness;

/// The comparison itself, shared with the RPC binding's check (src/Rpc) and the corpus
/// binding's (src/Corpus), each passing its own tables.
public static class LayoutCompare
{

    public static int Compare(Json probe, List<(string Name, int Size, int Fields, List<(string Name, int Off, int Size)> F)> mine,
                              string path, Func<List<string>> facts, int factCount, Func<string> info)
    {
        int bad = 0, members = 0;
        var seen = new HashSet<string>(StringComparer.Ordinal);
        foreach (var s in mine)
        {
            seen.Add(s.Name);
            var p = probe[s.Name];
            if (p == null) { Console.WriteLine("  FAIL {0}: declared in C#, absent from the Rust declaration", s.Name); bad++; continue; }
            if (s.Fields != s.F.Count) { Console.WriteLine("  FAIL {0}: {1} C# fields, the table lists {2}", s.Name, s.Fields, s.F.Count); bad++; }
            if (p["size"].AsInt != s.Size) { Console.WriteLine("  FAIL {0}: size Rust {1}, C# {2}", s.Name, p["size"].AsInt, s.Size); bad++; }
            var rust = p["fields"].Arr.ToDictionary(f => f["name"].AsString, f => (Off: f["offset"].AsInt, Size: f["size"].AsInt), StringComparer.Ordinal);
            var cs = s.F.ToDictionary(f => f.Name, f => (f.Off, f.Size), StringComparer.Ordinal);
            foreach (var n in rust.Keys.Where(n => !cs.ContainsKey(n)))
            { Console.WriteLine("  FAIL {0}.{1}: in the Rust declaration, not in the C# one", s.Name, n); bad++; }
            foreach (var n in cs.Keys.Where(n => !rust.ContainsKey(n)))
            { Console.WriteLine("  FAIL {0}.{1}: in the C# declaration, not in the Rust one", s.Name, n); bad++; }
            foreach (var n in cs.Keys.Where(rust.ContainsKey))
            {
                members++;
                if (cs[n].Off != rust[n].Off) { Console.WriteLine("  FAIL {0}.{1}: offset Rust {2}, C# {3}", s.Name, n, rust[n].Off, cs[n].Off); bad++; }
                if (cs[n].Size != rust[n].Size) { Console.WriteLine("  FAIL {0}.{1}: size Rust {2}, C# {3}", s.Name, n, rust[n].Size, cs[n].Size); bad++; }
            }
        }
        // decision 11: the per-root options struct must be bound too (WP5 step 9).
        var must = new Regex("^ak_[edu]fix_|^ak_[ed]vt_|^ak_dec_\\w+_opts$");
        var unbound = new List<string>();
        foreach (var n in probe.Keys)
        {
            if (seen.Contains(n)) continue;
            if (facts != null && must.IsMatch(n)) { Console.WriteLine("  FAIL {0}: a group/vtable of the Rust declaration the C# binding lacks", n); bad++; }
            else unbound.Add(n);
        }
        Console.WriteLine("layout, by name both ways: {0} C# structs / {1} members against the Rust declaration probed in {2}: {3}",
            mine.Count, members, Path.GetFileName(path), bad == 0 ? "all agree" : bad + " disagreement(s)");
        if (unbound.Count != 0)
            Console.WriteLine("  (declared in ak-abi, not bound by this binding: {0}{1})",
                string.Join(", ", unbound.Take(8)), unbound.Count > 8 ? ", ... " + (unbound.Count - 8) + " more" : "");
        if (facts != null)
        {
            var f = facts();
            foreach (var x in f.Take(20)) Console.WriteLine("  FAIL section 10: {0}", x);
            Console.WriteLine("section 10, the loaded core's ak_layout_facts against this compiler: {0} facts, {1}",
                factCount, f.Count == 0 ? "all agree" : f.Count + " disagreement(s)");
            bad += f.Count;
        }
        if (info != null) Console.WriteLine(info());
        Console.WriteLine("{0} failure(s)", bad);
        return bad;
    }
}
