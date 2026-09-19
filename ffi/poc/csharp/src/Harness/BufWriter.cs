// The buffer the incumbent's fastest encode path writes into.
//
// `Google.Protobuf` offers `msg.WriteTo(IBufferWriter<byte>)`, which sizes
// nothing at the top level, against `msg.WriteTo(Span<byte>)`, which needs the
// span sized and so needs a `CalculateSize()` first. That size pass was the
// handicap this slice's encode baseline was carrying (JOURNAL.md entry 14), so
// the buffer-writer path is measured as its own arm.
//
// Hand-written rather than `System.Buffers.ArrayBufferWriter<byte>`, for two
// reasons and both are load bearing:
//
//   * `ArrayBufferWriter<T>` arrived in .NET Core 3.0 and was never backported
//     to `System.Memory`, so it does not exist on the net48 floor. One writer
//     that compiles at every level keeps README 5.1's "identical wire bytes
//     across levels" checkable by running the same corpus on each, instead of
//     giving arm c a different incumbent from arms a and b;
//   * `ArrayBufferWriter<T>.Clear()` ZEROES the written span. Reusing one
//     across benchmark iterations with `Clear()` is exactly the per-iteration
//     buffer wipe that handicapped the C++ slice's incumbent. .NET 8 added
//     `ResetWrittenCount()` which does not, but a writer whose only reset does
//     the right thing cannot be got wrong by a later edit.
//
// It does nothing clever: one array, grown by doubling, never zeroed. The
// harness sizes it from the payload up front so no measured iteration grows.

using System;
using System.Buffers;

namespace Armonik.Ffi.Harness;

public sealed class BufWriter : IBufferWriter<byte>
{
    private byte[] _buf;
    private int _written;

    public BufWriter(int capacity) => _buf = new byte[Math.Max(capacity, 256)];

    public int WrittenCount => _written;

    public ReadOnlySpan<byte> WrittenSpan => new ReadOnlySpan<byte>(_buf, 0, _written);

    /// Reset the position. Deliberately does NOT clear: see the header.
    public void Reset() => _written = 0;

    public void Advance(int count)
    {
        if (count < 0 || _written + count > _buf.Length)
            throw new ArgumentOutOfRangeException(nameof(count));
        _written += count;
    }

    public Memory<byte> GetMemory(int sizeHint = 0)
    {
        Ensure(sizeHint);
        return new Memory<byte>(_buf, _written, _buf.Length - _written);
    }

    public Span<byte> GetSpan(int sizeHint = 0)
    {
        Ensure(sizeHint);
        return new Span<byte>(_buf, _written, _buf.Length - _written);
    }

    private void Ensure(int sizeHint)
    {
        if (sizeHint < 1) sizeHint = 1;
        if (_written + sizeHint <= _buf.Length) return;
        int want = Math.Max(_buf.Length * 2, _written + sizeHint);
        Array.Resize(ref _buf, want);
    }
}
