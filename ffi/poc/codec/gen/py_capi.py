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
compiles against is the C++ backend's `c_abi.emit` (plain C99, rendered from the same
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

Unknown fields (decision 11, WP5 step 9): the full plan renders drop AND retain per call
(`retain` and the `zero` mask); rendered from `relower(p, p.options.with_unknown("drop"))`
(WP5 step 10, THE NO-UNKNOWN VARIANT) the shim has NO retain path: no `ak_ufix`, no
`fillu_`/`loopu_`, no options, no `ak_dec_reset_<Root>`, no `ak_uencode_*`/`ak_uelem*`, no
grow/reclaim, no `unknown` member read at delivery, and `ak_dec_ctx_new_<Root>(void)`. A
call with `retain` set is refused. Which members exist comes from the plan
(`unknown_compiled_out`); this backend never re-derives it.

A backend: imports `plan` only.
"""
from plan import (abi_order_topo, as_plan, direct_fields, elem_type, loop_slots, presence_bits,
                  slot_name, unk_opts_layout, unk_opts_name, unk_positions, unknown_compiled_out)
import cpp_layout

CHUNK_BYTES = 32768   # ABI v1 sections 6 and 7.3: batched runs chunk at 32 KB.
NOUNK = [False]       # set per emit(): the plan's unknown fields are compiled out

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
    if not NOUNK[0]:   # the no-unknown variant's C facade type has no `_unknown` member
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


def chunk_macro(name, u=False):
    return "CHUNK%s_%s" % ("U" if u else "", name.upper())


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
        if ff:
            L.append('  if (!PyArg_ParseTupleAndKeywords(a, kw, "|%s", kwl, %s)) return -1;'
                     % ("".join(parse_fmt(a) for a in ff), ", ".join("&v_" + a.name for a in ff)))
        else:   # a message with no attribute at all (the no-unknown variant's empty message)
            L.append('  (void)o; if (!PyArg_ParseTupleAndKeywords(a, kw, "", kwl)) return -1;')
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
        if not objs:
            L.append("  (void)o; (void)visit; (void)arg;")
        for a in objs:
            L.append("  Py_VISIT(o->%s);" % a.name)
        L.append("  return 0;\n}")
        L.append("static int clear_%s(PyObject *s) {" % name)
        L.append("  C%s *o = (C%s *)s;" % (name, name))
        if not objs:
            L.append("  (void)o;")
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

def emit_fill(p, name, b, u=False):
    """`fill_<b>_<M>`: the group of M from a facade object, into zeroed memory (ABI v1
    decision 9's sparse fill: the memset is the group's default). Loop-slot fields cross
    through the vtable. A singular child's group is inlined, and filled by the same
    function (ABI v1 section 6).

    `u`: `fillu_<b>_<M>` fills the RETAIN group `ak_ufix_M` (decision 11: every group,
    inlined children included, carries its own `unknown: ak_blob`, taken from the facade's
    `_unknown` bytes and written by the core verbatim after the known fields)."""
    m = p.msg(name)
    bits = presence_bits(m)
    fn = "fillu" if u else "fill"
    L = ["static int %s_%s_%s(struct ak_%sfix_%s *e, PyObject *ob, HostCtx *h) {" % (fn, b, name, "u" if u else "e", name),
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
            L.append("      if (%s_%s_%s(&e->%s, v, h)) { %s return -1; }" % (fn, b, f.of, f.name, dec_v))
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
                L.append("      if (%s_%s_%s(&e->%s, mv, h)) { %s return -1; }" % (fn, b, g.of, gn, dec_mv))
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
    if u:
        ua = Attr(name, "_unknown", "bytes")
        L.append("  { /* decision 11: this message's retained runs, written after its known fields */")
        L.append(read_obj(b, name, "_unknown", "v", "ob", "    "))
        L.append("    BUMP(C_READ);")
        L.append("    char *up = NULL; Py_ssize_t ul = 0;")
        L.append("    if (v != Py_None && PyBytes_AsStringAndSize(v, &up, &ul)) { if (own_v) Py_DECREF(v); return -1; }")
        L.append("    if (ul) { e->unknown.data = up; e->unknown.len = (size_t)ul; }")
        L.append("    if (own_v) Py_DECREF(v);")
        L.append("  }")
        del ua
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


def emit_loop(p, ctxkind, owner, path, f, b, u=False):
    """One encode loop callback, for slot `path` of vtable owner `owner`.

    ctxkind "R": `owner` is a root and the holder is `h->root`.
    ctxkind "E": `owner` is a non-leaf element type and the holder is element `token` of
    the element list the enclosing root loop left in `h->cur[0]`."""
    sn = slot_name(path)
    lp = "loopu" if u else "loop"
    g = "u" if u else "e"
    L = ["static int32_t %s_%s_%s_%s(ak_enc_ctx *ctx, const void *obj, int64_t token) {"
         % (lp, b, ctx_name(ctxkind, owner), sn),
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
              "  struct ak_%sfix_%s chunk[%s];" % (g, f.entry, chunk_macro(f.entry, u)),
              "  for (Py_ssize_t i = 0; i < n; i += %s) {" % chunk_macro(f.entry, u),
              "    int32_t k = (int32_t)((n - i < %s) ? (n - i) : %s);" % (chunk_macro(f.entry, u), chunk_macro(f.entry, u)),
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
              "    if (ak_%selem_%s(ctx, chunk, k)) { Py_DECREF(keys); rc = -1; goto out; }" % ("u" if u else "", f.entry),
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
    L += ["  struct ak_%sfix_%s chunk[%s];" % (g, et, chunk_macro(et, u)),
          "  int64_t done = 0;"]
    if not leaf:
        # The element list, so the element's own loop slots can index it by token without
        # re-reading the attribute. Only a root loop sets it (non-leaf in non-leaf is refused).
        L.append("  h->cur[0] = lst;")
    L += ["  for (Py_ssize_t i = 0; i < n; i += %s) {" % chunk_macro(et, u),
          "    int32_t k = (int32_t)((n - i < %s) ? (n - i) : %s);" % (chunk_macro(et, u), chunk_macro(et, u)),
          "    /* ABI v1 decision 9: bulk clear once, then assign only what differs. */",
          "    memset(chunk, 0, sizeof(chunk[0]) * (size_t)k);",
          "    for (int32_t j = 0; j < k; j++) {",
          "      BUMP(C_ITEM);",
          "      PyObject *e = PyList_GetItem(lst, i + j);",
          "      if (!e || %s_%s_%s(&chunk[j], e, h)) { rc = -1; goto out; }" % ("fillu" if u else "fill", b, et),
          "    }"]
    if leaf:
        L.append("    if (ak_%selem_%s(ctx, chunk, k)) { rc = -1; goto out; }" % ("u" if u else "", et))
    else:
        L.append("    /* NOT a leaf: a token range, and each of the element's own containers is a")
        L.append("       reverse call per element (ABI v1 section 6). */")
        L.append("    if (ak_%selemu_%s(ctx, chunk, k, done)) { rc = -1; goto out; }" % ("u" if u else "", et))
    L += ["    done += k;",
          "  }"]
    if not leaf:
        tail = tail[:1] + ["  h->cur[0] = saved;"] + tail[1:]
    tail = tail[:1] + ["  (void)done;"] + tail[1:]
    return "\n".join(L + tail)


def has_groups(f):
    """A loop slot whose run carries groups (a map's entries, a repeated message): its retain
    variant differs (ufix groups, `ak_uelem*`); a blob or packed run is the same in both."""
    return f.card == "map" or (f.card == "repeated" and f.kind == "message")


def evt_init(p, owner, ctxkind, b, evt_names, u=False):
    """The designated initialiser of `struct ak_evt_<owner>` (`u`: the retain family's)."""
    items = []
    for path, f in loop_slots(p, owner):
        sn = slot_name(path)
        lp = "loopu" if (u and has_groups(f)) else "loop"
        items.append(".loop_%s = %s_%s_%s_%s" % (sn, lp, b, ctx_name(ctxkind, owner), sn))
        et = elem_type(f)
        if et and loop_slots(p, et):
            items.append(".elem_%s = &%s" % (sn, evt_names[et]))
    return "{%s}" % (", ".join(items) if items else "0")


def emit_encode_entry(p, root, b):
    """`retain` selects the family: `ak_encode_R` over `ak_efix` groups (unknown fields
    dropped) or `ak_uencode_R` over `ak_ufix` groups (each message's `_unknown` re-emitted)."""
    d = ", h->direct, h->direct_len" if direct_fields(p, root) else ""
    if NOUNK[0]:
        return _emit_encode_entry_nounk(p, root, b, d)
    L = ["static PyObject *encode_%s_%s(PyObject *rootobj, PyObject *acc, int retain) {" % (b, root),
         "  HostCtx hs; memset(&hs, 0, sizeof hs);",
         "  hs.root = rootobj; hs.acc = acc;",
         "  HostCtx *h = &hs;",
         "  static const struct ak_evt_%s VT = %s;" % (root, evt_init(p, root, "R", b, EVT_NAMES[b])),
         "  static const struct ak_evt_%s VTU = %s;" % (root, evt_init(p, root, "R", b, EVTU_NAMES[b], True)),
         # One encode context per thread, reused (ak_py_tls_enc_acquire), reset per encode.
         "  int tmp_ = 0;",
         "  ak_enc_ctx *ctx = ak_py_tls_enc_acquire(&tmp_);",
         "  if (!ctx) return PyErr_NoMemory();",
         "#ifdef AK_COUNT",
         "  ak_enc_counters_reset(ctx);",
         "#endif",
         "  intptr_t rc;",
         "  if (retain) {",
         "    struct ak_ufix_%s fixu;" % root,
         "    memset(&fixu, 0, sizeof fixu);",
         "    if (fillu_%s_%s(&fixu, rootobj, h)) { ak_py_tls_enc_release(ctx, tmp_); return NULL; }" % (b, root),
         "    rc = ak_uencode_%s(h, ctx, &VTU, &fixu%s);" % (root, d),
         "  } else {",
         "    struct ak_efix_%s fix;" % root,
         "    memset(&fix, 0, sizeof fix);",
         "    if (fill_%s_%s(&fix, rootobj, h)) { ak_py_tls_enc_release(ctx, tmp_); return NULL; }" % (b, root),
         "    rc = ak_encode_%s(h, ctx, &VT, &fix%s);" % (root, d),
         "  }",
         "  if (rc < 0) {",
         "    ak_py_tls_enc_release(ctx, tmp_);",
         "    if (!PyErr_Occurred()) PyErr_Format(PyExc_RuntimeError, \"ak_encode_%s returned %%ld\", (long)rc);" % root,
         "    return NULL;",
         "  }",
         "#ifdef AK_COUNT",
         "  { struct AkCounters c; ak_enc_counters(ctx, &c); CORE_ADD(CORE_ENC, c); }",
         "#endif",
         "  const uint8_t *pp = NULL; size_t len = 0;",
         "  if (ak_enc_take(ctx, &pp, &len)) { ak_py_tls_enc_release(ctx, tmp_);",
         "    PyErr_SetString(PyExc_RuntimeError, \"ak_enc_take\"); return NULL; }",
         "  PyObject *out = PyBytes_FromStringAndSize((const char *)pp, (Py_ssize_t)len);",
         "  ak_py_tls_enc_release(ctx, tmp_);",
         "  return out;\n}"]
    return "\n".join(L)


def _emit_encode_entry_nounk(p, root, b, d):
    """The no-unknown variant's encode: `ak_encode_R` only; `retain` is refused."""
    L = ["static PyObject *encode_%s_%s(PyObject *rootobj, PyObject *acc, int retain) {" % (b, root),
         "  if (retain) { PyErr_SetString(PyExc_ValueError, \"unknown fields are compiled out of this build\"); return NULL; }",
         "  HostCtx hs; memset(&hs, 0, sizeof hs);",
         "  hs.root = rootobj; hs.acc = acc;",
         "  HostCtx *h = &hs;",
         "  static const struct ak_evt_%s VT = %s;" % (root, evt_init(p, root, "R", b, EVT_NAMES[b])),
         "  int tmp_ = 0;",
         "  ak_enc_ctx *ctx = ak_py_tls_enc_acquire(&tmp_);",
         "  if (!ctx) return PyErr_NoMemory();",
         "#ifdef AK_COUNT",
         "  ak_enc_counters_reset(ctx);",
         "#endif",
         "  struct ak_efix_%s fix;" % root,
         "  memset(&fix, 0, sizeof fix);",
         "  if (fill_%s_%s(&fix, rootobj, h)) { ak_py_tls_enc_release(ctx, tmp_); return NULL; }" % (b, root),
         "  intptr_t rc = ak_encode_%s(h, ctx, &VT, &fix%s);" % (root, d),
         "  if (rc < 0) {",
         "    ak_py_tls_enc_release(ctx, tmp_);",
         "    if (!PyErr_Occurred()) PyErr_Format(PyExc_RuntimeError, \"ak_encode_%s returned %%ld\", (long)rc);" % root,
         "    return NULL;",
         "  }",
         "#ifdef AK_COUNT",
         "  { struct AkCounters c; ak_enc_counters(ctx, &c); CORE_ADD(CORE_ENC, c); }",
         "#endif",
         "  const uint8_t *pp = NULL; size_t len = 0;",
         "  if (ak_enc_take(ctx, &pp, &len)) { ak_py_tls_enc_release(ctx, tmp_);",
         "    PyErr_SetString(PyExc_RuntimeError, \"ak_enc_take\"); return NULL; }",
         "  PyObject *out = PyBytes_FromStringAndSize((const char *)pp, (Py_ssize_t)len);",
         "  ak_py_tls_enc_release(ctx, tmp_);",
         "  return out;\n}"]
    return "\n".join(L)


EVT_NAMES = {}
EVTU_NAMES = {}


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
            if NOUNK[0]:
                L.append("    }")
            else:
                L.append("    } else {")
                L.append("      dropgroup_%s(h, &e->%s);   /* decision 11: an absent child's slots are the host's too */" % (f.of, f.name))
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
            # Decision 11 rule 4: the oneof's one buffer sits in the ACTIVE message member's
            # slot; every other member slot that is non-NULL (an emptied buffer left behind by
            # a switch) is delivered with the group and freed here.
            for g2 in members:
                if g2.kind == "message" and g2 is not g and not NOUNK[0]:
                    L.append("      dropgroup_%s(h, &e->%s_%s);" % (g2.of, oname, g2.name))
            L.append("      break; }")
        L.append("    default:")
        for g2 in members:
            if g2.kind == "message" and not NOUNK[0]:
                L.append("      dropgroup_%s(h, &e->%s_%s);" % (g2.of, oname, g2.name))
        L.append("      break;")
        L.append("    }")
        # The discriminant LAST, so a watched facade never shows a case pointing at a member
        # that has not been written yet.
        L.append(write_field(b, name, ca, "e->%s_case" % oname, "    "))
        L.append("  }")
    # Decision 11: this occurrence's own buffer passes to the host with the group; it becomes
    # the facade's `_unknown` and is freed. NULL in drop mode, so drop pays one test.
    if NOUNK[0]:
        L.append("  return 0;\n}")
        return "\n".join(L)
    ua = Attr(name, "_unknown", "bytes")
    L.append("  if (e->unknown.data) {")
    L.append("    PyObject *ub_ = PyBytes_FromStringAndSize((const char *)e->unknown.data, (Py_ssize_t)e->unknown.len);")
    L.append("    ak_py_release(h, e->unknown.data);")
    L.append(write_field(b, name, ua, "ub_", "    "))
    L.append("  }")
    L.append("  return 0;\n}")
    return "\n".join(L)


def emit_dropgroup(p, name):
    """Free every non-NULL unknown-field slot of a delivered group that has no facade object
    to go to (an absent child, an inactive oneof member), depth first. Backend-independent."""
    m = p.msg(name)
    L = ["static inline void dropgroup_%s(HostCtx *h, const struct ak_dfix_%s *e) {" % (name, name),
         "  if (e->unknown.data) ak_py_release(h, e->unknown.data);"]
    for f in m.plain:
        if f.card == "singular" and f.kind == "message":
            L.append("  dropgroup_%s(h, &e->%s);" % (f.of, f.name))
    for oname, members in m.oneofs.items():
        for g in members:
            if g.kind == "message":
                L.append("  dropgroup_%s(h, &e->%s_%s);" % (g.of, oname, g.name))
    L.append("}")
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
        # Decision 11: a map entry is a message position, but the facade's dict has no bag for
        # it, so its buffer is freed at delivery (the U-map-entry retention gap, disputed).
        if not NOUNK[0]:
            L.append(p_ + "for (int32_t i = 0; i < n; i++) if (elems[i].unknown.data) ak_py_release(h, elems[i].unknown.data);")
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
    if NOUNK[0]:
        return "\n".join(L + _decode_body_nounk(p, root, b, lists, free, vt, R))
    layout = unk_opts_layout(p, root)
    if len(layout) > 64:
        raise Unsupported("%s has %d unknown-field positions; the zero mask holds 64" % (root, len(layout)))
    arm = ["    if (!(zero & (1ULL << %d))) o.%s.grow = ak_py_grow;" % (i, n) for i, (n, _m, _t) in enumerate(layout)]
    L += ["static PyObject *decode_%s_%s(PyObject *buf, PyObject *acc, HostTypes *T, int retain,"
          " unsigned long long zero) {" % (b, root),
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
          # Decision 11: the options live in THIS frame, unmoved, from the reset that arms them
          # to the reset that disarms them; every armed position grows from ak_py_grow (no
          # pre-allocated buffers). `zero` leaves chosen positions all-zero (discard there):
          # the harness's zeroed-position control.
          "  struct %s o;" % unk_opts_name(root),
          "  memset(&o, 0, sizeof o);",
          "  o.host = &h;",
          "  if (retain) {"] + arm + ["  }",
          # One context per root PER THREAD, created lazily and reused (ak_py_tls_acquire);
          # a reset per decode (rule 7) arms it, and a second reset disarms it only when it
          # was armed with &o, which lives in this frame. Drop mode: one reset, no disarm.
          "  int tmp_ = 0;",
          "  ak_dec_ctx *ctx = ak_py_tls_acquire(%d, &tmp_);   /* bound to this root (rule 6) */" % p.roots.index(root),
          "  if (!ctx) { %s Py_DECREF(rootobj); return PyErr_NoMemory(); }" % free,
          "#ifdef AK_COUNT",
          "  ak_dec_counters_reset(ctx);",
          "#endif",
          "  int32_t rc = ak_dec_reset_%s(ctx, retain ? &o : NULL);   /* a reset per decode (rule 7) */" % root,
          "  if (rc == 0) rc = ak_decode_%s(ctx, &h, (const uint8_t *)pp, (size_t)blen, &VT);" % root,
          "  if (retain) {",
          "    int32_t rr = ak_dec_reset_%s(ctx, NULL);   /* disarm: the core forgets &o */" % root,
          "    if (rc == 0 && rr != 0) rc = rr;",
          "  }",
          "#ifdef AK_COUNT",
          "  { struct AkCounters c; ak_dec_counters(ctx, &c); CORE_ADD(CORE_DEC, c); }",
          "#endif",
          "  ak_py_tls_release(%d, ctx, tmp_);" % p.roots.index(root),
          "  AK_LAST_RECLAIMED = ak_py_reclaim(&h);   /* undelivered buffers: a failed decode's */",
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


def _decode_body_nounk(p, root, b, lists, free, vt, R):
    """The no-unknown variant's decode: the thread's context for the root, the decode. No
    options, no reset (the core clears the context's error slot at every decode entry), no
    reclaim (nothing is ever grown). `retain` is refused; `zero` is accepted and unused."""
    L = ["static PyObject *decode_%s_%s(PyObject *buf, PyObject *acc, HostTypes *T, int retain,"
         " unsigned long long zero) {" % (b, root),
         "  (void)zero;",
         "  if (retain) { PyErr_SetString(PyExc_ValueError, \"unknown fields are compiled out of this build\"); return NULL; }",
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
          "  int tmp_ = 0;",
          "  ak_dec_ctx *ctx = ak_py_tls_acquire(%d, &tmp_);   /* bound to this root (rule 6) */" % p.roots.index(root),
          "  if (!ctx) { %s Py_DECREF(rootobj); return PyErr_NoMemory(); }" % free,
          "#ifdef AK_COUNT",
          "  ak_dec_counters_reset(ctx);",
          "#endif",
          "  int32_t rc = ak_decode_%s(ctx, &h, (const uint8_t *)pp, (size_t)blen, &VT);" % root,
          "#ifdef AK_COUNT",
          "  { struct AkCounters c; ak_dec_counters(ctx, &c); CORE_ADD(CORE_DEC, c); }",
          "#endif",
          "  ak_py_tls_release(%d, ctx, tmp_);" % p.roots.index(root),
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
    return L


FIRST_TYPE = [None]

UNK_HELPERS = r'''/* ---- decision 11 (WP5 step 9): the host side of the unknown-field buffers --------------
 * Every buffer the core fills comes from ak_py_grow (no pre-allocated buffers): a header in
 * front of the bytes links it into the HostCtx's live list, so a buffer the host never gets
 * back as a delivered slot (a failed decode, rule 3) is still freed, by ak_py_reclaim at the
 * end of the decode. A delivered slot is released at delivery (ak_py_release). */
struct ak_py_buf { struct ak_py_buf *prev, *next; };
/* Per THREAD: the count belongs to the decode that stored it. The GIL can pass to another
 * thread between a decode's store and its caller's read (a facade attribute store at
 * delivery may run Python code), so a process-wide slot could be overwritten by another
 * thread's decode in between. A build may predefine AK_THREAD_LOCAL empty: the harness's
 * must-fail control that shows the per-thread check can see a process-wide slot. */
#ifndef AK_THREAD_LOCAL
#if defined(__STDC_VERSION__) && __STDC_VERSION__ >= 201112L
#define AK_THREAD_LOCAL _Thread_local
#elif defined(__GNUC__)
#define AK_THREAD_LOCAL __thread
#else
#error "no thread-local storage class for AK_LAST_RECLAIMED"
#endif
#endif
static AK_THREAD_LOCAL unsigned long AK_LAST_RECLAIMED;

static void ak_py_link(HostCtx *h, struct ak_py_buf *b) {
  b->prev = NULL; b->next = h->live;
  if (h->live) h->live->prev = b;
  h->live = b;
}

static void ak_py_unlink(HostCtx *h, struct ak_py_buf *b) {
  if (b->prev) b->prev->next = b->next; else h->live = b->next;
  if (b->next) b->next->prev = b->prev;
}

/* ak_grow_fn: NULL/0 = a fresh buffer; otherwise realloc keeping the first *cap bytes. The
 * capacity doubles, so a message with many runs grows O(log n) times. */
static int32_t ak_py_grow(void *sink, int32_t want, uint8_t **dst, int32_t *cap) {
  HostCtx *h = (HostCtx *)sink;
  if (want <= 0) return AK_ERR_LIMIT;
  int64_t n = *dst ? 2 * (int64_t)*cap : 64;
  if (n < want) n = want;
  if (n > INT32_MAX) n = want;
  struct ak_py_buf *old = *dst ? ((struct ak_py_buf *)(void *)*dst) - 1 : NULL;
  if (old) ak_py_unlink(h, old);
  struct ak_py_buf *b = (struct ak_py_buf *)realloc(old, sizeof *b + (size_t)n);
  if (!b) { if (old) ak_py_link(h, old); return AK_ERR_CAPACITY; }
  ak_py_link(h, b);
  *dst = (uint8_t *)(b + 1);
  *cap = (int32_t)n;
  return AK_OK;
}

static void ak_py_release(HostCtx *h, void *data) {
  struct ak_py_buf *b = ((struct ak_py_buf *)data) - 1;
  ak_py_unlink(h, b);
  free(b);
}

static unsigned long ak_py_reclaim(HostCtx *h) {
  unsigned long n = 0;
  while (h->live) { struct ak_py_buf *b = h->live; h->live = b->next; free(b); n++; }
  return n;
}
'''


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
    NOUNK[0] = unknown_compiled_out(p)
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
        for u in ((False,) if NOUNK[0] else (False, True)):
            g = "u" if u else "e"
            L.append("#define %s ((int32_t)(AK_CHUNK_BYTES / sizeof(struct ak_%sfix_%s)) > 0 ? \\" % (chunk_macro(name, u), g, name))
            L.append("                  (int32_t)(AK_CHUNK_BYTES / sizeof(struct ak_%sfix_%s)) : 1)" % (g, name))
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
    if not NOUNK[0]:
        L.append("  struct ak_py_buf *live; /* decision 11: every unknown-field buffer ak_py_grow handed out */")
    for n in names:
        L.append("  PyObject *ty_%s;" % n)
    L += ["} HostCtx;", "", "" if NOUNK[0] else UNK_HELPERS, "",
          "static ak_dec_ctx *ak_py_tls_acquire(int root, int *tmp);",
          "static void ak_py_tls_release(int root, ak_dec_ctx *c, int tmp);",
          "static ak_enc_ctx *ak_py_tls_enc_acquire(int *tmp);",
          "static void ak_py_tls_enc_release(ak_enc_ctx *c, int tmp);", ""]
    # Decision 11: the backend-independent release of a delivered group's slots that have no
    # facade object (an absent child, an inactive oneof member).
    for n in abi_order_topo(p):
        if not p.msg(n).synthetic and not NOUNK[0]:
            L.append(emit_dropgroup(p, n))
    L.append("")
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
        EVTU_NAMES[b] = {et: "EVTU_%s_%s" % (b, et) for et in elem_ctx}
        L.append("/* ===================== backend: %s ===================== */" % b)
        for n in names:
            if n in fill_needed:
                L.append("static int fill_%s_%s(struct ak_efix_%s *, PyObject *, HostCtx *);" % (b, n, n))
                if not NOUNK[0]:
                    L.append("static int fillu_%s_%s(struct ak_ufix_%s *, PyObject *, HostCtx *);" % (b, n, n))
            if n in setgroup_needed:
                L.append("static int setgroup_%s_%s(HostCtx *, PyObject *, const struct ak_dfix_%s *);" % (b, n, n))
        L.append("")
        for n in names:
            if n in fill_needed:
                L.append(emit_fill(p, n, b))
                if not NOUNK[0]:
                    L.append(emit_fill(p, n, b, u=True))
        for et in elem_ctx:
            for path, f in loop_slots(p, et):
                L.append(emit_loop(p, "E", et, path, f, b))
                if has_groups(f) and not NOUNK[0]:
                    L.append(emit_loop(p, "E", et, path, f, b, u=True))
            L.append("static const struct ak_evt_%s %s = %s;" % (et, EVT_NAMES[b][et], evt_init(p, et, "E", b, EVT_NAMES[b])))
            if not NOUNK[0]:
                L.append("static const struct ak_evt_%s %s = %s;" % (et, EVTU_NAMES[b][et], evt_init(p, et, "E", b, EVTU_NAMES[b], True)))
        for r in p.roots:
            for path, f in loop_slots(p, r):
                L.append(emit_loop(p, "R", r, path, f, b))
                if has_groups(f) and not NOUNK[0]:
                    L.append(emit_loop(p, "R", r, path, f, b, u=True))
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
    L.append("typedef PyObject *(*ak_enc_f)(PyObject *, PyObject *, int);")
    L.append("typedef PyObject *(*ak_dec_f)(PyObject *, PyObject *, HostTypes *, int, unsigned long long);")
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
    L += emit_unk_tables(p)
    return "\n".join(L)


def emit_unk_tables(p):
    """Decision 11 facts the harness needs, and the wrong-root control.

    AK_UNKPOS[root]: one "name|kind|path" string per position, in `unk_opts_layout` order
    (the bit of the decode's `zero` mask), kind "msg" or "oneof", path the dotted field path
    from the root ("" for the root itself); a oneof's path ends at the oneof's name.

    ak_py_wrong_root(a, b): a context bound to root `a`, then `ak_dec_reset_<b>` and
    `ak_decode_<b>` on it. Rule 6 says both are refused (AK_ERR_INVALID_STATE) and nothing
    is delivered; returns the two codes and whether the trap `apply` ran."""
    L = ["/* ---- decision 11: unknown-field positions, and the wrong-root control ---- */"]
    if NOUNK[0]:
        L.append("#define AK_NOUNK_SHIM 1   /* the no-unknown variant: no positions, no reset */")
    for r in p.roots:
        items = []
        for (n, m, _t), (path, _mm) in ([] if NOUNK[0] else zip(unk_opts_layout(p, r), unk_positions(p, r))):
            items.append('"%s|%s|%s"' % (n, "oneof" if m == "oneof" else "msg", ".".join(path)))
        L.append("static const char *AK_UNKPOS_%s[] = {%s};" % (r, ", ".join(items + ["NULL"])))
    L.append("static const char *const *AK_UNKPOS[AK_NROOTS] = {%s};" % ", ".join("AK_UNKPOS_%s" % r for r in p.roots))
    L.append("static int AK_TRAP_DELIVERED;")
    for r in p.roots:
        L.append("static void trap_apply_%s(ak_dec_ctx *c, void *o, const struct ak_dfix_%s *f) {" % (r, r))
        L.append("  (void)c; (void)o; (void)f; AK_TRAP_DELIVERED = 1;\n}")
    L.append("static ak_dec_ctx *ak_py_ctx_new(int a) {")
    L.append("  switch (a) {")
    for i, r in enumerate(p.roots):
        L.append("  case %d: return ak_dec_ctx_new_%s(%s);" % (i, r, "" if NOUNK[0] else "NULL"))
    L.append("  }")
    L.append("  return NULL;\n}")
    L.append("static void ak_py_wrong_root(int a, int b, int32_t *reset_rc, int32_t *decode_rc) {")
    L.append("  ak_dec_ctx *ctx = ak_py_ctx_new(a);")
    L.append("  AK_TRAP_DELIVERED = 0;")
    L.append("  *reset_rc = *decode_rc = 0;")
    L.append("  if (!ctx) return;")
    L.append("  static const uint8_t empty[1] = {0};")
    L.append("  switch (b) {")
    for i, r in enumerate(p.roots):
        L.append("  case %d: {" % i)
        L.append("    static const struct ak_dvt_%s VT = {.apply = trap_apply_%s};" % (r, r))
        if not NOUNK[0]:
            L.append("    *reset_rc = ak_dec_reset_%s(ctx, NULL);" % r)
        L.append("    *decode_rc = ak_decode_%s(ctx, NULL, empty, 0, &VT);" % r)
        L.append("    break; }")
    L.append("  }")
    L.append("  ak_dec_ctx_free(ctx);\n}")
    L.append("")
    L.append(TLS_HELPERS)
    return L


# One decode context per root, and one encode context, PER THREAD. The GIL can pass to another thread inside a
# decode (a facade __init__ or a pyacc accessor is Python code), so a process-wide context
# could be reset under a decode in flight; a per-call context would add an allocation and a
# crossing to every decode. A pthread key holds each thread's block, created on first use and
# freed by the key's destructor at thread exit; ak_py_tls_fini (module free) frees the
# calling thread's block and deletes the key. A decode re-entered on the same thread for
# the same root (a callback that decodes) finds the slot busy and gets a temporary context.
TLS_HELPERS = r"""
#include <pthread.h>
#include <stdlib.h>
struct ak_py_tls {
  ak_dec_ctx *c[AK_NROOTS]; unsigned char busy[AK_NROOTS];
  ak_enc_ctx *e; unsigned char ebusy;
};
static pthread_key_t AK_TLS_KEY;
static int AK_TLS_OK;
static unsigned long AK_TLS_CREATED;   /* contexts created through the key, for the harness */
static void ak_py_tls_free(void *p) {
  struct ak_py_tls *t = (struct ak_py_tls *)p;
  if (!t) return;
  for (int i = 0; i < AK_NROOTS; i++) if (t->c[i]) ak_dec_ctx_free(t->c[i]);
  if (t->e) ak_enc_ctx_free(t->e);
  free(t);
}
static int ak_py_tls_init(void) {
  if (AK_TLS_OK) return 0;
  if (pthread_key_create(&AK_TLS_KEY, ak_py_tls_free)) return -1;
  AK_TLS_OK = 1;
  return 0;
}
static void ak_py_tls_fini(void) {
  if (!AK_TLS_OK) return;
  ak_py_tls_free(pthread_getspecific(AK_TLS_KEY));
  pthread_setspecific(AK_TLS_KEY, NULL);
  pthread_key_delete(AK_TLS_KEY);
  AK_TLS_OK = 0;
}
static struct ak_py_tls *ak_py_tls_block(void) {
  struct ak_py_tls *t = AK_TLS_OK ? (struct ak_py_tls *)pthread_getspecific(AK_TLS_KEY) : NULL;
  if (!t && AK_TLS_OK) {
    t = (struct ak_py_tls *)calloc(1, sizeof *t);
    if (t && pthread_setspecific(AK_TLS_KEY, t)) { free(t); t = NULL; }
  }
  return t;
}
static ak_dec_ctx *ak_py_tls_acquire(int root, int *tmp) {
  *tmp = 0;
  struct ak_py_tls *t = ak_py_tls_block();
  if (!t || t->busy[root]) { *tmp = 1; return ak_py_ctx_new(root); }
  if (!t->c[root]) { t->c[root] = ak_py_ctx_new(root); if (!t->c[root]) return NULL; AK_TLS_CREATED++; }
  t->busy[root] = 1;
  return t->c[root];
}
static void ak_py_tls_release(int root, ak_dec_ctx *c, int tmp) {
  if (tmp) { ak_dec_ctx_free(c); return; }
  struct ak_py_tls *t = (struct ak_py_tls *)pthread_getspecific(AK_TLS_KEY);
  if (t) t->busy[root] = 0;
}
/* The encode context: unbound, one per thread; reset per encode (the output buffer and the
 * sticky error slot start clean). */
static ak_enc_ctx *ak_py_tls_enc_acquire(int *tmp) {
  *tmp = 0;
  struct ak_py_tls *t = ak_py_tls_block();
  if (!t || t->ebusy) { *tmp = 1; return ak_enc_ctx_new(); }
  if (!t->e) { t->e = ak_enc_ctx_new(); if (!t->e) return NULL; AK_TLS_CREATED++; }
  else ak_enc_reset(t->e);
  t->ebusy = 1;
  return t->e;
}
static void ak_py_tls_enc_release(ak_enc_ctx *c, int tmp) {
  if (tmp) { ak_enc_ctx_free(c); return; }
  struct ak_py_tls *t = (struct ak_py_tls *)pthread_getspecific(AK_TLS_KEY);
  if (t) t->ebusy = 0;
}
"""
