// CAMPAIGN req 17 as amended (D10): Nagle off on every client socket, READ BACK on the live
// sockets of the timed process: every TCP socket of this process connected to the server's
// port (from /proc/self/net/tcp and /proc/self/fd), getsockopt(IPPROTO_TCP, TCP_NODELAY).

using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Runtime.InteropServices;

namespace Armonik.Ffi.Campaign;

internal static unsafe class NoDelay
{
    [DllImport("libc", SetLastError = true)] private static extern int getsockopt(int fd, int level, int name, int* val, int* len);

    /// (sockets to 127.0.0.1:port found, of them with TCP_NODELAY = 1).
    public static (int Found, int NoDelay) Check(int port)
    {
        var inodes = new HashSet<string>();
        string hp = ":" + port.ToString("X4", CultureInfo.InvariantCulture);
        foreach (var line in File.ReadAllLines("/proc/self/net/tcp"))
        {
            var f = line.Split(' ', StringSplitOptions.RemoveEmptyEntries);
            if (f.Length < 10 || f[0] == "sl") continue;
            // remote 0100007F:PORT, state 01 (established)
            if (f[2] == "0100007F" + hp && f[3] == "01") inodes.Add(f[9]);
        }
        int found = 0, nd = 0;
        foreach (var fdp in Directory.EnumerateFiles("/proc/self/fd"))
        {
            string target;
            try { target = new FileInfo(fdp).LinkTarget; } catch { continue; }
            if (target == null || !target.StartsWith("socket:[", StringComparison.Ordinal)) continue;
            if (!inodes.Contains(target.Substring(8, target.Length - 9))) continue;
            if (!int.TryParse(Path.GetFileName(fdp), out int fd)) continue;
            int v = 0, l = sizeof(int);
            if (getsockopt(fd, 6 /* IPPROTO_TCP */, 1 /* TCP_NODELAY */, &v, &l) != 0) continue;
            found++;
            if (v != 0) nd++;
        }
        return (found, nd);
    }
}

/// CAMPAIGN req 21 as amended: softirq and irq time on the CLIENT CPUs (AK_CPU_CLIENT), from
/// /proc/stat (USER_HZ ticks, usually 10 ms: a coarse, system-wide per-CPU record, kept beside
/// task-clock and the process clock, never subtracted from either).
internal static class Irq
{
    private static HashSet<int> _cpus;

    private static HashSet<int> Cpus()
    {
        if (_cpus != null) return _cpus;
        _cpus = new HashSet<int>();
        var l = Environment.GetEnvironmentVariable("AK_CPU_CLIENT");
        if (string.IsNullOrEmpty(l)) return _cpus;
        foreach (var r in l.Split(','))
        {
            var p = r.Split('-');
            int a = int.Parse(p[0], CultureInfo.InvariantCulture), b = p.Length > 1 ? int.Parse(p[1], CultureInfo.InvariantCulture) : a;
            for (int i = a; i <= b; i++) _cpus.Add(i);
        }
        return _cpus;
    }

    public static (long SoftIrq, long Irq) Client()
    {
        long s = 0, q = 0;
        var cpus = Cpus();
        foreach (var line in File.ReadAllLines("/proc/stat"))
        {
            if (!line.StartsWith("cpu", StringComparison.Ordinal) || line.StartsWith("cpu ", StringComparison.Ordinal)) continue;
            var f = line.Split(' ', StringSplitOptions.RemoveEmptyEntries);
            if (!int.TryParse(f[0].Substring(3), out int c) || (cpus.Count > 0 && !cpus.Contains(c))) continue;
            q += long.Parse(f[6], CultureInfo.InvariantCulture);
            s += long.Parse(f[7], CultureInfo.InvariantCulture);
        }
        return (s, q);
    }
}
