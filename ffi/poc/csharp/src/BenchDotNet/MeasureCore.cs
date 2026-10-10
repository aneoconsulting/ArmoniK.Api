// s14 (owner, 2026-10-10): whether the loaded core is the MEASUREMENT stub core (ak-core
// feature `tc-measure-u16stub`: its UTF-16 transcoder copies raw bytes, so every E1 / E1R
// output is wrong). Only that core exports `ak_measure_tc_stub`; with it the pre-timing byte
// checks are skipped and the run says so; with any other core nothing changes.

using System;
using System.IO;
using System.Runtime.InteropServices;

namespace Armonik.Ffi.Bdn;

public static class MeasureCore
{
    public static readonly bool Stub = Detect();

    private static bool Detect()
    {
        var lib = Environment.GetEnvironmentVariable("AK_CORE_LIB");
        var path = string.IsNullOrEmpty(lib) ? Path.Combine(AppContext.BaseDirectory, "libak_core.so") : lib;
        return NativeLibrary.TryLoad(path, out var h) && NativeLibrary.TryGetExport(h, "ak_measure_tc_stub", out _);
    }
}
