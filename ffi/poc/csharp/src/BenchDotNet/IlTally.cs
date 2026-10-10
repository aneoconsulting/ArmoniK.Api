// s17 (owner, 2026-10-10): the host tally is compiled into the counting build only. This
// check reads the IL of the built host (the harness assembly: the generated CoreFfi_* classes,
// the staging, the unknown-field host; the facade; this program's) and lists, per method, every store to a STATIC field and
// every call to the tally's [Conditional] methods (CountF, CountR, CountReset):
//
//   BenchDotNet --il-tally [--expect none|some]
//
// `none` (the timed builds): no call to a tally method and no store to _fwd, _rev, _resets or
// UnkHost.Grows anywhere in the assembly; `some` (the counting builds): at least one of each,
// which shows the walk sees them when they are there. The other static stores are listed, so a
// per-call static write that is not a tally shows up too (the sweep).
//
// The walk decodes the IL instruction by instruction (System.Reflection.Emit's opcode table
// gives each operand's size), so it reads no operand bytes as an opcode.
using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using System.Reflection.Emit;

namespace Armonik.Ffi.Bdn;

public static class IlTally
{
    private static readonly Dictionary<short, OpCode> Ops = typeof(OpCodes)
        .GetFields(BindingFlags.Public | BindingFlags.Static)
        .Select(f => (OpCode)f.GetValue(null)).ToDictionary(o => o.Value);

    private static int OperandSize(OpCode op, byte[] il, int at) => op.OperandType switch
    {
        OperandType.InlineNone => 0,
        OperandType.ShortInlineBrTarget or OperandType.ShortInlineI or OperandType.ShortInlineVar => 1,
        OperandType.InlineVar => 2,
        OperandType.InlineI8 or OperandType.InlineR => 8,
        OperandType.InlineSwitch => 4 + 4 * BitConverter.ToInt32(il, at),
        _ => 4,
    };

    private static readonly string[] Tally = { "_fwd", "_rev", "_resets", "Grows" };
    private static readonly string[] TallyCalls = { "CountF", "CountR", "CountReset" };

    public static int Run(string expect)
    {
        var asms = new[] { typeof(Armonik.Ffi.Harness.Stage).Assembly, typeof(Armonik.Ffi.Facade.Enc).Assembly, typeof(IlTally).Assembly };
        int tallyStores = 0, tallyCalls = 0, methods = 0;
        var other = new SortedDictionary<string, int>(StringComparer.Ordinal);
        const BindingFlags all = BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Static | BindingFlags.Instance | BindingFlags.DeclaredOnly;
        foreach (var t in asms.SelectMany(x => x.GetTypes()))
            foreach (var m in t.GetMethods(all).Cast<MethodBase>().Concat(t.GetConstructors(all)))
            {
                byte[] il;
                try { il = m.GetMethodBody()?.GetILAsByteArray(); } catch { il = null; }
                if (il == null) continue;
                methods++;
                for (int i = 0; i < il.Length;)
                {
                    short v = il[i] == 0xFE ? (short)(0xFE00 | il[i + 1]) : il[i];
                    int n = il[i] == 0xFE ? 2 : 1;
                    if (!Ops.TryGetValue(v, out var op)) throw new InvalidOperationException(t.Name + "." + m.Name + ": unknown opcode at " + i);
                    int at = i + n, sz = OperandSize(op, il, at);
                    if (op == OpCodes.Stsfld || op == OpCodes.Call)
                    {
                        int tok = BitConverter.ToInt32(il, at);
                        if (op == OpCodes.Stsfld)
                        {
                            var f = m.Module.ResolveField(tok, t.IsGenericType ? t.GetGenericArguments() : null, null);
                            if (Tally.Contains(f.Name)) tallyStores++;
                            // a static store outside a type initializer: listed for the sweep
                            else if (!(m is ConstructorInfo ci && ci.IsStatic))
                            {
                                var k = f.DeclaringType.Name + "." + f.Name + " in " + t.Name + "." + m.Name;
                                other[k] = other.TryGetValue(k, out var c) ? c + 1 : 1;
                            }
                        }
                        else
                        {
                            MethodBase cm = null;
                            try { cm = m.Module.ResolveMethod(tok); } catch { }
                            if (cm != null && TallyCalls.Contains(cm.Name)) tallyCalls++;
                        }
                    }
                    i = at + sz;
                }
            }
        Console.WriteLine("il-tally ({0} build, {1}): {2} methods read; tally call sites (CountF, CountR, CountReset) {3}; tally stores (_fwd, _rev, _resets, UnkHost.Grows) {4}",
            Armonik.Ffi.Harness.AbiVariant.Name,
#if AK_HOST_COUNT
            "counting",
#else
            "timed",
#endif
            methods, tallyCalls, tallyStores);
        Console.WriteLine("other static-field stores outside type initializers (field in method: sites):");
        foreach (var kv in other) Console.WriteLine("  " + kv.Key + ": " + kv.Value);
        bool ok = expect == "none" ? tallyCalls == 0 && tallyStores == 0
                : expect == "some" ? tallyCalls > 0 && tallyStores > 0
                : throw new ArgumentException("--expect none|some");
        Console.WriteLine("il-tally: expected {0}: {1}", expect, ok ? "ok" : "FAILED");
        return ok ? 0 : 1;
    }
}
