"""Python backend, part 3: the generated CPython shim over the ONE core, rendered from PLANS.

FIX-PLAN WP5 step 5. Replaces `poc/python/gen/py_binding.py`, which walked the IR (and a
hand-kept SCOPE list) itself. README 9.1 gives Python three layers -- the Rust core, a
generated C shim speaking the CPython API, the Python facade -- and this renders the middle
one: the C extension facade type, three accessor backends over one traversal, both
directions, the dispatch tables `native/binding.c` indexes, and `ak_init` (R-G7).

The shim decides no wire rule: the core writes and reads the wire. What the shim decides is
how a facade value reaches a group and back, and every group member, presence bit, loop
slot, element type and batching choice it uses comes from `plan`'s ABI LAYOUT functions
(`group_fields`, `presence_bits`, `loop_slots`, `slot_elem`, `elem_type`, `vtable_messages`,
`MessagePlan.leaf`), the same ones the core's ABI is rendered from. The C header it
compiles against is the C++ backend's `cpp_abi.emit` (plain C99, rendered from the same
plan): one C header renderer for both C-consuming hosts.

| backend | how a field is reached |
|---|---|
| `attr`  | `PyObject_GetAttr` / `SetAttr`: serves the plain AND the `__slots__` facade |
| `cext`  | a struct member of the generated C facade type: not a crossing at all |
| `pyacc` | the premise control (README 9.1): a Python-level accessor CALL per field |

**R-D4, one source for every CPython level.** The C-API calls newer than the 3.7 floor
(`Py_NewRef` 3.10, `PyObject_CallNoArgs` / `PyObject_CallOneArg` 3.9) are reached only
through `AK_NEWREF` / `AK_CALL0` / `AK_CALL1`, defined under `#if PY_VERSION_HEX >=` with a
3.7-compatible `#else`, so the floor and the target compile the same generated file.

Unknown fields: DROPPED in this shim (every `ak_unk_f` slot NULL, the `ak_encode_*` family).
Retention through the C ABI is not rendered here; the pure-Python codec renders both modes.

A backend: imports `plan` only.
"""
from plan import (as_plan, direct_fields, elem_type, loop_slots, presence_bits,
                  slot_name)
import cpp_layout

CHUNK_BYTES = 32768   # ABI v1 sections 6 and 7.3: batched runs chunk at 32 KB.

GSCALAR = {"int32": "int32_t", "enum": "int32_t", "int64": "int64_t", "bool": "uint8_t",
           "double": "double", "fixed32": "uint32_t"}
PACKED_C = {"int64": ("int64_t", "ak_run_i64"), "int32": ("int32_t", "ak_run_i32"),
            "enum": ("int32_t", "ak_run_i32"), "bool": ("uint8_t", "ak_run_u8"),
            "double": ("double", "ak_run_f64")}


class Unsupported(NotImplementedError):
    """A shape this backend has no case for. R1: raise, never skip."""


# ------------------------------------------------------------------ facade attributes

class Attr:
    """One attribute of the facade: a field, a `<oneof>_case` discriminant, or `_unknown`.
    Carries only what the C facade type and the accessors need."""

    def __init__(self, owner, name, kind, card="singular", explicit=False, of=None,
                 oneof=None, tag=0):
        self.owner, self.name, self.kind, self.card = owner, name, kind, card
        self.explicit, self.of, self.oneof, self.tag = explicit, of, oneof, tag

    @classmethod
    def of_field(cls, f):
        return cls(f.owner, f.name, f.kind, f.card, f.presence == "explicit", f.of, f.oneof, f.tag)


def attrs(m):
    out = [Attr.of_field(f) for f in m.fields]
    out += [Attr(m.name, "%s_case" % o, "int32") for o in m.oneofs]
    out.append(Attr(m.name, "_unknown", "bytes"))
    return out


def is_obj(a):
    """Stored as a PyObject* in the C facade type. An explicit-presence scalar is an object
    because absent is None, which no C scalar can hold."""
    return (a.explicit or a.kind in ("string", "bytes", "message", "map")
            or a.card in ("repeated", "packed", "map"))


def cty(a):
    if is_obj(a):
        return "PyObject *"
    return {"int64": "long long ", "int32": "long ", "enum": "long ", "bool": "int ",
            "double": "double ", "fixed32": "unsigned int "}[a.kind]


def member_type(a):
    if is_obj(a):
        return "T_OBJECT_EX"
    return {"int64": "T_LONGLONG", "int32": "T_LONG", "enum": "T_LONG", "bool": "T_BOOL",
            "double": "T_DOUBLE", "fixed32": "T_UINT"}[a.kind]


def parse_fmt(a):
    if is_obj(a):
        return "O"
    return {"int64": "L", "int32": "l", "enum": "l", "bool": "p", "double": "d",
            "fixed32": "I"}[a.kind]


def presence_bit(gp, owner, fname):
    return "AK_%sFIX_%s_PRESENT_%s" % (gp.upper(), owner.upper(), fname.upper())


def chunk_macro(name):
    return "CHUNK_%s" % name.upper()


def real_messages(p):
    return [n for n in p.order if not p.msg(n).synthetic]


def field_of(p, owner, name):
    return next(f for f in p.msg(owner).fields if f.name == name)


# ------------------------------------------------------------------ the C facade type

def emit_ctypes(p, names, modname):
    L = []
    for name in names:
        L.append("typedef struct {\n  PyObject_HEAD")
        for a in attrs(p.msg(name)):
            L.append("  %s%s;" % (cty(a), a.name))
        L.append("} C%s;\n" % name)
    for name in names:
        ff = attrs(p.msg(name))
        L.append("static PyMemberDef mem_%s[] = {" % name)
        for a in ff:
            L.append('  {"%s", %s, offsetof(C%s, %s), 0, NULL},' % (a.name, member_type(a), name, a.name))
        L.append("  {NULL}};")
        objs = [a for a in ff if is_obj(a)]
        L.append("static int init_%s(PyObject *self, PyObject *a, PyObject *kw) {" % name)
        L.append("  static char *kwl[] = {%s NULL};" % "".join('"%s", ' % a.name for a in ff))
        L.append("  C%s *o = (C%s *)self;" % (name, name))
        for a in ff:
            L.append("  %sv_%s = %s;" % (cty(a), a.name, "NULL" if is_obj(a) else "0"))
        L.append('  if (!PyArg_ParseTupleAndKeywords(a, kw, "|%s", kwl, %s)) return -1;'
                 % ("".join(parse_fmt(a) for a in ff), ", ".join("&v_" + a.name for a in ff)))
        for a in ff:
            if is_obj(a):
                if a.card in ("repeated", "packed"):
                    dflt = "PyList_New(0)"
                elif a.card == "map":
                    dflt = "PyDict_New()"
                elif a.name == "_unknown" or (a.kind == "bytes" and not a.explicit):
                    dflt = "PyBytes_FromStringAndSize(NULL, 0)"
                elif a.kind == "string" and not a.explicit:
                    # The same default the Plain facade has (py_pure.default): an implicit
                    # string holds "", not None, so a constructor call that omits it
                    # builds the same message in every storage.
                    dflt = "PyUnicode_FromStringAndSize(NULL, 0)"
                else:
                    dflt = "AK_NEWREF(Py_None)"
                L.append("  { PyObject *v = v_%s ? AK_NEWREF(v_%s) : %s;"
                         " if (!v) return -1; Py_XSETREF(o->%s, v); }" % (a.name, a.name, dflt, a.name))
            else:
                L.append("  o->%s = v_%s;" % (a.name, a.name))
        L.append("  return 0;\n}")
        L.append("static int trav_%s(PyObject *s, visitproc visit, void *arg) {" % name)
        L.append("  C%s *o = (C%s *)s;" % (name, name))
        for a in objs:
            L.append("  Py_VISIT(o->%s);" % a.name)
        L.append("  return 0;\n}")
        L.append("static int clear_%s(PyObject *s) {" % name)
        L.append("  C%s *o = (C%s *)s;" % (name, name))
        for a in objs:
            L.append("  Py_CLEAR(o->%s);" % a.name)
        L.append("  return 0;\n}")
        L += ["static void dealloc_%s(PyObject *s) {" % name,
              "  PyTypeObject *t = Py_TYPE(s);",
              "  PyObject_GC_UnTrack(s); clear_%s(s);" % name,
              "  ((freefunc)PyType_GetSlot(t, Py_tp_free))(s); Py_DECREF(t);\n}",
              "static PyType_Slot slots_%s[] = {" % name,
              "  {Py_tp_init, (void *)init_%s}, {Py_tp_members, (void *)mem_%s}," % (name, name),
              "  {Py_tp_new, (void *)PyType_GenericNew}, {Py_tp_dealloc, (void *)dealloc_%s}," % name,
              "  {Py_tp_traverse, (void *)trav_%s}, {Py_tp_clear, (void *)clear_%s}," % (name, name),
              "  {0, NULL}};",
              'static PyType_Spec spec_%s = {"%s.C%s", sizeof(C%s), 0,' % (name, modname, name, name),
              "  Py_TPFLAGS_DEFAULT | Py_TPFLAGS_BASETYPE | Py_TPFLAGS_HAVE_GC, slots_%s};\n" % name]
    return "\n".join(L)


# ------------------------------------------------------------------ reaching a field

def read_obj(b, owner, fname, var, objexpr="ob", ind="  ", ret="-1", fail=None):
    """`PyObject *var` = the attribute, and `own_var` whether the caller must release it.
    `fail` is the statement run when the read fails (default `return <ret>;`)."""
    p = ind
    fail = fail or ("return %s;" % ret)
    if b == "cext":
        return p + "PyObject *%s = ((C%s *)%s)->%s; const int own_%s = 0;" % (var, owner, objexpr, fname, var)
    if b == "attr":
        return "\n".join([
            p + "BUMP(C_ATTR);",
            p + "PyObject *%s = PyObject_GetAttr(%s, K_%s); const int own_%s = 1;" % (var, objexpr, fname, var),
            p + "if (!%s) { %s }" % (var, fail)])
    return "\n".join([
        p + "BUMP(C_PYCALL);",
        p + "PyObject *g_%s = PyDict_GetItemString(h->acc, \"get_%s\");" % (var, fname),
        p + "if (!g_%s) { PyErr_SetString(PyExc_KeyError, \"get_%s\"); %s }" % (var, fname, fail),
        p + "PyObject *%s = AK_CALL1(g_%s, %s); const int own_%s = 1;" % (var, var, objexpr, var),
        p + "if (!%s) { %s }" % (var, fail)])


def box(a, valexpr):
    k = a.kind
    if k == "bool":
        return "PyBool_FromLong((long)(%s))" % valexpr
    if k == "double":
        return "PyFloat_FromDouble((double)(%s))" % valexpr
    if k == "fixed32":
        return "PyLong_FromUnsignedLong((unsigned long)(%s))" % valexpr
    return "PyLong_FromLongLong((long long)(%s))" % valexpr


def write_field(b, owner, a, valexpr, ind="  ", fail="return -1;", objexpr="ob", boxed=None):
    """Store into the facade. `valexpr` is an owned new reference for an object-valued
    attribute, or a C scalar otherwise (boxed here for the attr and pyacc backends)."""
    p = ind
    obj = is_obj(a)
    if b == "cext":
        if obj:
            return p + "Py_XSETREF(((C%s *)%s)->%s, %s);" % (owner, objexpr, a.name, valexpr)
        return p + "((C%s *)%s)->%s = (%s)(%s);" % (owner, objexpr, a.name, cty(a).strip(), valexpr)
    v = valexpr if obj else box(a, valexpr)
    if b == "attr":
        return "\n".join([
            p + "{ PyObject *v_ = %s; if (!v_) { %s }" % (v, fail),
            p + "  BUMP(C_ATTR);",
            p + "  if (PyObject_SetAttr(%s, K_%s, v_) < 0) { Py_DECREF(v_); %s }" % (objexpr, a.name, fail),
            p + "  Py_DECREF(v_); }"])
    return "\n".join([
        p + "{ PyObject *v_ = %s; if (!v_) { %s }" % (v, fail),
        p + "  BUMP(C_PYCALL);",
        p + "  PyObject *s_ = PyDict_GetItemString(h->acc, \"set_%s\");" % a.name,
        p + "  PyObject *r_ = s_ ? PyObject_CallFunctionObjArgs(s_, %s, v_, NULL) : NULL;" % objexpr,
        p + "  if (!r_) { Py_DECREF(v_); %s } else Py_DECREF(r_);" % fail,
        p + "  Py_DECREF(v_); }"])


def read_scalar(b, owner, a, var, objexpr="ob", ind="  ", ret="-1"):
    """`<C type> var = <the facade attribute as a C scalar>`."""
    p = ind
    t = cty(a).strip()
    if b == "cext":
        return p + "%s %s = ((C%s *)%s)->%s;" % (t, var, owner, objexpr, a.name)
    L = [read_obj(b, owner, a.name, "py_" + var, objexpr, ind, ret), p + "BUMP(C_READ);"]
    err = " { if (own_py_%s) Py_DECREF(py_%s); return %s; }" % (var, var, ret)
    if a.kind == "bool":
        L.append(p + "int %s = PyObject_IsTrue(py_%s);" % (var, var))
        L.append(p + "if (%s < 0)%s" % (var, err))
    elif a.kind == "double":
        L.append(p + "double %s = PyFloat_AsDouble(py_%s);" % (var, var))
        L.append(p + "if (%s == -1.0 && PyErr_Occurred())%s" % (var, err))
    elif a.kind == "fixed32":
        L.append(p + "unsigned int %s = (unsigned int)PyLong_AsUnsignedLong(py_%s);" % (var, var))
        L.append(p + "if (PyErr_Occurred())%s" % err)
    else:
        L.append(p + "%s %s = (%s)PyLong_AsLongLong(py_%s);" % (t, var, t, var))
        L.append(p + "if (%s == (%s)-1 && PyErr_Occurred())%s" % (var, t, err))
    L.append(p + "if (own_py_%s) Py_DECREF(py_%s);" % (var, var))
    return "\n".join(L)


def obj_to_scalar(a, src, dst, ind, fail):
    """Convert an object read from the facade (explicit presence) into a group scalar."""
    p = ind
    ct = GSCALAR[a.kind]
    if a.kind == "bool":
        return "\n".join([p + "int t_ = PyObject_IsTrue(%s);" % src,
                          p + "if (t_ < 0) { %s }" % fail,
                          p + "%s = (uint8_t)t_;" % dst])
    if a.kind == "double":
        return "\n".join([p + "double d_ = PyFloat_AsDouble(%s);" % src,
                          p + "if (d_ == -1.0 && PyErr_Occurred()) { %s }" % fail,
                          p + "%s = d_;" % dst])
    if a.kind == "fixed32":
        return "\n".join([p + "unsigned long u_ = PyLong_AsUnsignedLong(%s);" % src,
                          p + "if (PyErr_Occurred()) { %s }" % fail,
                          p + "%s = (uint32_t)u_;" % dst])
    return "\n".join([p + "long long n_ = PyLong_AsLongLong(%s);" % src,
                      p + "if (n_ == -1 && PyErr_Occurred()) { %s }" % fail,
                      p + "%s = (%s)n_;" % (dst, ct)])


def blob_from(kind, src, ptr, ln, ind, fail):
    """`ptr`/`ln` from a str (UTF-8) or bytes object."""
    p = ind
    if kind == "string":
        return "\n".join([p + "Py_ssize_t %s = 0;" % ln,
                          p + "const char *%s = PyUnicode_AsUTF8AndSize(%s, &%s);" % (ptr, src, ln),
                          p + "if (!%s) { %s }" % (ptr, fail)])
    return "\n".join([p + "char *%s = NULL; Py_ssize_t %s = 0;" % (ptr, ln),
                      p + "if (PyBytes_AsStringAndSize(%s, &%s, &%s)) { %s }" % (src, ptr, ln, fail)])


def tc(kind):
    return "TC_UTF8" if kind == "string" else "TC_BYTES"


# ------------------------------------------------------------------ encode: group fill

def emit_fill(p, name, b):
    """`fill_<b>_<M>`: the group of M from a facade object, into zeroed memory (ABI v1
    decision 9's sparse fill: the memset is the group's default). Loop-slot fields cross
    through the vtable. A singular child's group is inlined, and filled by the same
    function (ABI v1 section 6)."""
    m = p.msg(name)
    bits = presence_bits(m)
    L = ["static int fill_%s_%s(struct ak_efix_%s *e, PyObject *ob, HostCtx *h) {" % (b, name, name),
         "  (void)h; (void)e; (void)ob;"]
    for f in m.plain:
        if f.card != "singular":
            L.append("  /* %s: a loop slot, reached through the vtable */" % f.name)
            continue
        a = Attr.of_field(f)
        L.append("  { /* %s, tag %d, %s, %s */" % (f.name, f.tag, f.kind, f.presence))
        dec_v = "if (own_v) Py_DECREF(v);"
        if f.is_blob:
            L.append(read_obj(b, name, f.name, "v", "ob", "    "))
            L.append("    BUMP(C_READ);")
            if f.presence == "explicit":
                L.append("    if (v != Py_None) {")
                L.append(blob_from(f.kind, "v", "sp", "sl", "      ", dec_v + " return -1;"))
                L.append("      e->presence |= %s;" % presence_bit("e", name, f.name))
                L.append("      e->%s.data = sp; e->%s.len = (size_t)sl; e->%s.tc = %s;"
                         % (f.name, f.name, f.name, tc(f.kind)))
                L.append("    }")
            else:
                L.append(blob_from(f.kind, "v", "sp", "sl", "    ", dec_v + " return -1;"))
                if f.presence == "direct":
                    # ABI v1 section 8: the group carries the SENTINEL and the length; the bytes
                    # go to the call BESIDE the group.
                    L.append("    e->%s.data = AK_STR_DIRECT; e->%s.len = (size_t)sl;" % (f.name, f.name))
                    L.append("    h->direct = (const uint8_t *)sp; h->direct_len = (size_t)sl;")
                else:
                    L.append("    if (sl) { e->%s.data = sp; e->%s.len = (size_t)sl; e->%s.tc = %s; }"
                             % (f.name, f.name, f.name, tc(f.kind)))
            L.append("    " + dec_v)
        elif f.kind == "message":
            if f.name not in bits:
                raise Unsupported("%s.%s: a message child with no presence bit" % (name, f.name))
            L.append(read_obj(b, name, f.name, "v", "ob", "    "))
            L.append("    if (v != Py_None) {")
            L.append("      e->presence |= %s;" % presence_bit("e", name, f.name))
            L.append("      if (fill_%s_%s(&e->%s, v, h)) { %s return -1; }" % (b, f.of, f.name, dec_v))
            L.append("    }")
            L.append("    " + dec_v)
        elif f.presence == "explicit":
            L.append(read_obj(b, name, f.name, "v", "ob", "    "))
            L.append("    BUMP(C_READ);")
            L.append("    if (v != Py_None) {")
            L.append("      e->presence |= %s;" % presence_bit("e", name, f.name))
            L.append(obj_to_scalar(a, "v", "e->%s" % f.name, "      ", dec_v + " return -1;"))
            L.append("    }")
            L.append("    " + dec_v)
        elif f.presence == "implicit":
            # Assigned unconditionally: the core applies the implicit-presence test (a
            # double by its bit pattern), so the shim never decides what is written.
            L.append(read_scalar(b, name, a, "sv", "ob", "    "))
            L.append("    e->%s = (%s)sv;" % (f.name, GSCALAR[f.kind]))
        else:
            raise Unsupported("%s.%s: presence %r" % (name, f.name, f.presence))
        L.append("  }")
    for oname, members in m.oneofs.items():
        ca = Attr(name, "%s_case" % oname, "int32")
        L.append("  { /* oneof %s: the discriminant carries the active member's TAG */" % oname)
        L.append(read_scalar(b, name, ca, "cs", "ob", "    "))
        L.append("    e->%s_case = (uint32_t)cs;" % oname)
        L.append("    switch (cs) {")
        for g in members:
            gn = "%s_%s" % (oname, g.name)
            ga = Attr.of_field(g)
            L.append("    case %d: {" % g.tag)
            dec_mv = "if (own_mv) Py_DECREF(mv);"
            if g.is_blob:
                L.append(read_obj(b, name, g.name, "mv", "ob", "      "))
                L.append("      BUMP(C_READ);")
                L.append(blob_from(g.kind, "mv", "mp", "ml", "      ", dec_mv + " return -1;"))
                # A selected member is written whatever its length: the CASE says so.
                L.append("      e->%s.data = mp; e->%s.len = (size_t)ml; e->%s.tc = %s;"
                         % (gn, gn, gn, tc(g.kind)))
                L.append("      " + dec_mv)
            elif g.kind == "message":
                L.append(read_obj(b, name, g.name, "mv", "ob", "      "))
                L.append("      if (mv == Py_None) { %s" % dec_mv)
                L.append("        PyErr_SetString(PyExc_ValueError, \"%s.%s is selected but None\"); return -1; }"
                         % (name, g.name))
                L.append("      if (fill_%s_%s(&e->%s, mv, h)) { %s return -1; }" % (b, g.of, gn, dec_mv))
                L.append("      " + dec_mv)
            else:
                L.append(read_scalar(b, name, ga, "mv", "ob", "      "))
                L.append("      e->%s = (%s)mv;" % (gn, GSCALAR[g.kind]))
            L.append("      break; }")
        L.append("    case 0: break;")
        L.append("    default:")
        L.append("      PyErr_Format(PyExc_ValueError, \"%s.%s_case = %%ld is not a member's tag\","
                 " (long)cs); return -1;" % (name, oname))
        L.append("    }")
        L.append("  }")
    L.append("  return 0;\n}")
    return "\n".join(L)


# ------------------------------------------------------------------ encode: loop callbacks

def ctx_name(kind, name):
    return "%s%s" % (kind, name)


def walk_path(p, owner, path, b, start, ind):
    """C that walks the singular children `path` from `start` into an OWNED `cur`; an absent
    child means the field is empty (`cur` is then Py_None and the caller returns 0)."""
    if not path:
        return ind + "PyObject *cur = AK_NEWREF(%s);" % start, owner
    L = []
    cur_owner, src = owner, start
    for i, step in enumerate(path):
        f = field_of(p, cur_owner, step)
        var = "p%d" % i
        L.append(read_obj(b, cur_owner, step, var, src, ind,
                          fail=("Py_DECREF(%s); " % src if i else "") + "return -1;"))
        L.append(ind + "(void)own_%s;" % var)
        if b == "cext":
            L.append(ind + "Py_INCREF(%s);" % var)
        if i:
            L.append(ind + "Py_DECREF(%s);" % src)
        if i < len(path) - 1:
            L.append(ind + "if (%s == Py_None) { Py_DECREF(%s); return 0; }" % (var, var))
        src = var
        cur_owner = f.of
    L.append(ind + "PyObject *cur = %s;" % src)
    return "\n".join(L), cur_owner


def emit_loop(p, ctxkind, owner, path, f, b):
    """One encode loop callback, for slot `path` of vtable owner `owner`.

    ctxkind "R": `owner` is a root and the holder is `h->root`.
    ctxkind "E": `owner` is a non-leaf element type and the holder is element `token` of
    the element list the enclosing root loop left in `h->cur[0]`."""
    sn = slot_name(path)
    L = ["static int32_t loop_%s_%s_%s(ak_enc_ctx *ctx, const void *obj, int64_t token) {"
         % (b, ctx_name(ctxkind, owner), sn),
         "  (void)token;",
         "  HostCtx *h = (HostCtx *)obj;"]
    if ctxkind == "R":
        L.append("  PyObject *hold = h->root;")
    else:
        L.append("  PyObject *hold = PyList_GetItem(h->cur[0], (Py_ssize_t)token);")
        L.append("  if (!hold) return -1;")
    code, holder_msg = walk_path(p, owner, path[:-1], b, "hold", "  ")
    L.append(code)
    L.append("  if (cur == Py_None) { Py_DECREF(cur); return 0; }")
    L.append(read_obj(b, holder_msg, f.name, "lst", "cur", "  ", fail="Py_DECREF(cur); return -1;"))
    et = elem_type(f)
    tail = ["out:", "  if (own_lst) Py_DECREF(lst);", "  Py_DECREF(cur);", "  return rc;\n}"]
    if f.card == "map":
        entry = p.msg(f.entry)
        L += ["  PyObject *keys = PyDict_Keys(lst);",
              "  int32_t rc = 0;",
              "  if (!keys) { rc = -1; goto out; }",
              # Entry ORDER: ascending by key. The plan states each entry's own plan and not
              # the order of the entries; ascending by key is the manifest's canonical form,
              # and the pure-Python codec and the Rust core-native control use it too.
              "  if (PyList_Sort(keys) < 0) { Py_DECREF(keys); rc = -1; goto out; }",
              "  Py_ssize_t n = PyList_Size(keys);",
              "  struct ak_efix_%s chunk[%s];" % (f.entry, chunk_macro(f.entry)),
              "  for (Py_ssize_t i = 0; i < n; i += %s) {" % chunk_macro(f.entry),
              "    int32_t k = (int32_t)((n - i < %s) ? (n - i) : %s);" % (chunk_macro(f.entry), chunk_macro(f.entry)),
              "    memset(chunk, 0, sizeof(chunk[0]) * (size_t)k);",
              "    for (int32_t j = 0; j < k; j++) {",
              "      BUMP(C_ITEM);",
              "      PyObject *kk = PyList_GetItem(keys, i + j);",
              "      PyObject *vv = kk ? PyDict_GetItem(lst, kk) : NULL;",
              "      if (!kk || !vv) { Py_DECREF(keys); rc = -1; goto out; }"]
        # The entry's group, member by member, from the plan's pair message.
        for g in entry.fields:
            if not g.is_blob or g.presence != "implicit":
                raise Unsupported("map %s.%s: a %s entry member" % (owner, f.name, g.kind))
            src = "kk" if g.name == "key" else "vv"
            L.append("      BUMP(C_READ);")
            L.append("      {")
            L.append(blob_from(g.kind, src, "bp", "bl", "        ", "Py_DECREF(keys); rc = -1; goto out;"))
            L.append("        if (bl) { chunk[j].%s.data = bp; chunk[j].%s.len = (size_t)bl; chunk[j].%s.tc = %s; }"
                     % (g.name, g.name, g.name, tc(g.kind)))
            L.append("      }")
        L += ["    }",
              "    if (ak_elem_%s(ctx, chunk, k)) { Py_DECREF(keys); rc = -1; goto out; }" % f.entry,
              "  }",
              "  Py_DECREF(keys);"]
        return "\n".join(L + tail)
    nonleaf_msg = (f.card == "repeated" and f.kind == "message" and et and not p.msg(et).leaf)
    if nonleaf_msg:
        # Saved before the first `goto out`, which restores it.
        L.append("  PyObject *saved = h->cur[0];")
    L += ["  Py_ssize_t n = PyList_Size(lst);",
          "  int32_t rc = 0;",
          "  if (n < 0) { rc = -1; goto out; }"]
    if f.card == "packed":
        if f.kind not in PACKED_C:
            raise Unsupported("packed %s (%s.%s): the C ABI has no run symbol for it" % (f.kind, owner, f.name))
        ct, runfn = PACKED_C[f.kind]
        # ABI v1 section 6: a packed run is "the host's own array, handed over whole", ONE
        # `ak_run_*` per field. The array is on the stack up to CHUNK_PACKED values and on
        # the heap beyond; it is never split, because each call writes its own LEN record
        # (legal protobuf, not the canonical form -- defect D13, found by the chunk256 arm).
        L += ["  %s stackbuf[CHUNK_PACKED];" % ct,
              "  %s *vals = stackbuf;" % ct,
              "  if (n > CHUNK_PACKED) {",
              "    vals = (%s *)PyMem_Malloc((size_t)n * sizeof(%s));" % (ct, ct),
              "    if (!vals) { PyErr_NoMemory(); rc = -1; goto out; }",
              "  }",
              "  for (Py_ssize_t j = 0; j < n; j++) {",
              "    BUMP(C_ITEM); BUMP(C_READ);",
              "    PyObject *v = PyList_GetItem(lst, j);",
              "    if (!v) { rc = -1; goto freed; }"]
        if f.kind == "double":
            L += ["    double d = PyFloat_AsDouble(v);",
                  "    if (d == -1.0 && PyErr_Occurred()) { rc = -1; goto freed; }",
                  "    vals[j] = d;"]
        elif f.kind == "bool":
            L += ["    int t = PyObject_IsTrue(v);",
                  "    if (t < 0) { rc = -1; goto freed; }",
                  "    vals[j] = (uint8_t)t;"]
        else:
            L += ["    long long x = PyLong_AsLongLong(v);",
                  "    if (x == -1 && PyErr_Occurred()) { rc = -1; goto freed; }",
                  "    vals[j] = (%s)x;" % ct]
        L += ["  }",
              "  if (n && %s(ctx, vals, (size_t)n)) rc = -1;" % runfn,
              "freed:",
              "  if (vals != stackbuf) PyMem_Free(vals);"]
        return "\n".join(L + tail)
    if f.is_blob:
        L += ["  struct ak_str chunk[CHUNK_BLOB];",
              "  for (Py_ssize_t i = 0; i < n; i += CHUNK_BLOB) {",
              "    int32_t k = (int32_t)((n - i < CHUNK_BLOB) ? (n - i) : CHUNK_BLOB);",
              "    memset(chunk, 0, sizeof(chunk[0]) * (size_t)k);",
              "    for (int32_t j = 0; j < k; j++) {",
              "      BUMP(C_ITEM); BUMP(C_READ);",
              "      PyObject *s = PyList_GetItem(lst, i + j);",
              "      if (!s) { rc = -1; goto out; }",
              blob_from(f.kind, "s", "sp", "sl", "      ", "rc = -1; goto out;"),
              "      chunk[j].data = sp; chunk[j].len = (size_t)sl; chunk[j].tc = %s;" % tc(f.kind),
              "    }",
              "    if (ak_blob_run(ctx, chunk, k)) { rc = -1; goto out; }",
              "  }"]
        return "\n".join(L + tail)
    if f.kind != "message" or not et:
        raise Unsupported("%s.%s: a %s %s loop slot" % (owner, f.name, f.card, f.kind))
    leaf = p.msg(et).leaf
    if not leaf and ctxkind == "E":
        raise Unsupported("%s.%s: a non-leaf element inside a non-leaf element" % (owner, f.name))
    L += ["  struct ak_efix_%s chunk[%s];" % (et, chunk_macro(et)),
          "  int64_t done = 0;"]
    if not leaf:
        # The element list, so the element's own loop slots can index it by token without
        # re-reading the attribute. Only a root loop sets it (non-leaf in non-leaf is refused).
        L.append("  h->cur[0] = lst;")
    L += ["  for (Py_ssize_t i = 0; i < n; i += %s) {" % chunk_macro(et),
          "    int32_t k = (int32_t)((n - i < %s) ? (n - i) : %s);" % (chunk_macro(et), chunk_macro(et)),
          "    /* ABI v1 decision 9: bulk clear once, then assign only what differs. */",
          "    memset(chunk, 0, sizeof(chunk[0]) * (size_t)k);",
          "    for (int32_t j = 0; j < k; j++) {",
          "      BUMP(C_ITEM);",
          "      PyObject *e = PyList_GetItem(lst, i + j);",
          "      if (!e || fill_%s_%s(&chunk[j], e, h)) { rc = -1; goto out; }" % (b, et),
          "    }"]
    if leaf:
        L.append("    if (ak_elem_%s(ctx, chunk, k)) { rc = -1; goto out; }" % et)
    else:
        L.append("    /* NOT a leaf: a token range, and each of the element's own containers is a")
        L.append("       reverse call per element (ABI v1 section 6). */")
        L.append("    if (ak_elemu_%s(ctx, chunk, k, done)) { rc = -1; goto out; }" % et)
    L += ["    done += k;",
          "  }"]
    if not leaf:
        tail = tail[:1] + ["  h->cur[0] = saved;"] + tail[1:]
    tail = tail[:1] + ["  (void)done;"] + tail[1:]
    return "\n".join(L + tail)


def evt_init(p, owner, ctxkind, b, evt_names):
    """The designated initialiser of `struct ak_evt_<owner>`."""
    items = []
    for path, f in loop_slots(p, owner):
        sn = slot_name(path)
        items.append(".loop_%s = loop_%s_%s_%s" % (sn, b, ctx_name(ctxkind, owner), sn))
        et = elem_type(f)
        if et and loop_slots(p, et):
            items.append(".elem_%s = &%s" % (sn, evt_names[et]))
    return "{%s}" % (", ".join(items) if items else "0")


def emit_encode_entry(p, root, b):
    L = ["static PyObject *encode_%s_%s(PyObject *rootobj, PyObject *acc) {" % (b, root),
         "  HostCtx hs; memset(&hs, 0, sizeof hs);",
         "  hs.root = rootobj; hs.acc = acc;",
         "  HostCtx *h = &hs;",
         "  struct ak_efix_%s fix;" % root,
         "  memset(&fix, 0, sizeof fix);",
         "  if (fill_%s_%s(&fix, rootobj, h)) return NULL;" % (b, root),
         "  static const struct ak_evt_%s VT = %s;" % (root, evt_init(p, root, "R", b, EVT_NAMES[b])),
         "  ak_enc_ctx *ctx = ak_enc_ctx_new();",
         "  if (!ctx) return PyErr_NoMemory();",
         ("  intptr_t rc = ak_encode_%s(h, ctx, &VT, &fix, h->direct, h->direct_len);" % root)
         if direct_fields(p, root) else ("  intptr_t rc = ak_encode_%s(h, ctx, &VT, &fix);" % root),
         "  if (rc < 0) {",
         "    ak_enc_ctx_free(ctx);",
         "    if (!PyErr_Occurred()) PyErr_Format(PyExc_RuntimeError, \"ak_encode_%s returned %%ld\", (long)rc);" % root,
         "    return NULL;",
         "  }",
         "#ifdef AK_COUNT",
         "  { struct AkCounters c; ak_enc_counters(ctx, &c); CORE_ADD(CORE_ENC, c); }",
         "#endif",
         "  const uint8_t *pp = NULL; size_t len = 0;",
         "  if (ak_enc_take(ctx, &pp, &len)) { ak_enc_ctx_free(ctx);",
         "    PyErr_SetString(PyExc_RuntimeError, \"ak_enc_take\"); return NULL; }",
         "  PyObject *out = PyBytes_FromStringAndSize((const char *)pp, (Py_ssize_t)len);",
         "  ak_enc_ctx_free(ctx);",
         "  return out;\n}"]
    return "\n".join(L)


EVT_NAMES = {}


# ------------------------------------------------------------------ decode

def span_value(kind, spanexpr):
    if kind == "string":
        return ("PyUnicode_DecodeUTF8((const char *)h->base + %s.off, (Py_ssize_t)%s.len, \"strict\")"
                % (spanexpr, spanexpr))
    return "PyBytes_FromStringAndSize((const char *)h->base + %s.off, (Py_ssize_t)%s.len)" % (spanexpr, spanexpr)


def reach_child(p, owner, path, b, ind, fail, start="ob"):
    """Get-or-CREATE the singular children `path` from `start` into an OWNED `cur`.
    Get-or-create, because ABI v1 decision 10 lets a run arrive before the group that would
    have constructed the child; a binding that only got would drop the run."""
    if not path:
        return ind + "PyObject *cur = AK_NEWREF(%s);" % start, owner
    L = []
    cur_owner, src = owner, start
    for i, step in enumerate(path):
        f = field_of(p, cur_owner, step)
        a = Attr.of_field(f)
        var = "c%d" % i
        rel = ("Py_DECREF(%s); " % src) if i else ""
        L.append(read_obj(b, cur_owner, step, var, src, ind, fail=rel + fail))
        L.append(ind + "(void)own_%s;" % var)
        if b == "cext":
            L.append(ind + "Py_INCREF(%s);" % var)
        L.append(ind + "if (%s == Py_None) {" % var)
        L.append(ind + "  PyObject *nw = AK_CALL0(h->ty_%s);" % f.of)
        L.append(ind + "  if (!nw) { Py_DECREF(%s); %s%s }" % (var, rel, fail))
        L.append(write_field(b, cur_owner, a, "AK_NEWREF(nw)", ind + "  ",
                             fail="Py_DECREF(nw); Py_DECREF(%s); %s%s" % (var, rel, fail), objexpr=src))
        L.append(ind + "  Py_DECREF(%s); %s = nw;" % (var, var))
        L.append(ind + "}")
        if i:
            L.append(ind + "Py_DECREF(%s);" % src)
        src = var
        cur_owner = f.of
    L.append(ind + "PyObject *cur = %s;" % src)
    return "\n".join(L), cur_owner


def emit_setgroup(p, name, b):
    """Fill an EXISTING facade object from one decoded group (get-or-create, never
    construct-and-replace: ABI v1 decision 10, defect D8)."""
    m = p.msg(name)
    bits = presence_bits(m)
    L = ["static int setgroup_%s_%s(HostCtx *h, PyObject *ob, const struct ak_dfix_%s *e) {" % (b, name, name),
         "  (void)h; (void)ob; (void)e;"]
    for f in m.plain:
        if f.card != "singular":
            continue
        a = Attr.of_field(f)
        sp = "e->%s" % f.name
        L.append("  { /* %s */" % f.name)
        if f.presence == "explicit":
            # The presence WORD decides; absent is written as None, because the facade
            # object may be reused and a stale value would read as present.
            L.append("    if (e->presence & %s) {" % presence_bit("d", name, f.name))
            if f.is_blob:
                L.append("      BUMP(C_READ);")
                L.append(write_field(b, name, a, span_value(f.kind, sp), "      "))
            else:
                L.append(write_field(b, name, a, box(a, sp), "      "))
            L.append("    } else {")
            L.append(write_field(b, name, a, "AK_NEWREF(Py_None)", "      "))
            L.append("    }")
        elif f.is_blob:
            L.append("    BUMP(C_READ);")
            L.append(write_field(b, name, a, span_value(f.kind, sp), "    "))
        elif f.kind == "message":
            if f.name not in bits:
                raise Unsupported("%s.%s: a message child with no presence bit" % (name, f.name))
            L.append("    if (e->presence & %s) {" % presence_bit("d", name, f.name))
            code, _ = reach_child(p, name, (f.name,), b, "      ", "return -1;")
            L.append(code)
            L.append("      if (setgroup_%s_%s(h, cur, &e->%s)) { Py_DECREF(cur); return -1; }" % (b, f.of, f.name))
            L.append("      Py_DECREF(cur);")
            L.append("    }")
        else:
            L.append(write_field(b, name, a, sp, "    "))
        L.append("  }")
    for oname, members in m.oneofs.items():
        ca = Attr(name, "%s_case" % oname, "int32")
        L.append("  { /* oneof %s */" % oname)
        L.append("    switch (e->%s_case) {" % oname)
        for g in members:
            gn = "%s_%s" % (oname, g.name)
            ga = Attr.of_field(g)
            L.append("    case %d: {" % g.tag)
            if g.is_blob:
                L.append("      BUMP(C_READ);")
                L.append(write_field(b, name, ga, span_value(g.kind, "e->%s" % gn), "      "))
            elif g.kind == "message":
                code, _ = reach_child(p, name, (g.name,), b, "      ", "return -1;")
                L.append(code)
                L.append("      if (setgroup_%s_%s(h, cur, &e->%s)) { Py_DECREF(cur); return -1; }" % (b, g.of, gn))
                L.append("      Py_DECREF(cur);")
            else:
                L.append(write_field(b, name, ga, "e->%s" % gn, "      "))
            L.append("      break; }")
        L.append("    default: break;")
        L.append("    }")
        # The discriminant LAST, so a watched facade never shows a case pointing at a member
        # that has not been written yet.
        L.append(write_field(b, name, ca, "e->%s_case" % oname, "    "))
        L.append("  }")
    L.append("  return 0;\n}")
    return "\n".join(L)


FAILD = "{ h->failed = 1; ak_fail(ctx, AK_ERR_HOST, NULL, 0); return; }"


def _append_run(p, f, b, owner_msg, ind, lstvar):
    """The body that appends `n` decoded elements of slot field `f` to the Python list or
    dict `lstvar` (a borrowed or owned reference the caller releases)."""
    L = []
    p_ = ind
    et = elem_type(f)
    if f.card == "map":
        entry = p.msg(f.entry)
        L.append(p_ + "for (int32_t i = 0; i < n; i++) {")
        L.append(p_ + "  BUMP(C_READ); BUMP(C_READ); BUMP(C_ITEM);")
        kv = {}
        for g in entry.fields:
            if not g.is_blob:
                raise Unsupported("map %s.%s: a %s entry member" % (owner_msg, f.name, g.kind))
            kv[g.name] = span_value(g.kind, "elems[i].%s" % g.name)
        L.append(p_ + "  PyObject *k = %s;" % kv["key"])
        L.append(p_ + "  PyObject *v = k ? %s : NULL;" % kv["value"])
        L.append(p_ + "  if (!k || !v) { h->failed = 1; Py_XDECREF(k); Py_XDECREF(v); break; }")
        # Plan rule: a duplicate key replaces the earlier value (the host's map insert).
        L.append(p_ + "  if (PyDict_SetItem(%s, k, v) < 0) h->failed = 1;" % lstvar)
        L.append(p_ + "  Py_DECREF(k); Py_DECREF(v);")
        L.append(p_ + "  if (h->failed) break;")
        L.append(p_ + "}")
        return L
    if f.card == "packed":
        if f.kind not in PACKED_C:
            raise Unsupported("packed %s (%s.%s)" % (f.kind, owner_msg, f.name))
        L.append(p_ + "for (int32_t i = 0; i < n; i++) {")
        L.append(p_ + "  BUMP(C_ITEM);")
        L.append(p_ + "  PyObject *v = %s;" % box(Attr.of_field(f), "elems[i]"))
        L.append(p_ + "  if (!v) { h->failed = 1; break; }")
        L.append(p_ + "  if (PyList_Append(%s, v) < 0) { h->failed = 1; Py_DECREF(v); break; }" % lstvar)
        L.append(p_ + "  Py_DECREF(v);")
        L.append(p_ + "}")
        return L
    if f.is_blob:
        L.append(p_ + "for (int32_t i = 0; i < n; i++) {")
        L.append(p_ + "  BUMP(C_READ); BUMP(C_ITEM);")
        L.append(p_ + "  PyObject *s = %s;" % span_value(f.kind, "elems[i]"))
        L.append(p_ + "  if (!s) { h->failed = 1; break; }")
        L.append(p_ + "  if (PyList_Append(%s, s) < 0) { h->failed = 1; Py_DECREF(s); break; }" % lstvar)
        L.append(p_ + "  Py_DECREF(s);")
        L.append(p_ + "}")
        return L
    if f.kind == "message" and et and p.msg(et).leaf:
        L.append(p_ + "for (int32_t i = 0; i < n; i++) {")
        L.append(p_ + "  /* ABI v1 7.4: a batched add may be called more than once per field. Append. */")
        L.append(p_ + "  PyObject *o = AK_CALL0(h->ty_%s);" % et)
        L.append(p_ + "  if (!o) { h->failed = 1; break; }")
        L.append(p_ + "  if (setgroup_%s_%s(h, o, &elems[i])) { h->failed = 1; Py_DECREF(o); break; }" % (b, et))
        L.append(p_ + "  BUMP(C_ITEM);")
        L.append(p_ + "  if (PyList_Append(%s, o) < 0) h->failed = 1;" % lstvar)
        L.append(p_ + "  Py_DECREF(o);")
        L.append(p_ + "  if (h->failed) break;")
        L.append(p_ + "}")
        return L
    raise Unsupported("%s.%s: a %s %s run on decode" % (owner_msg, f.name, f.card, f.kind))


def elem_ctype(p, f):
    et = elem_type(f)
    if f.card == "map":
        return "struct ak_dfix_%s" % f.entry
    if f.is_blob:
        return "struct ak_span"
    if f.card == "packed":
        return PACKED_C[f.kind][0]
    return "struct ak_dfix_%s" % et


def staged(path, f):
    """A root slot directly on the root whose values are a Python list: staged in
    `h->lists[i]` during the decode and stored on the root once at the end (one SetAttr per
    field rather than one read per run)."""
    return len(path) == 1 and f.card in ("repeated", "packed")


def root_lists(p, root):
    return [(path, f) for path, f in loop_slots(p, root) if staged(path, f)]


def emit_root_decode(p, root, b):
    """apply, the adds / new / apply / inner adds of every root slot, the setlists, and the
    decode entry point."""
    L = []
    li_of = {slot_name(path): i for i, (path, f) in enumerate(root_lists(p, root))}
    R = ctx_name("R", root)
    L += ["static void apply_%s_%s(ak_dec_ctx *ctx, void *obj, const struct ak_dfix_%s *fx) {" % (b, R, root),
          "  HostCtx *h = (HostCtx *)obj;",
          "  if (h->failed) return;",
          "  if (setgroup_%s_%s(h, h->root, fx)) %s" % (b, root, FAILD),
          "}"]
    vt = [".apply = apply_%s_%s" % (b, R)]
    for path, f in loop_slots(p, root):
        sn = slot_name(path)
        et = elem_type(f)
        batchable = not (et and not p.msg(et).leaf)
        if batchable:
            L += ["static void add_%s_%s_%s(ak_dec_ctx *ctx, void *obj, int64_t tok," % (b, R, sn),
                  "        const %s *elems, int32_t n) {" % elem_ctype(p, f),
                  "  (void)tok;",
                  "  HostCtx *h = (HostCtx *)obj;",
                  "  if (h->failed) return;"]
            if sn in li_of:
                L += _append_run(p, f, b, root, "  ", "h->lists[%d]" % li_of[sn])
            else:
                code, owner_msg = reach_child(p, root, path[:-1], b, "  ", FAILD, start="h->root")
                L.append(code)
                L.append(read_obj(b, owner_msg, f.name, "lst", "cur", "  ", fail="Py_DECREF(cur); " + FAILD))
                L += _append_run(p, f, b, owner_msg, "  ", "lst")
                L.append("  if (own_lst) Py_DECREF(lst);")
                L.append("  Py_DECREF(cur);")
            L.append("  if (h->failed) ak_fail(ctx, AK_ERR_HOST, NULL, 0);")
            L.append("}")
            vt.append(".add_%s = add_%s_%s_%s" % (sn, b, R, sn))
            continue
        if sn not in li_of:
            raise Unsupported("%s: a non-leaf element slot %s inside an inlined child" % (root, sn))
        li = li_of[sn]
        L += ["static int64_t new_%s_%s_%s(ak_dec_ctx *ctx, void *obj) {" % (b, R, sn),
              "  HostCtx *h = (HostCtx *)obj;",
              "  if (h->failed) return -1;",
              "  PyObject *ob = AK_CALL0(h->ty_%s);" % et,
              "  if (!ob) { h->failed = 1; ak_fail(ctx, AK_ERR_HOST, NULL, 0); return -1; }",
              "  BUMP(C_ITEM);",
              "  Py_ssize_t idx = PyList_Size(h->lists[%d]);" % li,
              "  if (PyList_Append(h->lists[%d], ob) < 0) { h->failed = 1; ak_fail(ctx, AK_ERR_HOST, NULL, 0); idx = -1; }" % li,
              "  Py_DECREF(ob);",
              "  return (int64_t)idx;\n}",
              "static void apply_%s_%s_%s(ak_dec_ctx *ctx, void *obj, int64_t tok," % (b, R, sn),
              "        const struct ak_dfix_%s *e) {" % et,
              "  HostCtx *h = (HostCtx *)obj;",
              "  if (h->failed) return;",
              "  PyObject *ob = PyList_GetItem(h->lists[%d], (Py_ssize_t)tok);" % li,
              "  if (!ob) %s" % FAILD,
              "  if (setgroup_%s_%s(h, ob, e)) %s" % (b, et, FAILD),
              "}"]
        vt.append(".new_%s = new_%s_%s_%s" % (sn, b, R, sn))
        vt.append(".apply_%s = apply_%s_%s_%s" % (sn, b, R, sn))
        for ipath, iff in loop_slots(p, et):
            isn = slot_name(ipath)
            iet = elem_type(iff)
            if iet and not p.msg(iet).leaf:
                raise Unsupported("%s.%s: a non-leaf element inside a non-leaf element" % (et, isn))
            L += ["static void add_%s_%s_%s_%s(ak_dec_ctx *ctx, void *obj, int64_t tok," % (b, R, sn, isn),
                  "        const %s *elems, int32_t n) {" % elem_ctype(p, iff),
                  "  HostCtx *h = (HostCtx *)obj;",
                  "  if (h->failed) return;",
                  "  PyObject *ob = PyList_GetItem(h->lists[%d], (Py_ssize_t)tok);" % li,
                  "  if (!ob) %s" % FAILD]
            code, owner_msg = reach_child(p, et, ipath[:-1], b, "  ", FAILD)
            L.append(code)
            L.append(read_obj(b, owner_msg, iff.name, "lst", "cur", "  ", fail="Py_DECREF(cur); " + FAILD))
            L += _append_run(p, iff, b, owner_msg, "  ", "lst")
            L += ["  if (own_lst) Py_DECREF(lst);",
                  "  Py_DECREF(cur);",
                  "  if (h->failed) ak_fail(ctx, AK_ERR_HOST, NULL, 0);",
                  "}"]
            vt.append(".add_%s_%s = add_%s_%s_%s_%s" % (sn, isn, b, R, sn, isn))
    # setlists: the staged runs stored on the root, once.
    for path, f in root_lists(p, root):
        a = Attr.of_field(f)
        L += ["static int setlist_%s_%s_%s(PyObject *ob, PyObject *lst, HostCtx *h) {" % (b, R, f.name),
              "  (void)h;",
              write_field(b, root, a, "AK_NEWREF(lst)", "  "),
              "  return 0;\n}"]
    lists = root_lists(p, root)
    free = " ".join("Py_DECREF(h.lists[%d]);" % i for i in range(len(lists)))
    L += ["static PyObject *decode_%s_%s(PyObject *buf, PyObject *acc, HostTypes *T) {" % (b, root),
          "  char *pp = NULL; Py_ssize_t blen = 0;",
          "  if (PyBytes_AsStringAndSize(buf, &pp, &blen)) return NULL;",
          "  PyObject *rootobj = AK_CALL0(T->ty_%s);" % root,
          "  if (!rootobj) return NULL;",
          "  HostCtx h;",
          "  memset(&h, 0, sizeof h);",
          "  h.root = rootobj; h.base = (const uint8_t *)pp; h.acc = acc;",
          "  memcpy(&h.ty_%s, T, sizeof *T);" % FIRST_TYPE[0]]
    for i in range(len(lists)):
        prev = " ".join("Py_DECREF(h.lists[%d]);" % j for j in range(i))
        L.append("  h.lists[%d] = PyList_New(0);" % i)
        L.append("  if (!h.lists[%d]) { %s Py_DECREF(rootobj); return NULL; }" % (i, prev))
    L += ["  static const struct ak_dvt_%s VT = {%s};" % (root, ", ".join(vt)),
          "  ak_dec_ctx *ctx = ak_dec_ctx_new();",
          "  if (!ctx) { %s Py_DECREF(rootobj); return PyErr_NoMemory(); }" % free,
          "  int32_t rc = ak_decode_%s(ctx, &h, (const uint8_t *)pp, (size_t)blen, &VT);" % root,
          "#ifdef AK_COUNT",
          "  { struct AkCounters c; ak_dec_counters(ctx, &c); CORE_ADD(CORE_DEC, c); }",
          "#endif",
          "  ak_dec_ctx_free(ctx);",
          "  if (rc || h.failed) {",
          "    %s Py_DECREF(rootobj);" % free,
          "    if (!PyErr_Occurred()) PyErr_Format(PyExc_ValueError, \"ak_decode_%s returned %%d\", (int)rc);" % root,
          "    return NULL;",
          "  }"]
    for i, (path, f) in enumerate(lists):
        L.append("  if (setlist_%s_%s_%s(rootobj, h.lists[%d], &h)) { %s Py_DECREF(rootobj); return NULL; }"
                 % (b, R, f.name, i, free))
    L.append("  %s" % free)
    L.append("  return rootobj;\n}")
    return "\n".join(L)


FIRST_TYPE = [None]


# ------------------------------------------------------------------ the whole file

PRELUDE = r'''/* @generated by ffi/poc/codec/gen/py_capi.py (plan from %(source)s). Do not edit.
 *
 * The python slice's CPython shim over the ONE core (poc/codec). Unknown fields: dropped.
 * `native/binding.c` includes this file and names no message and no field. */
/* `s#` and `y#` yield a Py_ssize_t only with this defined; it must precede Python.h. */
#define PY_SSIZE_T_CLEAN
#include <Python.h>
#include <structmember.h>
#include <string.h>
#include "ak_abi.h"

/* ---- R-D4: one source for every CPython level (floor 3.7, target 3.12) ----------------
 * The only C-API calls newer than 3.7 this file makes go through these three names. */
#if PY_VERSION_HEX >= 0x030A0000
#define AK_NEWREF(o) Py_NewRef(o)
#else
static inline PyObject *AK_NEWREF(PyObject *o) { Py_INCREF(o); return o; }
#endif
#if PY_VERSION_HEX >= 0x03090000
#define AK_CALL0(c) PyObject_CallNoArgs(c)
#define AK_CALL1(c, a) PyObject_CallOneArg((c), (a))
#else
#define AK_CALL0(c) PyObject_CallObject((c), NULL)
#define AK_CALL1(c, a) PyObject_CallFunctionObjArgs((c), (a), NULL)
#endif

/* README R5, counting build: what the SHIM does to CPython. The core counts its own
 * crossings through ak_enc_counters / ak_dec_counters. */
#ifdef AK_COUNT
static uint64_t CNT[4];
#define C_ATTR   0
#define C_READ   1
#define C_ITEM   2
#define C_PYCALL 3
#define BUMP(i) (CNT[i]++)
static struct AkCounters CORE_ENC, CORE_DEC;
#define CORE_ADD(dst, src) do { \
  (dst).forward += (src).forward; (dst).reverse += (src).reverse; \
  (dst).transcode += (src).transcode; (dst).prefix_moves += (src).prefix_moves; \
  (dst).prefix_bytes += (src).prefix_bytes; (dst).grows += (src).grows; } while (0)
#else
#define BUMP(i) ((void)0)
#endif

/* Resolved once at module init: ak_tc_utf8() is a call across the boundary. */
static ak_transcode_fn TC_UTF8, TC_BYTES;

/* ABI v1 sections 6 and 7.3: batched element runs chunk at 32 KB of host-side groups. A
 * run of strings chunks by count of `ak_str`; a packed run by count of values. Both are
 * overridable ONLY so a test build can force many chunks on small corpus vectors (the
 * chunking class of CONTRACT.md); the measured builds use the defaults. */
#ifndef AK_CHUNK_BYTES
#define AK_CHUNK_BYTES %(chunk)d
#endif
#ifndef AK_CHUNK_PACKED
#define AK_CHUNK_PACKED 4096
#endif
#define CHUNK_BLOB ((int32_t)(AK_CHUNK_BYTES / sizeof(struct ak_str)) > 0 ? \
                    (int32_t)(AK_CHUNK_BYTES / sizeof(struct ak_str)) : 1)
#define CHUNK_PACKED AK_CHUNK_PACKED

/* ---- ABI v1 section 3, rendered from plan.lifecycle (R-G7) ---------------------------
 * Called by native/binding.c's module exec before anything else. `AK_SKIP_INIT` is the
 * planted control ONLY: against a core built with `%(gate)s` every codec call must then
 * fail with AK_ERR_UNINITIALIZED, which is how a gate proves the guard is in the build. */
#ifndef AK_INIT_FLAGS
#define AK_INIT_FLAGS (%(flags)s)   /* plan.lifecycle.default_flags: what a codec binding passes */
#endif
static int ak_py_init(void) {
#ifdef AK_SKIP_INIT
  return 0;
#else
  struct ak_init_opts o;
  memset(&o, 0, sizeof o);
  o.abi_version = AK_ABI_VERSION;
  o.flags = AK_INIT_FLAGS;
  struct ak_err e = {0, 0};
  int32_t rc = %(init)s(&o, &e);
  if (rc != %(ok0)s && rc != %(ok1)s) {
    PyErr_Format(PyExc_ImportError, "%(init)s refused: %%d (detail %%u)", (int)rc, (unsigned)e.detail);
    return -1;
  }
  return 0;
#endif
}

/* ---- ABI v1 section 10: the host's own layout facts, compared with the core's at import.
 * The ONE enumeration (cpp_layout.facts, which the core's ak_layout_facts() is rendered
 * from), with the numbers this compiler produced for the header's structs. */
static const uint32_t AK_LAYOUT_HOST[AK_LAYOUT_FACTS] = {
%(layout_host)s
};
static const char *const AK_LAYOUT_NAMES[AK_LAYOUT_FACTS] = {
%(layout_names)s
};
static int ak_py_layout_check(void) {
  uint32_t core[AK_LAYOUT_FACTS];
  size_t n = ak_layout_facts(core, AK_LAYOUT_FACTS);
  if (n != AK_LAYOUT_FACTS) {
    PyErr_Format(PyExc_ImportError, "layout: the core exports %%zu facts, this shim pins %%d",
                 n, (int)AK_LAYOUT_FACTS);
    return -1;
  }
#ifdef AK_PLANT_LAYOUT_MISMATCH
  core[AK_PLANT_LAYOUT_MISMATCH] += 1;   /* the build's must-fail control ONLY */
#endif
  for (size_t i = 0; i < n; i++)
    if (core[i] != AK_LAYOUT_HOST[i]) {
      PyErr_Format(PyExc_ImportError, "layout: %%s is %%u in the core and %%u here",
                   AK_LAYOUT_NAMES[i], (unsigned)core[i], (unsigned)AK_LAYOUT_HOST[i]);
      return -1;
    }
  return 0;
}
'''


def emit(x, modname="_akffi", backends=("attr", "cext", "pyacc")):
    p = as_plan(x)
    lc = p.lifecycle
    names = real_messages(p)
    FIRST_TYPE[0] = names[0]
    maxlists = max([len(root_lists(p, r)) for r in p.roots] + [1])
    facts = cpp_layout.facts(p)
    host, fnames = [], []
    for sname, members in facts:
        host.append("  (uint32_t)sizeof(struct %s)," % sname)
        fnames.append("  \"sizeof %s\"," % sname)
        for mem in members:
            host.append("  (uint32_t)offsetof(struct %s, %s)," % (sname, mem))
            fnames.append("  \"%s.%s\"," % (sname, mem))
    L = [PRELUDE % {"source": p.source, "gate": lc.gate_feature, "init": lc.init[0],
                    "ok0": lc.success[0], "ok1": lc.success[1], "chunk": CHUNK_BYTES,
                    "flags": " | ".join(lc.default_flags),
                    "layout_host": "\n".join(host), "layout_names": "\n".join(fnames)}, ""]
    L.append(emit_ctypes(p, names, modname))
    groups = set(names) | {n for n in p.messages if p.msg(n).synthetic}
    for name in sorted(groups):
        L.append("#define %s ((int32_t)(AK_CHUNK_BYTES / sizeof(struct ak_efix_%s)) > 0 ? \\" % (chunk_macro(name), name))
        L.append("                  (int32_t)(AK_CHUNK_BYTES / sizeof(struct ak_efix_%s)) : 1)" % name)
    L.append("")
    keys = sorted({a.name for n in names for a in attrs(p.msg(n))})
    for k in keys:
        L.append("static PyObject *K_%s;" % k)
    L.append("")
    L.append("typedef struct {")
    for n in names:
        L.append("  PyObject *ty_%s;" % n)
    L.append("} HostTypes;")
    L.append("")
    L += ["typedef struct {",
          "  PyObject *root;      /* the facade root */",
          "  const uint8_t *base; /* decode: ABI v1 7.4, spans are offsets into this */",
          "  PyObject *acc;       /* pyacc backend: the accessor table */",
          "  PyObject *lists[%d];  /* decode: one staged run per root list slot */" % maxlists,
          "  PyObject *cur[1];    /* encode: the element list of the open non-leaf root slot */",
          "  int failed;",
          "  const uint8_t *direct;  /* ABI v1 section 8: the bulk field's bytes, */",
          "  size_t direct_len;      /* passed BESIDE the group rather than in it. */"]
    for n in names:
        L.append("  PyObject *ty_%s;" % n)
    L += ["} HostCtx;", ""]
    L.append("static int intern_keys(void) {")
    for k in keys:
        L.append('  K_%s = PyUnicode_InternFromString("%s"); if (!K_%s) return -1;' % (k, k, k))
    L.append("  TC_UTF8 = ak_tc_utf8(); TC_BYTES = ak_tc_bytes();")
    L.append("  return 0;\n}")
    L.append("")
    # Which messages need which functions.
    fill_needed, setgroup_needed = set(), set()
    elem_ctx = []                               # non-leaf element types with an element vtable

    def need_fill(n):
        if n in fill_needed:
            return
        fill_needed.add(n)
        m = p.msg(n)
        for f in m.fields:
            if f.card == "singular" and f.kind == "message":
                need_fill(f.of)

    def need_setgroup(n):
        if n in setgroup_needed:
            return
        setgroup_needed.add(n)
        for f in p.msg(n).fields:
            if f.card == "singular" and f.kind == "message":
                need_setgroup(f.of)

    for r in p.roots:
        need_fill(r)
        need_setgroup(r)
        for path, f in loop_slots(p, r):
            et = elem_type(f)
            if et and f.card != "map":
                need_fill(et)
                need_setgroup(et)
                if not p.msg(et).leaf and et not in elem_ctx:
                    elem_ctx.append(et)
                for ipath, iff in loop_slots(p, et):
                    iet = elem_type(iff)
                    if iet and iff.card != "map":
                        need_fill(iet)
                        need_setgroup(iet)
    for b in backends:
        EVT_NAMES[b] = {et: "EVT_%s_%s" % (b, et) for et in elem_ctx}
        L.append("/* ===================== backend: %s ===================== */" % b)
        for n in names:
            if n in fill_needed:
                L.append("static int fill_%s_%s(struct ak_efix_%s *, PyObject *, HostCtx *);" % (b, n, n))
            if n in setgroup_needed:
                L.append("static int setgroup_%s_%s(HostCtx *, PyObject *, const struct ak_dfix_%s *);" % (b, n, n))
        L.append("")
        for n in names:
            if n in fill_needed:
                L.append(emit_fill(p, n, b))
        for et in elem_ctx:
            for path, f in loop_slots(p, et):
                L.append(emit_loop(p, "E", et, path, f, b))
            L.append("static const struct ak_evt_%s %s = %s;" % (et, EVT_NAMES[b][et], evt_init(p, et, "E", b, EVT_NAMES[b])))
        for r in p.roots:
            for path, f in loop_slots(p, r):
                L.append(emit_loop(p, "R", r, path, f, b))
            L.append(emit_encode_entry(p, r, b))
        for n in names:
            if n in setgroup_needed:
                L.append(emit_setgroup(p, n, b))
        for r in p.roots:
            L.append(emit_root_decode(p, r, b))
        L.append("")
    L.append("/* ---- dispatch, so native/binding.c names no message and no field ---- */")
    L.append("#define AK_NTYPES %d" % len(names))
    L.append("static const char *AK_TYPE_NAMES[AK_NTYPES] = {%s};" % ", ".join('"C%s"' % n for n in names))
    L.append("static PyType_Spec *AK_TYPE_SPECS[AK_NTYPES] = {%s};" % ", ".join("&spec_%s" % n for n in names))
    L.append("")
    L += ["static int types_from_seq(HostTypes *T, PyObject *seq) {",
          "  if (!PySequence_Check(seq) || PySequence_Size(seq) != AK_NTYPES) {",
          "    PyErr_Format(PyExc_TypeError, \"expected %d types\", AK_NTYPES);",
          "    return -1;",
          "  }",
          "  PyObject **slot = &T->ty_%s;" % names[0],
          "  for (int i = 0; i < AK_NTYPES; i++) {",
          "    PyObject *t = PySequence_GetItem(seq, i);",
          "    if (!t) return -1;",
          "    slot[i] = t;   /* borrowed for the call; the caller holds the sequence */",
          "    Py_DECREF(t);",
          "  }",
          "  return 0;\n}",
          ""]
    L.append("typedef PyObject *(*ak_enc_f)(PyObject *, PyObject *);")
    L.append("typedef PyObject *(*ak_dec_f)(PyObject *, PyObject *, HostTypes *);")
    L.append("#define AK_NBACKENDS %d" % len(backends))
    L.append("static const char *AK_BACKENDS[AK_NBACKENDS] = {%s};" % ", ".join('"%s"' % b for b in backends))
    L.append("#define AK_NROOTS %d" % len(p.roots))
    L.append("static const char *AK_ROOTS[AK_NROOTS] = {%s};" % ", ".join('"%s"' % r for r in p.roots))
    L += ["static int root_index(const char *name) {",
          "  for (int i = 0; i < AK_NROOTS; i++)",
          "    if (strcmp(AK_ROOTS[i], name) == 0) return i;",
          "  PyErr_Format(PyExc_ValueError, \"unknown root %s\", name);",
          "  return -1;\n}"]
    L.append("static ak_enc_f AK_ENC[AK_NBACKENDS][AK_NROOTS] = {")
    for b in backends:
        L.append("  {%s}," % ", ".join("encode_%s_%s" % (b, r) for r in p.roots))
    L.append("};")
    L.append("static ak_dec_f AK_DEC[AK_NBACKENDS][AK_NROOTS] = {")
    for b in backends:
        L.append("  {%s}," % ", ".join("decode_%s_%s" % (b, r) for r in p.roots))
    L.append("};")
    L.append("")
    return "\n".join(L)
