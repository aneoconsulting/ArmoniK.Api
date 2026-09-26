// CAMPAIGN req 11, end state (ii): the form an arm's gRPC path hands to its transport.
//
// On the Grpc.Net path (cells A, D and F) a marshaller's serializer writes into the call's
// SerializationContext. Grpc.Net.Client 2.71's GrpcCallSerializationContext (internal), for a message
// whose length the serializer set with SetPayloadLength and no compression, rents ONE array
// of 5 + length bytes from ArrayPool<byte>.Shared, writes the 5-byte gRPC frame header
// (compressed flag 0, big-endian length) and hands itself out as the IBufferWriter over the
// rest; after the body has been written to the HTTP/2 stream the array goes back to the pool.
// (Checked by reflection on the 2.71.0 assembly, JOURNAL 58: ResolveBufferWriter rents from
// ArrayPool<byte>.Shared when direct serialization applies, WriteHeader writes the 5 bytes,
// Reset returns the array.) Its type is internal, so this class does the same in the open:
// the codec suite's `encode-transport` rows time the serializer the RPC grid's marshaller runs
// (RootOps.Ser*) into this context, up to the framed message ready to write (Release returns
// the array, as the call does after the write). The copy to the socket is transport, not
// codec, and is not in these rows.
//
// Every serializer of this slice sets the payload length first, so the context's other
// branch (length unknown: an ArrayBufferWriter, then a copy) is refused here, loudly, rather
// than silently measured as something else.

using System;
using System.Buffers;
using System.Buffers.Binary;
using Grpc.Core;

namespace Armonik.Ffi.Campaign;

public sealed class GrpcFrame : SerializationContext, IBufferWriter<byte>
{
    public const int HeaderSize = 5;
    private byte[] _buf;
    private int _pos, _len = -1;
    private bool _done;

    /// Bytes of the framed message (header included) after Complete.
    public int WrittenCount => _done ? _pos : throw new InvalidOperationException("the serializer did not complete the frame");
    public ReadOnlySpan<byte> Written => _done ? new ReadOnlySpan<byte>(_buf, 0, _pos) : throw new InvalidOperationException("not complete");

    public override void SetPayloadLength(int payloadLength) => _len = payloadLength;

    public override IBufferWriter<byte> GetBufferWriter()
    {
        if (_len < 0) throw new InvalidOperationException("GrpcFrame: the serializer did not set the payload length (the buffered branch is not modelled)");
        if (_buf != null) throw new InvalidOperationException("GrpcFrame: GetBufferWriter twice");
        _buf = ArrayPool<byte>.Shared.Rent(HeaderSize + _len);
        _buf[0] = 0;
        BinaryPrimitives.WriteUInt32BigEndian(_buf.AsSpan(1, 4), (uint)_len);
        _pos = HeaderSize;
        return this;
    }

    public override void Complete()
    {
        if (_buf == null || _pos != HeaderSize + _len) throw new InvalidOperationException("GrpcFrame: body " + (_pos - HeaderSize) + " bytes, payload length " + _len);
        _done = true;
    }

    public override void Complete(byte[] payload)
    {
        _len = payload.Length;
        GetBufferWriter();
        payload.CopyTo(_buf, HeaderSize);
        _pos = HeaderSize + _len;
        _done = true;
    }

    /// The pre-timing check only (Cases.Verify): Release keeps a copy of the frame.
    public bool Capture;
    public byte[] LastCopy;

    /// What the call does once the frame has been written: the array back to the pool.
    public void Release()
    {
        if (Capture && _done) LastCopy = new ReadOnlySpan<byte>(_buf, 0, _pos).ToArray();
        if (_buf != null) ArrayPool<byte>.Shared.Return(_buf);
        _buf = null;
        _pos = 0;
        _len = -1;
        _done = false;
    }

    public void Advance(int count)
    {
        if (count < 0 || _pos + count > HeaderSize + _len) throw new ArgumentOutOfRangeException(nameof(count));
        _pos += count;
    }

    public Memory<byte> GetMemory(int sizeHint = 0)
    {
        if (_pos + Math.Max(sizeHint, 0) > _buf.Length) throw new InvalidOperationException("GrpcFrame: a write past the payload length");
        return new Memory<byte>(_buf, _pos, _buf.Length - _pos);
    }

    public Span<byte> GetSpan(int sizeHint = 0)
    {
        if (_pos + Math.Max(sizeHint, 0) > _buf.Length) throw new InvalidOperationException("GrpcFrame: a write past the payload length");
        return new Span<byte>(_buf, _pos, _buf.Length - _pos);
    }
}
