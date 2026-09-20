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


class Unsupported(Exception):
    """A shape this backend has no case for. R1: raise, never skip."""


def fields(ir, name):
    return ir.msg(name).fields


def is_obj(f):
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
        for f in fields(ir, name):
            L.append("  %s%s;" % (cty(f), f.name))
        L.append("} C%s;\n" % name)
    for name in names:
        ff = fields(ir, name)
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
        L.append("static void dealloc_%s(PyObject *s) {" % name)
        L.append("  PyTypeObject *t = Py_TYPE(s);")
        L.append("  PyObject_GC_UnTrack(s); clear_%s(s);" % name)
        L.append("  ((freefunc)PyType_GetSlot(t, Py_tp_free))(s); Py_DECREF(t);\n}")
        L.append("static PyType_Slot slots_%s[] = {" % name)
        L.append("  {Py_tp_init, (void *)init_%s}, {Py_tp_members, (void *)mem_%s},"
                 % (name, name))
        L.append("  {Py_tp_new, (void *)PyType_GenericNew},"
                 " {Py_tp_dealloc, (void *)dealloc_%s}," % name)
        L.append("  {Py_tp_traverse, (void *)trav_%s}, {Py_tp_clear, (void *)clear_%s},"
                 % (name, name))
        L.append("  {0, NULL}};")
        L.append('static PyType_Spec spec_%s = {"_akffi.C%s", sizeof(C%s), 0,'
                 % (name, name, name))
        L.append("  Py_TPFLAGS_DEFAULT | Py_TPFLAGS_BASETYPE | Py_TPFLAGS_HAVE_GC,"
                 " slots_%s};\n" % name)
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

def emit_group_fill(ir, name, backend, fnname, gtype):
    """Fill `e` (already zeroed) from `ob`. Loop-slot fields are skipped: they cross
    through the vtable, not through the group."""
    L = ["static int %s(struct %s *e, PyObject *ob, HostCtx *h) {" % (fnname, gtype)]
    L.append("  (void)h; (void)e; (void)ob;")
    for f in fields(ir, name):
        if is_loop(f):
            L.append("  /* %s: a loop slot, reached through the vtable */" % f.name)
            continue
        L.append("  { /* %s, tag %d, %s */" % (f.name, f.tag, f.kind))
        if f.kind in ("string", "bytes"):
            L.append(read_obj(backend, name, f.name, "v", "ob", "    "))
            L.append("    BUMP(C_READ);")
            if f.kind == "string":
                L.append("    Py_ssize_t sl = 0;")
                L.append("    const char *sp = PyUnicode_AsUTF8AndSize(v, &sl);")
                L.append("    if (!sp) { if (own_v) Py_DECREF(v); return -1; }")
            else:
                L.append("    char *sp = NULL; Py_ssize_t sl = 0;")
                L.append("    if (PyBytes_AsStringAndSize(v, &sp, &sl))"
                         " { if (own_v) Py_DECREF(v); return -1; }")
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
        else:
            L.append(read_scalar(backend, name, f, "sv", "ob", "    "))
            L.append("    if (sv) e->%s = (%s)sv;"
                     % (f.name, "int32_t" if f.kind in ("int32", "enum")
                        else "int64_t" if f.kind == "int64" else "uint8_t"))
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
    L = ["static int fix_%s_%s(struct ak_efix_%s *fix, PyObject *ob, HostCtx *h) {"
         % (backend, root, root)]
    L.append("  (void)h; (void)ob;")
    L.append("  memset(fix, 0, sizeof *fix);")
    for f in fields(ir, root):
        if is_loop(f) or f.kind == "message":
            continue
        L.append("  {")
        L.append(read_scalar(backend, root, f, "sv_" + f.name, "ob", "    "))
        L.append("    if (sv_%s) fix->%s = (int32_t)sv_%s;" % (f.name, f.name, f.name))
        L.append("  }")
    L.append("  return 0;\n}")
    return "\n".join(L)


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
        "  intptr_t rc = ak_encode_%s(h, ctx, &VT, &fix);" % root,
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
        sp = "e->%s" % f.name
        L.append("  { /* %s */" % f.name)
        if f.kind in ("string", "bytes"):
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
    L.append("  return 0;\n}")
    return "\n".join(L)


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


def emit_leaf_add(ir, root, f, backend):
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
         "    if (PyList_Append(h->list, ob) < 0) h->failed = 1;",
         "    Py_DECREF(ob);",
         "    if (h->failed) return;",
         "  }\n}"]
    return "\n".join(L)


def emit_nonleaf_decode(ir, root, f, backend):
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
          "  Py_ssize_t idx = PyList_Size(h->list);",
          "  if (PyList_Append(h->list, ob) < 0) h->failed = 1;",
          "  Py_DECREF(ob);",
          "  return (int64_t)idx;\n}"]
    L += ["static void apply_%s_%s_%s(ak_dec_ctx *ctx, void *obj, int64_t tok,"
          % (backend, root, f.name),
          "        const struct ak_dfix_%s *e) {" % elem,
          "  (void)ctx;",
          "  HostCtx *h = (HostCtx *)obj;",
          "  if (h->failed) return;",
          "  PyObject *ob = PyList_GetItem(h->list, (Py_ssize_t)tok);",
          "  if (!ob) { h->failed = 1; return; }",
          "  if (setgroup_%s_%s(h, ob, e)) h->failed = 1;" % (backend, elem),
          "}"]
    for path, sf in IR.loop_slots(ir, elem):
        slot = IR.slot_name(path)
        if sf.card == "map":
            L.append(emit_map_add(ir, root, f, path, sf, backend))
        elif sf.kind in ("string", "bytes"):
            L.append(emit_blob_add(ir, root, f, path, sf, backend))
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


def emit_blob_add(ir, root, ef, path, sf, backend):
    slot = IR.slot_name(path)
    L = ["static void add_%s_%s_%s_%s(ak_dec_ctx *ctx, void *obj, int64_t tok,"
         % (backend, root, ef.name, slot),
         "        const struct ak_span *elems, int32_t n) {",
         "  (void)ctx;",
         "  HostCtx *h = (HostCtx *)obj;",
         "  if (h->failed) return;",
         "  PyObject *ob = PyList_GetItem(h->list, (Py_ssize_t)tok);",
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


def emit_map_add(ir, root, ef, path, sf, backend):
    slot = IR.slot_name(path)
    entry = sf.entry
    L = ["static void add_%s_%s_%s_%s(ak_dec_ctx *ctx, void *obj, int64_t tok,"
         % (backend, root, ef.name, slot),
         "        const struct ak_dfix_%s *elems, int32_t n) {" % entry,
         "  (void)ctx;",
         "  HostCtx *h = (HostCtx *)obj;",
         "  if (h->failed) return;",
         "  PyObject *ob = PyList_GetItem(h->list, (Py_ssize_t)tok);",
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


def emit_decode_entry(ir, root, backend, leafmap, scope):
    loops = [f for f in fields(ir, root) if f.card == "repeated"]
    lf = loops[0]
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
    L += [
        "  h.list = PyList_New(0);",
        "  if (!h.list) { Py_DECREF(rootobj); return NULL; }",
        "  static const struct ak_dvt_%s VT = {" % root,
        "    .apply = apply_%s_%s," % (backend, root),
    ]
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
        "  if (!ctx) { Py_DECREF(h.list); Py_DECREF(rootobj); return PyErr_NoMemory(); }",
        "  int32_t rc = ak_decode_%s(ctx, &h, (const uint8_t *)p, (size_t)blen, &VT);"
        % root,
        "#ifdef AK_COUNT",
        "  { struct AkCounters c; ak_dec_counters(ctx, &c); CORE_ADD(CORE_DEC, c); }",
        "#endif",
        "  ak_dec_ctx_free(ctx);",
        "  if (rc || h.failed) {",
        "    Py_DECREF(h.list); Py_DECREF(rootobj);",
        "    if (!PyErr_Occurred()) PyErr_Format(PyExc_ValueError,"
        " \"ak_decode_%s returned %%d\", (int)rc);" % root,
        "    return NULL;",
        "  }",
        "  if (setlist_%s_%s(rootobj, h.list, &h))" % (backend, root),
        "    { Py_DECREF(h.list); Py_DECREF(rootobj); return NULL; }",
        "  Py_DECREF(h.list);",
        "  return rootobj;\n}",
    ]
    return "\n".join(L)


def emit_root_setlist(ir, root, f, backend):
    L = ["static int setlist_%s_%s(PyObject *ob, PyObject *lst, HostCtx *h) {"
         % (backend, root)]
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

    L = [PRELUDE, ""]
    L.append(emit_ctypes(ir, allnames))
    for name in allnames:
        L.append("#define %s ((int32_t)(%d / sizeof(struct ak_efix_%s)) > 0 ? \\"
                 % (chunk_macro(name), CHUNK_BYTES, name))
        L.append("                  (int32_t)(%d / sizeof(struct ak_efix_%s)) : 1)"
                 % (CHUNK_BYTES, name))
    L.append("")
    fnames = sorted({f.name for n in allnames for f in fields(ir, n)})
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
    L.append("  PyObject *list;      /* decode: the run being appended to */")
    L.append("  PyObject *cur[%d];   /* encode: the element list, per nesting level */"
             % MAX_ELEM_DEPTH)
    L.append("  int failed;")
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
            L.append("static int setlist_%s_%s(PyObject *, PyObject *, HostCtx *);"
                     % (backend, r))
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
                    L.append(emit_leaf_add(ir, r, f, backend))
                else:
                    L.append(emit_nonleaf_decode(ir, r, f, backend))
            L.append(emit_decode_entry(ir, r, backend, leafmap, allnames))
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
