// D21 (owner, 2026-10-04): the string-length sweep that picks the E0/E1 threshold.
//
//   BenchDotNet --strsweep OUT.tsv [--rounds R] [--block-ms T] [--lengths 4,16,..] [--contents ascii,..]
//
// One core-ffi encode (EncodeInto: the form cell Cf hands the core's transport) of an
// UploadResultDataMessage whose ONE string field (upload.session_id) is L UTF-16 code units of
// one content (ascii 1 byte/char, latin1 U+00E0.. 2 bytes, wide CJK 3 bytes, astral U+1F600..
// as surrogate pairs, 4 bytes per 2 units), for L in 4, 16, 64, 256, 1 Ki, 16 Ki, under each
// string path E0, E1, E2 (Stage.Mode switched in this process), all in ONE process so the
// comparison shares it. Per (content, L): byte identity of the three paths first (each against
// the incumbent's encoding of the same message), then a warm-up (every path, >= 1 s in total, so
// tier-1 code is in place), then R rounds; in each round every path times one block of ~T ms
// (process CPU and wall per op), the path order rotated by round. A pin-only column times
// GCHandle.Alloc(s, Pinned) + Free alone (E1's own cost). Container instrumentation.

using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text;
using Armonik.Ffi.Facade;
using Armonik.Ffi.Harness;
using Google.Protobuf;

namespace Armonik.Ffi.Bdn;

public static class StrSweep
{
    private static string Make(string content, int n)
    {
        var sb = new StringBuilder(n);
        for (int i = 0; sb.Length < n; i++)
        {
            switch (content)
            {
                case "ascii": sb.Append((char)('a' + i % 26)); break;
                case "latin1": sb.Append((char)(0xE0 + i % 32)); break;
                case "wide": sb.Append((char)(0x4E00 + i % 512)); break;
                default:
                    if (sb.Length + 2 <= n) sb.Append(char.ConvertFromUtf32(0x1F600 + i % 64));
                    else sb.Append('a');
                    break;
            }
        }
        return sb.ToString();
    }

    private static string Env()
    {
        var l = new List<string>();
        foreach (var v in new[] { "SIMDUTF_FORCE_IMPLEMENTATION", "DOTNET_EnableAVX512F", "DOTNET_PreferredVectorBitWidth", "DOTNET_EnableAVX2" })
        {
            var x = Environment.GetEnvironmentVariable(v);
            if (x != null) l.Add(v + "=" + x);
        }
        l.Add("Vector512.IsHardwareAccelerated=" + System.Runtime.Intrinsics.Vector512.IsHardwareAccelerated);
        return string.Join(", ", l);
    }

    public static unsafe int Run(string outp, int rounds, double blockMs, int[] lengths, string[] contents, string[] paths)
    {
        var core = new CoreFfi_UploadResultDataMessage();
        // D21 step 7: the paths are AK_STR_ENC specs (E0 first: the incumbent the bytes are checked
        // against is Google.Protobuf's); each is set with its threshold and ASCII split.
        var names = paths;
        int[] modes = new int[names.Length], ths = new int[names.Length]; bool[] nas = new bool[names.Length];
        for (int q = 0; q < names.Length; q++) modes[q] = Stage.ParseMode(names[q], out ths[q], out nas[q]);
        void Set(int q) { Stage.Mode = modes[q]; Stage.Threshold = ths[q]; Stage.NonAsciiOnly = nas[q]; }
        var lines = new List<string> { "content\tlen\tutf8_bytes\tpath\tround\tcpu_ns_per_op\twall_ns_per_op\tops" };
        var summary = new List<string>();
        int bad = 0;
        foreach (var content in contents)
            foreach (var n in lengths)
            {
                var s = Make(content, n);
                var f = new UploadResultDataMessage { Upload = new UploadResultData { SessionId = s } };
                var g = new Armonik.Ffi.Shapes.V1.UploadResultDataMessage { Upload = new Armonik.Ffi.Shapes.V1.UploadResultData { SessionId = s } };
                var want = g.ToByteArray();
                int utf8 = Encoding.UTF8.GetByteCount(s);
                for (int k = 0; k < modes.Length; k++)
                {
                    Set(k);
                    if (core.EncodeInto(f, false) < 0) { Console.Error.WriteLine("encode failed " + names[k]); return 1; }
                    if (!core.ContextBytes().AsSpan().SequenceEqual(want)) { Console.Error.WriteLine("BYTE IDENTITY FAILED: " + content + " " + n + " " + names[k]); bad++; }
                }
                // per-op cost estimate for block sizing, then a warm-up of >= 1 s across the paths
                long Iters(int k)
                {
                    Set(k);
                    long it = 1; double t;
                    while (true)
                    {
                        long w0 = ProcCpu.Wall();
                        for (long i = 0; i < it; i++) core.EncodeInto(f, false);
                        t = (ProcCpu.Wall() - w0) / 1e6;
                        if (t > blockMs / 4 || it > 1 << 26) break;
                        it *= 2;
                    }
                    return Math.Max(1, (long)(it * blockMs / Math.Max(t, 1e-3)));
                }
                var iters = new long[modes.Length];
                long wstart = ProcCpu.Wall();
                while ((ProcCpu.Wall() - wstart) / 1e6 < 1000)
                    for (int k = 0; k < modes.Length; k++) iters[k] = Iters(k);
                var med = new double[modes.Length];
                var per = new List<double>[modes.Length];
                for (int k = 0; k < modes.Length; k++) per[k] = new List<double>();
                for (int r = 0; r < rounds; r++)
                    for (int j = 0; j < modes.Length; j++)
                    {
                        int k = (j + r) % modes.Length;
                        Set(k);
                        long c0 = ProcCpu.Ns(), w0 = ProcCpu.Wall();
                        for (long i = 0; i < iters[k]; i++) core.EncodeInto(f, false);
                        long c1 = ProcCpu.Ns(), w1 = ProcCpu.Wall();
                        double cpu = (double)(c1 - c0) / iters[k], wall = (double)(w1 - w0) / iters[k];
                        per[k].Add(cpu);
                        lines.Add(string.Join("\t", content, n, utf8, names[k], r + 1, cpu.ToString("F1", CultureInfo.InvariantCulture), wall.ToString("F1", CultureInfo.InvariantCulture), iters[k]));
                    }
                // the pin alone (E1's per-string cost beside the core's transcode)
                long pi = Math.Max(1000, iters[Math.Min(1, iters.Length - 1)]);
                for (int w = 0; w < 3; w++) { var h0 = GCHandle.Alloc(s, GCHandleType.Pinned); h0.Free(); }
                long pc0 = ProcCpu.Ns();
                for (long i = 0; i < pi; i++) { var h = GCHandle.Alloc(s, GCHandleType.Pinned); h.Free(); }
                double pin = (double)(ProcCpu.Ns() - pc0) / pi;
                for (int k = 0; k < modes.Length; k++) { var a = per[k].OrderBy(x => x).ToList(); med[k] = a[a.Count / 2]; }
                string Fm(List<double> v) { var a = v.OrderBy(x => x).ToList(); return string.Format(CultureInfo.InvariantCulture, "{0:F0} [{1:F0}-{2:F0}]", a[a.Count / 2], a[0], a[^1]); }
                summary.Add(string.Format(CultureInfo.InvariantCulture, "| {0} | {1} | {2} | {3} | {4:F1} | {5} |", content, n, utf8,
                    string.Join(" | ", Enumerable.Range(0, names.Length).Select(q => Fm(per[q]))), pin, names[Array.IndexOf(med, med.Min())]));
                Set(0); Stage.Mode = Stage.E0; Stage.Threshold = 0; Stage.NonAsciiOnly = false;
            }
        File.WriteAllLines(outp, lines);
        var md = new List<string>
        {
            "# D21 string-length sweep: process CPU per encode (ns), median [min-max] over " + rounds + " rounds, one process, paths interleaved and rotated per round",
            "",
            "CONTAINER INSTRUMENTATION. One core-ffi EncodeInto of UploadResultDataMessage{upload.session_id = the string}; E0 = .NET UTF-8 into native staging + ak_tc_bytes; E1 = the string pinned (GCHandle) + ak_tc_utf16 (simdutf); E2 = the C# transcoder callback writing into the core's buffer; E3 = that callback calling the core's ak_utf16_to_utf8 (worst-case grow), E3L = sized by ak_utf16_utf8_len first; E1R = the string pinned by `fixed` in the root frame + ak_tc_utf16; E1C = a GCHandle per chunk. Environment: " + Env() + ". `pin` = GCHandle.Alloc(Pinned) + Free alone, ns. Byte identity of every path checked first: " + (bad == 0 ? "all identical" : bad + " FAILURES") + ".",
            "",
            "| content | chars | UTF-8 bytes | " + string.Join(" | ", names) + " | pin | fastest |",
            "|---|---:|---:|" + string.Concat(names.Select(_ => "---:|")) + "---:|---|",
        };
        md.AddRange(summary);
        File.WriteAllLines(Path.ChangeExtension(outp, ".md"), md);
        foreach (var l in md) Console.WriteLine(l);
        return bad == 0 ? 0 : 1;
    }
}
