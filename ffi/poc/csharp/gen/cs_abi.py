"""Backend: the C ABI surface of ABI v1, declared for C#.

**The C# slice binds to the SAME core the Rust slice built.** One native core
with N bindings is the proposal under test, so a slice that grows its own core
is not testing it. The core lives once, at `ffi/poc/codec/` (R0), and every slice
depends on it by path. Built with its default features it is a cdylib
exporting **66** `ak_` symbols implementing ABI v1 over exactly these shapes --
66 and not 68 because ABI v1 section 9's RPC half is behind an `rpc` feature
this slice does not turn on, so the object carries no tonic and no tokio. This
emits the managed half.

Two things make the C# binding different from every other slice's, and both
are emitted here rather than written by hand.

**The layout is declared twice and only agreement makes it work.** Rust owns
the first declaration; a `[StructLayout]` struct is the second. `abi/` prints
the real sizes and offsets and `gen/abi-layout.json` carries them, so the
structs below are emitted with EXPLICIT offsets taken from the Rust build
rather than from C#'s packing rules happening to match. ABI v1 obligation 12.3.

**A managed exception inside `[UnmanagedCallersOnly]` does not propagate: it
aborts the process.** Every reverse callback is emitted wrapped, and the guard
returns an error code the codec understands. The published C# margins were
measured without it, so it is also an arm to price.
"""
import json
import os

from cs_facade import Head

HERE = os.path.dirname(os.path.abspath(__file__))

# The C# spelling of each Rust ABI scalar.
CS = {"i32": "int", "i64": "long", "u32": "uint", "u8": "byte", "usize": "nuint"}


def load_layout():
    with open(os.path.join(HERE, "abi-layout.json")) as f:
        return json.load(f)


# Field type by (struct, field), for the structs M1 needs. Driven by the Rust
# declarations in ak-abi; the probe supplies the offsets and this supplies the
# widths, and a disagreement between them shows up as a size mismatch in the
# emitted assert rather than as a wrong payload.
FIELDS = {
    "ak_str": [("data", "IntPtr"), ("len", "nuint"), ("tc", "IntPtr")],
    "ak_span": [("off", "uint"), ("len", "uint"), ("coder", "uint")],
    "ak_efix_Timestamp": [("seconds", "long"), ("nanos", "int"), ("presence", "uint")],
    "ak_efix_ResultRaw": [
        ("session_id", "ak_str"), ("name", "ak_str"), ("owner_task_id", "ak_str"),
        ("status", "int"), ("created_at", "ak_efix_Timestamp"),
        ("completed_at", "ak_efix_Timestamp"), ("result_id", "ak_str"),
        ("size", "long"), ("created_by", "ak_str"), ("opaque_id", "ak_str"),
        ("manual_deletion", "byte"), ("presence", "uint"),
    ],
    "ak_efix_ListResultsResponse": [("page", "int"), ("total", "int"), ("presence", "uint")],
    "ak_dfix_Timestamp": [("seconds", "long"), ("nanos", "int"), ("presence", "uint")],
    "ak_dfix_ResultRaw": [
        ("session_id", "ak_span"), ("name", "ak_span"), ("owner_task_id", "ak_span"),
        ("status", "int"), ("created_at", "ak_dfix_Timestamp"),
        ("completed_at", "ak_dfix_Timestamp"), ("result_id", "ak_span"),
        ("size", "long"), ("created_by", "ak_span"), ("opaque_id", "ak_span"),
        ("manual_deletion", "byte"), ("presence", "uint"),
    ],
    "ak_dfix_ListResultsResponse": [("page", "int"), ("total", "int"), ("presence", "uint")],
    "ak_efix_Duration": [("seconds", "long"), ("nanos", "int"), ("presence", "uint")],
    "ak_efix_TaskOptionsOptionsEntry": [("key", "ak_str"), ("value", "ak_str"), ("presence", "uint")],
    "ak_efix_TaskOptions": [("max_duration", "ak_efix_Duration"), ("max_retries", "int"), ("priority", "int"), ("partition_id", "ak_str"), ("application_name", "ak_str"), ("application_version", "ak_str"), ("application_namespace", "ak_str"), ("application_service", "ak_str"), ("engine_type", "ak_str"), ("presence", "uint")],
    "ak_efix_TaskOutput": [("success", "byte"), ("error", "ak_str"), ("presence", "uint")],
    "ak_efix_TaskDetailed": [("id", "ak_str"), ("session_id", "ak_str"), ("owner_pod_id", "ak_str"), ("status", "int"), ("status_message", "ak_str"), ("options", "ak_efix_TaskOptions"), ("created_at", "ak_efix_Timestamp"), ("submitted_at", "ak_efix_Timestamp"), ("started_at", "ak_efix_Timestamp"), ("ended_at", "ak_efix_Timestamp"), ("pod_ttl", "ak_efix_Timestamp"), ("output", "ak_efix_TaskOutput"), ("pod_hostname", "ak_str"), ("received_at", "ak_efix_Timestamp"), ("acquired_at", "ak_efix_Timestamp"), ("creation_to_end_duration", "ak_efix_Duration"), ("processing_to_end_duration", "ak_efix_Duration"), ("initial_task_id", "ak_str"), ("received_to_end_duration", "ak_efix_Duration"), ("processed_at", "ak_efix_Timestamp"), ("fetched_at", "ak_efix_Timestamp"), ("payload_id", "ak_str"), ("created_by", "ak_str"), ("presence", "uint")],
    "ak_efix_ListTasksDetailedResponse": [("page", "int"), ("total", "int"), ("presence", "uint")],
    "ak_dfix_Duration": [("seconds", "long"), ("nanos", "int"), ("presence", "uint")],
    "ak_dfix_TaskOptionsOptionsEntry": [("key", "ak_span"), ("value", "ak_span"), ("presence", "uint")],
    "ak_dfix_TaskOptions": [("max_duration", "ak_dfix_Duration"), ("max_retries", "int"), ("priority", "int"), ("partition_id", "ak_span"), ("application_name", "ak_span"), ("application_version", "ak_span"), ("application_namespace", "ak_span"), ("application_service", "ak_span"), ("engine_type", "ak_span"), ("presence", "uint")],
    "ak_dfix_TaskOutput": [("success", "byte"), ("error", "ak_span"), ("presence", "uint")],
    "ak_dfix_TaskDetailed": [("id", "ak_span"), ("session_id", "ak_span"), ("owner_pod_id", "ak_span"), ("status", "int"), ("status_message", "ak_span"), ("options", "ak_dfix_TaskOptions"), ("created_at", "ak_dfix_Timestamp"), ("submitted_at", "ak_dfix_Timestamp"), ("started_at", "ak_dfix_Timestamp"), ("ended_at", "ak_dfix_Timestamp"), ("pod_ttl", "ak_dfix_Timestamp"), ("output", "ak_dfix_TaskOutput"), ("pod_hostname", "ak_span"), ("received_at", "ak_dfix_Timestamp"), ("acquired_at", "ak_dfix_Timestamp"), ("creation_to_end_duration", "ak_dfix_Duration"), ("processing_to_end_duration", "ak_dfix_Duration"), ("initial_task_id", "ak_span"), ("received_to_end_duration", "ak_dfix_Duration"), ("processed_at", "ak_dfix_Timestamp"), ("fetched_at", "ak_dfix_Timestamp"), ("payload_id", "ak_span"), ("created_by", "ak_span"), ("presence", "uint")],
    "ak_dfix_ListTasksDetailedResponse": [("page", "int"), ("total", "int"), ("presence", "uint")],
}


def emit(ir):
    lay = load_layout()
    st = lay["structs"]
    o = Head("The C ABI of ABI v1, declared for C#, at the offsets the Rust build reports.")
    o += "using System;"
    o += "using System.Runtime.CompilerServices;"
    o += "using System.Runtime.InteropServices;"
    o += ""
    o += "namespace Armonik.Ffi.Harness;"
    o += ""

    # ---- the structs, at explicit offsets ---------------------------
    for name, fields in FIELDS.items():
        if name not in st:
            raise KeyError("%s is not in abi-layout.json; re-run abi/ and regenerate" % name)
        info = st[name]
        o.doc("Rust `%s`: %d bytes, align %d. Offsets are the Rust build's own, "
              "not C#'s packing." % (name, info["size"], info["align"]))
        o += "[StructLayout(LayoutKind.Explicit, Size = %d)]" % info["size"]
        o += "public struct %s" % name
        o += "{"
        for fname, ftype in fields:
            if fname not in info["fields"]:
                raise KeyError("%s.%s is not in the layout probe's output" % (name, fname))
            o += "    [FieldOffset(%d)] public %s %s;" % (info["fields"][fname], ftype, fname)
        o += "}"
        o += ""

    o.doc("The core's own counters. **This is the convention the cross-language "
          "table uses** (R5): `forward` is every `extern \"C\"` entry point the host "
          "called, `reverse` is every function pointer the core invoked INCLUDING "
          "transcoders. A host-side tally is not the same quantity and the two must not "
          "be compared -- which is exactly the discrepancy this slice reported against "
          "the Rust slice, and reading the core's counters is how it is resolved rather "
          "than negotiated.")
    o += "[StructLayout(LayoutKind.Sequential)]"
    o += "public struct AkCounters"
    o += "{"
    o += "    public ulong forward;"
    o += "    public ulong reverse;"
    o += "    public ulong transcode;"
    o += "    public ulong prefix_moves;"
    o += "    public ulong prefix_bytes;"
    o += "    public ulong grows;"
    o += "}"
    o += ""

    # ---- the imports -------------------------------------------------
    o.doc("ABI v1 section 6: what a host calls in the codec are PLAIN EXPORTS, not a "
          "table, so the host declares the symbols it uses and a missing one is a load "
          "failure rather than a null slot found at the wrong moment.")
    o += "public static unsafe partial class Abi"
    o += "{"
    o += '    public const string Lib = "ak_core";'
    o += ""
    o += "    public const int AK_TOKEN_ROOT = -1;"
    o += "    /// What the abort guard returns when the host throws. ABI v1 section 5:"
    o += "    /// sticky, first error wins."
    o += "    public const int AK_ERR_HOST = -9;"
    o += ""
    for decl in [
        ("uint", "ak_abi_version", ""),
        ("IntPtr", "ak_enc_ctx_new", ""),
        ("void", "ak_enc_ctx_free", "IntPtr ctx"),
        ("void", "ak_enc_reset", "IntPtr ctx"),
        ("int", "ak_enc_take", "IntPtr ctx, byte** ptr, nuint* len"),
        ("int", "ak_enc_err", "IntPtr ctx"),
        ("IntPtr", "ak_tc_bytes", ""),
        ("IntPtr", "ak_tc_utf8_trusted", ""),
        ("int", "ak_elem_ResultRaw", "IntPtr ctx, ak_efix_ResultRaw* elems, int n"),
        ("nint", "ak_encode_ListResultsResponse",
         "void* obj, IntPtr ctx, ak_evt_ListResultsResponse* vt, ak_efix_ListResultsResponse* fix"),
        ("IntPtr", "ak_dec_ctx_new", ""),
        ("void", "ak_dec_ctx_free", "IntPtr ctx"),
        ("int", "ak_dec_err", "IntPtr ctx"),
        ("void", "ak_dec_err_reset", "IntPtr ctx"),
        ("void", "ak_fail", "IntPtr ctx, int code, byte* msg, uint msgLen"),
        ("int", "ak_decode_ListResultsResponse",
         "IntPtr ctx, void* obj, byte* buf, nuint len, ak_dvt_ListResultsResponse* vt"),
        # M2. TaskDetailed is NOT a leaf, so its element entry is `elemu` and the
        # codec calls back per element; `ak_blob_run` carries a repeated string
        # field's whole run, and the map entry type IS a leaf so it batches.
        ("int", "ak_elemu_TaskDetailed",
         "IntPtr ctx, ak_efix_TaskDetailed* elems, int n, long tok0"),
        ("int", "ak_blob_run", "IntPtr ctx, ak_str* elems, int n"),
        ("int", "ak_elem_TaskOptionsOptionsEntry",
         "IntPtr ctx, ak_efix_TaskOptionsOptionsEntry* elems, int n"),
        ("nint", "ak_encode_ListTasksDetailedResponse",
         "void* obj, IntPtr ctx, ak_evt_ListTasksDetailedResponse* vt, ak_efix_ListTasksDetailedResponse* fix"),
        ("int", "ak_decode_ListTasksDetailedResponse",
         "IntPtr ctx, void* obj, byte* buf, nuint len, ak_dvt_ListTasksDetailedResponse* vt"),
        # R5, in the CORE's own convention. Counting build only (--features count);
        # in a non-counting core these return zeroes, which is why the harness
        # reports the build it read them from.
        ("void", "ak_enc_counters", "IntPtr ctx, AkCounters* outp"),
        ("void", "ak_enc_counters_reset", "IntPtr ctx"),
        ("void", "ak_dec_counters", "IntPtr ctx, AkCounters* outp"),
        ("void", "ak_dec_counters_reset", "IntPtr ctx"),
    ]:
        ret, name, args = decl
        o += '    [LibraryImport(Lib)]'
        o += '    [UnmanagedCallConv(CallConvs = new[] { typeof(System.Runtime.CompilerServices.CallConvCdecl) })]'
        o += "    internal static partial %s %s(%s);" % (ret, name, args)
        o += ""
    o += "}"
    o += ""

    # ---- the encode vtable ------------------------------------------
    o.doc("The encode vtable for `ListResultsResponse`: one reverse call, the loop over "
          "`results`. `ResultRaw` is a LEAF, so its own encode vtable is empty and an "
          "element costs no reverse call at all -- which is the property ABI v1's batching "
          "predicate exists to create.")
    o += "[StructLayout(LayoutKind.Sequential)]"
    o += "public unsafe struct ak_evt_ListResultsResponse"
    o += "{"
    o += "    public delegate* unmanaged[Cdecl]<IntPtr, void*, long, int> loop_results;"
    o += "}"
    o += ""

    o.doc("M2's encode vtables. `TaskDetailed` has FIVE loop slots -- the four "
          "repeated string fields and the map -- so unlike `ResultRaw` the codec calls "
          "back into the host once per slot PER ELEMENT, and the crossing count stops "
          "being constant in the element count. The root carries a pointer to the "
          "element's vtable because the codec has to reach those slots.")
    o += "[StructLayout(LayoutKind.Sequential)]"
    o += "public unsafe struct ak_evt_TaskDetailed"
    o += "{"
    for f in ("loop_parent_task_ids","loop_data_dependencies","loop_expected_output_ids",
              "loop_retry_of_ids","loop_options_options"):
        o += "    public delegate* unmanaged[Cdecl]<IntPtr, void*, long, int> %s;" % f
    o += "}"
    o += ""
    o += "[StructLayout(LayoutKind.Sequential)]"
    o += "public unsafe struct ak_evt_ListTasksDetailedResponse"
    o += "{"
    o += "    public delegate* unmanaged[Cdecl]<IntPtr, void*, long, int> loop_tasks;"
    o += "    public ak_evt_TaskDetailed* elem_tasks;"
    o += "}"
    o += ""
    o.doc("M2's decode vtable, and ABI v1 section 7.2's refusal in the interface. "
          "`TaskDetailed` carries repeated and map fields of its own, so the root's "
          "slot is **NOT batchable**: `new_tasks` then `apply_tasks` per element, plus "
          "one run per inner field that occurred. Two reverse calls per element before "
          "any content, against one for the whole run on a leaf.")
    o += "[StructLayout(LayoutKind.Sequential)]"
    o += "public unsafe struct ak_dvt_ListTasksDetailedResponse"
    o += "{"
    o += "    public delegate* unmanaged[Cdecl]<IntPtr, void*, ak_dfix_ListTasksDetailedResponse*, void> apply;"
    o += "    public IntPtr unknown;"
    o += "    public IntPtr unk_tasks;"
    o += "    public delegate* unmanaged[Cdecl]<IntPtr, void*, long> new_tasks;"
    o += "    public delegate* unmanaged[Cdecl]<IntPtr, void*, long, ak_dfix_TaskDetailed*, void> apply_tasks;"
    for f in ("parent_task_ids","data_dependencies","expected_output_ids","retry_of_ids"):
        o += "    public delegate* unmanaged[Cdecl]<IntPtr, void*, long, ak_span*, int, void> add_tasks_%s;" % f
    o += "    // NOTE: there is NO unk_tasks_options_options here. The ELEMENT"
    o += "    // vtable ak_dvt_TaskDetailed has an unk_options_options slot and the"
    o += "    // root does not, and inferring one by analogy shifted every later"
    o += "    // slot by a pointer. VtableSlots below asserts the count."
    o += "    public delegate* unmanaged[Cdecl]<IntPtr, void*, long, ak_dfix_TaskOptionsOptionsEntry*, int, void> add_tasks_options_options;"
    o += "}"
    o += ""

    o.doc("The DECODE vtable for `ListResultsResponse`. `add_results` is handed a whole "
          "RUN of elements per call, so a thousand-element response costs one reverse "
          "call and not a thousand -- the same batching property as the encode side, in "
          "the other direction. Its elements arrive as `ak_dfix_ResultRaw`, whose strings "
          "are `ak_span` OFFSETS into the buffer the host handed in.")
    o += "[StructLayout(LayoutKind.Sequential)]"
    o += "public unsafe struct ak_dvt_ListResultsResponse"
    o += "{"
    o += "    public delegate* unmanaged[Cdecl]<IntPtr, void*, ak_dfix_ListResultsResponse*, void> apply;"
    o += "    public IntPtr unknown;          // decision 11: null is today's behaviour"
    o += "    public IntPtr unk_results;      // decision 11"
    o += "    public delegate* unmanaged[Cdecl]<IntPtr, void*, long, ak_dfix_ResultRaw*, int, void> add_results;"
    o += "}"
    o += ""

    # ---- the layout assert -------------------------------------------
    o.doc("ABI v1 obligation 12.3, as far as it can be taken here.")
    o += "public static class AbiLayout"
    o += "{"
    o += "    /// What the Rust build reported when `gen/abi-layout.json` was written."
    o += "    public static readonly (string Name, int Size, (string F, int Off)[] Fields)[] Expected ="
    o += "    {"
    for name, fields in FIELDS.items():
        info = st[name]
        fs = ", ".join('("%s", %d)' % (f, info["fields"][f]) for f, _ in fields)
        o += '        ("%s", %d, new (string, int)[] { %s }),' % (name, info["size"], fs)
    o += "    };"
    o += ""
    o.doc("Vtable SLOT COUNTS, from the Rust source. Every slot is pointer sized, so "
          "a wrong count is a wrong size and nothing else is needed to catch it. This "
          "exists because a slot WAS invented by analogy -- the element vtable has an "
          "`unk_options_options` and the root does not -- which would have shifted every "
          "later slot by eight bytes and called a garbage address.", "    ")
    o += "    public static readonly (string Name, int Slots)[] Vtables ="
    o += "    {"
    o += '        ("ak_evt_ListResultsResponse", 1),'
    o += '        ("ak_dvt_ListResultsResponse", 4),'
    o += '        ("ak_evt_TaskDetailed", 5),'
    o += '        ("ak_evt_ListTasksDetailedResponse", 2),'
    o += '        ("ak_dvt_ListTasksDetailedResponse", 10),'
    o += "    };"
    o += ""
    o += "    /// Checks the MANAGED declaration against the Rust build, and the loaded"
    o += "    /// core's ABI version."
    o += "    ///"
    o += "    /// **What this does not check, and it is a gap in the ABI rather than in"
    o += "    /// this slice**: the core exports `ak_abi_version` but no LAYOUT. So this"
    o += "    /// verifies C# against the Rust SOURCE at generator time, not against the"
    o += "    /// `.so` actually loaded. A core rebuilt with a changed group layout and an"
    op = "    /// unchanged version number would pass here and fail as a wrong payload."
    o += op
    o += "    /// Obligation 12.3 asks the CORE for a layout export; there isn't one."
    o += "    public static string Check()"
    o += "    {"
    o += "        var bad = new System.Collections.Generic.List<string>();"
    for name, fields in FIELDS.items():
        info = st[name]
        o += "        if (Unsafe.SizeOf<%s>() != %d) bad.Add($\"%s size {Unsafe.SizeOf<%s>()} != %d\");" % (
            name, info["size"], name, name, info["size"])
        for f, _ in fields:
            o += "        if ((int)Marshal.OffsetOf<%s>(\"%s\") != %d) bad.Add($\"%s.%s offset {(int)Marshal.OffsetOf<%s>(\"%s\")} != %d\");" % (
                name, f, info["fields"][f], name, f, name, f, info["fields"][f])
    for nm, slots in (("ak_evt_ListResultsResponse",1),("ak_dvt_ListResultsResponse",4),
                      ("ak_evt_TaskDetailed",5),("ak_evt_ListTasksDetailedResponse",2),
                      ("ak_dvt_ListTasksDetailedResponse",10)):
        o += "        if (Unsafe.SizeOf<%s>() != %d * IntPtr.Size) bad.Add($\"%s has {Unsafe.SizeOf<%s>() / IntPtr.Size} slots, expected %d\");" % (
            nm, slots, nm, nm, slots)
    o += "        uint v = Abi.ak_abi_version();"
    o += "        return bad.Count == 0"
    o += "            ? $\"ok: {Expected.Length} structs and {Vtables.Length} vtables match the Rust build; core ak_abi_version()={v}\""
    o += "            : string.Join(\"; \", bad);"
    o += "    }"
    o += "}"
    return str(o)
