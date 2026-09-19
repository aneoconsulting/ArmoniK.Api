// Which sources this assembly was built from, so a log cannot claim the wrong
// arm. README 5.2's three arms differ by exactly this flag and the runtime
// under them, and a configuration line written by hand is the one part of a log
// that can be wrong without anything failing.

namespace Armonik.Ffi.Facade;

public static class BuildInfo
{
    // `static readonly`, not `const`. A `const` is inlined into every assembly
    // that reads it, so the harness would report the flag IT was compiled with
    // rather than the one the facade was -- and the first floor build here was
    // wrong in exactly the other direction (the define had been dropped from
    // the csproj and the harness cheerfully reported `no`, which is what a
    // configuration line that can lie looks like). Reading it out of the facade
    // assembly means the line is a fact about the code that runs.
#if AK_FLOOR
    public static readonly bool Floor = true;
#else
    public static readonly bool Floor = false;
#endif

#if NETSTANDARD2_0
    public static readonly string Tfm = "netstandard2.0";
#elif NET48
    public static readonly string Tfm = "net48";
#elif NET8_0_OR_GREATER
    public static readonly string Tfm = "net8.0";
#else
    public static readonly string Tfm = "unknown";
#endif

    /// Which transcode path `Enc.StringField` compiled to. Printed beside the
    /// flag because the flag alone does not say what it selected.
    public static readonly string Transcoder =
#if AK_FLOOR
        "Encoding.UTF8.GetBytes(char*, int, byte*, int) -- netstandard2.0, unsafe";
#else
        "Encoding.UTF8.GetBytes(ReadOnlySpan<char>, Span<byte>) -- net8.0";
#endif
}
