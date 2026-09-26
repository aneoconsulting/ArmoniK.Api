using System;
using System.Globalization;
using System.Linq;
using System.Runtime;
using System.Runtime.InteropServices;

// The campaign runner (src/Rpc, assembly akrpc) calls the generated imports directly
// (ak_noop for the crossing calibration), which the generator declares internal.
[assembly: System.Runtime.CompilerServices.InternalsVisibleTo("akrpc")]

namespace Armonik.Ffi.Harness;

public static class Program
{
    public static int Main(string[] argv)
    {
        var what = argv.Length > 0 ? argv[0] : "conformance";
        Config.Print(what);
        switch (what)
        {
            case "conformance":
                return Conformance.Run();
            case "unknown":
#if AK_NO_UNKNOWN_FIELDS
                Console.WriteLine("harness unknown: not in the no-unknown build (unknown fields compiled out)");
                return 0;
#else
                return UnknownFields.Run();
#endif
            case "groups":
                return GroupVectors.Run(argv.Skip(1).ToArray());
            case "corpus":
                Console.Error.WriteLine("the corpus runner is src/Corpus (managed + core-ffi, drop + retain, a child process per row)");
                return 2;
            case "layout":
                return LayoutCheck.Run(argv.Skip(1).ToArray());
            case "utf8":
                return Utf8Policy.Run(argv.Skip(1).ToArray());
            case "content":
                return ContentSets.Run(argv.Skip(1).ToArray());
            case "coreffi":
#if NET5_0_OR_GREATER
                // net8.0 and net6.0: the generated binding's LibraryImport (7+) or
                // DllImport (6) branch, and the [UnmanagedCallersOnly] host half.
                return CoreGate.Run(argv.Skip(1).ToArray());
#else
                // net48: the P/Invoke binding compiles (its DllImport branch), but
                // the host half needs [UnmanagedCallersOnly] (.NET 5+) and is
                // compiled out. Saying so beats a stale binary reporting a pass.
                Console.Error.WriteLine("core-ffi host half is not built on net48: no UnmanagedCallersOnly.");
                return 2;
#endif
            case "mapforms":
                return MapForms.Run(argv.Skip(1).ToArray());
            case "counts":
                return Counts.Run();
            case "bench":
                return Bench.Run(argv.Skip(1).ToArray());
            default:
                Console.Error.WriteLine("usage: harness [conformance|unknown|groups|utf8|counts|content|coreffi|layout|mapforms|bench]");
                return 2;
        }
    }
}

/// R7: name the configuration. Runtime version, incumbent library version,
/// binding mechanism, linkage, machine -- printed by the harness rather than
/// written into a log by hand, because a hand-written configuration line is the
/// one part of a log that can be wrong without anything failing.
public static class Config
{
    public static void Print(string what)
    {
        Console.WriteLine("# harness: {0}", what);
        Console.WriteLine("# utc:                 {0:yyyy-MM-ddTHH:mm:ssZ}", DateTime.UtcNow);
        Console.WriteLine("# runtime:             {0}", RuntimeInformation.FrameworkDescription);
        Console.WriteLine("# os:                  {0}", RuntimeInformation.OSDescription.Trim());
        Console.WriteLine("# arch:                {0}", RuntimeInformation.ProcessArchitecture);
        Console.WriteLine("# processors:          {0}", Environment.ProcessorCount);
        Console.WriteLine("# Google.Protobuf:     {0}",
            typeof(Google.Protobuf.MessageParser).Assembly.GetName().Version);
        Console.WriteLine("# server GC:           {0}", GCSettings.IsServerGC);
        Console.WriteLine("# latency mode:        {0}", GCSettings.LatencyMode);
        Console.WriteLine("# tiered compilation:  {0}", Env("DOTNET_TieredCompilation", "on (default)"));
        Console.WriteLine("# tiered PGO:          {0}", Env("DOTNET_TieredPGO", "on (default)"));
        Console.WriteLine("# R2R:                 {0}", Env("DOTNET_ReadyToRun", "on (default)"));
        Console.WriteLine("# floor sources:       {0}", Facade.BuildInfo.Floor ? "YES (AK_FLOOR)" : "no");
        Console.WriteLine("# managed codec:       plan options utf8={0} unknown={1} recursion_limit={2} (rendered by poc/codec/gen/cs_managed.py)",
            Armonik.Ffi.Facade.Codec.Utf8Policy, Armonik.Ffi.Facade.Codec.UnknownMode, Armonik.Ffi.Facade.Codec.Limit);
        Console.WriteLine("# facade TFM:          {0}", Facade.BuildInfo.Tfm);
        Console.WriteLine("# transcoder:          {0}", Facade.BuildInfo.Transcoder);
        Console.WriteLine("# length-prefix sites: {0}", Armonik.Ffi.Facade.Codec.Sites);
        Console.WriteLine("#");
        // This used to print a crossing calibration as a constant, "on THIS
        // machine", whatever machine ran it: a hand-written configuration line
        // that is wrong without anything failing. Removed (WP6). Crossing
        // calibration is a campaign measurement (design/CAMPAIGN.md).
        Console.WriteLine("# timings:             any timing a container prints is instrumentation (README 1.1)");
        Console.WriteLine("#");
    }

    private static string Env(string k, string dflt)
    {
        var v = Environment.GetEnvironmentVariable(k);
        return string.IsNullOrEmpty(v) ? dflt : v;
    }
}
