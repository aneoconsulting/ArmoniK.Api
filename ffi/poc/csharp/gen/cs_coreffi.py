"""Backend: the `core-ffi` arm's host binding. M1 encode.

The host side of ABI v1 section 6's by-value group, over the same
`libak_core.so` the Rust slice built.

**Strings are staged, not transcoded through a callback, and that is a .NET
decision with a number behind it.** `ak_str` offers both: point `data` at the
host's own representation and supply a `tc` callback, or transcode up front and
point `data` at UTF-8 with `tc = ak_tc_bytes()`. The first is one REVERSE
CROSSING PER STRING -- 5 per `ResultRaw`, so 5,000 for P1.2 -- and .NET crosses
at 7.5 to 12 ns, which would be 37 to 60 us against a whole managed encode of
about 210 us. The second costs a staging buffer and a second copy and crosses
nothing, because `ak_tc_bytes` is a function pointer INTO THE CORE. The second
is emitted. The first is left as an arm to measure, because "make the crossings
fewer, not cheaper" predicts the answer and a prediction is not a measurement.

**One reverse call per message, not per field.** `ResultRaw` is a leaf, so its
encode vtable is empty and an element costs no call at all; the only callback
is the loop over `results`. That matters for the abort guard: a managed
exception inside `[UnmanagedCallersOnly]` does not propagate, it aborts the
process, so every such callback is wrapped -- and here that is one `try/catch`
per encode rather than one per field, which is the difference between a
rounding error and a column.
"""
from cs_facade import Head

# Which presence bit each optional child occupies, from ak-abi's generated
# constants. Emitted rather than hard-coded at the call site so that a change
# in the Rust constants shows up in one place.
PRESENCE = {"created_at": 1 << 0, "completed_at": 1 << 1}


def emit(ir):
    o = Head("The core-ffi arm: the host binding over libak_core.so. M1 encode.")
    o += "using System;"
    o += "using System.Runtime.CompilerServices;"
    o += "using System.Runtime.InteropServices;"
    o += "using System.Text;"
    o += "using Armonik.Ffi.Facade;"
    o += ""
    o += "namespace Armonik.Ffi.Harness;"
    o += ""
    o.doc("What the loop callback is handed as `obj`. A native struct rather than a "
          "GCHandle: `[UnmanagedCallersOnly]` cannot capture, and a handle round trip "
          "would put a managed allocation on a path whose whole point is not having one.")
    o += "[StructLayout(LayoutKind.Sequential)]"
    o += "public unsafe struct HostRun"
    o += "{"
    o += "    public ak_efix_ResultRaw* Groups;"
    o += "    public int Count;"
    o += "}"
    o += ""
    o += "public sealed unsafe class CoreFfiM1 : IDisposable"
    o += "{"
    o += "    private IntPtr _ctx;"
    o += "    private byte* _staging;"
    o += "    private int _stagingCap;"
    o += "    private ak_efix_ResultRaw* _groups;"
    o += "    private int _groupCap;"
    o += "    private HostRun* _run;"
    o += "    private static IntPtr _tcBytes;"
    o += ""
    o += "    /// Counted so the arm can report crossings without a counting build:"
    o += "    /// the host's own side of R5. The codec's counters are the other half."
    o += "    public long ForwardCalls;"
    o += "    public long ReverseCalls;"
    o += ""
    o += "    public CoreFfiM1(int elements, int stagingBytes)"
    o += "    {"
    o += "        _ctx = Abi.ak_enc_ctx_new();"
    o += "        if (_ctx == IntPtr.Zero) throw new InvalidOperationException(\"ak_enc_ctx_new returned null\");"
    o += "        _tcBytes = Abi.ak_tc_bytes();"
    o += "        _groupCap = elements;"
    o += "        _groups = (ak_efix_ResultRaw*)NativeMemory.AlignedAlloc("
    o += "            (nuint)(sizeof(ak_efix_ResultRaw) * elements), 16);"
    o += "        _stagingCap = stagingBytes;"
    o += "        _staging = (byte*)NativeMemory.Alloc((nuint)stagingBytes);"
    o += "        _run = (HostRun*)NativeMemory.Alloc((nuint)sizeof(HostRun));"
    o += "    }"
    o += ""
    o.doc("The loop over `results`. ABI v1 section 6's batched run: the host hands the "
          "codec the WHOLE array in one forward call, which is what makes the crossing "
          "count a constant rather than a function of the element count.", "    ")
    o += "    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]"
    o += "    private static int LoopResults(IntPtr ctx, void* obj, long token)"
    o += "    {"
    o += "        // THE ABORT GUARD. A managed exception crossing this frame does not"
    o += "        // unwind into Rust, it terminates the process. There is nothing to"
    o += "        // catch it further out, so it is caught here and reported as a code."
    o += "        try"
    o += "        {"
    o += "            var run = (HostRun*)obj;"
    o += "            return Abi.ak_elem_ResultRaw(ctx, run->Groups, run->Count);"
    o += "        }"
    o += "        catch"
    o += "        {"
    o += "            return Abi.AK_ERR_HOST;"
    o += "        }"
    o += "    }"
    o += ""
    o.doc("Transcode one string into the staging buffer and describe it as data. "
          "An EMPTY string is reported absent (`tc == null`), which for an "
          "implicit-presence field is the same thing on the wire and is what the "
          "canonical form asks for.", "    ")
    o += "    private ak_str Stage(string s, ref int at)"
    o += "    {"
    o += "        if (s == null || s.Length == 0) return default;"
    o += "        int n = Encoding.UTF8.GetBytes(s.AsSpan(), new Span<byte>(_staging + at, _stagingCap - at));"
    o += "        var r = new ak_str { data = (IntPtr)(_staging + at), len = (nuint)n, tc = _tcBytes };"
    o += "        at += n;"
    o += "        return r;"
    o += "    }"
    o += ""
    o += "    private ak_str StageBytes(byte[] b, ref int at)"
    o += "    {"
    o += "        if (b == null || b.Length == 0) return default;"
    o += "        new ReadOnlySpan<byte>(b).CopyTo(new Span<byte>(_staging + at, _stagingCap - at));"
    o += "        var r = new ak_str { data = (IntPtr)(_staging + at), len = (nuint)b.Length, tc = _tcBytes };"
    o += "        at += b.Length;"
    o += "        return r;"
    o += "    }"
    o += ""
    o.doc("Fill the by-value group array from the facade, then one forward call. "
          "Returns the encoded length; the bytes are borrowed from the context and "
          "are valid until the next reset.", "    ")
    o += "    public int Encode(ListResultsResponse src, out byte* outPtr, out int outLen)"
    o += "    {"
    o += "        int n = src.Results.Count;"
    o += "        if (n > _groupCap) throw new InvalidOperationException(\"group array too small\");"
    o += "        Abi.ak_enc_reset(_ctx);"
    o += "        int at = 0;"
    o += "        for (int i = 0; i < n; i++)"
    o += "        {"
    o += "            var r = src.Results[i];"
    o += "            ref var g = ref _groups[i];"
    o += "            g = default;"
    o += "            g.session_id = Stage(r.SessionId, ref at);"
    o += "            g.name = Stage(r.Name, ref at);"
    o += "            g.owner_task_id = Stage(r.OwnerTaskId, ref at);"
    o += "            g.status = (int)r.Status;"
    for f, bit in PRESENCE.items():
        prop = "".join(p.capitalize() for p in f.split("_"))
        o += "            if (r.%s != null)" % prop
        o += "            {"
        o += "                g.%s.seconds = r.%s.Seconds;" % (f, prop)
        o += "                g.%s.nanos = r.%s.Nanos;" % (f, prop)
        o += "                g.presence |= %d;" % bit
        o += "            }"
    o += "            g.result_id = Stage(r.ResultId, ref at);"
    o += "            g.size = r.Size;"
    o += "            g.created_by = Stage(r.CreatedBy, ref at);"
    o += "            g.opaque_id = StageBytes(r.OpaqueId, ref at);"
    o += "            g.manual_deletion = r.ManualDeletion ? (byte)1 : (byte)0;"
    o += "        }"
    o += "        _run->Groups = _groups;"
    o += "        _run->Count = n;"
    o += "        var vt = new ak_evt_ListResultsResponse { loop_results = &LoopResults };"
    o += "        var fix = new ak_efix_ListResultsResponse { page = src.Page, total = src.Total };"
    o += "        ForwardCalls++;        // ak_encode_*"
    o += "        ReverseCalls++;        // loop_results"
    o += "        ForwardCalls++;        // ak_elem_* inside it"
    o += "        nint rc = Abi.ak_encode_ListResultsResponse(_run, _ctx, &vt, &fix);"
    o += "        if (rc < 0) throw new InvalidOperationException($\"core encode failed: {rc}\");"
    o += "        byte* p; nuint len;"
    o += "        int tk = Abi.ak_enc_take(_ctx, &p, &len);"
    o += "        if (tk != 0) throw new InvalidOperationException($\"ak_enc_take failed: {tk}\");"
    o += "        outPtr = p; outLen = (int)len;"
    o += "        return outLen;"
    o += "    }"
    o += ""
    o += "    public byte[] EncodeToArray(ListResultsResponse src)"
    o += "    {"
    o += "        Encode(src, out byte* p, out int len);"
    o += "        var a = new byte[len];"
    o += "        new ReadOnlySpan<byte>(p, len).CopyTo(a);"
    o += "        return a;"
    o += "    }"
    o += ""
    o += "    public void Dispose()"
    o += "    {"
    o += "        if (_ctx != IntPtr.Zero) { Abi.ak_enc_ctx_free(_ctx); _ctx = IntPtr.Zero; }"
    o += "        if (_groups != null) { NativeMemory.AlignedFree(_groups); _groups = null; }"
    o += "        if (_staging != null) { NativeMemory.Free(_staging); _staging = null; }"
    o += "        if (_run != null) { NativeMemory.Free(_run); _run = null; }"
    o += "    }"
    o += "}"
    return str(o)
