// Step a2 (i), 2026-10-04 (JOURNAL 74): the push decode no longer calls ak_dec_err_reset /
// ak_dec_err around ak_decode_*, which clears the context's sticky slot on entry and returns
// it. This checks, per shape, that a HOST failure reported through ak_fail from inside a
// reverse call still reaches the caller as AK_ERR_HOST through the return value alone, in
// drop and retain modes. Run with AK_GATE_PLANT_HOST_FAIL=apply (every root apply throws) or =add
// (every add/new callback throws; shapes without one only pass through apply), and without it
// (every decode must succeed).
#if NET5_0_OR_GREATER
using System;
using System.Linq;

namespace Armonik.Ffi.Harness;

public static class HostFail
{
    public static int Run()
    {
        var plant = Environment.GetEnvironmentVariable("AK_GATE_PLANT_HOST_FAIL") ?? "";
        int bad = 0, failed = 0, ok = 0;
        foreach (var id in CoreArms.Ids)
        {
            using var arm = CoreArms.New(id);
            var got = arm.EncodeToArray();
            foreach (var retain in AbiVariant.UnknownCompiledOut ? new[] { false } : new[] { false, true })
            {
                string res;
                try { if (retain) arm.DecodeU(got, got.Length); else arm.Decode(got, got.Length); res = "ok"; }
                catch (InvalidOperationException e) { res = e.Message; }
                bool expectFail = plant == "apply" || (plant == "add" && res != "ok");
                bool isHost = res.EndsWith(": " + Abi.AK_ERR_HOST, StringComparison.Ordinal);
                if (plant == "") { if (res != "ok") { Console.WriteLine("  FAIL {0} {1}: {2}", id, retain ? "retain" : "drop", res); bad++; } else ok++; }
                else if (plant == "apply" && !isHost) { Console.WriteLine("  FAIL {0} {1}: expected AK_ERR_HOST ({2}) from the return value, got {3}", id, retain ? "retain" : "drop", Abi.AK_ERR_HOST, res); bad++; }
                else if (expectFail && !isHost) { Console.WriteLine("  FAIL {0} {1}: {2}", id, retain ? "retain" : "drop", res); bad++; }
                else if (isHost) failed++; else ok++;
            }
        }
        Console.WriteLine("hostfail (plant '{0}'): {1} decode(s) returned AK_ERR_HOST, {2} succeeded, {3} failure(s)", plant, failed, ok, bad);
        if (plant != "" && failed == 0) { Console.WriteLine("  FAIL the plant produced no host failure"); bad++; }
        return bad == 0 ? 0 : 1;
    }
}
#endif
