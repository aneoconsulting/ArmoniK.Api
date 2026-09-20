// The three marshallers, which is the only thing that differs between the arms.
//
// gRPC's `Marshaller<T>` is the seam: everything above it -- HTTP/2 framing,
// flow control, the ASP.NET pipeline, the client's call machinery -- is the same
// object graph in every arm, so a difference between them is the codec's.
//
//   gp        what Grpc.Tools emits into the stub: SetPayloadLength(
//             CalculateSize()) then WriteTo(bufferWriter), and
//             ParseFrom(PayloadAsReadOnlySequence) inward. R14's baseline.
//   managed   the generated pure-C# codec over the facade.
//   core-ffi  the C ABI, push decode.
//   core-pull the C ABI with ABI v1 7.1's pull family, which makes no upcall.
//
// The facade arms have to hand gRPC a facade object, and the incumbent has to
// hand it a Google.Protobuf message, so the three cannot share one T. They share
// the BYTES instead: every arm is a unary call returning the same P2.2 payload,
// and what is timed is the round trip.

using System;
using System.Buffers;
using Armonik.Ffi.Facade;
using Armonik.Ffi.Harness;
using Google.Protobuf;
using Grpc.Core;
using Gp = Armonik.Ffi.Shapes.V1;

namespace Armonik.Ffi.Rpc;

public static class Codecs
{
    /// R14's path, and the exact sequence `Grpc.Tools` emits.
    public static Marshaller<Gp.ListTasksDetailedResponse> Incumbent { get; } =
        Marshallers.Create<Gp.ListTasksDetailedResponse>(
            (m, ctx) =>
            {
                ctx.SetPayloadLength(m.CalculateSize());
                m.WriteTo(ctx.GetBufferWriter());
                ctx.Complete();
            },
            ctx => Gp.ListTasksDetailedResponse.Parser.ParseFrom(ctx.PayloadAsReadOnlySequence()));

    /// The generated pure-C# codec. One pass out, ABI v1 section 6's learned
    /// length width; `Codec.Read` inward.
    public static Marshaller<ListTasksDetailedResponse> Managed { get; } =
        Marshallers.Create<ListTasksDetailedResponse>(
            (m, ctx) =>
            {
                var e = Enc.New(Codec.Sites, 1 << 20);
                Codec.WriteListTasksDetailedResponse(ref e, m);
                ctx.SetPayloadLength(e.Pos);
                ctx.GetBufferWriter().Write(new ReadOnlySpan<byte>(e.Buf, 0, e.Pos));
                ctx.Complete();
            },
            ctx => Read(ctx.PayloadAsReadOnlySequence(), false));

    /// The C ABI, push decode.
    public static Marshaller<ListTasksDetailedResponse> Core { get; } =
        Marshallers.Create<ListTasksDetailedResponse>(WriteCore,
            ctx => Read(ctx.PayloadAsReadOnlySequence(), false, core: true));

    /// The C ABI, ABI v1 7.1's PULL family: no upcall at all.
    public static Marshaller<ListTasksDetailedResponse> CorePull { get; } =
        Marshallers.Create<ListTasksDetailedResponse>(WriteCore,
            ctx => Read(ctx.PayloadAsReadOnlySequence(), true, core: true));

    /// One binding per direction per process. gRPC serialises a given call's
    /// marshaller work on one thread, and the bench drives concurrency with
    /// several in flight, so these are guarded rather than assumed single
    /// threaded -- an encode context is explicitly NOT thread safe (ABI v1
    /// section 3) and sharing one unguarded would be a data race, not a slow
    /// path.
    [ThreadStatic] private static CoreFfi_ListTasksDetailedResponse _enc;
    [ThreadStatic] private static CoreFfi_ListTasksDetailedResponse _dec;

    private static unsafe void WriteCore(ListTasksDetailedResponse m, SerializationContext ctx)
    {
        var c = _enc ??= new CoreFfi_ListTasksDetailedResponse(
            CoreFfi_ListTasksDetailedResponse.CapsFor(m));
        c.Encode(m, out byte* p, out int len);
        ctx.SetPayloadLength(len);
        ctx.GetBufferWriter().Write(new ReadOnlySpan<byte>(p, len));
        ctx.Complete();
    }

    /// gRPC hands the parser a `ReadOnlySequence`, which may be segmented. The
    /// facade's `Dec` is over `byte[]`, so a segmented body is flattened first
    /// and the copy is charged to these arms -- the incumbent's
    /// `ParseFrom(ReadOnlySequence)` does not need it. That is a real cost of
    /// the facade's reader shape and it is named rather than hidden.
    [ThreadStatic] private static byte[] _flat;

    private static ListTasksDetailedResponse Read(ReadOnlySequence<byte> seq, bool pull, bool core = false)
    {
        byte[] buf;
        int len = checked((int)seq.Length);
        if (seq.IsSingleSegment && System.Runtime.InteropServices.MemoryMarshal.TryGetArray(
                seq.First, out var seg) && seg.Offset == 0 && seg.Array.Length == len)
        {
            buf = seg.Array;
        }
        else
        {
            // **A real cost of the facade's reader shape, and it is charged here
            // rather than hidden.** `Dec` is over `byte[]`, so a segmented body
            // has to be flattened, where the incumbent's
            // `ParseFrom(ReadOnlySequence)` reads the segments in place. Kestrel
            // delivers a 540 KB response in several, so this fires on every call.
            // The buffer is REUSED rather than allocated per call, because
            // allocating one would charge these arms 540 KB a call that no
            // serious implementation would pay -- but the COPY is still theirs
            // and is in the numbers.
            if (_flat == null || _flat.Length < len) _flat = new byte[Math.Max(len, 1 << 20)];
            seq.CopyTo(_flat);
            buf = _flat;
        }

        if (!core)
        {
            var d = new Dec { Buf = buf, Pos = 0, End = len, Err = 0 };
            var m = new ListTasksDetailedResponse();
            Codec.ReadListTasksDetailedResponse(ref d, m, len);
            if (d.Err != 0) throw new InvalidOperationException("managed decode err " + d.Err);
            return m;
        }
        var c = _dec ??= new CoreFfi_ListTasksDetailedResponse(
            new CoreFfi_ListTasksDetailedResponse.Caps());
        return pull ? c.Pull(buf, len) : c.Decode(buf, len);
    }
}
