"""The generated C shim: the python slice's binding to the ONE core at `poc/codec/`.

README 9.1 gives Python three layers where the others have two -- the Rust core, a
generated C shim speaking the CPython API, and the Python facade -- and this emits the
middle one.  Work unit 1 priced the shim-to-facade edge with no core behind it; this is
the composed arm, so there is finally something that is *the design* rather than one of
its edges.

**R0**: nothing here is a copy of the core.  The shim calls `ak_encode_*`, `ak_elem_*` and
`ak_decode_*` in `libak_core.so`, built from `poc/codec/`, and `poc/codec/gen/one_core.sh`
is what checks that.

Three accessor backends over ONE emitted traversal shape, the same three work unit 1 used
so the two are comparable:

| backend | how a field is reached |
|---|---|
| `attr`  | `PyObject_GetAttr` / `PyObject_SetAttr`.  Serves BOTH the plain and the `__slots__` facade: the code is identical and only the object differs, which is why they are one backend and two arms |
| `cext`  | a struct member on the generated C facade type: not a crossing at all |
| `pyacc` | **the premise control** (README 9.1, last bullet): a Python-level accessor CALL per field, same traversal, same crossing count |

**ABI v1 decision 9's sparse fill is the specified path and is what this emits**: the host
memsets the element-group chunk once and assigns only the fields that differ from the
default.  Three hosts settled it, and the C# slice's composed arm not doing it cost its
absent path a factor of two.  A memset to zero is exactly the group's default -- `tc ==
NULL` means absent (section 4), a zero scalar is the proto zero, and a zero presence word
means no child -- so the sparse fill and the canonical form agree by construction.
"""

import ir as IR

CHUNK_BYTES = 32768   # ABI v1 section 7.3 and section 6: batched runs chunk at 32 KB.


def cfields(ir, name):
    return ir.msg(name).fields


def is_obj(f):
    return f.kind in ("string", "bytes", "message") or f.card == "repeated"


def cty(f):
    """The C type of a field in the generated facade struct."""
    if is_obj(f):
        return "PyObject *"
    if f.kind == "int64":
        return "long long "
    if f.kind in ("int32", "enum"):
        return "long "
    if f.kind == "bool":
        return "int "
    raise NotImplementedError(f.kind)


# --------------------------------------------------------------------------------------
# The C facade type: storage candidate 3 of README 9.1, generated rather than written by
# hand, so it cannot drift from the two Python ones.
# --------------------------------------------------------------------------------------

def emit_ctypes(ir, scope):
    L = []
    for name in scope:
        L.append("typedef struct {\n  PyObject_HEAD")
        for f in cfields(ir, name):
            L.append("  %s%s;" % (cty(f), f.name))
        L.append("} C%s;\n" % name)
    for name in scope:
        L.append("static PyMemberDef mem_%s[] = {" % name)
        for f in cfields(ir, name):
            t = ("T_OBJECT_EX" if is_obj(f) else
                 "T_LONGLONG" if f.kind == "int64" else
                 "T_BOOL" if f.kind == "bool" else "T_LONG")
            L.append('  {"%s", %s, offsetof(C%s, %s), 0, NULL},'
                     % (f.name, t, name, f.name))
        L.append("  {NULL}};")
        objs = [f for f in cfields(ir, name) if is_obj(f)]
        fmt = "".join("O" if is_obj(f) else
                      "L" if f.kind == "int64" else
                      "p" if f.kind == "bool" else "l" for f in cfields(ir, name))
        L.append("static int init_%s(PyObject *self, PyObject *a, PyObject *kw) {" % name)
        L.append("  static char *kwl[] = {%s NULL};"
                 % "".join('"%s", ' % f.name for f in cfields(ir, name)))
        L.append("  C%s *o = (C%s *)self;" % (name, name))
        for f in cfields(ir, name):
            L.append("  %sv_%s = %s;" % (cty(f), f.name, "NULL" if is_obj(f) else "0"))
        L.append('  if (!PyArg_ParseTupleAndKeywords(a, kw, "|%s", kwl, %s)) return -1;'
                 % (fmt, ", ".join("&v_" + f.name for f in cfields(ir, name))))
        for f in cfields(ir, name):
            if is_obj(f):
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
# Reading a field off the facade, per backend.  One place, so a backend cannot answer a
# shape in one traversal and differently in another.
# --------------------------------------------------------------------------------------

def read_obj(backend, f, var, ind="  "):
    """Emit C that puts a NEW-or-BORROWED PyObject* for field `f` in `var`.

    `own_<var>` says whether it must be released.  A borrowed reference is safe for the
    whole fill because the facade holds the value and the facade is alive for the call.
    """
    p = ind
    if backend == "cext":
        return (p + "PyObject *%s = o->%s; const int own_%s = 0;" % (var, f.name, var))
    if backend == "attr":
        return "\n".join([
            p + "BUMP(C_ATTR);",
            p + "PyObject *%s = PyObject_GetAttr(ob, K_%s); const int own_%s = 1;"
              % (var, f.name, var),
            p + "if (!%s) return -1;" % var,
        ])
    return "\n".join([
        p + "BUMP(C_PYCALL);",
        p + "PyObject *g_%s = PyDict_GetItemString(h->acc, \"get_%s\");" % (var, f.name),
        p + "if (!g_%s) { PyErr_SetString(PyExc_KeyError, \"get_%s\"); return -1; }"
          % (var, f.name),
        p + "PyObject *%s = PyObject_CallOneArg(g_%s, ob); const int own_%s = 1;"
          % (var, var, var),
        p + "if (!%s) return -1;" % var,
    ])


def write_field(backend, f, valexpr, ind="  ", fail=None):
    """Emit C that stores `valexpr` (a NEW reference, or a C scalar) into field `f`.

    `fail` is what to emit when the store fails.  The decode callbacks return void and
    report through `h->failed`; the entry points return `PyObject *`.  Passing it in is
    what lets ONE writer serve both, instead of two writers that can disagree on a shape.
    """
    fail = fail or "h->failed = 1; return;"
    p = ind
    if is_obj(f):
        if backend == "cext":
            return p + "Py_XSETREF(o->%s, %s);" % (f.name, valexpr)
        if backend == "attr":
            return "\n".join([
                p + "{ PyObject *v_ = %s; if (!v_) { %s }" % (valexpr, fail),
                p + "  BUMP(C_ATTR);",
                p + "  if (PyObject_SetAttr(ob, K_%s, v_) < 0) { Py_DECREF(v_); %s }"
                  % (f.name, fail),
                p + "  Py_DECREF(v_); }",
            ])
        return "\n".join([
            p + "{ PyObject *v_ = %s; if (!v_) { %s }" % (valexpr, fail),
            p + "  BUMP(C_PYCALL);",
            p + "  PyObject *s_ = PyDict_GetItemString(h->acc, \"set_%s\");" % f.name,
            p + "  PyObject *r_ = s_ ? PyObject_CallFunctionObjArgs(s_, ob, v_, NULL) : NULL;",
            p + "  if (!r_) { Py_DECREF(v_); %s } else Py_DECREF(r_);" % fail,
            p + "  Py_DECREF(v_); }",
        ])
    # A scalar.
    if backend == "cext":
        return p + "o->%s = (%s)(%s);" % (f.name, cty(f).strip(), valexpr)
    boxed = ("PyLong_FromLongLong((long long)(%s))" % valexpr if f.kind != "bool"
             else "PyBool_FromLong((long)(%s))" % valexpr)
    if backend == "attr":
        return "\n".join([
            p + "{ PyObject *v_ = %s; if (!v_) { %s }" % (boxed, fail),
            p + "  BUMP(C_ATTR);",
            p + "  if (PyObject_SetAttr(ob, K_%s, v_) < 0) { Py_DECREF(v_); %s }"
              % (f.name, fail),
            p + "  Py_DECREF(v_); }",
        ])
    return "\n".join([
        p + "{ PyObject *v_ = %s; if (!v_) { %s }" % (boxed, fail),
        p + "  BUMP(C_PYCALL);",
        p + "  PyObject *s_ = PyDict_GetItemString(h->acc, \"set_%s\");" % f.name,
        p + "  PyObject *r_ = s_ ? PyObject_CallFunctionObjArgs(s_, ob, v_, NULL) : NULL;",
        p + "  if (!r_) { Py_DECREF(v_); %s } else Py_DECREF(r_);" % fail,
        p + "  Py_DECREF(v_); }",
    ])


def read_scalar(backend, f, var, ind="  "):
    """A scalar straight into a C variable, so the `cext` backend never makes a PyObject."""
    p = ind
    t = cty(f).strip()
    if backend == "cext":
        return p + "%s %s = o->%s;" % (t, var, f.name)
    L = [read_obj(backend, f, "py_" + var, ind)]
    L.append(p + "BUMP(C_READ);")
    if f.kind == "bool":
        L.append(p + "int %s = PyObject_IsTrue(py_%s);" % (var, var))
        L.append(p + "if (%s < 0) { if (own_py_%s) Py_DECREF(py_%s); return -1; }"
                 % (var, var, var))
    else:
        L.append(p + "%s %s = (%s)PyLong_AsLongLong(py_%s);" % (t, var, t, var))
        L.append(p + "if (%s == (%s)-1 && PyErr_Occurred())"
                 " { if (own_py_%s) Py_DECREF(py_%s); return -1; }" % (var, t, var, var))
    L.append(p + "if (own_py_%s) Py_DECREF(py_%s);" % (var, var))
    return "\n".join(L)


# --------------------------------------------------------------------------------------
# Encode: fill one element group, sparsely (ABI v1 decision 9).
# --------------------------------------------------------------------------------------

def emit_fill(ir, name, backend):
    """`e` is already zeroed by the chunk memset; assign only what differs."""
    L = ["static int fill_%s_%s(struct ak_efix_%s *e, PyObject *ob, HostCtx *h) {"
         % (backend, name, name)]
    L.append("  (void)h;")
    if backend == "cext":
        L.append("  C%s *o = (C%s *)ob;" % (name, name))
    for f in cfields(ir, name):
        L.append("  { /* %s, tag %d, %s %s */" % (f.name, f.tag, f.card, f.kind))
        if f.card == "repeated":
            L.append("    /* a loop slot: it does not ride in the group */")
        elif f.kind in ("string", "bytes"):
            L.append(read_obj(backend, f, "v", "    "))
            L.append("    BUMP(C_READ);")
            if f.kind == "string":
                L.append("    Py_ssize_t sl = 0;")
                L.append("    const char *sp = PyUnicode_AsUTF8AndSize(v, &sl);")
                L.append("    if (!sp) { if (own_v) Py_DECREF(v); return -1; }")
            else:
                L.append("    char *sp = NULL; Py_ssize_t sl = 0;")
                L.append("    if (PyBytes_AsStringAndSize(v, &sp, &sl))"
                         " { if (own_v) Py_DECREF(v); return -1; }")
            # Sparse: an implicit-presence blob holding the zero value is left absent,
            # which the memset already did. `tc == NULL` IS absent (section 4).
            L.append("    if (sl) { e->%s.data = sp; e->%s.len = (size_t)sl;"
                     " e->%s.tc = %s; }"
                     % (f.name, f.name, f.name,
                        "TC_UTF8" if f.kind == "string" else "TC_BYTES"))
            L.append("    if (own_v) Py_DECREF(v);")
        elif f.kind == "message":
            L.append(read_obj(backend, f, "v", "    "))
            L.append("    if (v != Py_None) {")
            L.append("      e->presence |= AK_EFIX_%s_PRESENT_%s;"
                     % (name.upper(), f.name.upper()))
            if backend == "cext":
                L.append("      C%s *c = (C%s *)v;" % (f.of, f.of))
                for sub in cfields(ir, f.of):
                    L.append("      e->%s.%s = (%s)c->%s;"
                             % (f.name, sub.name, cty(sub).strip(), sub.name))
            else:
                L.append("      if (fill_%s_%s_into(&e->%s, v, h))"
                         " { if (own_v) Py_DECREF(v); return -1; }"
                         % (backend, f.of, f.name))
            L.append("    }")
            L.append("    if (own_v) Py_DECREF(v);")
        else:
            L.append(read_scalar(backend, f, "sv", "    "))
            L.append("    if (sv) e->%s = (%s)sv;"
                     % (f.name, "int32_t" if f.kind in ("int32", "enum")
                        else "int64_t" if f.kind == "int64" else "uint8_t"))
        L.append("  }")
    L.append("  return 0;\n}")
    return "\n".join(L)


def emit_fill_into(ir, name, backend):
    """A singular child's group, inlined into its parent's (ABI v1 section 6)."""
    L = ["static int fill_%s_%s_into(struct ak_efix_%s *e, PyObject *ob, HostCtx *h) {"
         % (backend, name, name)]
    L.append("  (void)h; (void)e; (void)ob;")
    for f in cfields(ir, name):
        if f.card == "repeated" or f.kind in ("string", "bytes", "message"):
            raise IR.__dict__.get("Unsupported", NotImplementedError)(
                "%s.%s: an inlined child with a non-scalar field is outside this "
                "backend's cases; R1 says raise rather than skip" % (name, f.name))
        L.append("  {")
        L.append(read_scalar(backend, f, "sv_" + f.name, "    "))
        L.append("    if (sv_%s) e->%s = (%s)sv_%s;"
                 % (f.name, f.name,
                    "int32_t" if f.kind in ("int32", "enum") else "int64_t", f.name))
        L.append("  }")
    L.append("  return 0;\n}")
    return "\n".join(L)


def emit_loop(ir, root, f, backend):
    elem = f.of
    L = ["static int32_t loop_%s_%s(ak_enc_ctx *ctx, const void *obj, int64_t token) {"
         % (backend, f.name)]
    L += ["  (void)token;",
          "  HostCtx *h = (HostCtx *)obj;",
          "  PyObject *ob = h->root;"]
    if backend == "cext":
        L.append("  C%s *o = (C%s *)ob;" % (root, root))
    L.append(read_obj(backend, f, "lst", "  "))
    L += [
        "  Py_ssize_t n = PyList_Size(lst);",
        "  if (n < 0) { if (own_lst) Py_DECREF(lst); return -1; }",
        "  struct ak_efix_%s chunk[CHUNK_%s];" % (elem, elem.upper()),
        "  for (Py_ssize_t i = 0; i < n; i += CHUNK_%s) {" % elem.upper(),
        "    int32_t k = (int32_t)((n - i < CHUNK_%s) ? (n - i) : CHUNK_%s);"
        % (elem.upper(), elem.upper()),
        "    /* ABI v1 decision 9: bulk clear once, then assign only what differs. */",
        "    memset(chunk, 0, sizeof(chunk[0]) * (size_t)k);",
        "    for (int32_t j = 0; j < k; j++) {",
        "      BUMP(C_ITEM);",
        "      PyObject *e = PyList_GetItem(lst, i + j);",
        "      if (!e || fill_%s_%s(&chunk[j], e, h))" % (backend, elem),
        "        { if (own_lst) Py_DECREF(lst); return -1; }",
        "    }",
        "    if (ak_elem_%s(ctx, chunk, k))" % elem,
        "      { if (own_lst) Py_DECREF(lst); return -1; }",
        "  }",
        "  if (own_lst) Py_DECREF(lst);",
        "  return 0;\n}",
    ]
    return "\n".join(L)


def emit_root_fix(ir, root, backend):
    """The root's own group, in an int-returning helper so the field reader can `return
    -1` the way it does everywhere else."""
    L = ["static int fix_%s_%s(struct ak_efix_%s *fix, PyObject *ob, HostCtx *h) {"
         % (backend, root, root)]
    L.append("  (void)h; (void)ob;")
    if backend == "cext":
        L.append("  C%s *o = (C%s *)ob;" % (root, root))
    L.append("  memset(fix, 0, sizeof *fix);")
    for f in cfields(ir, root):
        if f.card == "repeated" or f.kind == "message":
            continue
        L.append("  {")
        L.append(read_scalar(backend, f, "sv_" + f.name, "    "))
        L.append("    if (sv_%s) fix->%s = (int32_t)sv_%s;" % (f.name, f.name, f.name))
        L.append("  }")
    L.append("  return 0;\n}")
    return "\n".join(L)


def emit_root_setlist(ir, root, f, backend):
    """Hand the decoded run to the root facade, from an int-returning helper."""
    L = ["static int setlist_%s_%s(PyObject *ob, PyObject *lst, HostCtx *h) {"
         % (backend, f.name)]
    L.append("  (void)h; (void)ob; (void)lst;")
    if backend == "cext":
        L.append("  C%s *o = (C%s *)ob;" % (root, root))
    L.append(write_field(backend, f, "Py_NewRef(lst)", "  ", fail="return -1;"))
    L.append("  return 0;\n}")
    return "\n".join(L)


def emit_encode_entry(ir, root, backend):
    loops = [f for f in cfields(ir, root) if f.card == "repeated"]
    L = ["static PyObject *encode_%s_%s(PyObject *rootobj, PyObject *acc) {"
         % (backend, root)]
    L += ["  HostCtx hs; memset(&hs, 0, sizeof hs);",
          "  hs.root = rootobj; hs.acc = acc;",
          "  HostCtx *h = &hs;",
          "  struct ak_efix_%s fix;" % root,
          "  if (fix_%s_%s(&fix, rootobj, h)) return NULL;" % (backend, root)]
    L.append("  static const struct ak_evt_%s VT = {%s};"
             % (root, ", ".join("loop_%s_%s" % (backend, f.name) for f in loops)))
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


def emit_set_elem(ir, name, backend, fixty, src, parent_pref=""):
    """Write every field of one decoded group into one facade object `ob`."""
    L = []
    if backend == "cext":
        L.append("    C%s *o = (C%s *)ob;" % (name, name))
    for f in cfields(ir, name):
        sp = "%s->%s%s" % (src, parent_pref, f.name)
        L.append("    { /* %s */" % f.name)
        if f.card == "repeated":
            L.append("      /* a run slot: delivered by its own vtable entry */")
        elif f.kind in ("string", "bytes"):
            L.append("      BUMP(C_READ);")
            L.append(write_field(backend, f, span_value(f, sp), "      "))
        elif f.kind == "message":
            L.append("      if (%s->presence & AK_DFIX_%s_PRESENT_%s) {"
                     % (src, name.upper(), f.name.upper()))
            L.append("        PyObject *child = PyObject_CallNoArgs(h->ty_%s);" % f.of)
            L.append("        if (!child) { h->failed = 1; return; }")
            L.append("        { PyObject *ob = child;")
            if backend == "cext":
                L.append("          C%s *o = (C%s *)ob; (void)o;" % (f.of, f.of))
            for sub in cfields(ir, f.of):
                L.append(write_field(backend, sub, "%s.%s" % (sp, sub.name), "          "))
            L.append("        }")
            L.append(write_field(backend, f, "child", "        "))
            L.append("      } else {")
            L.append(write_field(backend, f, "Py_NewRef(Py_None)", "        "))
            L.append("      }")
        else:
            L.append(write_field(backend, f, sp, "      "))
        L.append("    }")
    return "\n".join(L)


def emit_add_run(ir, root, f, backend):
    elem = f.of
    L = ["static void add_%s_%s(ak_dec_ctx *ctx, void *obj, int64_t tok,"
         % (backend, f.name),
         "                       const struct ak_dfix_%s *elems, int32_t n) {" % elem,
         "  (void)ctx; (void)tok;",
         "  HostCtx *h = (HostCtx *)obj;",
         "  if (h->failed) return;",
         "  for (int32_t i = 0; i < n; i++) {",
         "    /* ABI v1 7.4: a batched add may be called more than once per field."
         " Append. */",
         "    PyObject *ob = PyObject_CallNoArgs(h->ty_%s);" % elem,
         "    if (!ob) { h->failed = 1; return; }",
         "    const struct ak_dfix_%s *e = &elems[i];" % elem,
         emit_set_elem(ir, elem, backend, "ak_dfix_" + elem, "e"),
         "    BUMP(C_ITEM);",
         "    if (PyList_Append(h->list, ob) < 0) h->failed = 1;",
         "    Py_DECREF(ob);",
         "    if (h->failed) return;",
         "  }\n}"]
    return "\n".join(L)


def emit_apply(ir, root, backend):
    L = ["static void apply_%s_%s(ak_dec_ctx *ctx, void *obj," % (backend, root),
         "                         const struct ak_dfix_%s *fx) {" % root,
         "  (void)ctx;",
         "  HostCtx *h = (HostCtx *)obj;",
         "  if (h->failed) return;",
         "  PyObject *ob = h->root;"]
    if backend == "cext":
        L.append("  C%s *o = (C%s *)ob;" % (root, root))
    for f in cfields(ir, root):
        if f.card == "repeated" or f.kind == "message":
            continue
        L.append("  {")
        L.append(write_field(backend, f, "fx->%s" % f.name, "    "))
        L.append("  }")
    L.append("}")
    return "\n".join(L)


def emit_decode_entry(ir, root, backend):
    loops = [f for f in cfields(ir, root) if f.card == "repeated"]
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
        "  h.ty_%s = T->ty_%s;" % (root, root),
    ]
    seen = set()
    for f in cfields(ir, root):
        if f.card == "repeated":
            for n2 in (f.of,):
                if n2 not in seen:
                    seen.add(n2)
                    L.append("  h.ty_%s = T->ty_%s;" % (n2, n2))
            for sub in cfields(ir, f.of):
                if sub.kind == "message" and sub.of not in seen:
                    seen.add(sub.of)
                    L.append("  h.ty_%s = T->ty_%s;" % (sub.of, sub.of))
    lf = loops[0]
    L += [
        "  h.list = PyList_New(0);",
        "  if (!h.list) { Py_DECREF(rootobj); return NULL; }",
        "  static const struct ak_dvt_%s VT = {apply_%s_%s, NULL, NULL, add_%s_%s};"
        % (root, backend, root, backend, lf.name),
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
    ]
    L.append("  if (setlist_%s_%s(rootobj, h.list, &h))" % (backend, lf.name))
    L.append("    { Py_DECREF(h.list); Py_DECREF(rootobj); return NULL; }")
    L.append("  Py_DECREF(h.list);")
    L.append("  return rootobj;\n}")
    return "\n".join(L)


# --------------------------------------------------------------------------------------

PRELUDE = r'''/* @generated by ffi/poc/python/gen/generate.py from ffi/schema/shapes.json.
 * Do not edit. */
#include <Python.h>
#include <structmember.h>
#include <string.h>
#include "ak_abi.h"

/* README R5, counting build: what the SHIM does to CPython. The core counts its own
 * crossings and exports them through ak_enc_counters / ak_dec_counters, so the two halves
 * of the design are counted by the half that can see them. */
#ifdef AK_COUNT
static uint64_t CNT[4];
#define C_ATTR   0
#define C_READ   1
#define C_ITEM   2
#define C_PYCALL 3
#define BUMP(i) (CNT[i]++)
#else
#define BUMP(i) ((void)0)
#endif

/* Resolved once at module init rather than per call: ak_tc_utf8() is a function CALL
 * across the boundary and a generated shim would not make it per string. */
static ak_transcode_fn TC_UTF8, TC_BYTES;

#ifdef AK_COUNT
/* The CORE's own counters, accumulated per call before its context is freed. The core
 * counts what the core can see -- forward entries, reverse upcalls, transcodes, prefix
 * moves -- and the shim counts what it does to CPython. Neither half can count the
 * other's, which is why both are here rather than one being inferred. */
static struct AkCounters CORE_ENC, CORE_DEC;
#define CORE_ADD(dst, src) do { \
  (dst).forward += (src).forward; (dst).reverse += (src).reverse; \
  (dst).transcode += (src).transcode; (dst).prefix_moves += (src).prefix_moves; \
  (dst).prefix_bytes += (src).prefix_bytes; (dst).grows += (src).grows; } while (0)
#endif
'''


def emit(ir, scope, root):
    L = [PRELUDE, ""]
    L.append(emit_ctypes(ir, scope))
    # The chunk size, from the group's own size, so the 32 KB budget of ABI v1 section 6
    # is a property of the emitted code rather than of a constant someone typed.
    for name in scope:
        L.append("#define CHUNK_%s ((int32_t)(%d / sizeof(struct ak_efix_%s)) > 0 ? \\"
                 % (name.upper(), CHUNK_BYTES, name))
        L.append("                  (int32_t)(%d / sizeof(struct ak_efix_%s)) : 1)"
                 % (CHUNK_BYTES, name))
    L.append("")
    names = sorted({f.name for n in scope for f in cfields(ir, n)})
    for n in names:
        L.append("static PyObject *K_%s;" % n)
    L.append("")
    L.append("typedef struct {")
    for n in scope:
        L.append("  PyObject *ty_%s;" % n)
    L.append("} HostTypes;")
    L.append("")
    L.append("typedef struct {")
    L.append("  PyObject *root;      /* the facade root */")
    L.append("  const uint8_t *base; /* decode: ABI v1 7.4, spans are offsets into this */")
    L.append("  PyObject *acc;       /* pyacc backend: the accessor table */")
    L.append("  PyObject *list;      /* decode: the run being appended to */")
    L.append("  int failed;")
    for n in scope:
        L.append("  PyObject *ty_%s;" % n)
    L.append("} HostCtx;")
    L.append("")
    L.append("static int intern_keys(void) {")
    for n in names:
        L.append('  K_%s = PyUnicode_InternFromString("%s"); if (!K_%s) return -1;'
                 % (n, n, n))
    L.append("  TC_UTF8 = ak_tc_utf8(); TC_BYTES = ak_tc_bytes();")
    L.append("  return 0;\n}")
    L.append("")

    # A SINGULAR message child is inlined into its parent's group (ABI v1 section 6);
    # a REPEATED one is an element run and gets its own fill. The first spelling of this
    # set did not make the distinction and put ResultRaw in both, which the walker's
    # raise caught at generate time rather than in a wrong byte.
    inlined = sorted({f.of for n in scope for f in cfields(ir, n)
                      if f.kind == "message" and f.card != "repeated"})
    elems = sorted({f.of for n in scope for f in cfields(ir, n)
                    if f.card == "repeated"})

    for backend in ("attr", "cext", "pyacc"):
        L.append("/* ===================== backend: %s ===================== */" % backend)
        # The `cext` backend reads an inlined child's members straight out of the
        # child's struct, so it has no `fill_into` and emitting one would be an unused
        # function -- which -Werror turns into a build failure, which is the right place
        # for "this backend does not use that path" to show up.
        if backend != "cext":
            for n in inlined:
                L.append(emit_fill_into(ir, n, backend))
        for n in elems:
            L.append(emit_fill(ir, n, backend))
        for f in cfields(ir, root):
            if f.card == "repeated":
                L.append(emit_loop(ir, root, f, backend))
        L.append(emit_root_fix(ir, root, backend))
        L.append(emit_encode_entry(ir, root, backend))
        for f in cfields(ir, root):
            if f.card == "repeated":
                L.append(emit_root_setlist(ir, root, f, backend))
        L.append(emit_apply(ir, root, backend))
        for f in cfields(ir, root):
            if f.card == "repeated":
                L.append(emit_add_run(ir, root, f, backend))
        L.append(emit_decode_entry(ir, root, backend))
        L.append("")

    # The schema-free half of the module needs a way to reach the per-backend entry
    # points and the facade types without naming either, so the generator emits the
    # tables and `native/binding.c` stays a wrapper that knows no field names.
    L.append("/* ---- dispatch, so native/binding.c names no message and no field ---- */")
    L.append("#define AK_NTYPES %d" % len(scope))
    L.append("static const char *AK_TYPE_NAMES[AK_NTYPES] = {%s};"
             % ", ".join('"C%s"' % n for n in scope))
    L.append("static PyType_Spec *AK_TYPE_SPECS[AK_NTYPES] = {%s};"
             % ", ".join("&spec_%s" % n for n in scope))
    L.append("")
    L.append("static int types_from_seq(HostTypes *T, PyObject *seq) {")
    L.append("  if (!PySequence_Check(seq) || PySequence_Size(seq) != AK_NTYPES) {")
    L.append("    PyErr_Format(PyExc_TypeError, \"expected %d types\", AK_NTYPES);")
    L.append("    return -1;")
    L.append("  }")
    L.append("  PyObject **slot = &T->ty_%s;" % scope[0])
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
    L.append("static ak_enc_f AK_ENC[3] = {%s};"
             % ", ".join("encode_%s_%s" % (b, root) for b in ("attr", "cext", "pyacc")))
    L.append("static ak_dec_f AK_DEC[3] = {%s};"
             % ", ".join("decode_%s_%s" % (b, root) for b in ("attr", "cext", "pyacc")))
    return "\n".join(L)
