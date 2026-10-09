// s13 (2026-10-09): AK_CORE_LIB=<absolute path of a libak_core.so> makes every ak_core import of
// this assembly bind to THAT file. BenchmarkDotNet's default toolchain rebuilds each case's
// child from the project, and Harness.csproj copies target-core's library into it, so a host-side
// copy of another core reaches the host process only; the variable (inherited by the children)
// is what puts a measurement core under the timed code. Unset: the default resolution, unchanged.

#if NET5_0_OR_GREATER
using System;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace Armonik.Ffi.Harness;

internal static class CoreLibOverride
{
    public static string Path { get; private set; }

    [ModuleInitializer]
    internal static void Init()
    {
        var p = Environment.GetEnvironmentVariable("AK_CORE_LIB");
        if (string.IsNullOrEmpty(p)) return;
        Path = p;
        NativeLibrary.SetDllImportResolver(typeof(CoreLibOverride).Assembly,
            (name, asm, search) => name == Abi.Lib ? NativeLibrary.Load(p) : IntPtr.Zero);
    }
}
#endif
