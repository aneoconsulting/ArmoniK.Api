/* The module wrapper around the GENERATED encode backends.
 *
 * `gen/out/_akcodec_gen.c` is emitted from `ffi/schema/shapes.json` and holds
 * the C facade types and the three encode backends.  This file is the part that
 * is not a function of the schema: the module definition, the entry points and
 * the counting build's readout.  Nothing here knows a field name.
 *
 * What it does NOT contain, stated because the omission is the point: there is
 * no Rust core in this artifact.  Work unit 1 prices the **shim-to-facade**
 * half of README 9.1's three layers -- the half that is one crossing per field
 * and is therefore where the design's cost lives.  The shim-to-core half is a
 * plain C call across a .so and is priced separately in `bench_mech.py` group 1
 * (forward) and group 2's C-to-C floor (reverse).
 */
#include "../gen/out/_akcodec_gen.c"

typedef struct {
  PyObject *t[3];
} codec_state;

static const char *TYPE_NAMES[3] = {"CListResultsResponse", "CResultRaw",
                                    "CTimestamp"};
static PyType_Spec *TYPE_SPECS[3];

static PyObject *run(PyObject *obj, PyObject *getters,
                     int (*enc)(PyObject *, Buf *, PyObject *)) {
  Buf b = {NULL, 0, 0};
  if (enc(obj, &b, getters)) {
    PyMem_Free(b.p);
    return NULL;
  }
  PyObject *r = PyBytes_FromStringAndSize((const char *)b.p, (Py_ssize_t)b.len);
  PyMem_Free(b.p);
  return r;
}

static PyObject *py_encode_attr(PyObject *m, PyObject *o) {
  (void)m;
  return run(o, NULL, enc_attr_ListResultsResponse);
}

static PyObject *py_encode_cext(PyObject *m, PyObject *o) {
  (void)m;
  return run(o, NULL, enc_cext_ListResultsResponse);
}

static PyObject *py_encode_pyacc(PyObject *m, PyObject *args) {
  (void)m;
  PyObject *o, *getters;
  if (!PyArg_ParseTuple(args, "OO!", &o, &PyDict_Type, &getters)) return NULL;
  return run(o, getters, enc_pyacc_ListResultsResponse);
}

static PyObject *py_counts(PyObject *m, PyObject *unused) {
  (void)m;
  (void)unused;
#ifdef AK_COUNT
  PyObject *d = PyDict_New();
  if (!d) return NULL;
  static const char *nm[4] = {"PyObject_GetAttr", "C-API value read",
                              "PyList_GetItem", "call into Python"};
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
  memset(CNT, 0, sizeof(CNT));
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

static PyMethodDef methods[] = {
    {"encode_attr", py_encode_attr, METH_O,
     "encode through PyObject_GetAttr: serves the plain and __slots__ facades"},
    {"encode_cext", py_encode_cext, METH_O,
     "encode through a struct member read on the generated C facade type"},
    {"encode_pyacc", py_encode_pyacc, METH_VARARGS,
     "THE PREMISE CONTROL: one Python-level accessor CALL per field"},
    {"counts", py_counts, METH_NOARGS, "crossing counts, counting build only"},
    {"reset_counts", py_reset_counts, METH_NOARGS, "zero the counters"},
    {"counting", py_counting, METH_NOARGS, "is this the counting build"},
    {NULL, NULL, 0, NULL}};

static int codec_exec(PyObject *m) {
  if (intern_keys()) return -1;
  TYPE_SPECS[0] = &spec_ListResultsResponse;
  TYPE_SPECS[1] = &spec_ResultRaw;
  TYPE_SPECS[2] = &spec_Timestamp;
  codec_state *st = (codec_state *)PyModule_GetState(m);
  for (int i = 0; i < 3; i++) {
    st->t[i] = PyType_FromSpec(TYPE_SPECS[i]);
    if (!st->t[i]) return -1;
    if (PyModule_AddObjectRef(m, TYPE_NAMES[i], st->t[i]) < 0) return -1;
  }
  return 0;
}

static PyModuleDef_Slot codec_slots[] = {{Py_mod_exec, (void *)codec_exec},
                                         {0, NULL}};

static int codec_traverse(PyObject *m, visitproc visit, void *arg) {
  codec_state *st = (codec_state *)PyModule_GetState(m);
  for (int i = 0; i < 3; i++) Py_VISIT(st->t[i]);
  return 0;
}

static int codec_clear(PyObject *m) {
  codec_state *st = (codec_state *)PyModule_GetState(m);
  for (int i = 0; i < 3; i++) Py_CLEAR(st->t[i]);
  return 0;
}

static struct PyModuleDef moduledef = {
    PyModuleDef_HEAD_INIT, AK_MODNAME_STR,
    "generated encode backends over the M1 subtree", sizeof(codec_state),
    methods, codec_slots, codec_traverse, codec_clear, NULL};

PyMODINIT_FUNC AK_INITFUNC(void) { return PyModuleDef_Init(&moduledef); }
