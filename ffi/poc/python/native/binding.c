/* The module wrapper around the GENERATED shim.
 *
 * `gen/out/binding.c` is emitted from `ffi/schema/shapes.json` and holds the C facade
 * types, the three accessor backends and both directions.  This file is the part that is
 * not a function of the schema: the module definition, the entry points, the layout
 * assertion and the counter readout.  **Nothing here names a message or a field**, which
 * is what the generated dispatch tables at the end of the emitted file are for.
 *
 * This is the COMPOSED arm (README 9.1's three layers, all three present): the Rust core
 * at `poc/codec/` does the wire, this shim speaks the CPython API, and the facade is
 * Python.  Work unit 1's `mech/` arms have no core in them and price one edge; this one
 * is the design.
 */
#include "../gen/out/binding.c"

typedef struct {
  PyObject *t[AK_NTYPES];
} mod_state;

static int backend_index(const char *name) {
  for (int i = 0; i < 3; i++)
    if (strcmp(AK_BACKENDS[i], name) == 0) return i;
  PyErr_Format(PyExc_ValueError, "unknown backend %s", name);
  return -1;
}

static PyObject *py_encode(PyObject *m, PyObject *args) {
  (void)m;
  const char *backend;
  PyObject *root, *acc = NULL;
  if (!PyArg_ParseTuple(args, "sO|O", &backend, &root, &acc)) return NULL;
  int b = backend_index(backend);
  if (b < 0) return NULL;
  return AK_ENC[b](root, acc);
}

static PyObject *py_decode(PyObject *m, PyObject *args) {
  (void)m;
  const char *backend;
  PyObject *buf, *types, *acc = NULL;
  if (!PyArg_ParseTuple(args, "sOO|O", &backend, &buf, &types, &acc)) return NULL;
  int b = backend_index(backend);
  if (b < 0) return NULL;
  HostTypes T;
  memset(&T, 0, sizeof T);
  if (types_from_seq(&T, types)) return NULL;
  return AK_DEC[b](buf, acc, &T);
}

/* ABI v1 section 10 and obligation 12.3.  The shim and the core restate the same group
 * layouts -- a C struct here and `#[repr(C)]` there -- which is exactly the case the
 * specification names, and whose failure mode is a wrong VALUE in a field.  Checked at
 * import, not asserted in prose. */
static PyObject *py_layout_check(PyObject *m, PyObject *unused) {
  (void)m;
  (void)unused;
  size_t n = ak_layout_facts(NULL, 0);
  uint32_t *core = (uint32_t *)PyMem_Malloc(n * sizeof(uint32_t));
  if (!core) return PyErr_NoMemory();
  size_t got = ak_layout_facts(core, n);
  PyObject *out = PyList_New(0);
  for (size_t i = 0; i < got && out; i++) {
    PyObject *v = PyLong_FromUnsignedLong(core[i]);
    if (!v || PyList_Append(out, v) < 0) Py_CLEAR(out);
    Py_XDECREF(v);
  }
  PyMem_Free(core);
  return out;
}

/* The core's own counters (README R5): forward and reverse crossings counted where they
 * happen rather than inferred from the host side.  Zero unless the core was built with
 * --features count. */
static PyObject *py_core_counters(PyObject *m, PyObject *args) {
  (void)m;
  const char *which = "enc";
  if (!PyArg_ParseTuple(args, "|s", &which)) return NULL;
#ifdef AK_COUNT
  const struct AkCounters *c = (strcmp(which, "dec") == 0) ? &CORE_DEC : &CORE_ENC;
  return Py_BuildValue("{s:K,s:K,s:K,s:K,s:K,s:K}",
                       "forward", (unsigned long long)c->forward,
                       "reverse", (unsigned long long)c->reverse,
                       "transcode", (unsigned long long)c->transcode,
                       "prefix_moves", (unsigned long long)c->prefix_moves,
                       "prefix_bytes", (unsigned long long)c->prefix_bytes,
                       "grows", (unsigned long long)c->grows);
#else
  Py_RETURN_NONE;
#endif
}

static PyObject *py_shim_counts(PyObject *m, PyObject *unused) {
  (void)m;
  (void)unused;
#ifdef AK_COUNT
  PyObject *d = PyDict_New();
  if (!d) return NULL;
  static const char *nm[4] = {"PyObject_GetAttr/SetAttr", "C-API value read",
                              "PyList element", "call into Python"};
  for (int i = 0; i < 4; i++) {
    PyObject *v = PyLong_FromUnsignedLongLong(CNT[i]);
    PyDict_SetItemString(d, nm[i], v);
    Py_DECREF(v);
  }
  return d;
#else
  Py_RETURN_NONE;
#endif
}

static PyObject *py_reset_counts(PyObject *m, PyObject *unused) {
  (void)m;
  (void)unused;
#ifdef AK_COUNT
  memset(CNT, 0, sizeof CNT);
  memset(&CORE_ENC, 0, sizeof CORE_ENC);
  memset(&CORE_DEC, 0, sizeof CORE_DEC);
#endif
  Py_RETURN_NONE;
}

static PyObject *py_counting(PyObject *m, PyObject *unused) {
  (void)m;
  (void)unused;
#ifdef AK_COUNT
  Py_RETURN_TRUE;
#else
  Py_RETURN_FALSE;
#endif
}

static PyObject *py_abi_version(PyObject *m, PyObject *unused) {
  (void)m;
  (void)unused;
  return PyLong_FromUnsignedLong(ak_abi_version());
}

/* The boundary priced in THIS process and THIS build, which is what lets a figure here
 * be quoted as a multiple of a crossing rather than only in nanoseconds (README R13). */
static uint64_t host_noop(uint64_t x) { return x ^ 2; }

static PyObject *py_crossing(PyObject *m, PyObject *args) {
  (void)m;
  Py_ssize_t n;
  const char *kind = "forward";
  if (!PyArg_ParseTuple(args, "n|s", &n, &kind)) return NULL;
  uint64_t x = 1;
  if (strcmp(kind, "forward") == 0) {
    for (Py_ssize_t i = 0; i < n; i++) x = ak_noop(x);
  } else {
    for (Py_ssize_t i = 0; i < n; i++) x = ak_noop_reverse(host_noop, x);
  }
  return PyLong_FromUnsignedLongLong(x);
}

static PyMethodDef methods[] = {
    {"encode", py_encode, METH_VARARGS,
     "encode(backend, root[, accessors]) -> bytes, through the shared core"},
    {"decode", py_decode, METH_VARARGS,
     "decode(backend, buf, types[, accessors]) -> facade, through the shared core"},
    {"layout_facts", py_layout_check, METH_NOARGS,
     "the core's own view of every group layout (ABI v1 section 10)"},
    {"core_counters", py_core_counters, METH_VARARGS, "reserved"},
    {"shim_counts", py_shim_counts, METH_NOARGS, "what the shim did to CPython"},
    {"reset_counts", py_reset_counts, METH_NOARGS, "zero the shim's counters"},
    {"counting", py_counting, METH_NOARGS, "is this the counting build"},
    {"abi_version", py_abi_version, METH_NOARGS, "ak_abi_version() from the core"},
    {"crossing", py_crossing, METH_VARARGS,
     "crossing(n[, 'forward'|'reverse']) -- the boundary, in this process"},
    {NULL, NULL, 0, NULL}};

static int mod_exec(PyObject *m) {
  if (intern_keys()) return -1;
  if (ak_abi_version() != AK_ABI_VERSION) {
    PyErr_Format(PyExc_ImportError,
                 "ABI version mismatch: the core says %u, this shim was generated "
                 "against %u", (unsigned)ak_abi_version(), (unsigned)AK_ABI_VERSION);
    return -1;
  }
  mod_state *st = (mod_state *)PyModule_GetState(m);
  for (int i = 0; i < AK_NTYPES; i++) {
    st->t[i] = PyType_FromSpec(AK_TYPE_SPECS[i]);
    if (!st->t[i]) return -1;
    if (PyModule_AddObjectRef(m, AK_TYPE_NAMES[i], st->t[i]) < 0) return -1;
  }
  return 0;
}

static PyModuleDef_Slot mod_slots[] = {{Py_mod_exec, (void *)mod_exec}, {0, NULL}};

static int mod_traverse(PyObject *m, visitproc visit, void *arg) {
  mod_state *st = (mod_state *)PyModule_GetState(m);
  for (int i = 0; i < AK_NTYPES; i++) Py_VISIT(st->t[i]);
  return 0;
}

static int mod_clear(PyObject *m) {
  mod_state *st = (mod_state *)PyModule_GetState(m);
  for (int i = 0; i < AK_NTYPES; i++) Py_CLEAR(st->t[i]);
  return 0;
}

static struct PyModuleDef moduledef = {
    PyModuleDef_HEAD_INIT, AK_MODNAME_STR,
    "the composed arm: the shared core behind a generated CPython shim",
    sizeof(mod_state), methods, mod_slots, mod_traverse, mod_clear, NULL};

PyMODINIT_FUNC AK_INITFUNC(void) { return PyModuleDef_Init(&moduledef); }
