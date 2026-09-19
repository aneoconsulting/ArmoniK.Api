// The oracle: ffi/schema/generated/manifest.json, validated against prost
// 0.14.4 and prost-reflect 0.16.5 by the Rust slice. A slice that disagrees
// with a hash has found a defect in itself.
//
// Parsed by hand rather than with a JSON library: the harness has one
// dependency that matters (Google.Protobuf, the incumbent), and a second one
// would have to be excluded from every allocation figure by argument instead
// of by construction.

using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Security.Cryptography;

namespace Armonik.Ffi.Harness;

public sealed class PayloadRow
{
    public string Id;
    public string Root;
    public int Elements;
    public int Strings;
    public int Bytes;
    public string Sha256;
    public string Mode;
    public string Vector;      // may be null: over 64 KB is a hash only
}

public static class Manifest
{
    public static string SchemaDir()
    {
        var d = AppContext.BaseDirectory;
        for (int i = 0; i < 12 && d != null; i++)
        {
            var c = Path.Combine(d, "ffi", "schema", "generated");
            if (Directory.Exists(c)) return c;
            d = Path.GetDirectoryName(d.TrimEnd(Path.DirectorySeparatorChar));
        }
        throw new DirectoryNotFoundException("cannot find ffi/schema/generated from " + AppContext.BaseDirectory);
    }

    public static Dictionary<string, PayloadRow> Load()
    {
        var dir = SchemaDir();
        var text = File.ReadAllText(Path.Combine(dir, "manifest.json"));
        int at = text.IndexOf("\"payloads\"", StringComparison.Ordinal);
        if (at < 0) throw new InvalidDataException("manifest.json has no payloads object");
        var rows = new Dictionary<string, PayloadRow>(StringComparer.Ordinal);
        int i = text.IndexOf('{', at) + 1;
        while (true)
        {
            int k0 = text.IndexOf('"', i);
            if (k0 < 0) break;
            int k1 = text.IndexOf('"', k0 + 1);
            string id = text.Substring(k0 + 1, k1 - k0 - 1);
            int o0 = text.IndexOf('{', k1);
            int o1 = text.IndexOf('}', o0);
            if (o0 < 0 || o1 < 0) break;
            string body = text.Substring(o0 + 1, o1 - o0 - 1);
            rows[id] = new PayloadRow
            {
                Id = id,
                Root = Str(body, "root"),
                Elements = Int(body, "elements"),
                Strings = Int(body, "strings"),
                Bytes = Int(body, "bytes"),
                Sha256 = Str(body, "sha256"),
                Mode = Str(body, "mode"),
                Vector = Str(body, "vector"),
            };
            i = o1 + 1;
            int next = text.IndexOf('"', i);
            int close = text.IndexOf('}', i);
            if (next < 0 || (close >= 0 && close < next)) break;
        }
        if (rows.Count == 0) throw new InvalidDataException("manifest.json parsed to no rows");
        return rows;
    }

    private static string Str(string body, string key)
    {
        int at = body.IndexOf("\"" + key + "\"", StringComparison.Ordinal);
        if (at < 0) return null;
        int c = body.IndexOf(':', at);
        int q0 = body.IndexOf('"', c);
        if (q0 < 0) return null;
        int q1 = body.IndexOf('"', q0 + 1);
        return body.Substring(q0 + 1, q1 - q0 - 1);
    }

    private static int Int(string body, string key)
    {
        int at = body.IndexOf("\"" + key + "\"", StringComparison.Ordinal);
        if (at < 0) return -1;
        int c = body.IndexOf(':', at) + 1;
        int e = c;
        while (e < body.Length && (char.IsDigit(body[e]) || body[e] == ' ' || body[e] == '-')) e++;
        return int.Parse(body.Substring(c, e - c).Trim(), CultureInfo.InvariantCulture);
    }

    public static byte[] Vector(PayloadRow r)
    {
        return r.Vector == null ? null : File.ReadAllBytes(Path.Combine(SchemaDir(), r.Vector));
    }

    public static string Sha(byte[] b, int len)
    {
        using var s = SHA256.Create();
        var h = s.ComputeHash(b, 0, len);
        var c = new char[h.Length * 2];
        const string H = "0123456789abcdef";
        for (int i = 0; i < h.Length; i++) { c[2 * i] = H[h[i] >> 4]; c[2 * i + 1] = H[h[i] & 15]; }
        return new string(c);
    }
}
