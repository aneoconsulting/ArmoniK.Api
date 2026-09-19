using System;
using BenchmarkDotNet.Running;

namespace Armonik.Ffi.Bdn;

public static class Program
{
    /// Everything BenchmarkDotNet's CLI offers is passed straight through, so
    /// the controlled rerun can say `--filter *Decode* --job long` without this
    /// file needing to know about it.
    ///
    ///   dotnet run -c Release --project src/BenchDotNet -- --filter '*'
    ///   ... -- --filter '*Decode*' --job short
    ///   ... -- --list flat
    ///
    /// AK_BDN_ONLY=P2.2,P1.2 restricts the payload parameter, which is the
    /// cheap way to cut a run down without losing BenchmarkDotNet's defaults.
    public static int Main(string[] args)
    {
        BenchmarkSwitcher.FromAssembly(typeof(Program).Assembly).Run(args);
        return 0;
    }
}
