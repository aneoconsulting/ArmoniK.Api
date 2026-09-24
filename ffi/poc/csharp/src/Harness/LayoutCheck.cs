// The layout agreement, by NAME, both ways (FIX-PLAN WP5 step 4, R-E6).
//
// Two independently produced descriptions of every ABI struct are compared here:
//
//   * the RUST declaration, as the Rust compiler lays it out, printed by the layout
//     probe (`abi/`), whose struct and member lists were parsed out of ak-abi's Rust
//     source text by `poc/codec/gen/cs_layout_probe.py` -- not taken from any plan;
//   * the C# declaration, as the C# compiler lays it out (`AbiLayout.Table()`: every
//     member of every struct the generated binding declares, with the compiler's own
//     offset and size; the reflection field count proves the table covers the struct).
//
// A member one side has and the other lacks, a different order (a different offset), a
// different width, a different total size: each is a failure. A group or vtable the Rust
// declaration has and the C# one lacks is a failure too. The earlier probe was emitted
// from the same field list the C# structs were, and checked sizes only (R-E6).
//
// Then ABI v1 section 10: the LOADED core's own `ak_layout_facts` against the same C#
// compiler's numbers (`AbiLayout.Facts()`), so a core built from a different declaration
// than the one probed is caught too.
//
//   harness layout PROBE.json [--plant]    --plant: swap two members' offsets in the C#
//                                          table before comparing (a control: must fail)

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.RegularExpressions;

namespace Armonik.Ffi.Harness;

public static class LayoutCheck
{
    public static int Run(params string[] argv)
    {
        var path = argv.FirstOrDefault(a => !a.StartsWith("--", StringComparison.Ordinal))
                   ?? Environment.GetEnvironmentVariable("AK_LAYOUT_PROBE");
        bool plant = argv.Contains("--plant");
        if (path == null) { Console.Error.WriteLine("usage: harness layout PROBE.json [--plant]"); return 2; }
        var probe = Json.Parse(File.ReadAllText(path));
        var mine = AbiLayout.Table();
        if (plant)
        {
            // Swap the offsets of the first two members of the first group with two or
            // more members: the comparison must see it.
            var s = mine.First(x => x.Name.StartsWith("ak_efix_", StringComparison.Ordinal) && x.F.Count >= 2);
            var a = s.F[0]; var b = s.F[1];
            s.F[0] = (a.Name, b.Off, a.Size); s.F[1] = (b.Name, a.Off, b.Size);
            Console.WriteLine("# PLANTED: offsets of {0}.{1} and {0}.{2} swapped in the C# table", s.Name, a.Name, b.Name);
        }
        return LayoutCompare.Compare(probe, mine.Select(s => (s.Name, s.Size, s.Fields, s.F)).ToList(), path,
                       AbiLayout.Facts, AbiLayout.FactCount, () => "ak_abi_version() = " + Abi.ak_abi_version() + "; ak_init() returned " + AbiInit.Code);
    }
}
