// D21 step 7: byte identity of the string path chosen for this process (AK_STR_ENC) with
// several threads encoding at once, each with its own codec objects, as the RPC callers do.
//
//   BenchDotNet --verify-mt [--threads N] [--rounds R]
//
// Every payload and content set, encode-core form, retain (full build) and drop: each thread
// encodes R times and compares with the incumbent's bytes. Exit 0 only if every encode matched.
// (Written after a k = 8 RPC caller under E1R failed its per-encode mark check: the counters
// were process-wide; Cases.Verify and the corpus are single-threaded and could not see it.)

using System;
using System.Linq;
using System.Threading;
using Armonik.Ffi.Campaign;
using Armonik.Ffi.Facade;

namespace Armonik.Ffi.Bdn;

public static class VerifyMt
{
    public static int Run(int threads, int rounds)
    {
        int bad = 0; long n = 0;
        foreach (var pid in OpsTable.Payloads)
            foreach (var cs in Cases.SetsOf(pid))
            {
                Values.ContentSet = cs;
                var ops = Enumerable.Range(0, threads).Select(_ => OpsTable.ForPayload(pid)).ToArray();   // built before the threads (ContentSet is static)
                Values.ContentSet = Values.Ascii;
                var want = ops[0].IncumbentBytes();
                var go = new ManualResetEventSlim(false);
                var th = Enumerable.Range(0, threads).Select(t => new Thread(() =>
                {
                    go.Wait();
                    for (int r = 0; r < rounds; r++)
                    {
                        foreach (var retain in new[] { false, true })
                        {
#if AK_NO_UNKNOWN_FIELDS
                            if (retain) continue;
#endif
                            byte[] got;
                            try { got = ops[t].EncFfiCoreBytes(retain); }
                            catch (Exception e) { Console.Error.WriteLine(pid + "/" + cs + " thread " + t + ": " + e.Message); Interlocked.Increment(ref bad); return; }
                            if (!got.AsSpan().SequenceEqual(want)) { Console.Error.WriteLine(pid + "/" + cs + " thread " + t + ": bytes differ"); Interlocked.Increment(ref bad); return; }
                            Interlocked.Increment(ref n);
                        }
                    }
                }, 16 << 20)).ToArray();
                foreach (var x in th) x.Start();
                go.Set();
                foreach (var x in th) x.Join();
            }
        Console.WriteLine("verify-mt (" + Armonik.Ffi.Harness.Stage.ModeName + ", " + threads + " threads): " + n + " concurrent encodes compared, " + bad + " failure(s)");
        return bad == 0 ? 0 : 1;
    }
}
