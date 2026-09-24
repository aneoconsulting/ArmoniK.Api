// The STREAMING arm. design/SHAPES.md lists streaming under "not in the RPC arm,
// and listed as not measured", with the reason: "Streaming is where the
// concurrency invariant actually bites, and no slice has touched it."
//
// Two questions, and they are different questions.
//
// **1. What does the codec cost per MESSAGE when the messages arrive in a
// stream?** A unary call pays for a request, a response, headers and trailers
// per message. A stream pays for them once and then carries N messages on one
// HTTP/2 stream, so everything the transport does per message is smaller and the
// codec's SHARE of the call should rise. That is a prediction and this arm is
// what checks it.
//
// **2. What does a managed host have to do to satisfy the ABI's concurrency
// contract, and what does that cost?** An encode context is explicitly not
// thread safe (ABI v1 section 3), the rust slice's positive control found that
// sharing one does not produce wrong bytes but ABORTS THE PROCESS -- a panic
// cannot unwind through `extern "C"` -- and a stream is where a host is most
// tempted to cache one context and reuse it. `Codecs` already answers this with
// `[ThreadStatic]`, which is correct by construction; what nobody has measured
// is what that costs when the thread pool is the thing that decides how many
// contexts exist.
//
// **Two payload families, deliberately.** P5.3 is ArmoniK's actual streaming
// shape: a 1 MB chunk of a result, which is what its 2 MiB chunking produces.
// P2.2 is the field-heavy shape the rest of this slice measures. They are
// expected to disagree completely and the disagreement is the finding.
//
// Nothing here changes a shape (design/SHAPES.md fixes those); it adds an arm
// over two existing payloads.

using System;
using System.Buffers;
using System.Collections.Generic;
using System.Threading.Tasks;
using Armonik.Ffi.Facade;
using Armonik.Ffi.Harness;
using Google.Protobuf;
using Grpc.AspNetCore.Server.Model;
using Grpc.Core;
using Gp = Armonik.Ffi.Shapes.V1;

namespace Armonik.Ffi.Rpc;

/// The four marshallers over M5, ArmoniK's chunk message. Same four codecs as
/// `Codecs`, same seam, a different root -- so the two payload families differ
/// in the message and in nothing else.
public static class ChunkCodecs
{
    public static Marshaller<Gp.UploadResultDataMessage> Incumbent { get; } =
        Marshallers.Create<Gp.UploadResultDataMessage>(
            (m, ctx) =>
            {
                ctx.SetPayloadLength(m.CalculateSize());
                m.WriteTo(ctx.GetBufferWriter());
                ctx.Complete();
            },
            ctx => Gp.UploadResultDataMessage.Parser.ParseFrom(ctx.PayloadAsReadOnlySequence()));

    public static Marshaller<UploadResultDataMessage> Managed { get; } =
        Marshallers.Create<UploadResultDataMessage>(
            (m, ctx) =>
            {
                // A FIXED generous cap, not `SizeOf` -- ABI v1 section 6's
                // learned length width is a ONE-pass encode, and `Codecs`'
                // P2.2 arm is the same shape at 1 MB. A size pass here would
                // make the two payload families different arms.
                var e = Enc.New(Codec.Sites, 1 << 21);
                Codec.WriteUploadResultDataMessage(ref e, m);
                ctx.SetPayloadLength(e.Pos);
                ctx.GetBufferWriter().Write(new ReadOnlySpan<byte>(e.Buf, 0, e.Pos));
                ctx.Complete();
            },
            ctx => Read(ctx.PayloadAsReadOnlySequence(), false));

    public static Marshaller<UploadResultDataMessage> Core { get; } =
        Marshallers.Create<UploadResultDataMessage>(WriteCore,
            ctx => Read(ctx.PayloadAsReadOnlySequence(), false, core: true));

    public static Marshaller<UploadResultDataMessage> CorePull { get; } =
        Marshallers.Create<UploadResultDataMessage>(WriteCore,
            ctx => Read(ctx.PayloadAsReadOnlySequence(), true, core: true));

    /// **This is the concurrency contract, and `[ThreadStatic]` is how a managed
    /// host satisfies it without a lock.** gRPC drives a given call's marshaller
    /// on whatever thread-pool thread is running the continuation, and with N
    /// streams in flight that is N threads at once on one process. A single
    /// shared context would be a data race into code that aborts rather than
    /// throws. What it costs is one context -- and one staging buffer sized from
    /// the payload -- per thread the pool ever uses, which is what `Contexts`
    /// and `StagingBytes` count.
    [ThreadStatic] private static CoreFfi_UploadResultDataMessage _enc;
    [ThreadStatic] private static CoreFfi_UploadResultDataMessage _dec;

    /// How many distinct contexts the thread pool caused to exist, and the
    /// native memory they hold. Nobody had counted these; the cost of the
    /// correct answer to the concurrency contract is exactly this number, and
    /// it is a function of the POOL's size and not of the call rate.
    public static long EncContexts, DecContexts, StagingBytes;

    private static unsafe void WriteCore(UploadResultDataMessage m, SerializationContext ctx)
    {
        var c = _enc;
        if (c == null)
        {
            System.Threading.Interlocked.Increment(ref EncContexts);
            // The staging is a block list now (cs_host.Stage): one 64 KiB block to
            // start, more only while an encode needs them.
            System.Threading.Interlocked.Add(ref StagingBytes, 1 << 16);
            c = _enc = new CoreFfi_UploadResultDataMessage();
        }
        c.Encode(m, out byte* p, out int len);
        ctx.SetPayloadLength(len);
        ctx.GetBufferWriter().Write(new ReadOnlySpan<byte>(p, len));
        ctx.Complete();
    }

    [ThreadStatic] private static byte[] _flat;
    public static long Segmented, Single, Copied;

    private static UploadResultDataMessage Read(ReadOnlySequence<byte> seq, bool pull, bool core = false)
    {
        byte[] buf;
        int len = checked((int)seq.Length);
        if (seq.IsSingleSegment && System.Runtime.InteropServices.MemoryMarshal.TryGetArray(
                seq.First, out var seg) && seg.Offset == 0 && seg.Array.Length == len)
        {
            Single++;
            buf = seg.Array;
        }
        else
        {
            Segmented++;
            Copied += len;
            if (_flat == null || _flat.Length < len) _flat = new byte[Math.Max(len, 1 << 21)];
            seq.CopyTo(_flat);
            buf = _flat;
        }

        if (!core)
        {
            var d = new Dec { Buf = buf, Pos = 0, End = len, Err = 0 };
            var m = new UploadResultDataMessage();
            Codec.ReadUploadResultDataMessage(ref d, m, 0);
            if (d.Err != 0) throw new InvalidOperationException("managed decode err " + d.Err);
            return m;
        }
        var c = _dec;
        if (c == null)
        {
            System.Threading.Interlocked.Increment(ref DecContexts);
            c = _dec = new CoreFfi_UploadResultDataMessage();
        }
        return pull ? c.Pull(buf, len) : c.Decode(buf, len);
    }
}

/// The transport floor's marshaller, and it is NOT `Bench.Raw`.
///
/// `Bench.Raw` is `Marshallers.Create<byte[]>(b => b, b => b)`, the SIMPLE form,
/// and gRPC then copies the returned array into its send buffer. Every codec arm
/// here uses the CONTEXTUAL form and writes straight into `GetBufferWriter()`, so
/// a floor built on the simple form carries one whole payload copy the arms it is
/// supposed to floor do not -- which is how it came out ABOVE the incumbent on
/// the first run, and a control that costs more than the thing it floors is not a
/// control. This is the same passthrough in the contextual form.
public static class RawCtx
{
    public static readonly Marshaller<byte[]> M = Marshallers.Create<byte[]>(
        (b, ctx) =>
        {
            ctx.SetPayloadLength(b.Length);
            ctx.GetBufferWriter().Write(b);
            ctx.Complete();
        },
        ctx => ctx.PayloadAsReadOnlySequence().ToArray());
}

/// The streamed service. The SERVER's marshaller is a `byte[]` passthrough in
/// every arm, exactly as in the unary arm, so the only codec in the process is
/// the client's and the two arms are comparable.
public sealed class Streamer
{
    public const string Name = "armonik.ffi.Streamer";

    /// The wire bytes the server streams down, one entry per payload family.
    public static byte[] Wire22, Wire53;

    public static readonly Method<byte[], byte[]> Down22 = new Method<byte[], byte[]>(
        MethodType.ServerStreaming, Name, "Down22", Bench.Raw, Bench.Raw);
    public static readonly Method<byte[], byte[]> Down53 = new Method<byte[], byte[]>(
        MethodType.ServerStreaming, Name, "Down53", Bench.Raw, Bench.Raw);
    public static readonly Method<byte[], byte[]> Up22 = new Method<byte[], byte[]>(
        MethodType.ClientStreaming, Name, "Up22", Bench.Raw, Bench.Raw);
    public static readonly Method<byte[], byte[]> Up53 = new Method<byte[], byte[]>(
        MethodType.ClientStreaming, Name, "Up53", Bench.Raw, Bench.Raw);

    /// The request carries the message count as four little-endian bytes. The
    /// server knows nothing else about the payload -- it replays one buffer.
    private static async Task Down(byte[] req, IServerStreamWriter<byte[]> w, byte[] wire)
    {
        int n = req.Length >= 4 ? BitConverter.ToInt32(req, 0) : 1;
        for (int i = 0; i < n; i++) await w.WriteAsync(wire);
    }

    public Task Down22H(byte[] r, IServerStreamWriter<byte[]> w, ServerCallContext c) => Down(r, w, Wire22);
    public Task Down53H(byte[] r, IServerStreamWriter<byte[]> w, ServerCallContext c) => Down(r, w, Wire53);

    /// The server drains, counts and CHECKS. It does no codec work -- its
    /// marshaller is the same `byte[]` passthrough -- so what the upstream column
    /// measures is the client's encode plus the transport. The check is the
    /// encode direction's byte-identity gate and it runs end to end: every
    /// message the client's codec produced must equal, byte for byte, the wire
    /// the harness built the payload from. It returns `(received, mismatched)`.
    private static async Task<byte[]> Up(IAsyncStreamReader<byte[]> r, byte[] wire)
    {
        long n = 0, bad = 0;
        while (await r.MoveNext(default))
        {
            n++;
            var g = r.Current;
            if (g.Length != wire.Length) { bad++; continue; }
            for (int i = 0; i < g.Length; i++)
                if (g[i] != wire[i]) { bad++; break; }
        }
        var o = new byte[16];
        BitConverter.GetBytes(n).CopyTo(o, 0);
        BitConverter.GetBytes(bad).CopyTo(o, 8);
        return o;
    }

    public Task<byte[]> Up22H(IAsyncStreamReader<byte[]> r, ServerCallContext c) => Up(r, Wire22);
    public Task<byte[]> Up53H(IAsyncStreamReader<byte[]> r, ServerCallContext c) => Up(r, Wire53);
}

public sealed class StreamerProvider : IServiceMethodProvider<Streamer>
{
    public void OnServiceMethodDiscovery(ServiceMethodProviderContext<Streamer> ctx)
    {
        ctx.AddServerStreamingMethod<byte[], byte[]>(Streamer.Down22, Array.Empty<object>(),
            (s, r, w, c) => s.Down22H(r, w, c));
        ctx.AddServerStreamingMethod<byte[], byte[]>(Streamer.Down53, Array.Empty<object>(),
            (s, r, w, c) => s.Down53H(r, w, c));
        ctx.AddClientStreamingMethod<byte[], byte[]>(Streamer.Up22, Array.Empty<object>(),
            (s, r, c) => s.Up22H(r, c));
        ctx.AddClientStreamingMethod<byte[], byte[]>(Streamer.Up53, Array.Empty<object>(),
            (s, r, c) => s.Up53H(r, c));
    }
}
