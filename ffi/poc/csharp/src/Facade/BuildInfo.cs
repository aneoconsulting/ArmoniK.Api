// Which sources this assembly was built from, so a log cannot claim the wrong
// arm. README 5.2's three arms differ by exactly this flag and the runtime
// under them, and a configuration line written by hand is the one part of a log
// that can be wrong without anything failing.

namespace Armonik.Ffi.Facade;

public static class BuildInfo
{
#if AK_FLOOR
    public const bool Floor = true;
#else
    public const bool Floor = false;
#endif

#if NETSTANDARD2_0
    public const string Tfm = "netstandard2.0";
#elif NET8_0_OR_GREATER
    public const string Tfm = "net8.0";
#else
    public const string Tfm = "unknown";
#endif
}
