// D26 (s15, owner 2026-10-10): no thread-local storage in the C# slice. The RPC harness's
// per-thread caches (the core-ffi bindings, the managed decode buffer, host-gen's Enc) are the
// fields of ONE OBJECT PER CALLER: each thread of the CallerPool and each async loop of
// RunCell has its own, created with the pool and passed to every call of a cell explicitly
// (Cell.One / OneAsync / Check / CheckAsync take it). Each core-ffi binding inside holds its
// own host contexts (encode and decode, D26 as amended), so a caller's calls reuse them.
//
// Grpc.Net's marshallers (cells D and F) get no caller state: Grpc.Net runs a serializer or a
// deserializer on whichever thread the call is on, with only the message and its context. They
// rent a Caller per call from a shared queue and give it back (ConcurrentQueue: no
// thread-keyed store; ConcurrentBag, used before, keeps per-thread lists in thread-local
// storage).
using System;
using System.Collections.Concurrent;
using Armonik.Ffi.Facade;
using Armonik.Ffi.Harness;

namespace Armonik.Ffi.Rpc;

internal sealed class Caller : IDisposable
{
    private CoreFfi_ListTasksDetailedResponse _core;
    private CoreFfi_UploadResultDataMessage _ucore;
    private byte[] _buf;
    /// host-gen's encoder (cells E and F's encode), reused across this caller's calls.
    public Enc He;

    /// The P2.2 grid's core-ffi binding of this caller (created on its first use).
    public CoreFfi_ListTasksDetailedResponse Core => _core ??= new CoreFfi_ListTasksDetailedResponse();
    /// The upload directions' core-ffi binding of this caller (created on its first use).
    public CoreFfi_UploadResultDataMessage UCore => _ucore ??= new CoreFfi_UploadResultDataMessage();

    /// A managed buffer of at least n bytes, reused (the managed decoders take a managed buffer).
    public byte[] Buf(int n) => (_buf == null || _buf.Length < n) ? (_buf = new byte[Math.Max(n, 1 << 20)]) : _buf;

    /// host-gen's encoder, created on first use, reset for the call.
    public ref Enc HostEnc()
    {
        if (He.Buf == null) He = Enc.New(Codec.Sites, 1 << 16);
        He.Reset();
        return ref He;
    }

    public void Dispose()
    {
        _core?.Dispose(); _core = null;
        _ucore?.Dispose(); _ucore = null;
    }

    private static readonly ConcurrentQueue<Caller> _free = new ConcurrentQueue<Caller>();
    /// Grpc.Net's marshallers: a caller object for one call, from the shared queue.
    public static Caller Rent() => _free.TryDequeue(out var c) ? c : new Caller();
    public static void Return(Caller c) => _free.Enqueue(c);
}
