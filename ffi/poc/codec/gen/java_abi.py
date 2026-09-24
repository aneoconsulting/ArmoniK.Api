"""Java backend: the C ABI the JNI shim compiles against, and the slot tables both halves
of the Java binding index, rendered from a plan (FIX-PLAN WP5 step 3).

Two things live here.

1. **`emit_header(p)`**: the C declaration of the ABI (`ak_abi.h`) for this plan. Before
   WP5 the java slice compiled against a header rendered by the cpp slice's
   `cpp_header.py` (another slice's generator, over the IR) and declared the pull family,
   the UTF-16 transcoders and the whole RPC half by hand in `shim.c` and `native/rpc.c`
   (R-G5's Java instance). Now every group comes from `plan.group_fields` /
   `ugroup_fields` / `presence_bits`, every RPC struct, callback, constant and prototype
   from `plan.rpc`, and `ak_init` with its options struct from `plan.lifecycle` (R-G7). The
   header also static-asserts every offset `java_layout` computed, so a disagreement
   between the Java binding's numbers and the C compiler's stops the shim's build.

2. **The slot tables** (`enc_slots`, `dec_slots`, `enc_vtable`, `dec_vtable`,
   `pull_records`): which vtable member is which, in the order the core reads them.
   Both the C shim's trampoline arrays and the Java binding's vtable fill index these, so
   they are enumerated once for the two.

PLAN GAP, reported rather than resolved here: the ORDER and SHAPE of the vtable structs
(`ak_evt_M`: loop slots with an `elem_` pointer where the element has slots of its own;
`ak_dvt_M`: apply, unknown, then per slot `unk_`, and either `add_` (batchable, the element
is a leaf) or `new_` / `apply_` / `add_<slot>_<inner>`), and the pull family's record slot
numbering (`(outer << 16) | inner`), are not in the plan contract: they are rendered by
`rust_abi.emit_abi` / `emit_codec`, a Rust BACKEND, from `loop_slots`, `elem_type` and
`MessagePlan.leaf`. This module restates that rendering from the same plan facts, and the
layout guard cannot check a vtable (it is not a group). The numeric values of the lifecycle
flags (`AK_INIT_*`), `ak_err`'s members, the error codes and the record header are likewise
fixed text in `ak-abi/src/lib.rs`, named but not valued by `plan.lifecycle`.
"""
from plan import (abi_order_topo, direct_fields, elem_type, element_types, group_fields,
                  loop_slots, presence_bits, slot_elem, slot_name, ugroup_fields,
                  vtable_messages)
import cpp_layout
import java_layout as JL
import java_names as N

WHO = "java_abi.py"

# The plan's abi vocabulary -> C.
C_ABI = {"i32": "int32_t", "i64": "int64_t", "u8": "uint8_t", "u32": "uint32_t",
         "f64": "double", "u64": "uint64_t", "usize": "size_t", "void": "void",
         "ak_str": "struct ak_str", "ak_span": "struct ak_span", "ak_blob": "struct ak_blob"}


def c_type(t):
    """A plan type spelling (`i32`, `ak_efix_M`, `*const u8`, `*mut ak_runtime`) in C."""
    t = t.strip()
    if t.startswith("*const "):
        return "const %s *" % c_type(t[len("*const "):])
    if t.startswith("*mut "):
        return "%s *" % c_type(t[len("*mut "):])
    if t in C_ABI:
        return C_ABI[t]
    if t.startswith(("ak_efix_", "ak_dfix_", "ak_ufix_")):
        return "struct %s" % t
    return t                      # a typedef'd name: ak_runtime, ak_bytes, ak_completion_cb


# ---------------------------------------------------------------- slot tables

def enc_vtable(p, name):
    """`ak_evt_<name>`'s members as (kind, slot name, element type): kind loop | elem |
    reserved. See the module's PLAN GAP note: rust_abi.emit_abi's order."""
    slots = loop_slots(p, name)
    if not slots:
        return [("reserved", "_reserved", None)]
    rows = []
    for path, f in slots:
        sn = slot_name(path)
        rows.append(("loop", sn, None))
        et = elem_type(f)
        if et and loop_slots(p, et):
            rows.append(("elem", sn, et))
    return rows


def batchable(p, f):
    """ABI v1 section 7.2 as rust_abi renders it: a run is batched iff its element type,
    if it has one, is a leaf."""
    et = elem_type(f)
    return not (et and not p.msg(et).leaf)


def dec_vtable(p, name):
    """`ak_dvt_<name>`'s members as (kind, slot name, x): kind apply | unknown | unk | add |
    new | applyelem | addinner; x is the element type name (new, applyelem, unk, add of a
    message run), the inner FieldPlan (addinner) or None."""
    rows = [("apply", "", None), ("unknown", "", None)]
    for path, f in loop_slots(p, name):
        sn = slot_name(path)
        et = elem_type(f)
        if et:
            rows.append(("unk", sn, et))
        if batchable(p, f):
            rows.append(("add", sn, et))
        else:
            rows.append(("new", sn, et))
            rows.append(("applyelem", sn, et))
            for ipath, iff in loop_slots(p, et):
                iet = elem_type(iff)
                if iet and not p.msg(iet).leaf:
                    raise NotImplementedError(
                        "a non-leaf element inside a non-leaf element (%s.%s): refused, as"
                        " rust_abi refuses it" % (et, slot_name(ipath)))
                rows.append(("addinner", "%s_%s" % (sn, slot_name(ipath)), iff))
    return rows


def enc_slots(p):
    """[(vtable message, slot path, field)] in vtable order, globally numbered."""
    out = []
    for name in vtable_messages(p):
        for path, f in loop_slots(p, name):
            out.append((name, path, f))
    return out


def dec_slots(p):
    """Every decode callback that needs a trampoline, globally numbered, as (root, kind,
    slot name, x). Roots only: a non-root element's callbacks live in the root's vtable,
    flattened (`addinner`). Decision 11's unknown-field slots are not wired by this
    binding (they stay NULL: unknown fields dropped)."""
    out = []
    for name in p.roots:
        for kind, sn, x in dec_vtable(p, name):
            if kind in ("unknown", "unk"):
                continue
            out.append((name, kind, sn, x))
    return out


AK_BDR_APPLY, AK_BDR_ADD, AK_BDR_NEW, AK_BDR_APPLY_ELEM = 1, 2, 3, 4


def pull_records(p, root, dec_ix):
    """[(op, record slot, dec slot index, kind)] for one root: the pull family's slot
    numbering, `(outer << 16) | inner`, 1-based (see the PLAN GAP note)."""
    out = [(AK_BDR_APPLY, 0, dec_ix[(root, "apply", "")], "apply")]
    for j, (path, f) in enumerate(loop_slots(p, root)):
        sn = slot_name(path)
        if not batchable(p, f):
            et = elem_type(f)
            out.append((AK_BDR_NEW, (j + 1) << 16, dec_ix[(root, "new", sn)], "new"))
            out.append((AK_BDR_APPLY_ELEM, (j + 1) << 16, dec_ix[(root, "applyelem", sn)],
                        "applyelem"))
            for k, (ipath, _iff) in enumerate(loop_slots(p, et)):
                isn = "%s_%s" % (sn, slot_name(ipath))
                out.append((AK_BDR_ADD, ((j + 1) << 16) | (k + 1),
                            dec_ix[(root, "addinner", isn)], "addinner"))
        else:
            out.append((AK_BDR_ADD, j + 1, dec_ix[(root, "add", sn)], "add"))
    return out


# ---------------------------------------------------------------- the header

FIXED_TOP = r'''
#ifndef AK_ABI_H
#define AK_ABI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ---- section 5: errors ---------------------------------------------------------- */
#define AK_OK                   0
#define AK_ALREADY_INITIALIZED  1
#define AK_ERR_HOST            (-1)
#define AK_ERR_MALFORMED       (-2)
#define AK_ERR_TRUNCATED       (-3)
#define AK_ERR_DEPTH           (-4)
#define AK_ERR_LIMIT           (-5)
#define AK_ERR_TRANSCODE       (-6)
#define AK_ERR_CAPACITY        (-7)
#define AK_ERR_INVALID_STATE   (-8)
#define AK_ERR_PANIC           (-9)
#define AK_ERR_UNINITIALIZED  (-10)
#define AK_ERR_ABI            (-11)

#define AK_ABI_VERSION 1u

#ifdef __cplusplus
#define AK_SASSERT(c, m) static_assert(c, m)
#else
#define AK_SASSERT(c, m) _Static_assert(c, m)
#endif
'''

VOCAB = r'''
/* ---- section 4: common vocabulary ------------------------------------------------ */

typedef int32_t (*ak_grow_fn)(void *sink, int32_t want, uint8_t **dst, int32_t *cap);
typedef int32_t (*ak_transcode_fn)(const void *src, size_t len,
                                   uint8_t *dst, int32_t cap,
                                   ak_grow_fn grow, void *sink);

/* Encode: a string or bytes field as DATA inside the group. `len` counts SOURCE code
 * units. `tc == NULL` means the field is ABSENT. */
struct ak_str { const void *data; size_t len; ak_transcode_fn tc; };

/* ABI v1 section 8: a reserved `data` value meaning "this field is a direct argument". */
#define AK_STR_DIRECT ((const void *)(uintptr_t)1)

/* Decode: an OFFSET into the buffer the host handed in, plus a byte length. */
struct ak_span { uint32_t off, len, coder; };

/* Decision 11's unknown-field bag: two words, not three. */
struct ak_blob { const void *data; size_t len; };

struct ak_uspan { int64_t token; uint32_t off, len; };

#define AK_TOKEN_ROOT ((int64_t)-1)

typedef struct ak_enc_ctx ak_enc_ctx;
typedef struct ak_dec_ctx ak_dec_ctx;

typedef int32_t (*ak_loop_f)(ak_enc_ctx *ctx, const void *obj, int64_t token);
typedef void (*ak_unk_f)(ak_dec_ctx *ctx, void *obj, const struct ak_uspan *spans, int32_t n);
'''

FIXED_EXPORTS = r'''
/* ---- the fixed exports (sections 3, 4, 5) ---------------------------------------- */
uint32_t ak_abi_version(void);
ak_enc_ctx *ak_enc_ctx_new(void);
void ak_enc_ctx_free(ak_enc_ctx *);
void ak_enc_reset(ak_enc_ctx *);
int32_t ak_enc_take(ak_enc_ctx *, const uint8_t **ptr, size_t *len);
int32_t ak_enc_err(const ak_enc_ctx *);
ak_dec_ctx *ak_dec_ctx_new(void);
void ak_dec_ctx_free(ak_dec_ctx *);
void ak_fail(void *ctx, int32_t code, const uint8_t *msg, uint32_t msg_len);
int32_t ak_dec_err(const ak_dec_ctx *);
void ak_dec_err_reset(ak_dec_ctx *);
int32_t ak_initialized(void);
const char *ak_build_id(void);

/* Section 4's transcoders: the two a UTF-8 host needs, the passthrough, and the two a
 * UTF-16 host (the JVM) reaches. */
ak_transcode_fn ak_tc_utf8(void);
ak_transcode_fn ak_tc_utf8_trusted(void);
ak_transcode_fn ak_tc_utf8_simd(void);
ak_transcode_fn ak_tc_bytes(void);
ak_transcode_fn ak_tc_utf16(void);
ak_transcode_fn ak_tc_latin1(void);

/* ---- section 7.1's PULL family ---------------------------------------------------- */
struct ak_bdr_rec { uint32_t op; uint32_t slot; int64_t token; uint32_t n; uint32_t bytes; };
#define AK_BDR_APPLY       1u
#define AK_BDR_ADD         2u
#define AK_BDR_NEW         3u
#define AK_BDR_APPLY_ELEM  4u
#define AK_BDR_MIN_CHUNK   (32u * 1024u + 24u)
AK_SASSERT(sizeof(struct ak_bdr_rec) == 24, "sizeof ak_bdr_rec");
AK_SASSERT(offsetof(struct ak_bdr_rec, token) == 8, "ak_bdr_rec.token");
AK_SASSERT(offsetof(struct ak_bdr_rec, n) == 16, "ak_bdr_rec.n");
AK_SASSERT(offsetof(struct ak_bdr_rec, bytes) == 20, "ak_bdr_rec.bytes");
int32_t ak_bdr_reserve(ak_dec_ctx *ctx, size_t bytes);
size_t ak_bdr_footprint(const ak_dec_ctx *ctx);
intptr_t ak_bdr_drain(ak_dec_ctx *ctx, uint8_t *dst, size_t cap, size_t *cursor);
int32_t ak_bdr_ptr(ak_dec_ctx *ctx, const uint8_t **ptr, size_t *len);
void ak_bdr_reset(ak_dec_ctx *ctx);
void ak_bdr_count_forward(ak_dec_ctx *ctx, uint32_t n);

/* README R5: counted in the CORE, so a call the optimiser removed is not counted. */
struct AkCounters {
  uint64_t forward, reverse, transcode, prefix_moves, prefix_bytes, grows;
};
void ak_enc_counters(const ak_enc_ctx *, struct AkCounters *out);
void ak_enc_count_reverse(ak_enc_ctx *);
void ak_enc_counters_reset(ak_enc_ctx *);
void ak_dec_counters(const ak_dec_ctx *, struct AkCounters *out);
void ak_dec_counters_reset(ak_dec_ctx *);
size_t ak_enc_site_moves(const ak_enc_ctx *, uint32_t *out, size_t cap);

/* The boundary priced on its own, in the same process and build as the arms. */
uint64_t ak_noop(uint64_t x);
uint64_t ak_noop_reverse(uint64_t (*f)(uint64_t), uint64_t x);

/* ABI v1 section 10: the core's own view of every group layout. */
size_t ak_layout_facts(uint32_t *out, size_t cap);
'''

# plan.lifecycle names these flags; their values are ak-abi's (see the PLAN GAP note).
INIT_FLAGS = [("AK_INIT_OWN_LOGGING", "(1u << 0)"), ("AK_INIT_NO_PANIC_HOOK", "(1u << 1)"),
              ("AK_INIT_NO_CRYPTO", "(1u << 2)")]


def _lifecycle(p):
    lc = p.lifecycle
    o = ["", "/* ---- section 3: the lifecycle, from plan.lifecycle (R-G7) -----------------",
         " * %s requires `%s` to have returned %s first; a gate builds the core with" % (
             lc.required_before, lc.init[0], " or ".join(lc.success)),
         " * `%s`, so a binding that skips it fails instead of passing silently. */" % lc.gate_feature]
    for n, v in INIT_FLAGS:
        o.append("#define %s %s" % (n, v))
    for fl in lc.default_flags:
        if fl not in dict(INIT_FLAGS):
            raise NotImplementedError("plan.lifecycle flag %s has no value in this header" % fl)
    o.append("typedef void (*ak_log_fn)(void *ctx, uint32_t level, const uint8_t *msg, size_t msg_len);")
    sname, members = lc.opts_struct
    o.append("struct %s {" % sname)
    lay = []
    for mname, ty in members:
        base = ty.split(" (")[0]
        if base == "ak_log_fn":
            o.append("  ak_log_fn %s;   /* nullable */" % mname)
            lay.append((mname, "ptr"))
        else:
            o.append("  %s %s;" % (c_type(base), mname))
            lay.append((mname, "ptr" if base.startswith("*") else base))
    o.append("};")
    o.append("typedef struct %s %s;" % (sname, sname))
    o.append("struct ak_err { int32_t code; uint32_t detail; };")
    o.append("typedef struct ak_err ak_err;")
    L = JL.Layout()
    L.add(sname, lay)
    o += _sasserts(L, sname)
    fn, params, ret = lc.init
    o.append("%s %s(%s);" % (c_type(ret), fn, ", ".join("%s%s" % (_decl(c_type(t)), pn) for pn, t in params)))
    return o


def _decl(ct):
    return ct if ct.endswith("*") else ct + " "


def _sasserts(L, sname):
    size, _al, members = L.groups[sname]
    o = ["AK_SASSERT(sizeof(struct %s) == %d, \"sizeof %s\");" % (sname, size, sname)]
    for mname, off, _ty in members:
        o.append("AK_SASSERT(offsetof(struct %s, %s) == %d, \"%s.%s\");" % (sname, mname, off, sname, mname))
    return o


def _rpc(p):
    r = p.rpc
    o = ["", "/* ---- section 9: the RPC half, from plan.rpc (R-G5) ---------------------- */"]
    for h in r.handles:
        o.append("typedef struct %s %s;" % (h, h))
    L = JL.Layout()
    for sname, doc, members in r.structs:
        o.append("/* %s */" % doc)
        o.append("struct %s {" % sname)
        lay = []
        for mname, ty, mdoc in members:
            o.append("  %s%s;%s" % (_decl(c_type(ty)), mname, ("   /* %s */" % mdoc) if mdoc else ""))
            lay.append((mname, "ptr" if ty.startswith("*") else ty))
        o.append("};")
        o.append("typedef struct %s %s;" % (sname, sname))
        L.add(sname, lay)
        o += _sasserts(L, sname)
    for cname, params, ret, doc in r.callbacks:
        o.append("/* %s */" % doc)
        o.append("typedef %s (*%s)(%s);" % (c_type(ret) if ret else "void", cname,
                                            ", ".join("%s%s" % (_decl(c_type(t)), pn) for pn, t in params)))
    for cname, ty, val, doc in r.constants:
        o.append("#define %s %d   /* %s */" % (cname, val, doc))
    for fname, params, ret, doc in r.functions:
        if doc:
            o.append("/* %s */" % doc)
        ps = ", ".join("%s%s" % (_decl(c_type(t)), pn) for pn, t in params) or "void"
        o.append("%s%s(%s);" % (_decl(c_type(ret)) if ret else "void ", fname, ps))
    return o


def emit_header(p):
    lay = JL.build(p)
    o = [N.c_head(p, WHO),
         "/* The C ABI of ffi/design/ABI-v1.md, version 1, as the Java binding's JNI shim",
         " * compiles against it. Rendered from the plan; see poc/codec/gen/java_abi.py. */",
         FIXED_TOP]
    o += _lifecycle(p)
    o.append(VOCAB)

    for name in abi_order_topo(p):
        m = p.msg(name)
        bits = presence_bits(m)
        for pre in ("e", "d", "u"):
            sname = "ak_%sfix_%s" % (pre, name)
            fields = ugroup_fields(m) if pre == "u" else group_fields(m, pre != "d")
            o.append("/* %s group for `%s`. */" % ({"e": "Encode", "d": "Decode",
                                                  "u": "Encode (with the unknown-field bag)"}[pre], name))
            o.append("struct %s {" % sname)
            for fname, ty in fields:
                o.append("  %s %s;" % (c_type(ty), fname))
            o.append("  uint32_t presence;")
            o.append("};")
            for fname, b in bits.items():
                o.append("#define %s_PRESENT_%s (1u << %d)" % (sname.upper(), fname.upper(), b))
            # The Java binding's offsets, checked by the C compiler (see java_layout).
            o += _sasserts(lay, sname)
            o.append("")

    for name in vtable_messages(p):
        o.append("/* Encode vtable for `%s`. */" % name)
        o.append("struct ak_evt_%s {" % name)
        for kind, sn, et in enc_vtable(p, name):
            if kind == "reserved":
                o.append("  const void *_reserved;   /* an empty struct has no size in C */")
            elif kind == "loop":
                o.append("  ak_loop_f loop_%s;" % sn)
            else:
                o.append("  const struct ak_evt_%s *elem_%s;" % (et, sn))
        o.append("};")
        o.append("")
        o.append("/* Decode vtable for `%s` (the push family, ABI v1 section 7.1). */" % name)
        o.append("struct ak_dvt_%s {" % name)
        slots = {slot_name(path): f for path, f in loop_slots(p, name)}
        for kind, sn, x in dec_vtable(p, name):
            if kind == "apply":
                o.append("  void (*apply)(ak_dec_ctx *, void *, const struct ak_dfix_%s *);" % name)
            elif kind == "unknown":
                o.append("  ak_unk_f unknown;")
            elif kind == "unk":
                o.append("  ak_unk_f unk_%s;" % sn)
            elif kind == "add":
                dty, _ = slot_elem(slots[sn])
                o.append("  void (*add_%s)(ak_dec_ctx *, void *, int64_t, const %s *, int32_t);"
                         % (sn, c_type(dty)))
            elif kind == "new":
                o.append("  int64_t (*new_%s)(ak_dec_ctx *, void *);" % sn)
            elif kind == "applyelem":
                o.append("  void (*apply_%s)(ak_dec_ctx *, void *, int64_t, const struct ak_dfix_%s *);"
                         % (sn, x))
            else:
                idty, _ = slot_elem(x)
                o.append("  void (*add_%s)(ak_dec_ctx *, void *, int64_t, const %s *, int32_t);"
                         % (sn, c_type(idty)))
        o.append("};")
        o.append("")

    o.append("/* ABI v1 section 6: what the host calls are PLAIN EXPORTS, not a table. */")
    for root in p.roots:
        d = ", const uint8_t *direct, size_t direct_len" if direct_fields(p, root) else ""
        o.append("intptr_t ak_encode_%s(const void *obj, ak_enc_ctx *ctx, const struct ak_evt_%s *vt,"
                 " const struct ak_efix_%s *fix%s);" % (root, root, root, d))
        o.append("intptr_t ak_uencode_%s(const void *obj, ak_enc_ctx *ctx, const struct ak_evt_%s *vt,"
                 " const struct ak_ufix_%s *fix%s);" % (root, root, root, d))
        o.append("int32_t ak_decode_%s(ak_dec_ctx *ctx, void *obj, const uint8_t *buf, size_t len,"
                 " const struct ak_dvt_%s *vt);" % (root, root))
        o.append("int32_t ak_parse_%s(ak_dec_ctx *ctx, const uint8_t *buf, size_t len);" % root)
    for et in sorted(element_types(p)):
        if p.msg(et).leaf:
            o.append("int32_t ak_elem_%s(ak_enc_ctx *ctx, const struct ak_efix_%s *elems, int32_t n);" % (et, et))
            o.append("int32_t ak_uelem_%s(ak_enc_ctx *ctx, const struct ak_ufix_%s *elems, int32_t n);" % (et, et))
        else:
            o.append("int32_t ak_elemu_%s(ak_enc_ctx *ctx, const struct ak_efix_%s *elems, int32_t n,"
                     " int64_t tok0);" % (et, et))
            o.append("int32_t ak_uelemu_%s(ak_enc_ctx *ctx, const struct ak_ufix_%s *elems, int32_t n,"
                     " int64_t tok0);" % (et, et))
    o.append("int32_t ak_blob_run(ak_enc_ctx *ctx, const struct ak_str *elems, int32_t n);")
    for ty in ("i32", "i64", "f64", "u8"):
        o.append("int32_t ak_run_%s(ak_enc_ctx *ctx, const %s *p, size_t n);" % (ty, C_ABI[ty]))
    o.append(FIXED_EXPORTS)
    o += _rpc(p)
    n_facts = sum(1 + len(ms) for _s, ms in cpp_layout.facts(p))
    o.append("")
    o.append("/* Number of (struct, member) layout facts: what ak_layout_facts must return. */")
    o.append("#define AK_LAYOUT_FACTS %d" % n_facts)
    o.append("")
    o.append("#ifdef __cplusplus")
    o.append("} /* extern \"C\" */")
    o.append("#endif")
    o.append("#endif /* AK_ABI_H */")
    o.append("")
    return "\n".join(o)
