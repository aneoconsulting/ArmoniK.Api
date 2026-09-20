"""The generated C shim: the python slice's binding to the ONE core at `poc/codec/`.

README 9.1 gives Python three layers where the others have two -- the Rust core, a
generated C shim speaking the CPython API, and the Python facade -- and this emits the
middle one.

**R0**: nothing here is a copy of the core.  The shim calls `ak_encode_*`, `ak_elem_*`,
`ak_elemu_*`, `ak_blob_run` and `ak_decode_*` in `libak_core.so`, built from `poc/codec/`,
and `poc/codec/gen/one_core.sh` is what checks that.

Three accessor backends over ONE emitted traversal shape, the same three since work unit
1 so every table is comparable:

| backend | how a field is reached |
|---|---|
| `attr`  | `PyObject_GetAttr` / `PyObject_SetAttr`.  Serves BOTH the plain and the `__slots__` facade: the code is identical and only the object differs |
| `cext`  | a struct member on the generated C facade type: not a crossing at all |
| `pyacc` | **the premise control** (README 9.1): a Python-level accessor CALL per field |

**ABI v1 decision 9's sparse fill is the specified path and is what this emits.**  A
memset to zero IS the group's default -- `tc == NULL` is absent, a zero scalar is the
proto zero, a zero presence word is no child -- so the sparse fill and the canonical form
agree by construction.

## What M2 adds, and why it is the shape that decides the verdict

`ResultRaw` is a **leaf**: the batching predicate admits it, so a thousand elements cross
in about ten calls and the crossing count per element is constant.  `TaskDetailed` is
**not**: it carries four repeated string fields and a map, so it goes through
`ak_elemu_*` with a token range and each of those five fields is a **reverse call per
element** (ABI v1 section 6: "on encode the host drives every one of its own containers,
so a loop slot is a crossing whatever the predicate says").  Everything M1 measured rests
on the leaf property, and this is where it is tested.

Three further shapes arrive with it:

*   **a map**, which the ABI deliberately has no case for (section 11): it is a repeated
    field of a synthetic pair message, and `ir.py` does that synthesis so no backend
    needs a map case on the wire.  The **facade** still holds a `dict`, so the shim sorts
    the keys -- the canonical form requires it and that sort is a real cost this slice
    reports rather than hides;
*   **a repeated field inside an inlined child**: `TaskOptions.options` keeps its loop
    slot on `TaskDetailed`'s vtable and is reached through the parent (section 6);
*   **depth 4**: root -> `TaskDetailed` -> `TaskOptions` -> `Duration`.
"""

import ir as IR

CHUNK_BYTES = 32768   # ABI v1 sections 6 and 7.3: batched runs chunk at 32 KB.

# One repeated-message nesting level is all `design/SHAPES.md` needs, and the emitter
# raises rather than emitting something it has not thought about (R1).
MAX_ELEM_DEPTH = 1

# How many repeated fields a ROOT may have. M1 to M4 and M6 have one, M5 has none and M7
# has two, so a shim with a single `h->list` had "exactly one" written into it rather than
# derived from the description -- which is the R1 failure in miniature.
MAX_ROOT_LOOPS = 4


class Unsupported(Exception):
    """A shape this backend has no case for. R1: raise, never skip."""


def fields(ir, name):
    return ir.msg(name).fields


class CaseField:
    """The facade attribute that says which member of a oneof is selected.

    It is not in the description, because a oneof's discriminant is not a field: it has no
    tag of its own and nothing on the wire. It IS an attribute of the facade, and the group
    has `<name>_case` for it (ABI v1 section 6), so it needs a C struct member, a
    `PyMemberDef` entry, an interned key and an `__init__` keyword exactly like a field
    does. Giving it the shape of a `Field` is what lets every one of those emitters stay
    generic instead of growing a second path.

    `int32` and not `uint32`: the group's field is `u32`, but the facade's storage is a
    Python int and the C member is what `read_scalar` produces, and a tag never comes near
    2^31. The cast at the group is explicit.
    """

    def __init__(self, owner, oname, members):
        self.owner = owner
        self.oneof = None
        self.name = "%s_case" % oname
        self.kind = "int32"
        self.card = "singular"
        self.tag = 0
        self.of = None
        self.explicit = False
        self.members = members
        self.direct = False
        self.adapter_site = None


def case_fields(ir, name):
    return [CaseField(name, o, ms) for o, ms in ir.msg(name).oneofs.items()]


def attrs(ir, name):
    """Every attribute the facade carries: the fields, then one discriminant per oneof.

    The order matters in one place only -- the C extension type's `__init__` keyword list --
    and it matches `py_facade.py`, so a positional construction means the same thing in
    both storages.
    """
    return list(fields(ir, name)) + case_fields(ir, name)


def oneof_list(ir, name):
    """[(oneof name, [member field, ...])] in declaration order."""
    return list(ir.msg(name).oneofs.items())


def presence_bit(gprefix, owner, f):
    return "AK_%sFIX_%s_PRESENT_%s" % (gprefix.upper(), owner.upper(), f.name.upper())


def is_obj(f):
    if isinstance(f, _AsObj):
        return True
    # An explicit-presence scalar holds `None` for absent, and `None` does not fit in a
    # `long`. So the C extension type stores it as an object like a string, and the cost of
    # explicit presence on this facade is one boxed int per present field rather than one
    # struct member -- which is a real cost and belongs in the M3 table, not in a footnote.
    if getattr(f, "explicit", False):
        return True
    return (f.kind in ("string", "bytes", "message", "map")
            or f.card in ("repeated", "packed", "map"))


def cty(f):
    if is_obj(f):
        return "PyObject *"
    if f.kind == "int64":
        return "long long "
    if f.kind in ("int32", "enum"):
        return "long "
    if f.kind == "bool":
        return "int "
    raise Unsupported("no C type for %s.%s (%s)" % (f.owner, f.name, f.kind))


def is_loop(f):
    """A field that gets a vtable slot instead of riding in the group."""
    return f.card in ("repeated", "packed", "map")


def elem_types(ir, scope):
    """Message types that are the element of a repeated MESSAGE field.

    A map's synthetic pair message is NOT one of these: its group is filled from a dict
    key and value rather than from a facade object, so the generic element fill has no
    case for it and `emit_map_loop` builds it directly. Kept separate rather than filtered
    later, because an unused generated function is a build failure here and that is the
    right place for "this type does not go through that path" to show up.
    """
    out = []
    for n in scope:
        for f in fields(ir, n):
            if f.card == "repeated" and f.kind == "message" and f.of not in out:
                out.append(f.of)
    return out


def map_entry_types(ir, scope):
    out = []
    for n in scope:
        for f in fields(ir, n):
            if f.card == "map" and f.entry not in out:
                out.append(f.entry)
    return out


def inlined_types(ir, scope):
    """Every message that is a SINGULAR child and so is inlined into its parent's group."""
    out = []
    seen = set()

    def visit(n):
        for f in fields(ir, n):
            if f.kind == "message" and f.card == "singular" and f.of not in seen:
                seen.add(f.of)
                out.append(f.of)
                visit(f.of)
    for n in scope:
        visit(n)
    return out


# --------------------------------------------------------------------------------------
# The generated C facade type.
# --------------------------------------------------------------------------------------

def emit_ctypes(ir, names):
    L = []
    for name in names:
        L.append("typedef struct {\n  PyObject_HEAD")
        for f in attrs(ir, name):
            L.append("  %s%s;" % (cty(f), f.name))
        L.append("} C%s;\n" % name)
    for name in names:
        ff = attrs(ir, name)
        L.append("static PyMemberDef mem_%s[] = {" % name)
        for f in ff:
            t = ("T_OBJECT_EX" if is_obj(f) else
                 "T_LONGLONG" if f.kind == "int64" else
                 "T_BOOL" if f.kind == "bool" else "T_LONG")
            L.append('  {"%s", %s, offsetof(C%s, %s), 0, NULL},'
                     % (f.name, t, name, f.name))
        L.append("  {NULL}};")
        objs = [f for f in ff if is_obj(f)]
        fmt = "".join("O" if is_obj(f) else
                      "L" if f.kind == "int64" else
                      "p" if f.kind == "bool" else "l" for f in ff)
        L.append("static int init_%s(PyObject *self, PyObject *a, PyObject *kw) {" % name)
        L.append("  static char *kwl[] = {%s NULL};"
                 % "".join('"%s", ' % f.name for f in ff))
        # `Empty` has no fields at all -- it is the oneof's payload-free member -- and an
        # argument list with nothing in it is a syntax error in C rather than an empty one.
        if not ff:
            L.append("  (void)self;")
            L.append('  if (!PyArg_ParseTupleAndKeywords(a, kw, "", kwl)) return -1;')
            L.append("  return 0;\n}")
            L.append("static int trav_%s(PyObject *s, visitproc visit, void *arg) {" % name)
            L.append("  (void)s; (void)visit; (void)arg;")
            L.append("  return 0;\n}")
            L.append("static int clear_%s(PyObject *s) {" % name)
            L.append("  (void)s;")
            L.append("  return 0;\n}")
            L.append(_ctype_tail(name))
            continue
        L.append("  C%s *o = (C%s *)self;" % (name, name))
        for f in ff:
            L.append("  %sv_%s = %s;" % (cty(f), f.name, "NULL" if is_obj(f) else "0"))
        L.append('  if (!PyArg_ParseTupleAndKeywords(a, kw, "|%s", kwl, %s)) return -1;'
                 % (fmt, ", ".join("&v_" + f.name for f in ff)))
        for f in ff:
            if is_obj(f):
                # A repeated or map field defaults to a fresh container, not to None: the
                # facade is idiomatic, and a decoder that appends has to have something
                # to append to before the group arrives (ABI v1 decision 10).
                if f.card in ("repeated", "packed"):
                    L.append("  { PyObject *v = v_%s ? Py_NewRef(v_%s) : PyList_New(0);"
                             " if (!v) return -1; Py_XSETREF(o->%s, v); }"
                             % (f.name, f.name, f.name))
                elif f.card == "map":
                    L.append("  { PyObject *v = v_%s ? Py_NewRef(v_%s) : PyDict_New();"
                             " if (!v) return -1; Py_XSETREF(o->%s, v); }"
                             % (f.name, f.name, f.name))
                else:
                    L.append("  { PyObject *v = v_%s ? v_%s : Py_None; Py_INCREF(v);"
                             " Py_XSETREF(o->%s, v); }" % (f.name, f.name, f.name))
            else:
                L.append("  o->%s = v_%s;" % (f.name, f.name))
        L.append("  return 0;\n}")
        L.append("static int trav_%s(PyObject *s, visitproc visit, void *arg) {" % name)
        if objs:
            L.append("  C%s *o = (C%s *)s;" % (name, name))
            for f in objs:
                L.append("  Py_VISIT(o->%s);" % f.name)
        else:
            L.append("  (void)s; (void)visit; (void)arg;")
        L.append("  return 0;\n}")
        L.append("static int clear_%s(PyObject *s) {" % name)
        if objs:
            L.append("  C%s *o = (C%s *)s;" % (name, name))
            for f in objs:
                L.append("  Py_CLEAR(o->%s);" % f.name)
        else:
            L.append("  (void)s;")
        L.append("  return 0;\n}")
        L.append(_ctype_tail(name))
    return "\n".join(L)


def _ctype_tail(name):
    L = ["static void dealloc_%s(PyObject *s) {" % name,
         "  PyTypeObject *t = Py_TYPE(s);",
         "  PyObject_GC_UnTrack(s); clear_%s(s);" % name,
         "  ((freefunc)PyType_GetSlot(t, Py_tp_free))(s); Py_DECREF(t);\n}",
         "static PyType_Slot slots_%s[] = {" % name,
         "  {Py_tp_init, (void *)init_%s}, {Py_tp_members, (void *)mem_%s},"
         % (name, name),
         "  {Py_tp_new, (void *)PyType_GenericNew},"
         " {Py_tp_dealloc, (void *)dealloc_%s}," % name,
         "  {Py_tp_traverse, (void *)trav_%s}, {Py_tp_clear, (void *)clear_%s},"
         % (name, name),
         "  {0, NULL}};",
         'static PyType_Spec spec_%s = {"_akffi.C%s", sizeof(C%s), 0,'
         % (name, name, name),
         "  Py_TPFLAGS_DEFAULT | Py_TPFLAGS_BASETYPE | Py_TPFLAGS_HAVE_GC,"
         " slots_%s};\n" % name]
    return "\n".join(L)


# --------------------------------------------------------------------------------------
# Reaching a field, per backend. One place, so a backend cannot answer a shape one way in
# one traversal and differently in another.
# --------------------------------------------------------------------------------------

def read_obj(backend, owner, fname, var, objexpr="ob", ind="  ", ret="-1"):
    p = ind
    if backend == "cext":
        return p + "PyObject *%s = ((C%s *)%s)->%s; const int own_%s = 0;" \
            % (var, owner, objexpr, fname, var)
    if backend == "attr":
        return "\n".join([
            p + "BUMP(C_ATTR);",
            p + "PyObject *%s = PyObject_GetAttr(%s, K_%s); const int own_%s = 1;"
              % (var, objexpr, fname, var),
            p + "if (!%s) return %s;" % (var, ret),
        ])
    return "\n".join([
        p + "BUMP(C_PYCALL);",
        p + "PyObject *g_%s = PyDict_GetItemString(h->acc, \"get_%s\");" % (var, fname),
        p + "if (!g_%s) { PyErr_SetString(PyExc_KeyError, \"get_%s\"); return %s; }"
          % (var, fname, ret),
        p + "PyObject *%s = PyObject_CallOneArg(g_%s, %s); const int own_%s = 1;"
          % (var, var, objexpr, var),
        p + "if (!%s) return %s;" % (var, ret),
    ])


def write_field(backend, owner, f, valexpr, ind="  ", fail=None, objexpr="ob"):
    fail = fail or "h->failed = 1; return;"
    p = ind
    if is_obj(f):
        if backend == "cext":
            return p + "Py_XSETREF(((C%s *)%s)->%s, %s);" % (owner, objexpr, f.name, valexpr)
        if backend == "attr":
            return "\n".join([
                p + "{ PyObject *v_ = %s; if (!v_) { %s }" % (valexpr, fail),
                p + "  BUMP(C_ATTR);",
                p + "  if (PyObject_SetAttr(%s, K_%s, v_) < 0) { Py_DECREF(v_); %s }"
                  % (objexpr, f.name, fail),
                p + "  Py_DECREF(v_); }",
            ])
        return "\n".join([
            p + "{ PyObject *v_ = %s; if (!v_) { %s }" % (valexpr, fail),
            p + "  BUMP(C_PYCALL);",
            p + "  PyObject *s_ = PyDict_GetItemString(h->acc, \"set_%s\");" % f.name,
            p + "  PyObject *r_ = s_ ? PyObject_CallFunctionObjArgs(s_, %s, v_, NULL)"
              " : NULL;" % objexpr,
            p + "  if (!r_) { Py_DECREF(v_); %s } else Py_DECREF(r_);" % fail,
            p + "  Py_DECREF(v_); }",
        ])
    if backend == "cext":
        return p + "((C%s *)%s)->%s = (%s)(%s);" \
            % (owner, objexpr, f.name, cty(f).strip(), valexpr)
    boxed = ("PyBool_FromLong((long)(%s))" % valexpr if f.kind == "bool"
             else "PyLong_FromLongLong((long long)(%s))" % valexpr)
    if backend == "attr":
        return "\n".join([
            p + "{ PyObject *v_ = %s; if (!v_) { %s }" % (boxed, fail),
            p + "  BUMP(C_ATTR);",
            p + "  if (PyObject_SetAttr(%s, K_%s, v_) < 0) { Py_DECREF(v_); %s }"
              % (objexpr, f.name, fail),
            p + "  Py_DECREF(v_); }",
        ])
    return "\n".join([
        p + "{ PyObject *v_ = %s; if (!v_) { %s }" % (boxed, fail),
        p + "  BUMP(C_PYCALL);",
        p + "  PyObject *s_ = PyDict_GetItemString(h->acc, \"set_%s\");" % f.name,
        p + "  PyObject *r_ = s_ ? PyObject_CallFunctionObjArgs(s_, %s, v_, NULL) : NULL;"
          % objexpr,
        p + "  if (!r_) { Py_DECREF(v_); %s } else Py_DECREF(r_);" % fail,
        p + "  Py_DECREF(v_); }",
    ])


def read_scalar(backend, owner, f, var, objexpr="ob", ind="  ", ret="-1"):
    p = ind
    t = cty(f).strip()
    if backend == "cext":
        return p + "%s %s = ((C%s *)%s)->%s;" % (t, var, owner, objexpr, f.name)
    L = [read_obj(backend, owner, f.name, "py_" + var, objexpr, ind, ret)]
    L.append(p + "BUMP(C_READ);")
    if f.kind == "bool":
        L.append(p + "int %s = PyObject_IsTrue(py_%s);" % (var, var))
        L.append(p + "if (%s < 0) { if (own_py_%s) Py_DECREF(py_%s); return %s; }"
                 % (var, var, var, ret))
    else:
        L.append(p + "%s %s = (%s)PyLong_AsLongLong(py_%s);" % (t, var, t, var))
        L.append(p + "if (%s == (%s)-1 && PyErr_Occurred())"
                 " { if (own_py_%s) Py_DECREF(py_%s); return %s; }"
                 % (var, t, var, var, ret))
    L.append(p + "if (own_py_%s) Py_DECREF(py_%s);" % (var, var))
    return "\n".join(L)


# --------------------------------------------------------------------------------------
# Encode: the group fill, sparse (ABI v1 decision 9).
# --------------------------------------------------------------------------------------

def emit_group_fill(ir, name, backend, fnname, gtype, zero=False):
    """Fill `e` from `ob`. Loop-slot fields are skipped: they cross through the vtable,
    not through the group.

    `zero` memsets first. An inlined child's group is a member of its parent's and was
    already cleared by the parent's memset; a ROOT's is a local in the encode entry point
    and is not.
    """
    L = ["static int %s(struct %s *e, PyObject *ob, HostCtx *h) {" % (fnname, gtype)]
    L.append("  (void)h; (void)e; (void)ob;")
    if zero:
        L.append("  memset(e, 0, sizeof *e);")
    gp = "e" if gtype.startswith("ak_efix") else "d"
    for f in fields(ir, name):
        if is_loop(f):
            L.append("  /* %s: a loop slot, reached through the vtable */" % f.name)
            continue
        if f.oneof:
            continue                     # emitted below, once per oneof, under its case
        L.append("  { /* %s, tag %d, %s */" % (f.name, f.tag, f.kind))
        if f.kind in ("string", "bytes"):
            L.append(read_obj(backend, name, f.name, "v", "ob", "    "))
            L.append("    BUMP(C_READ);")
            if f.explicit:
                # Absent reads as None and has no bytes to point at, so the conversion is
                # skipped rather than attempted: PyUnicode_AsUTF8AndSize(None) is a TypeError.
                L.append("    const char *sp = NULL; Py_ssize_t sl = 0;")
                if f.kind == "string":
                    L.append("    if (v != Py_None) { sp = PyUnicode_AsUTF8AndSize(v, &sl);")
                    L.append("      if (!sp) { if (own_v) Py_DECREF(v); return -1; } }")
                else:
                    L.append("    if (v != Py_None) {")
                    L.append("      char *bp_ = NULL;")
                    L.append("      if (PyBytes_AsStringAndSize(v, &bp_, &sl))"
                             " { if (own_v) Py_DECREF(v); return -1; }")
                    L.append("      sp = bp_; }")
            elif f.kind == "string":
                L.append("    Py_ssize_t sl = 0;")
                L.append("    const char *sp = PyUnicode_AsUTF8AndSize(v, &sl);")
                L.append("    if (!sp) { if (own_v) Py_DECREF(v); return -1; }")
            else:
                L.append("    char *sp = NULL; Py_ssize_t sl = 0;")
                L.append("    if (PyBytes_AsStringAndSize(v, &sp, &sl))"
                         " { if (own_v) Py_DECREF(v); return -1; }")
            if f.explicit:
                # Explicit presence: `None` is absent, and "" is PRESENT and empty. The
                # length cannot say which, so the presence word does and the span is set
                # whatever the length is. Guarding on `sl` here is exactly the bug that
                # makes an `optional string = ""` encode as absent.
                L.append("    if (v != Py_None) {")
                L.append("      e->presence |= %s;" % presence_bit(gp, name, f))
                L.append("      e->%s.data = sp; e->%s.len = (size_t)sl; e->%s.tc = %s;"
                         % (f.name, f.name, f.name,
                            "TC_UTF8" if f.kind == "string" else "TC_BYTES"))
                L.append("    }")
            elif f.direct:
                # ABI v1 section 8: the group carries the SENTINEL and the length, and the
                # bytes go to the call BESIDE the group, so a host with a pinning cost can
                # hold a critical section across it. CPython has no pinning cost -- a
                # `bytes` object's buffer is already a stable address for as long as the
                # facade holds it -- so this path should save nothing here, and the arm
                # exists to say that with a measurement rather than by assertion.
                #
                # No transcoder: a direct field carries the sentinel instead, which is why
                # the core's implicit-presence guard for it tests the LENGTH and not `tc`
                # (poc/codec/gen/rust_abi.py, the `f.direct` branch).
                L.append("    e->%s.data = AK_STR_DIRECT; e->%s.len = (size_t)sl;"
                         % (f.name, f.name))
                L.append("    h->direct = (const uint8_t *)sp; h->direct_len = (size_t)sl;")
            else:
                L.append("    if (sl) { e->%s.data = sp; e->%s.len = (size_t)sl;"
                         " e->%s.tc = %s; }"
                         % (f.name, f.name, f.name,
                            "TC_UTF8" if f.kind == "string" else "TC_BYTES"))
            L.append("    if (own_v) Py_DECREF(v);")
        elif f.kind == "message":
            L.append(read_obj(backend, name, f.name, "v", "ob", "    "))
            L.append("    if (v != Py_None) {")
            L.append("      e->presence |= AK_%s_%s_PRESENT_%s;"
                     % (gtype.split("_")[1].upper(), name.upper(), f.name.upper()))
            L.append("      if (into_%s_%s(&e->%s, v, h))"
                     " { if (own_v) Py_DECREF(v); return -1; }"
                     % (backend, f.of, f.name))
            L.append("    }")
            L.append("    if (own_v) Py_DECREF(v);")
        elif f.explicit:
            # `None` is absent; anything else is present, INCLUDING 0 and False. So the
            # value is read as an object first and only then converted -- the sparse fill's
            # "assign only what differs from zero" shortcut does not apply to a field whose
            # zero is a value.
            ct = ("int32_t" if f.kind in ("int32", "enum")
                  else "int64_t" if f.kind == "int64" else "uint8_t")
            L.append(read_obj(backend, name, f.name, "v", "ob", "    "))
            L.append("    BUMP(C_READ);")
            L.append("    if (v != Py_None) {")
            L.append("      e->presence |= %s;" % presence_bit(gp, name, f))
            if f.kind == "bool":
                L.append("      int t_ = PyObject_IsTrue(v);")
                L.append("      if (t_ < 0) { if (own_v) Py_DECREF(v); return -1; }")
                L.append("      e->%s = (uint8_t)t_;" % f.name)
            else:
                L.append("      long long n_ = PyLong_AsLongLong(v);")
                L.append("      if (n_ == -1 && PyErr_Occurred())"
                         " { if (own_v) Py_DECREF(v); return -1; }")
                L.append("      e->%s = (%s)n_;" % (f.name, ct))
            L.append("    }")
            L.append("    if (own_v) Py_DECREF(v);")
        else:
            L.append(read_scalar(backend, name, f, "sv", "ob", "    "))
            L.append("    if (sv) e->%s = (%s)sv;"
                     % (f.name, "int32_t" if f.kind in ("int32", "enum")
                        else "int64_t" if f.kind == "int64" else "uint8_t"))
        L.append("  }")

    for oname, members in oneof_list(ir, name):
        # ONE read of the discriminant per element, then a switch. The alternative -- ask
        # each member whether it is set -- is five reads where this is one, and it cannot
        # express "selected and holding the zero" at all.
        cf = CaseField(name, oname, members)
        L.append("  { /* oneof %s: the discriminant carries the active member's TAG */"
                 % oname)
        L.append(read_scalar(backend, name, cf, "cs", "ob", "    "))
        L.append("    e->%s_case = (uint32_t)cs;" % oname)
        L.append("    switch (cs) {")
        for g in members:
            gn = "%s_%s" % (oname, g.name)
            L.append("    case %d: {" % g.tag)
            if g.kind in ("string", "bytes"):
                L.append(read_obj(backend, name, g.name, "mv", "ob", "      "))
                L.append("      BUMP(C_READ);")
                if g.kind == "string":
                    L.append("      Py_ssize_t ml = 0;")
                    L.append("      const char *mp = PyUnicode_AsUTF8AndSize(mv, &ml);")
                    L.append("      if (!mp) { if (own_mv) Py_DECREF(mv); return -1; }")
                else:
                    L.append("      char *mp = NULL; Py_ssize_t ml = 0;")
                    L.append("      if (PyBytes_AsStringAndSize(mv, &mp, &ml))"
                             " { if (own_mv) Py_DECREF(mv); return -1; }")
                # A selected member is written whatever its length: "" is a message, and
                # the codec writes it because the CASE says so, not because the value does.
                L.append("      e->%s.data = mp; e->%s.len = (size_t)ml; e->%s.tc = %s;"
                         % (gn, gn, gn,
                            "TC_UTF8" if g.kind == "string" else "TC_BYTES"))
                L.append("      if (own_mv) Py_DECREF(mv);")
            elif g.kind == "message":
                L.append(read_obj(backend, name, g.name, "mv", "ob", "      "))
                L.append("      if (mv == Py_None) { if (own_mv) Py_DECREF(mv);")
                L.append("        PyErr_SetString(PyExc_ValueError,"
                         " \"%s.%s is selected but None\"); return -1; }" % (name, g.name))
                L.append("      if (into_%s_%s(&e->%s, mv, h))"
                         " { if (own_mv) Py_DECREF(mv); return -1; }"
                         % (backend, g.of, gn))
                L.append("      if (own_mv) Py_DECREF(mv);")
            else:
                L.append(read_scalar(backend, name, g, "mv", "ob", "      "))
                L.append("      e->%s = (%s)mv;"
                         % (gn, "int32_t" if g.kind in ("int32", "enum")
                            else "int64_t" if g.kind == "int64" else "uint8_t"))
            L.append("      break; }")
        L.append("    case 0: break;")
        L.append("    default:")
        L.append("      PyErr_Format(PyExc_ValueError, \"%s.%s_case = %%ld is not a"
                 " member's tag\", (long)cs); return -1;" % (name, oname))
        L.append("    }")
        L.append("  }")
    L.append("  return 0;\n}")
    return "\n".join(L)


def emit_into(ir, name, backend):
    """A singular child's group, inlined into its parent's (ABI v1 section 6)."""
    return emit_group_fill(ir, name, backend, "into_%s_%s" % (backend, name),
                           "ak_efix_%s" % name)


def emit_elem_fill(ir, name, backend):
    return emit_group_fill(ir, name, backend, "fill_%s_%s" % (backend, name),
                           "ak_efix_%s" % name)


def chunk_macro(name):
    return "CHUNK_%s" % name.upper()


def emit_elem_loop(ir, owner, f, backend, leaf, depth):
    """The loop callback for a repeated MESSAGE field."""
    if depth > MAX_ELEM_DEPTH:
        raise Unsupported("%s.%s nests repeated messages %d deep; this backend's element "
                          "stack is %d (R1: raise rather than emit something untested)"
                          % (owner, f.name, depth, MAX_ELEM_DEPTH))
    elem = f.of
    L = ["static int32_t loop_%s_%s_%s(ak_enc_ctx *ctx, const void *obj, int64_t token) {"
         % (backend, owner, f.name)]
    L += ["  (void)token;", "  HostCtx *h = (HostCtx *)obj;",
          "  PyObject *ob = h->root;"]
    L.append(read_obj(backend, owner, f.name, "lst", "ob", "  "))
    L += [
        "  Py_ssize_t n = PyList_Size(lst);",
        "  if (n < 0) { if (own_lst) Py_DECREF(lst); return -1; }",
        "  /* The element list, so an inner loop slot can index it by token without",
        "     re-reading the attribute. The core only calls an inner loop from inside",
        "     this one, so one slot per nesting level is enough -- and the emitter",
        "     raises above if the schema ever nests deeper than the stack. */",
        "  PyObject *saved = h->cur[%d]; h->cur[%d] = lst;" % (depth - 1, depth - 1),
        "  struct ak_efix_%s chunk[%s];" % (elem, chunk_macro(elem)),
        "  int64_t done = 0;",
        "  int32_t rc = 0;",
        "  for (Py_ssize_t i = 0; i < n; i += %s) {" % chunk_macro(elem),
        "    int32_t k = (int32_t)((n - i < %s) ? (n - i) : %s);"
        % (chunk_macro(elem), chunk_macro(elem)),
        "    /* ABI v1 decision 9: bulk clear once, then assign only what differs. */",
        "    memset(chunk, 0, sizeof(chunk[0]) * (size_t)k);",
        "    for (int32_t j = 0; j < k; j++) {",
        "      BUMP(C_ITEM);",
        "      PyObject *e = PyList_GetItem(lst, i + j);",
        "      if (!e || fill_%s_%s(&chunk[j], e, h)) { rc = -1; goto out; }"
        % (backend, elem),
        "    }",
    ]
    if leaf:
        L.append("    if (ak_elem_%s(ctx, chunk, k)) { rc = -1; goto out; }" % elem)
    else:
        L.append("    /* NOT a leaf: the batching predicate refuses it, so the element")
        L.append("       run carries a token range and each of the element's own")
        L.append("       containers is a reverse call per element (ABI v1 section 6). */")
        L.append("    if (ak_elemu_%s(ctx, chunk, k, done)) { rc = -1; goto out; }" % elem)
    L += [
        "    done += k;",
        "  }",
        "out:",
        "  h->cur[%d] = saved;" % (depth - 1),
        "  if (own_lst) Py_DECREF(lst);",
        "  return rc;\n}",
    ]
    return "\n".join(L)


def emit_blob_loop(ir, owner, path, f, backend, depth):
    """A repeated string or bytes field on an element: `ak_blob_run`."""
    slot = IR.slot_name(path)
    L = ["static int32_t loop_%s_%s_%s(ak_enc_ctx *ctx, const void *obj, int64_t token) {"
         % (backend, owner, slot)]
    L += ["  HostCtx *h = (HostCtx *)obj;",
          "  PyObject *el = PyList_GetItem(h->cur[%d], (Py_ssize_t)token);" % (depth - 1),
          "  if (!el) return -1;"]
    holder, owner_msg = emit_walk_path(ir, owner, path[:-1], backend, "el", "  ")
    L.append(holder)
    L.append(read_obj(backend, owner_msg, f.name, "lst", "cur", "  "))
    L += [
        "  Py_ssize_t n = PyList_Size(lst);",
        "  if (n < 0) { if (own_lst) Py_DECREF(lst); return -1; }",
        "  struct ak_str chunk[CHUNK_BLOB];",
        "  int32_t rc = 0;",
        "  for (Py_ssize_t i = 0; i < n; i += CHUNK_BLOB) {",
        "    int32_t k = (int32_t)((n - i < CHUNK_BLOB) ? (n - i) : CHUNK_BLOB);",
        "    memset(chunk, 0, sizeof(chunk[0]) * (size_t)k);",
        "    for (int32_t j = 0; j < k; j++) {",
        "      BUMP(C_ITEM); BUMP(C_READ);",
        "      PyObject *s = PyList_GetItem(lst, i + j);",
        "      if (!s) { rc = -1; goto out; }",
        "      Py_ssize_t sl = 0;",
        "      const char *sp = PyUnicode_AsUTF8AndSize(s, &sl);",
        "      if (!sp) { rc = -1; goto out; }",
        "      chunk[j].data = sp; chunk[j].len = (size_t)sl; chunk[j].tc = TC_UTF8;",
        "    }",
        "    if (ak_blob_run(ctx, chunk, k)) { rc = -1; goto out; }",
        "  }",
        "out:",
        "  if (own_lst) Py_DECREF(lst);",
        "  return rc;\n}",
    ]
    return "\n".join(L)


PACKED_C = {"int64": ("int64_t", "ak_run_i64"), "int32": ("int32_t", "ak_run_i32"),
            "enum": ("int32_t", "ak_run_i32"), "bool": ("uint8_t", "ak_run_u8"),
            "double": ("double", "ak_run_f64")}


def emit_packed_loop(ir, owner, path, f, backend, depth):
    """A packed scalar field on an element: one `ak_run_*` per chunk.

    The crossing count is what this shape is in the payload set FOR: 30 values cross in
    one call rather than thirty, so a packed field of any length costs the same one
    boundary call as an empty one, and the per-element crossing count does not move with
    the run length. The read of each Python int still happens, once per value, on this
    side of the boundary -- which is the cost the control is meant to expose.
    """
    slot = IR.slot_name(path)
    cty_, runfn = PACKED_C[f.kind]
    L = ["static int32_t loop_%s_%s_%s(ak_enc_ctx *ctx, const void *obj, int64_t token) {"
         % (backend, owner, slot)]
    L += ["  HostCtx *h = (HostCtx *)obj;",
          "  PyObject *el = PyList_GetItem(h->cur[%d], (Py_ssize_t)token);" % (depth - 1),
          "  if (!el) return -1;"]
    holder, owner_msg = emit_walk_path(ir, owner, path[:-1], backend, "el", "  ")
    L.append(holder)
    L.append(read_obj(backend, owner_msg, f.name, "lst", "cur", "  "))
    L += [
        "  Py_ssize_t n = PyList_Size(lst);",
        "  if (n < 0) { if (own_lst) Py_DECREF(lst); return -1; }",
        "  %s chunk[CHUNK_PACKED];" % cty_,
        "  int32_t rc = 0;",
        "  for (Py_ssize_t i = 0; i < n; i += CHUNK_PACKED) {",
        "    int32_t k = (int32_t)((n - i < CHUNK_PACKED) ? (n - i) : CHUNK_PACKED);",
        "    for (int32_t j = 0; j < k; j++) {",
        "      BUMP(C_ITEM); BUMP(C_READ);",
        "      PyObject *v = PyList_GetItem(lst, i + j);",
        "      if (!v) { rc = -1; goto out; }",
    ]
    if f.kind == "double":
        L += ["      double d = PyFloat_AsDouble(v);",
              "      if (d == -1.0 && PyErr_Occurred()) { rc = -1; goto out; }",
              "      chunk[j] = d;"]
    elif f.kind == "bool":
        L += ["      int t = PyObject_IsTrue(v);",
              "      if (t < 0) { rc = -1; goto out; }",
              "      chunk[j] = (uint8_t)t;"]
    else:
        L += ["      long long x = PyLong_AsLongLong(v);",
              "      if (x == -1 && PyErr_Occurred()) { rc = -1; goto out; }",
              "      chunk[j] = (%s)x;" % cty_]
    L += [
        "    }",
        "    if (%s(ctx, chunk, (size_t)k)) { rc = -1; goto out; }" % runfn,
        "  }",
        "out:",
        "  if (own_lst) Py_DECREF(lst);",
        "  return rc;\n}",
    ]
    return "\n".join(L)


def emit_packed_add(ir, root, ef, path, sf, backend, li):
    """The decode side: a batch of packed values, appended to the facade's list."""
    slot = IR.slot_name(path)
    cty_, _ = PACKED_C[sf.kind]
    L = ["static void add_%s_%s_%s_%s(ak_dec_ctx *ctx, void *obj, int64_t tok,"
         % (backend, root, ef.name, slot),
         "        const %s *elems, int32_t n) {" % cty_,
         "  (void)ctx;",
         "  HostCtx *h = (HostCtx *)obj;",
         "  if (h->failed) return;",
         "  PyObject *ob = PyList_GetItem(h->lists[%d], (Py_ssize_t)tok);" % li,
         "  if (!ob) { h->failed = 1; return; }"]
    code, owner_msg = emit_reach_child(ir, ef.of, path[:-1], backend, "  ")
    L.append(code)
    L.append(read_obj(backend, owner_msg, sf.name, "lst", "cur", "  ", ret=""))
    box = ("PyFloat_FromDouble(elems[i])" if sf.kind == "double"
           else "PyBool_FromLong((long)elems[i])" if sf.kind == "bool"
           else "PyLong_FromLongLong((long long)elems[i])")
    L += ["  for (int32_t i = 0; i < n; i++) {",
          "    BUMP(C_ITEM);",
          "    PyObject *v = %s;" % box,
          "    if (!v) { h->failed = 1; break; }",
          "    if (PyList_Append(lst, v) < 0) { h->failed = 1; Py_DECREF(v); break; }",
          "    Py_DECREF(v);",
          "  }",
          "  if (own_lst) Py_DECREF(lst);",
          "  Py_DECREF(cur);",
          "}"]
    return "\n".join(L)


def emit_map_loop(ir, owner, path, f, backend, depth):
    """A map field: a repeated run of the synthetic pair message (ABI v1 section 11).

    The facade holds a `dict`, which is what a Python user expects, and the canonical form
    sorts entries by key -- so the shim sorts. That sort is a cost the incumbent does not
    pay on its default path (`SerializeToString` is not deterministic for maps) and this
    slice reports it rather than hiding it.
    """
    slot = IR.slot_name(path)
    entry = f.entry
    L = ["static int32_t loop_%s_%s_%s(ak_enc_ctx *ctx, const void *obj, int64_t token) {"
         % (backend, owner, slot)]
    L += ["  HostCtx *h = (HostCtx *)obj;",
          "  PyObject *el = PyList_GetItem(h->cur[%d], (Py_ssize_t)token);" % (depth - 1),
          "  if (!el) return -1;"]
    holder, owner_msg = emit_walk_path(ir, owner, path[:-1], backend, "el", "  ")
    L.append(holder)
    L.append("  if (cur == Py_None) { %s return 0; }" % ("Py_DECREF(cur);"
                                                         if backend != "cext" else ""))
    L.append(read_obj(backend, owner_msg, f.name, "d", "cur", "  "))
    L += [
        "  PyObject *keys = PyDict_Keys(d);",
        "  if (!keys) { if (own_d) Py_DECREF(d); return -1; }",
        "  if (PyList_Sort(keys) < 0) { Py_DECREF(keys);"
        " if (own_d) Py_DECREF(d); return -1; }",
        "  Py_ssize_t n = PyList_Size(keys);",
        "  struct ak_efix_%s chunk[%s];" % (entry, chunk_macro(entry)),
        "  int32_t rc = 0;",
        "  for (Py_ssize_t i = 0; i < n; i += %s) {" % chunk_macro(entry),
        "    int32_t k = (int32_t)((n - i < %s) ? (n - i) : %s);"
        % (chunk_macro(entry), chunk_macro(entry)),
        "    memset(chunk, 0, sizeof(chunk[0]) * (size_t)k);",
        "    for (int32_t j = 0; j < k; j++) {",
        "      BUMP(C_ITEM);",
        "      PyObject *kk = PyList_GetItem(keys, i + j);",
        "      PyObject *vv = kk ? PyDict_GetItem(d, kk) : NULL;",
        "      if (!kk || !vv) { rc = -1; goto out; }",
        "      BUMP(C_READ); BUMP(C_READ);",
        "      Py_ssize_t kl = 0, vl = 0;",
        "      const char *kp = PyUnicode_AsUTF8AndSize(kk, &kl);",
        "      const char *vp = PyUnicode_AsUTF8AndSize(vv, &vl);",
        "      if (!kp || !vp) { rc = -1; goto out; }",
        "      chunk[j].key.data = kp; chunk[j].key.len = (size_t)kl;"
        " chunk[j].key.tc = TC_UTF8;",
        "      /* An empty value is an implicit-presence leaf holding the proto zero, so",
        "         it is left absent by the memset. P2.5 is the payload that reaches it. */",
        "      if (vl) { chunk[j].value.data = vp; chunk[j].value.len = (size_t)vl;"
        " chunk[j].value.tc = TC_UTF8; }",
        "    }",
        "    if (ak_elem_%s(ctx, chunk, k)) { rc = -1; goto out; }" % entry,
        "  }",
        "out:",
        "  Py_DECREF(keys);",
        "  if (own_d) Py_DECREF(d);",
    ]
    if backend != "cext":
        L.append("  Py_DECREF(cur);")
    L.append("  return rc;\n}")
    return "\n".join(L)


def emit_walk_path(ir, owner, path, backend, start, ind):
    """Emit C that walks `path` of singular message children from `start` into `cur`.

    Returns (code, the message type `cur` holds). For the `cext` backend `cur` is a
    borrowed reference; for the others it is owned, which is why the callers that use it
    decref explicitly.
    """
    if not path:
        if backend == "cext":
            return ind + "PyObject *cur = %s;" % start, owner
        return ind + "PyObject *cur = Py_NewRef(%s);" % start, owner
    L = []
    cur_owner = owner
    src = start
    for i, step in enumerate(path):
        f = next(x for x in fields(ir, cur_owner) if x.name == step)
        var = "p%d" % i
        L.append(read_obj(backend, cur_owner, step, var, src, ind))
        L.append(ind + "(void)own_%s;" % var)
        if backend == "cext":
            L.append(ind + "Py_INCREF(%s);" % var)
        src = var
        cur_owner = f.of
    L.append(ind + "PyObject *cur = %s;" % src)
    return "\n".join(L), cur_owner


# --------------------------------------------------------------------------------------
# Encode entry point.
# --------------------------------------------------------------------------------------

def emit_root_fix(ir, root, backend):
    """The root's own group.

    The same fill every other group gets, rather than a scalars-only special case: M5's
    root carries a singular message child and nothing else, and a root fix that skipped
    message fields encoded it as absent -- while emitting `into_<backend>_UploadResultData`
    for nobody to call, which is what `-Werror=unused-function` turned into a build
    failure instead of a wrong payload.
    """
    return emit_group_fill(ir, root, backend, "fix_%s_%s" % (backend, root),
                           "ak_efix_%s" % root, zero=True)


def emit_encode_entry(ir, root, backend, leafmap):
    loops = [f for f in fields(ir, root) if f.card == "repeated"]
    L = ["static PyObject *encode_%s_%s(PyObject *rootobj, PyObject *acc) {"
         % (backend, root)]
    L += ["  HostCtx hs; memset(&hs, 0, sizeof hs);",
          "  hs.root = rootobj; hs.acc = acc;",
          "  HostCtx *h = &hs;",
          "  struct ak_efix_%s fix;" % root,
          "  if (fix_%s_%s(&fix, rootobj, h)) return NULL;" % (backend, root)]
    # The element vtables, one per non-leaf repeated field.
    for f in loops:
        if leafmap[f.of]:
            continue
        slots = IR.loop_slots(ir, f.of)
        L.append("  static const struct ak_evt_%s EVT_%s = {" % (f.of, f.name))
        for path, sf in slots:
            L.append("    .loop_%s = loop_%s_%s_%s,"
                     % (IR.slot_name(path), backend, f.of, IR.slot_name(path)))
        L.append("  };")
    L.append("  static const struct ak_evt_%s VT = {" % root)
    for f in loops:
        L.append("    .loop_%s = loop_%s_%s_%s," % (f.name, backend, root, f.name))
        if not leafmap[f.of]:
            L.append("    .elem_%s = &EVT_%s," % (f.name, f.name))
    L.append("  };")
    L += [
        "  ak_enc_ctx *ctx = ak_enc_ctx_new();",
        "  if (!ctx) return PyErr_NoMemory();",
        ("  intptr_t rc = ak_encode_%s(h, ctx, &VT, &fix, h->direct, h->direct_len);"
         % root) if IR.direct_fields(ir, root) else
        ("  intptr_t rc = ak_encode_%s(h, ctx, &VT, &fix);" % root),
        "  if (rc < 0) {",
        "    ak_enc_ctx_free(ctx);",
        "    if (!PyErr_Occurred()) PyErr_Format(PyExc_RuntimeError,"
        " \"ak_encode_%s returned %%ld\", (long)rc);" % root,
        "    return NULL;",
        "  }",
        "#ifdef AK_COUNT",
        "  { struct AkCounters c; ak_enc_counters(ctx, &c); CORE_ADD(CORE_ENC, c); }",
        "#endif",
        "  const uint8_t *p = NULL; size_t len = 0;",
        "  if (ak_enc_take(ctx, &p, &len)) { ak_enc_ctx_free(ctx);",
        "    PyErr_SetString(PyExc_RuntimeError, \"ak_enc_take\"); return NULL; }",
        "  PyObject *out = PyBytes_FromStringAndSize((const char *)p, (Py_ssize_t)len);",
        "  ak_enc_ctx_free(ctx);",
        "  return out;\n}",
    ]
    return "\n".join(L)


# --------------------------------------------------------------------------------------
# Decode.
# --------------------------------------------------------------------------------------

def span_value(f, spanexpr):
    if f.kind == "string":
        return ("PyUnicode_DecodeUTF8((const char *)h->base + %s.off, (Py_ssize_t)%s.len,"
                " \"strict\")" % (spanexpr, spanexpr))
    return ("PyBytes_FromStringAndSize((const char *)h->base + %s.off,"
            " (Py_ssize_t)%s.len)" % (spanexpr, spanexpr))


def emit_setgroup(ir, name, backend):
    """Fill an EXISTING facade object from one decoded group. Returns 0 or -1.

    **Get-or-create, never construct-and-replace**, and that is ABI v1 decision 10 rather
    than a style choice: a run may arrive before the group that would have constructed the
    child, so `apply` can find a child the run already made. The first version of this
    constructed a fresh child and assigned it, which threw the whole map away -- decode
    returned `options.options == {}` on every M2 payload while encode was byte-perfect,
    and the field-by-field check against the incumbent is what caught it. Defect D8.
    """
    L = ["static int setgroup_%s_%s(HostCtx *h, PyObject *ob,"
         " const struct ak_dfix_%s *e) {" % (backend, name, name)]
    L.append("  (void)h; (void)ob; (void)e;")
    for f in fields(ir, name):
        if is_loop(f):
            continue
        if f.oneof:
            continue                     # emitted below, once per oneof
        sp = "e->%s" % f.name
        L.append("  { /* %s */" % f.name)
        if f.explicit:
            # The mirror of the encode side: the presence WORD decides, and absent is
            # written as None rather than left at the facade's default, because the facade
            # object may be reused (decision 10's get-or-create) and a stale value from a
            # previous message would read as present.
            L.append("    if (e->presence & %s) {" % presence_bit("d", name, f))
            if f.kind in ("string", "bytes"):
                L.append("      BUMP(C_READ);")
                L.append(write_field(backend, name, f, span_value(f, sp), "      ",
                                     fail="return -1;"))
            else:
                # The value is boxed here rather than by `write_field`, because the field
                # now takes the OBJECT path (absent is None) and `write_field` only boxes
                # for a field it believes is a scalar.
                boxed = ("PyBool_FromLong((long)(%s))" % sp if f.kind == "bool"
                         else "PyLong_FromLongLong((long long)(%s))" % sp)
                L.append(write_field(backend, name, _AsObj(f), boxed, "      ",
                                     fail="return -1;"))
            L.append("    } else {")
            L.append(write_field(backend, name, _AsObj(f), "Py_NewRef(Py_None)", "      ",
                                 fail="return -1;"))
            L.append("    }")
        elif f.kind in ("string", "bytes"):
            L.append("    BUMP(C_READ);")
            L.append(write_field(backend, name, f, span_value(f, sp), "    ",
                                 fail="return -1;"))
        elif f.kind == "message":
            L.append("    if (e->presence & AK_DFIX_%s_PRESENT_%s) {"
                     % (name.upper(), f.name.upper()))
            code, _ = emit_reach_child(ir, name, (f.name,), backend, "      ",
                                       fail="return -1;")
            L.append(code)
            L.append("      if (setgroup_%s_%s(h, cur, &e->%s))"
                     " { Py_DECREF(cur); return -1; }" % (backend, f.of, f.name))
            L.append("      Py_DECREF(cur);")
            L.append("    }")
        else:
            L.append(write_field(backend, name, f, sp, "    ", fail="return -1;"))
        L.append("  }")

    for oname, members in oneof_list(ir, name):
        cf = CaseField(name, oname, members)
        L.append("  { /* oneof %s */" % oname)
        L.append("    switch (e->%s_case) {" % oname)
        for g in members:
            gn = "%s_%s" % (oname, g.name)
            L.append("    case %d: {" % g.tag)
            if g.kind in ("string", "bytes"):
                L.append("      BUMP(C_READ);")
                L.append(write_field(backend, name, g, span_value(g, "e->%s" % gn),
                                     "      ", fail="return -1;"))
            elif g.kind == "message":
                code, _ = emit_reach_child(ir, name, (g.name,), backend, "      ",
                                           fail="return -1;")
                L.append(code)
                L.append("      if (setgroup_%s_%s(h, cur, &e->%s))"
                         " { Py_DECREF(cur); return -1; }" % (backend, g.of, gn))
                L.append("      Py_DECREF(cur);")
            else:
                L.append(write_field(backend, name, g, "e->%s" % gn, "      ",
                                     fail="return -1;"))
            L.append("      break; }")
        L.append("    default: break;")
        L.append("    }")
        # The discriminant LAST, so a facade that is watched while it fills never shows a
        # case pointing at a member that has not been written yet.
        L.append(write_field(backend, name, cf, "e->%s_case" % oname, "    ",
                             fail="return -1;"))
        L.append("  }")
    L.append("  return 0;\n}")
    return "\n".join(L)


class _AsObj:
    """One field, seen as an object-valued one, so `write_field` stores None into it.

    An `optional int32` is a C `long` in the extension type and a Python int elsewhere, and
    absent is `None` in both -- which the C storage cannot hold. So the extension type keeps
    an object for every explicit-presence field (`cty` says so) and this wrapper is what
    tells `write_field` to take the object path for a field whose kind says scalar.
    """

    def __init__(self, f):
        self.__dict__.update({k: getattr(f, k) for k in
                              ("owner", "name", "kind", "card", "tag", "of", "explicit")})
        self.oneof = getattr(f, "oneof", None)


def emit_apply_root(ir, root, backend):
    L = ["static void apply_%s_%s(ak_dec_ctx *ctx, void *obj," % (backend, root),
         "                         const struct ak_dfix_%s *fx) {" % root,
         "  (void)ctx;",
         "  HostCtx *h = (HostCtx *)obj;",
         "  if (h->failed) return;",
         "  PyObject *ob = h->root;"]
    L.append("  if (setgroup_%s_%s(h, ob, fx)) h->failed = 1;" % (backend, root))
    L.append("}")
    return "\n".join(L)


def emit_leaf_add(ir, root, f, backend, li):
    elem = f.of
    L = ["static void add_%s_%s_%s(ak_dec_ctx *ctx, void *obj, int64_t tok,"
         % (backend, root, f.name),
         "        const struct ak_dfix_%s *elems, int32_t n) {" % elem,
         "  (void)ctx; (void)tok;",
         "  HostCtx *h = (HostCtx *)obj;",
         "  if (h->failed) return;",
         "  for (int32_t i = 0; i < n; i++) {",
         "    /* ABI v1 7.4: a batched add may be called more than once per field."
         " Append. */",
         "    PyObject *ob = PyObject_CallNoArgs(h->ty_%s);" % elem,
         "    if (!ob) { h->failed = 1; return; }",
         "    const struct ak_dfix_%s *e = &elems[i];" % elem,
         "    if (setgroup_%s_%s(h, ob, e)) { h->failed = 1; Py_DECREF(ob); return; }"
         % (backend, elem),
         "    BUMP(C_ITEM);",
         "    if (PyList_Append(h->lists[%d], ob) < 0) h->failed = 1;" % li,
         "    Py_DECREF(ob);",
         "    if (h->failed) return;",
         "  }\n}"]
    return "\n".join(L)


def emit_nonleaf_decode(ir, root, f, backend, li):
    """`new_`, `apply_` and one `add_` per loop slot of the element."""
    elem = f.of
    L = []
    L += ["static int64_t new_%s_%s_%s(ak_dec_ctx *ctx, void *obj) {"
          % (backend, root, f.name),
          "  (void)ctx;",
          "  HostCtx *h = (HostCtx *)obj;",
          "  if (h->failed) return 0;",
          "  PyObject *ob = PyObject_CallNoArgs(h->ty_%s);" % elem,
          "  if (!ob) { h->failed = 1; return 0; }",
          "  BUMP(C_ITEM);",
          "  Py_ssize_t idx = PyList_Size(h->lists[%d]);" % li,
          "  if (PyList_Append(h->lists[%d], ob) < 0) h->failed = 1;" % li,
          "  Py_DECREF(ob);",
          "  return (int64_t)idx;\n}"]
    L += ["static void apply_%s_%s_%s(ak_dec_ctx *ctx, void *obj, int64_t tok,"
          % (backend, root, f.name),
          "        const struct ak_dfix_%s *e) {" % elem,
          "  (void)ctx;",
          "  HostCtx *h = (HostCtx *)obj;",
          "  if (h->failed) return;",
          "  PyObject *ob = PyList_GetItem(h->lists[%d], (Py_ssize_t)tok);" % li,
          "  if (!ob) { h->failed = 1; return; }",
          "  if (setgroup_%s_%s(h, ob, e)) h->failed = 1;" % (backend, elem),
          "}"]
    for path, sf in IR.loop_slots(ir, elem):
        slot = IR.slot_name(path)
        if sf.card == "map":
            L.append(emit_map_add(ir, root, f, path, sf, backend, li))
        elif sf.card == "packed":
            L.append(emit_packed_add(ir, root, f, path, sf, backend, li))
        elif sf.kind in ("string", "bytes"):
            L.append(emit_blob_add(ir, root, f, path, sf, backend, li))
        else:
            raise Unsupported("%s.%s: a %s %s run on decode" % (elem, sf.name,
                                                                sf.card, sf.kind))
    return "\n".join(L)


def emit_reach_child(ir, owner, path, backend, ind, fail=None, start="ob"):
    """Get-or-create the chain of singular children `path` from `ob` into `cur`.

    Get-or-CREATE, because ABI v1 decision 10 allows a run to arrive before the group that
    would have constructed the child. A binding that only got would drop the run.
    """
    fail = fail or "h->failed = 1; return;"
    if not path:
        return ind + "PyObject *cur = Py_NewRef(%s);" % start, owner
    L = []
    cur_owner, src = owner, start
    ret = "-1" if "return -1" in fail else ""
    for i, step in enumerate(path):
        fld = next(x for x in fields(ir, cur_owner) if x.name == step)
        var = "c%d" % i
        L.append(read_obj(backend, cur_owner, step, var, src, ind, ret=ret))
        L.append(ind + "(void)own_%s;" % var)
        if backend == "cext":
            L.append(ind + "Py_INCREF(%s);" % var)
        L.append(ind + "if (%s == Py_None) {" % var)
        L.append(ind + "  PyObject *nw = PyObject_CallNoArgs(h->ty_%s);" % fld.of)
        L.append(ind + "  if (!nw) { Py_DECREF(%s); %s }" % (var, fail))
        L.append(write_field(backend, cur_owner, fld, "Py_NewRef(nw)", ind + "  ",
                             fail="Py_DECREF(nw); Py_DECREF(%s); %s" % (var, fail),
                             objexpr=src))
        L.append(ind + "  Py_DECREF(%s); %s = nw;" % (var, var))
        L.append(ind + "}")
        src = var
        cur_owner = fld.of
    L.append(ind + "PyObject *cur = %s;" % src)
    return "\n".join(L), cur_owner


def emit_blob_add(ir, root, ef, path, sf, backend, li):
    slot = IR.slot_name(path)
    L = ["static void add_%s_%s_%s_%s(ak_dec_ctx *ctx, void *obj, int64_t tok,"
         % (backend, root, ef.name, slot),
         "        const struct ak_span *elems, int32_t n) {",
         "  (void)ctx;",
         "  HostCtx *h = (HostCtx *)obj;",
         "  if (h->failed) return;",
         "  PyObject *ob = PyList_GetItem(h->lists[%d], (Py_ssize_t)tok);" % li,
         "  if (!ob) { h->failed = 1; return; }"]
    code, owner_msg = emit_reach_child(ir, ef.of, path[:-1], backend, "  ")
    L.append(code)
    L.append(read_obj(backend, owner_msg, sf.name, "lst", "cur", "  ", ret=""))
    L += ["  for (int32_t i = 0; i < n; i++) {",
          "    BUMP(C_READ); BUMP(C_ITEM);",
          "    PyObject *s = %s;" % span_value(sf, "elems[i]"),
          "    if (!s) { h->failed = 1; break; }",
          "    if (PyList_Append(lst, s) < 0) { h->failed = 1; Py_DECREF(s); break; }",
          "    Py_DECREF(s);",
          "  }",
          "  if (own_lst) Py_DECREF(lst);",
          "  Py_DECREF(cur);" if True else "",
          "}"]
    return "\n".join(x for x in L if x)


def emit_map_add(ir, root, ef, path, sf, backend, li):
    slot = IR.slot_name(path)
    entry = sf.entry
    L = ["static void add_%s_%s_%s_%s(ak_dec_ctx *ctx, void *obj, int64_t tok,"
         % (backend, root, ef.name, slot),
         "        const struct ak_dfix_%s *elems, int32_t n) {" % entry,
         "  (void)ctx;",
         "  HostCtx *h = (HostCtx *)obj;",
         "  if (h->failed) return;",
         "  PyObject *ob = PyList_GetItem(h->lists[%d], (Py_ssize_t)tok);" % li,
         "  if (!ob) { h->failed = 1; return; }"]
    code, owner_msg = emit_reach_child(ir, ef.of, path[:-1], backend, "  ")
    L.append(code)
    L.append(read_obj(backend, owner_msg, sf.name, "d", "cur", "  ", ret=""))
    L += ["  for (int32_t i = 0; i < n; i++) {",
          "    BUMP(C_READ); BUMP(C_READ); BUMP(C_ITEM);",
          "    PyObject *k = PyUnicode_DecodeUTF8((const char *)h->base + elems[i].key.off,"
          " (Py_ssize_t)elems[i].key.len, \"strict\");",
          "    PyObject *v = k ? PyUnicode_DecodeUTF8("
          "(const char *)h->base + elems[i].value.off,"
          " (Py_ssize_t)elems[i].value.len, \"strict\") : NULL;",
          "    if (!k || !v) { h->failed = 1; Py_XDECREF(k); Py_XDECREF(v); break; }",
          "    if (PyDict_SetItem(d, k, v) < 0) h->failed = 1;",
          "    Py_DECREF(k); Py_DECREF(v);",
          "    if (h->failed) break;",
          "  }",
          "  if (own_d) Py_DECREF(d);",
          "  Py_DECREF(cur);",
          "}"]
    return "\n".join(L)


def emit_decode_entry(ir, root, backend, leafmap, scope, li_of):
    """The decode entry point for one root.

    Written for N repeated fields at the root rather than for one: M5's root has none at
    all and M7's has two, and each of the two needs a run list of its own -- `add_left`
    and `add_right` appending to one list is a decoder that returns both halves in both
    fields and passes a hash check that only looks at the bytes it wrote.
    """
    loops = [f for f in fields(ir, root) if f.card == "repeated"]
    L = ["static PyObject *decode_%s_%s(PyObject *buf, PyObject *acc, HostTypes *T) {"
         % (backend, root)]
    L += [
        "  char *p = NULL; Py_ssize_t blen = 0;",
        "  if (PyBytes_AsStringAndSize(buf, &p, &blen)) return NULL;",
        "  PyObject *rootobj = PyObject_CallNoArgs(T->ty_%s);" % root,
        "  if (!rootobj) return NULL;",
        "  HostCtx h;",
        "  memset(&h, 0, sizeof h);",
        "  h.root = rootobj; h.base = (const uint8_t *)p; h.acc = acc;",
    ]
    for n in scope:
        L.append("  h.ty_%s = T->ty_%s;" % (n, n))
    # One run list per root repeated field, allocated up front so the error paths have a
    # single shape whatever N is.
    for f in loops:
        i = li_of[f.name]
        L.append("  h.lists[%d] = PyList_New(0);" % i)
        L.append("  if (!h.lists[%d]) { %s Py_DECREF(rootobj); return NULL; }"
                 % (i, _free_lists(loops, li_of, upto=i)))
    freeall = _free_lists(loops, li_of)
    L += [
        "  static const struct ak_dvt_%s VT = {" % root,
        "    .apply = apply_%s_%s," % (backend, root),
    ]
    for lf in loops:
        if leafmap[lf.of]:
            L.append("    .add_%s = add_%s_%s_%s," % (lf.name, backend, root, lf.name))
        else:
            L.append("    .new_%s = new_%s_%s_%s," % (lf.name, backend, root, lf.name))
            L.append("    .apply_%s = apply_%s_%s_%s," % (lf.name, backend, root, lf.name))
            for path, sf in IR.loop_slots(ir, lf.of):
                slot = IR.slot_name(path)
                L.append("    .add_%s_%s = add_%s_%s_%s_%s,"
                         % (lf.name, slot, backend, root, lf.name, slot))
    L += [
        "  };",
        "  ak_dec_ctx *ctx = ak_dec_ctx_new();",
        "  if (!ctx) { %s Py_DECREF(rootobj); return PyErr_NoMemory(); }" % freeall,
        "  int32_t rc = ak_decode_%s(ctx, &h, (const uint8_t *)p, (size_t)blen, &VT);"
        % root,
        "#ifdef AK_COUNT",
        "  { struct AkCounters c; ak_dec_counters(ctx, &c); CORE_ADD(CORE_DEC, c); }",
        "#endif",
        "  ak_dec_ctx_free(ctx);",
        "  if (rc || h.failed) {",
        "    %s Py_DECREF(rootobj);" % freeall,
        "    if (!PyErr_Occurred()) PyErr_Format(PyExc_ValueError,"
        " \"ak_decode_%s returned %%d\", (int)rc);" % root,
        "    return NULL;",
        "  }",
    ]
    for f in loops:
        L.append("  if (setlist_%s_%s_%s(rootobj, h.lists[%d], &h))"
                 % (backend, root, f.name, li_of[f.name]))
        L.append("    { %s Py_DECREF(rootobj); return NULL; }" % freeall)
    L.append("  %s" % freeall)
    L.append("  return rootobj;\n}")
    return "\n".join(L)


def _free_lists(loops, li_of, upto=None):
    """`Py_DECREF` for every run list allocated so far, as one line."""
    out = []
    for f in loops:
        i = li_of[f.name]
        if upto is not None and i >= upto:
            continue
        out.append("Py_DECREF(h.lists[%d]);" % i)
    return " ".join(out)


def emit_root_setlist(ir, root, f, backend):
    L = ["static int setlist_%s_%s_%s(PyObject *ob, PyObject *lst, HostCtx *h) {"
         % (backend, root, f.name)]
    L.append("  (void)h; (void)ob; (void)lst;")
    L.append(write_field(backend, root, f, "Py_NewRef(lst)", "  ", fail="return -1;"))
    L.append("  return 0;\n}")
    return "\n".join(L)


# --------------------------------------------------------------------------------------

PRELUDE = r'''/* @generated by ffi/poc/python/gen/generate.py from ffi/schema/shapes.json.
 * Do not edit. */
#include <Python.h>
#include <structmember.h>
#include <string.h>
#include "ak_abi.h"

/* README R5, counting build: what the SHIM does to CPython. The core counts its own
 * crossings through ak_enc_counters / ak_dec_counters, so each half is counted by the
 * half that can see it. */
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

/* Resolved once at module init: ak_tc_utf8() is a call across the boundary and a
 * generated shim would not make it per string. */
static ak_transcode_fn TC_UTF8, TC_BYTES;

/* A run of strings chunks by count rather than by group size. */
#define CHUNK_BLOB ((int32_t)(32768 / sizeof(struct ak_str)))

/* A packed run chunks by count too: a packed value is not a group, so the group-size
 * chunking of ak_elem_* does not apply to it. */
#define CHUNK_PACKED 4096
'''


def is_leaf(ir, name, seen=None):
    """ABI v1 section 7.2's batching predicate, over the IR rather than over the raw
    description, because the IR has already turned a map into its synthetic pair message
    and the raw description has no entry for that name.

    A message is a leaf if it has no repeated and no map field, transitively. A leaf's
    elements batch through `ak_elem_*`; a non-leaf's go through `ak_elemu_*` with a token
    range and each of its own containers is a reverse call per element.
    """
    seen = seen or set()
    if name in seen:
        return False
    seen = seen | {name}
    for f in fields(ir, name):
        if f.card in ("repeated", "packed", "map"):
            return False
        if f.kind == "message" and not is_leaf(ir, f.of, seen):
            return False
    return True


def emit(ir, scope, roots):
    allnames = list(scope)
    for n in elem_types(ir, scope) + map_entry_types(ir, scope) + inlined_types(ir, scope):
        if n not in allnames:
            allnames.append(n)
    # The batching predicate, from the descriptor, computed once (ABI v1 section 7.2).
    leafmap = {n: is_leaf(ir, n) for n in allnames}
    # Which run list each ROOT repeated field appends to.
    li_of = {}
    for r in roots:
        loops = [f for f in fields(ir, r) if f.card == "repeated"]
        if len(loops) > MAX_ROOT_LOOPS:
            raise Unsupported("%s has %d repeated fields; MAX_ROOT_LOOPS is %d"
                              % (r, len(loops), MAX_ROOT_LOOPS))
        li_of[r] = {f.name: i for i, f in enumerate(loops)}

    L = [PRELUDE, ""]
    L.append(emit_ctypes(ir, allnames))
    for name in allnames:
        L.append("#define %s ((int32_t)(%d / sizeof(struct ak_efix_%s)) > 0 ? \\"
                 % (chunk_macro(name), CHUNK_BYTES, name))
        L.append("                  (int32_t)(%d / sizeof(struct ak_efix_%s)) : 1)"
                 % (CHUNK_BYTES, name))
    L.append("")
    fnames = sorted({f.name for n in allnames for f in attrs(ir, n)})
    for n in fnames:
        L.append("static PyObject *K_%s;" % n)
    L.append("")
    L.append("typedef struct {")
    for n in allnames:
        L.append("  PyObject *ty_%s;" % n)
    L.append("} HostTypes;")
    L.append("")
    L.append("typedef struct {")
    L.append("  PyObject *root;      /* the facade root */")
    L.append("  const uint8_t *base; /* decode: ABI v1 7.4, spans are offsets into this */")
    L.append("  PyObject *acc;       /* pyacc backend: the accessor table */")
    L.append("  PyObject *lists[%d];  /* decode: one run per ROOT repeated field."
             % max(1, MAX_ROOT_LOOPS))
    L.append("                        M7's root has two and M5's has none, so a single")
    L.append("                        `list` was a root with exactly one repeated field")
    L.append("                        written into the shim rather than derived. */")
    L.append("  PyObject *cur[%d];   /* encode: the element list, per nesting level */"
             % MAX_ELEM_DEPTH)
    L.append("  int failed;")
    L.append("  const uint8_t *direct;  /* ABI v1 section 8: the bulk field's bytes, */")
    L.append("  size_t direct_len;      /* passed BESIDE the group rather than in it. */")
    for n in allnames:
        L.append("  PyObject *ty_%s;" % n)
    L.append("} HostCtx;")
    L.append("")
    L.append("static int intern_keys(void) {")
    for n in fnames:
        L.append('  K_%s = PyUnicode_InternFromString("%s"); if (!K_%s) return -1;'
                 % (n, n, n))
    L.append("  TC_UTF8 = ak_tc_utf8(); TC_BYTES = ak_tc_bytes();")
    L.append("  return 0;\n}")
    L.append("")

    inl = inlined_types(ir, scope)
    els = elem_types(ir, scope)
    pairs = map_entry_types(ir, scope)

    for backend in ("attr", "cext", "pyacc"):
        L.append("/* ===================== backend: %s ===================== */" % backend)
        # Forward declarations: an encoder calls its children's and a decoder its
        # children's builders, and the scope order is not a topological order.
        for n in inl:
            L.append("static int into_%s_%s(struct ak_efix_%s *, PyObject *, HostCtx *);"
                     % (backend, n, n))
        # No setgroup for a map's pair message: `emit_map_add` writes the key and the
        # value straight into the facade's dict, so the pair never becomes an object.
        for n in [x for x in allnames if x not in pairs]:
            L.append("static int setgroup_%s_%s(HostCtx *, PyObject *,"
                     " const struct ak_dfix_%s *);" % (backend, n, n))
        for r in roots:
            for f in fields(ir, r):
                if f.card == "repeated":
                    L.append("static int setlist_%s_%s_%s(PyObject *, PyObject *,"
                             " HostCtx *);" % (backend, r, f.name))
        L.append("")
        for n in inl:
            L.append(emit_into(ir, n, backend))
        for n in els:
            L.append(emit_elem_fill(ir, n, backend))
        # No fill for a map's pair message: emit_map_loop builds its group from the dict
        # key and value, and an emitted-but-unused function is a build failure.
        assert pairs is not None
        for r in roots:
            depth = 1
            for f in fields(ir, r):
                if f.card == "repeated":
                    L.append(emit_elem_loop(ir, r, f, backend, leafmap[f.of], depth))
                    for path, sf in IR.loop_slots(ir, f.of):
                        if sf.card == "map":
                            L.append(emit_map_loop(ir, f.of, path, sf, backend, depth))
                        elif sf.card == "packed":
                            L.append(emit_packed_loop(ir, f.of, path, sf, backend, depth))
                        elif sf.kind in ("string", "bytes"):
                            L.append(emit_blob_loop(ir, f.of, path, sf, backend, depth))
                        else:
                            raise Unsupported("%s.%s: %s %s loop slot"
                                              % (f.of, sf.name, sf.card, sf.kind))
            L.append(emit_root_fix(ir, r, backend))
            L.append(emit_encode_entry(ir, r, backend, leafmap))
        for n in [x for x in allnames if x not in pairs]:
            L.append(emit_setgroup(ir, n, backend))
        for r in roots:
            for f in fields(ir, r):
                if f.card == "repeated":
                    L.append(emit_root_setlist(ir, r, f, backend))
            L.append(emit_apply_root(ir, r, backend))
            for f in fields(ir, r):
                if f.card != "repeated":
                    continue
                if leafmap[f.of]:
                    L.append(emit_leaf_add(ir, r, f, backend, li_of[r][f.name]))
                else:
                    L.append(emit_nonleaf_decode(ir, r, f, backend, li_of[r][f.name]))
            L.append(emit_decode_entry(ir, r, backend, leafmap, allnames,
                                       li_of[r]))
        L.append("")

    L.append("/* ---- dispatch, so native/binding.c names no message and no field ---- */")
    L.append("#define AK_NTYPES %d" % len(allnames))
    L.append("static const char *AK_TYPE_NAMES[AK_NTYPES] = {%s};"
             % ", ".join('"C%s"' % n for n in allnames))
    L.append("static PyType_Spec *AK_TYPE_SPECS[AK_NTYPES] = {%s};"
             % ", ".join("&spec_%s" % n for n in allnames))
    L.append("")
    L.append("static int types_from_seq(HostTypes *T, PyObject *seq) {")
    L.append("  if (!PySequence_Check(seq) || PySequence_Size(seq) != AK_NTYPES) {")
    L.append("    PyErr_Format(PyExc_TypeError, \"expected %d types\", AK_NTYPES);")
    L.append("    return -1;")
    L.append("  }")
    L.append("  PyObject **slot = &T->ty_%s;" % allnames[0])
    L.append("  for (int i = 0; i < AK_NTYPES; i++) {")
    L.append("    PyObject *t = PySequence_GetItem(seq, i);")
    L.append("    if (!t) return -1;")
    L.append("    slot[i] = t;   /* borrowed for the call; the caller holds the tuple */")
    L.append("    Py_DECREF(t);")
    L.append("  }")
    L.append("  return 0;\n}")
    L.append("")
    L.append("typedef PyObject *(*ak_enc_f)(PyObject *, PyObject *);")
    L.append("typedef PyObject *(*ak_dec_f)(PyObject *, PyObject *, HostTypes *);")
    L.append("static const char *AK_BACKENDS[3] = {\"attr\", \"cext\", \"pyacc\"};")
    L.append("#define AK_NROOTS %d" % len(roots))
    L.append("static const char *AK_ROOTS[AK_NROOTS] = {%s};"
             % ", ".join('"%s"' % r for r in roots))
    L.append("static int root_index(const char *name) {")
    L.append("  for (int i = 0; i < AK_NROOTS; i++)")
    L.append("    if (strcmp(AK_ROOTS[i], name) == 0) return i;")
    L.append("  PyErr_Format(PyExc_ValueError, \"unknown root %s\", name);")
    L.append("  return -1;\n}")
    L.append("static ak_enc_f AK_ENC[3][AK_NROOTS] = {")
    for b in ("attr", "cext", "pyacc"):
        L.append("  {%s}," % ", ".join("encode_%s_%s" % (b, r) for r in roots))
    L.append("};")
    L.append("static ak_dec_f AK_DEC[3][AK_NROOTS] = {")
    for b in ("attr", "cext", "pyacc"):
        L.append("  {%s}," % ", ".join("decode_%s_%s" % (b, r) for r in roots))
    L.append("};")
    return "\n".join(L)
