// CAMPAIGN req 14 as amended (owner, 2026-09-27; FIX-PLAN D5 and WP8): the upload directions,
// required in every slice.
//
//   c  a unary upload: the request is P5.3 or P5.4 (M5 UploadResultDataMessage, 1 MB and 4 MB
//      of data; P5.4 is 4,194,390 B on the wire), the server decodes it with the incumbent,
//      refuses an empty upload, and answers empty;
//   d  the streamed upload, ArmoniK's UploadResultData(stream ...) shape: M5 messages of 2 MiB
//      chunks, the ids on the first message only, 4 MiB (2 messages) and 16 MiB (8) in total;
//      the server decodes every message with the incumbent and answers the data byte count
//      (8 bytes LE). Its twin method UploadStreamCheck also answers the SHA-256 of every
//      message's bytes as received (the campaign server, poc/rust/SERVER.md): the pre-timing check (req 18/26) runs every d cell once through it and
//      compares count and digest with the client's.
//
// Paths, per cell (the same codecs as b): A Grpc.Net + Grpc.Tools' serializer (SerInc);
// B the incumbent into a buffer + the core's transport; C core-ffi on the MOVE path (the encode
// left in the core's context, moved as the request by ak_call_unary_enc / ak_call_send_enc; Cc,
// a labelled extra, the copy path: ak_enc_take then ak_call_unary / ak_call_send); D Grpc.Net + core-ffi (SerFfi); E host-gen (Enc) +
// the core's transport; F Grpc.Net + host-gen (SerHost). The core's transport: c through
// ak_call_unary, d through ABI v1 section 9's client streaming (ak_call_open with
// AK_CALL_CLIENT_STREAM, ak_call_send per message with `last` on the final one, ak_call_recv,
// ak_call_destroy), both blocking (req 16). Grpc.Net: c through AsyncUnaryCall, d through
// AsyncClientStreamingCall (WriteAsync per message, CompleteAsync, the response awaited), its
// idiomatic async form. The framed twins Bf, Cf, Ef are the same cells on a client with
// ak_client_set_framed(1) (ABI v1 section 9's second send path, beside its reference).
// Every message is built before the run (graph construction is not timed); every call's
// status and response are checked (req 18).

using System;
using System.Buffers.Binary;
using System.Security.Cryptography;
using System.Text;
using System.Threading.Tasks;
using Armonik.Ffi.Facade;
using Armonik.Ffi.Harness;
using Armonik.Ffi.Rpc;
using Google.Protobuf;
using Grpc.Core;
using Gp = Armonik.Ffi.Shapes.V1;

namespace Armonik.Ffi.Campaign;

/// One upload's messages, in both object models, and what the server must answer.
internal sealed class UpData
{
    public string Payload;           // "P5.3", "P5.4", "stream-4MiB", "stream-16MiB"
    public bool Stream;
    public Gp.UploadResultDataMessage[] G;
    public UploadResultDataMessage[] F;
    public long DataBytes;
    public byte[] Sha;               // SHA-256 of the data bytes, in order
}

internal static class Uploads
{
    public const int Chunk = 2 << 20;

    /// splitmix64 bytes: deterministic, incompressible data for the stream's chunks.
    private static byte[] Data(long n, ulong seed)
    {
        var b = new byte[n];
        ulong x = seed;
        for (long i = 0; i < n; i += 8)
        {
            x += 0x9E3779B97F4A7C15UL;
            ulong z = x;
            z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9UL;
            z = (z ^ (z >> 27)) * 0x94D049BB133111EBUL;
            z ^= z >> 31;
            for (int k = 0; k < 8 && i + k < n; k++) b[i + k] = (byte)(z >> (8 * k));
        }
        return b;
    }

    public static UpData Unary(string pid)
    {
        var g = pid == "P5.3" ? BuildGp.P5_3() : BuildGp.P5_4();
        var f = pid == "P5.3" ? BuildFacade.P5_3() : BuildFacade.P5_4();
        return new UpData { Payload = pid, Stream = false, G = new[] { g }, F = new[] { f }, DataBytes = f.Upload.DataChunk.Length };
    }

    public static UpData Streamed(int mib)
    {
        long total = (long)mib << 20;
        var data = Data(total, 0xA5A5_0000UL + (ulong)mib);
        int n = (int)(total / Chunk);
        var d = new UpData { Payload = "stream-" + mib + "MiB", Stream = true, G = new Gp.UploadResultDataMessage[n], F = new UploadResultDataMessage[n], DataBytes = total };
        for (int i = 0; i < n; i++)
        {
            var chunk = new byte[Chunk];
            Buffer.BlockCopy(data, i * Chunk, chunk, 0, Chunk);
            // The ids on the first message only; empty strings are not on the wire.
            string sid = i == 0 ? "00000000-0000-4000-8000-00000000d001" : "", rid = i == 0 ? "00000000-0000-4000-8000-00000000d002" : "";
            d.G[i] = new Gp.UploadResultDataMessage { Upload = new Gp.UploadResultData { SessionId = sid, ResultId = rid, DataChunk = ByteString.CopyFrom(chunk) } };
            d.F[i] = new UploadResultDataMessage { Upload = new UploadResultData { SessionId = sid, ResultId = rid, DataChunk = chunk } };
        }
        // WP10: the campaign server's UploadStreamCheck answers the SHA-256 of every MESSAGE's
        // bytes as received, in order (poc/rust/SERVER.md); every codec's wire is the same
        // bytes (CheckWire), so the incumbent's is the expectation.
        using (var h = IncrementalHash.CreateHash(HashAlgorithmName.SHA256))
        {
            foreach (var m in d.G) h.AppendData(m.ToByteArray());
            d.Sha = h.GetHashAndReset();
        }
        return d;
    }

    /// Before any call: every message's wire is the same from the incumbent, host-gen and
    /// core-ffi (drop, and retain where the build has it). Throws otherwise.
    public static void CheckWire(UpData u)
    {
        var core = new CoreFfi_UploadResultDataMessage();
        for (int i = 0; i < u.G.Length; i++)
        {
            var w = u.G[i].ToByteArray();
            var e = Enc.New(Armonik.Ffi.Facade.Codec.Sites, w.Length + 4096);
            Armonik.Ffi.Facade.Codec.WriteUploadResultDataMessage(ref e, u.F[i]);
            if (!e.ToArray().AsSpan().SequenceEqual(w)) throw new InvalidOperationException(u.Payload + " message " + i + ": host-gen bytes differ from the incumbent's");
            if (!core.EncodeToArray(u.F[i], false).AsSpan().SequenceEqual(w)) throw new InvalidOperationException(u.Payload + " message " + i + ": core-ffi bytes differ from the incumbent's");
#if !AK_NO_UNKNOWN_FIELDS
            if (!core.EncodeToArray(u.F[i], true).AsSpan().SequenceEqual(w)) throw new InvalidOperationException(u.Payload + " message " + i + ": core-ffi retain bytes differ from the incumbent's");
#endif
        }
        core.Dispose();
    }

    // D26 (s15): the caches that were thread-static are the caller's object's (Caller.cs: UCore,
    // Buf, HostEnc), passed to every call; Grpc.Net's marshallers rent one per call.

    /// Encode message i with the cell's codec (0 incumbent, 1 core-ffi, 2 host-gen) and hand
    /// the bytes to `send`, which runs while they are pinned.
    private unsafe delegate int Sender(byte* body, int len);
    private static unsafe int EncodeAndSend(Caller cl, int codec, bool retain, UpData u, int i, Sender send, string cell)
    {
        if (codec == 3)   // Cc, the copy path
        {
            int rc = cl.UCore.TryEncode(u.F[i], retain, out byte* p, out int n);
            if (rc < 0) throw new CampaignAbort(cell + ": core encode " + rc);
            return send(p, n);
        }
        byte[] arr;
        int len;
        if (codec == 0)
        {
            len = u.G[i].CalculateSize();
            arr = cl.Buf(len);
            u.G[i].WriteTo(new Span<byte>(arr, 0, len));
        }
        else
        {
            ref var he = ref cl.HostEnc();
            if (retain) HostR.WriteUploadResultDataMessage(ref he, u.F[i]); else Armonik.Ffi.Facade.Codec.WriteUploadResultDataMessage(ref he, u.F[i]);
            if (he.Err != 0) throw new CampaignAbort(cell + ": managed encode " + he.Err);
            len = he.Pos;
            arr = he.Buf;
        }
        fixed (byte* q = arr) return send(q, len);
    }

    /// D7: direction c's encode and start for a delivery cell (as CoreUnary encodes).
    public static unsafe IntPtr StartUpload(Caller cl, DeliveryChannel dc, byte[] path, int codec, bool retain, UpData u, IntPtr user, ulong tag, string cell)
    {
        if (codec == 1)
        {
            int er = cl.UCore.EncodeInto(u.F[0], retain);
            if (er < 0) throw new CampaignAbort(cell + ": core encode " + er);
            return dc.StartUnaryEnc(path, cl.UCore.EncContext, user, tag);
        }
        IntPtr call = IntPtr.Zero;
        EncodeAndSend(cl, codec, retain, u, 0, (body, len) => { call = dc.StartUnary(path, body, len, user, tag); return 0; }, cell);
        return call;
    }

    /// D7: direction d's message i, encoded and its send started (as CoreStream encodes).
    public static unsafe int StartSend(Caller cl, DeliveryChannel dc, IntPtr h, int codec, bool retain, UpData u, int i, IntPtr user, ulong tag, string cell)
    {
        int last = i == u.G.Length - 1 ? 1 : 0;
        if (codec == 1)
        {
            int er = cl.UCore.EncodeInto(u.F[i], retain);
            if (er < 0) throw new CampaignAbort(cell + ": core encode " + er);
            return dc.StartSendEnc(h, cl.UCore.EncContext, last, user, tag);
        }
        return EncodeAndSend(cl, codec, retain, u, i, (body, len) => dc.StartSend(h, body, len, last, user, tag), cell);
    }

    /// Direction c through the core's transport: one blocking ak_call_unary.
    public static unsafe void CoreUnary(Caller cl, CoreChannel ch, byte[] path, int codec, bool retain, UpData u, int wantLen, string cell)
    {
        if (codec == 1)
        {
            // C, the MOVE path (WP8 parity): encode into the core's context, ak_call_unary_enc moves it.
            int er = cl.UCore.EncodeInto(u.F[0], retain);
            if (er < 0) throw new CampaignAbort(cell + ": core encode " + er);
            ak_bytes r = default;
            int rc, gs = -1;
            fixed (byte* p = path) rc = AkRpc.ak_call_unary_enc(ch.Client, p, (nuint)path.Length, cl.UCore.EncContext, &r, &gs);
            try
            {
                if (rc != AkRpc.AK_OK) throw new CampaignAbort(cell + " c: status " + rc + " grpc " + gs);
                if ((int)r.len != wantLen) throw new CampaignAbort(cell + " c: response length " + r.len + ", expected " + wantLen);
            }
            finally { AkRpc.ak_bytes_free(&r); }
            return;
        }
        EncodeAndSend(cl, codec, retain, u, 0, (body, len) =>
        {
            ak_bytes r = default;
            int rc, gs = -1;
            fixed (byte* p = path) rc = AkRpc.ak_call_unary(ch.Client, p, (nuint)path.Length, body, (nuint)len, &r, &gs);
            try
            {
                if (rc != AkRpc.AK_OK) throw new CampaignAbort(cell + " c: status " + rc + " grpc " + gs);
                if ((int)r.len != wantLen) throw new CampaignAbort(cell + " c: response length " + r.len + ", expected " + wantLen);
            }
            finally { AkRpc.ak_bytes_free(&r); }
            return 0;
        }, cell);
    }

    /// Direction d through the core's client streaming; returns the response bytes (checked
    /// by the caller: the count, and in the pre-timing check the digest).
    public static unsafe byte[] CoreStream(Caller cl, CoreChannel ch, byte[] path, int codec, bool retain, UpData u, string cell)
    {
        IntPtr h;
        fixed (byte* p = path) h = AkRpc.ak_call_open(ch.Client, p, (nuint)path.Length, AkRpc.AK_CALL_CLIENT_STREAM, null);
        if (h == IntPtr.Zero) throw new CampaignAbort(cell + " d: ak_call_open returned NULL");
        try
        {
            for (int i = 0; i < u.G.Length; i++)
            {
                int last = i == u.G.Length - 1 ? 1 : 0;
                int rc;
                if (codec == 1)
                {
                    // C, the MOVE path: each message encoded into the context, ak_call_send_enc moves it.
                    int er = cl.UCore.EncodeInto(u.F[i], retain);
                    if (er < 0) throw new CampaignAbort(cell + ": core encode " + er);
                    rc = AkRpc.ak_call_send_enc(h, cl.UCore.EncContext, last);
                }
                else rc = EncodeAndSend(cl, codec, retain, u, i, (body, len) => AkRpc.ak_call_send(h, body, (nuint)len, last), cell);
                if (rc != AkRpc.AK_OK) throw new CampaignAbort(cell + " d: ak_call_send " + rc + " (message " + i + ")");
            }
            ak_bytes r = default;
            int gs = -1;
            int rr = AkRpc.ak_call_recv(h, &r, &gs);
            try
            {
                if (rr != AkRpc.AK_OK) throw new CampaignAbort(cell + " d: ak_call_recv " + rr + " grpc " + gs);
                return new ReadOnlySpan<byte>((void*)r.ptr, (int)r.len).ToArray();
            }
            finally { AkRpc.ak_bytes_free(&r); }
        }
        finally { AkRpc.ak_call_destroy(h); }
    }

    /// The server warm-up's stream: raw message bytes through the core's client streaming.
    public static unsafe byte[] CoreStreamRaw(CoreChannel ch, byte[] path, byte[][] msgs)
    {
        IntPtr h;
        fixed (byte* p = path) h = AkRpc.ak_call_open(ch.Client, p, (nuint)path.Length, AkRpc.AK_CALL_CLIENT_STREAM, null);
        if (h == IntPtr.Zero) throw new CampaignAbort("warm d: ak_call_open returned NULL");
        try
        {
            for (int i = 0; i < msgs.Length; i++)
            {
                int rc;
                fixed (byte* q = msgs[i]) rc = AkRpc.ak_call_send(h, q, (nuint)msgs[i].Length, i == msgs.Length - 1 ? 1 : 0);
                if (rc != AkRpc.AK_OK) throw new CampaignAbort("warm d: ak_call_send " + rc);
            }
            ak_bytes r = default;
            int gs = -1;
            int rr = AkRpc.ak_call_recv(h, &r, &gs);
            try
            {
                if (rr != AkRpc.AK_OK) throw new CampaignAbort("warm d: ak_call_recv " + rr + " grpc " + gs);
                return new ReadOnlySpan<byte>((void*)r.ptr, (int)r.len).ToArray();
            }
            finally { AkRpc.ak_bytes_free(&r); }
        }
        finally { AkRpc.ak_call_destroy(h); }
    }

    // ---- Grpc.Net (A, D, F): the marshallers, and the calls

    public static Marshaller<Gp.UploadResultDataMessage> MInc => Ops_UploadResultDataMessage.MInc;
    public static Marshaller<UploadResultDataMessage> MHost(bool retain) => Ops_UploadResultDataMessage.MHost(retain);
    /// core-ffi's serializer, on whichever thread Grpc.Net runs it: a caller object rented per call.
    public static Marshaller<UploadResultDataMessage> MFfi(bool retain) => Marshallers.Create<UploadResultDataMessage>((m, c) =>
    {
        var cl = Caller.Rent();
        try { Ops_UploadResultDataMessage.SerFfi(cl.UCore, m, retain, c); }
        finally { Caller.Return(cl); }
    }, c => throw new NotSupportedException());

    public static async Task GrpcUnary<T>(CallInvoker inv, Method<T, byte[]> m, T req, int wantLen, string cell) where T : class
    {
        var r = await inv.AsyncUnaryCall(m, null, new CallOptions(), req);
        if (r.Length != wantLen) throw new CampaignAbort(cell + " c: response length " + r.Length + ", expected " + wantLen);
    }

    public static async Task<byte[]> GrpcStream<T>(CallInvoker inv, Method<T, byte[]> m, T[] msgs) where T : class
    {
        using var call = inv.AsyncClientStreamingCall(m, null, new CallOptions());
        foreach (var x in msgs) await call.RequestStream.WriteAsync(x);
        await call.RequestStream.CompleteAsync();
        return await call.ResponseAsync;
    }

    /// The d response: the data byte count, 8 bytes LE (+ the SHA-256 on StreamCheck).
    public static void CheckStreamResponse(byte[] r, UpData u, bool withDigest, long plantCount, byte[] plantSha, string cell)
    {
        long want = u.DataBytes + plantCount;
        if (r.Length != (withDigest ? 40 : 8)) throw new CampaignAbort(cell + " d: response " + r.Length + " bytes, expected " + (withDigest ? 40 : 8));
        long got = BinaryPrimitives.ReadInt64LittleEndian(r);
        if (got != want) throw new CampaignAbort(cell + " d: server counted " + got + " data bytes, expected " + want);
        if (withDigest)
        {
            var sha = plantSha ?? u.Sha;
            if (!r.AsSpan(8, 32).SequenceEqual(sha)) throw new CampaignAbort(cell + " d: the server's SHA-256 of the data differs from the client's");
        }
    }
}

/// Requirement 18's abort, shared by the grid and the uploads.
internal sealed class CampaignAbort : Exception { public CampaignAbort(string m) : base(m) { } }
