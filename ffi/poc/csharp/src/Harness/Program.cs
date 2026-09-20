using System;
using System.Globalization;
using System.Linq;
using System.Runtime;
using System.Runtime.InteropServices;

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
                return UnknownFields.Run();
            case "content":
                return ContentSets.Run(argv.Skip(1).ToArray());
            case "coreffi2":
#if NET8_0_OR_GREATER
                return CoreFfiGate2.Run(argv.Skip(1).ToArray());
#else
                Console.Error.WriteLine("core-ffi is not built on the floor runtime: "
                    + "net48 has no LibraryImport and no UnmanagedCallersOnly.");
                return 2;
#endif
            case "coreffi":
#if NET8_0_OR_GREATER
                return CoreFfiGate.Run(argv.Skip(1).ToArray());
#else
                // Arm c: .NET Framework 4.8 has no LibraryImport and no
                // UnmanagedCallersOnly, so the core-ffi binding does not exist
                // in this build. Saying so beats a stale binary reporting a pass.
                Console.Error.WriteLine("core-ffi is not built on the floor runtime: "
                    + "net48 has no LibraryImport and no UnmanagedCallersOnly.");
                return 2;
#endif
            case "mapforms":
                return MapForms.Run(argv.Skip(1).ToArray());
            case "counts":
                return Counts.Run();
            case "bench":
                return Bench.Run(argv.Skip(1).ToArray());
            default:
                Console.Error.WriteLine("usage: harness [conformance|unknown|counts|content|coreffi|coreffi2|mapforms|bench]");
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
        Console.WriteLine("# facade TFM:          {0}", Facade.BuildInfo.Tfm);
        Console.WriteLine("# transcoder:          {0}", Facade.BuildInfo.Transcoder);
        Console.WriteLine("# length-prefix sites: {0}", Armonik.Ffi.Facade.Codec.Sites);
        Console.WriteLine("#");
        Console.WriteLine("# R13 calibration on THIS machine: the Rust slice's crossing benchmark");
        Console.WriteLine("#   measures 1.8 ns forward and 2.1 ns forward-plus-reverse here");
        Console.WriteLine("#   (ffi/logs/csharp/calibration-rust-crossing.log). Every absolute");
        Console.WriteLine("#   below is also quotable as a multiple of that 1.8 ns.");
        Console.WriteLine("#");
    }

    private static string Env(string k, string dflt)
    {
        var v = Environment.GetEnvironmentVariable(k);
        return string.IsNullOrEmpty(v) ? dflt : v;
    }
}
