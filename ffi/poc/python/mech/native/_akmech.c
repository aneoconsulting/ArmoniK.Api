/* The mechanism and primitive-cost probe, as a CPython extension module.
 *
 * ONE source, compiled twice: once against the full C-API and once with
 * Py_LIMITED_API (abi3).  That is deliberate and it is what prices README 9.2's
 * "one wheel per minor version against one wheel across 3.x": the same
 * measurement, the same machine, the same process shape, with only the API
 * surface changed.  Where an operation does not exist under the limited API the
 * arm is COMPILED OUT AND NAMED, never silently skipped (README R1), and
 * `available()` is what the driver prints.
 *
 * What it measures, and why each one is here:
 *
 *  - forward: what it costs a Python host to reach native code at all.  The
 *    callee is `ak_noop` in libakmech_cabi.so for EVERY mechanism arm, so the
 *    ctypes / cffi / C-extension / PyO3 rows differ in the mechanism and in
 *    nothing else (README R7).
 *  - reverse into the interpreter: `PyObject_CallFunctionObjArgs` on a Python
 *    callable.  This is what ctypes and cffi do for a callback and it is the
 *    thing README 9.1 says the design avoids.
 *  - reverse as a C-API call on a primitive: the design's actual default.
 *  - a field read off each candidate facade storage: README 9.1's third bullet.
 *
 * Every loop runs `n` times INSIDE one forward call, so the driver divides by
 * `n` and the forward cost is amortised to nothing.  Every loop has a floor arm
 * (`op_floor`) with the operation removed and nothing else changed, because a
 * per-iteration figure that has not had its own loop subtracted is a figure
 * about the loop.
 */
#include <Python.h>
#include <structmember.h>
#include <string.h>
#include <stddef.h>
#include <stdint.h>

#ifdef Py_LIMITED_API
#define AK_ABI3 1
#else
#define AK_ABI3 0
#endif

/* The forward target lives in the plain C library so that every mechanism arm
 * calls the same callee.  Declared rather than included: cabi.c has no header
 * worth the file. */
extern uint64_t ak_noop(uint64_t x);
extern uint64_t ak_reverse_n(uint64_t (*cb)(uint64_t), uint64_t x, size_t n);
extern uint64_t ak_reverse_n_floor(uint64_t x, size_t n);

/* --------------------------------------------------------------------------
 * The C-extension facade type: storage candidate 3 of README 9.1.
 *
 * Idiomatic on the Python side (attributes, a constructor, a repr-able object);
 * a plain struct on the C side, so the shim reads a member instead of making a
 * crossing.  Laid out to mirror ResultRaw of design/SHAPES.md: 6 strings, an
 * enum, an int64, a bytes, a bool, 2 singular children.
 * ----------------------------------------------------------------------- */

typedef struct {
  PyObject_HEAD
  PyObject *session_id;      /* str */
  PyObject *name;            /* str */
  PyObject *owner_task_id;   /* str */
  PyObject *result_id;       /* str */
  PyObject *created_by;      /* str */
  PyObject *opaque_id;       /* bytes */
  PyObject *created_at;      /* CStamp or None */
  PyObject *completed_at;    /* CStamp or None */
  long long size;
  long status;
  int manual_deletion;
} CResultRaw;

typedef struct {
  PyObject_HEAD
  long long seconds;
  long nanos;
} CStamp;

static PyMemberDef cresult_members[] = {
    {"session_id", T_OBJECT_EX, offsetof(CResultRaw, session_id), 0, NULL},
    {"name", T_OBJECT_EX, offsetof(CResultRaw, name), 0, NULL},
    {"owner_task_id", T_OBJECT_EX, offsetof(CResultRaw, owner_task_id), 0, NULL},
    {"result_id", T_OBJECT_EX, offsetof(CResultRaw, result_id), 0, NULL},
    {"created_by", T_OBJECT_EX, offsetof(CResultRaw, created_by), 0, NULL},
    {"opaque_id", T_OBJECT_EX, offsetof(CResultRaw, opaque_id), 0, NULL},
    {"created_at", T_OBJECT_EX, offsetof(CResultRaw, created_at), 0, NULL},
    {"completed_at", T_OBJECT_EX, offsetof(CResultRaw, completed_at), 0, NULL},
    {"size", T_LONGLONG, offsetof(CResultRaw, size), 0, NULL},
    {"status", T_LONG, offsetof(CResultRaw, status), 0, NULL},
    {"manual_deletion", T_BOOL, offsetof(CResultRaw, manual_deletion), 0, NULL},
    {NULL}};

static PyMemberDef cstamp_members[] = {
    {"seconds", T_LONGLONG, offsetof(CStamp, seconds), 0, NULL},
    {"nanos", T_LONG, offsetof(CStamp, nanos), 0, NULL},
    {NULL}};

static int cresult_init(PyObject *self, PyObject *args, PyObject *kwds) {
  static char *kw[] = {"session_id", "name",       "owner_task_id", "result_id",
                       "created_by", "opaque_id",  "created_at",    "completed_at",
                       "size",       "status",     "manual_deletion", NULL};
  CResultRaw *o = (CResultRaw *)self;
  PyObject *a[8] = {NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL};
  long long size = 0;
  long status = 0;
  int man = 0;
  if (!PyArg_ParseTupleAndKeywords(args, kwds, "|OOOOOOOOLlp", kw, &a[0], &a[1],
                                   &a[2], &a[3], &a[4], &a[5], &a[6], &a[7],
                                   &size, &status, &man))
    return -1;
  PyObject **slot = &o->session_id;
  for (int i = 0; i < 8; i++) {
    PyObject *v = a[i] ? a[i] : Py_None;
    Py_INCREF(v);
    Py_XDECREF(slot[i]);
    slot[i] = v;
  }
  o->size = size;
  o->status = status;
  o->manual_deletion = man ? 1 : 0;
  return 0;
}

static int cresult_traverse(PyObject *self, visitproc visit, void *arg) {
  CResultRaw *o = (CResultRaw *)self;
  PyObject **slot = &o->session_id;
  for (int i = 0; i < 8; i++) Py_VISIT(slot[i]);
  return 0;
}

static int cresult_clear(PyObject *self) {
  CResultRaw *o = (CResultRaw *)self;
  PyObject **slot = &o->session_id;
  for (int i = 0; i < 8; i++) Py_CLEAR(slot[i]);
  return 0;
}

static void ak_free(PyObject *self) {
  /* Under the limited API a PyTypeObject is opaque, so tp_free is reached
     through PyType_GetSlot rather than through the struct.  One spelling for
     both builds keeps the two artifacts the same code. */
  PyTypeObject *t = Py_TYPE(self);
  freefunc f = (freefunc)PyType_GetSlot(t, Py_tp_free);
  f(self);
  Py_DECREF(t);
}

static void cresult_dealloc(PyObject *self) {
  PyObject_GC_UnTrack(self);
  cresult_clear(self);
  ak_free(self);
}

static int cstamp_init(PyObject *self, PyObject *args, PyObject *kwds) {
  static char *kw[] = {"seconds", "nanos", NULL};
  CStamp *o = (CStamp *)self;
  long long sec = 0;
  long nanos = 0;
  if (!PyArg_ParseTupleAndKeywords(args, kwds, "|Ll", kw, &sec, &nanos)) return -1;
  o->seconds = sec;
  o->nanos = nanos;
  return 0;
}

static void cstamp_dealloc(PyObject *self) { ak_free(self); }

static PyType_Slot cresult_slots[] = {
    {Py_tp_init, (void *)cresult_init},
    {Py_tp_members, (void *)cresult_members},
    {Py_tp_new, (void *)PyType_GenericNew},
    {Py_tp_dealloc, (void *)cresult_dealloc},
    {Py_tp_traverse, (void *)cresult_traverse},
    {Py_tp_clear, (void *)cresult_clear},
    {0, NULL}};

static PyType_Spec cresult_spec = {
    "_akmech.CResultRaw", sizeof(CResultRaw), 0,
    Py_TPFLAGS_DEFAULT | Py_TPFLAGS_BASETYPE | Py_TPFLAGS_HAVE_GC, cresult_slots};

static PyType_Slot cstamp_slots[] = {{Py_tp_init, (void *)cstamp_init},
                                     {Py_tp_members, (void *)cstamp_members},
                                     {Py_tp_new, (void *)PyType_GenericNew},
                                     {Py_tp_dealloc, (void *)cstamp_dealloc},
                                     {0, NULL}};

static PyType_Spec cstamp_spec = {"_akmech.CStamp", sizeof(CStamp), 0,
                                  Py_TPFLAGS_DEFAULT | Py_TPFLAGS_BASETYPE,
                                  cstamp_slots};

typedef struct {
  PyObject *cresult_type;
  PyObject *cstamp_type;
} akmech_state;

static akmech_state *getstate(PyObject *m) {
  return (akmech_state *)PyModule_GetState(m);
}

/* --------------------------------------------------------------------------
 * Forward: the Python host reaching native code.
 * ----------------------------------------------------------------------- */

/* The forward target, reached through the shared library, so this arm's callee
 * is exactly the callee of the ctypes and cffi arms. */
static PyObject *py_fwd_noop(PyObject *m, PyObject *arg) {
  (void)m;
  /* UNSIGNED, deliberately.  The first spelling used PyLong_AsLongLong and the
     correctness gate in bench_mech.py caught it: the arm raised on an input
     above 2^63 while the PyO3 and cffi arms answered, so the arms were not
     doing the same work.  Defect D1 in STATE.md. */
  unsigned long long x = PyLong_AsUnsignedLongLong(arg);
  if (x == (unsigned long long)-1 && PyErr_Occurred()) return NULL;
  return PyLong_FromUnsignedLongLong(ak_noop((uint64_t)x));
}

/* The same entry point with the crossing into the shared library removed: the
 * floor that says how much of a forward row is Python's own call machinery and
 * how much is the boundary. */
static PyObject *py_fwd_selfcontained(PyObject *m, PyObject *arg) {
  (void)m;
  unsigned long long x = PyLong_AsUnsignedLongLong(arg);
  if (x == (unsigned long long)-1 && PyErr_Occurred()) return NULL;
  return PyLong_FromUnsignedLongLong(x ^ 2);
}

/* METH_FASTCALL, which is what a generated binding would actually emit and what
 * ctypes and cffi cannot produce. */
static PyObject *py_fwd_fast(PyObject *m, PyObject *const *args,
                             Py_ssize_t nargs) {
  (void)m;
  if (nargs != 1) {
    PyErr_SetString(PyExc_TypeError, "one argument");
    return NULL;
  }
  unsigned long long x = PyLong_AsUnsignedLongLong(args[0]);
  if (x == (unsigned long long)-1 && PyErr_Occurred()) return NULL;
  return PyLong_FromUnsignedLongLong(ak_noop((uint64_t)x));
}

/* --------------------------------------------------------------------------
 * Reverse.
 * ----------------------------------------------------------------------- */

static PyObject *g_cb = NULL; /* the Python callable a C trampoline invokes */

/* A C function pointer the plain C library can call, which re-enters the
 * interpreter.  This is exactly the shape ctypes' CFUNCTYPE and cffi's
 * def_extern build for you, written out so the C-extension arm measures the
 * same thing they do rather than a different thing. */
static uint64_t trampoline_into_python(uint64_t x) {
  PyObject *a = PyLong_FromLongLong((long long)x);
  if (!a) return x;
  PyObject *r = PyObject_CallFunctionObjArgs(g_cb, a, NULL);
  Py_DECREF(a);
  if (!r) {
    PyErr_Clear();
    return x;
  }
  long long out = PyLong_AsLongLong(r);
  Py_DECREF(r);
  return (uint64_t)out;
}

/* n reverse calls INTO THE INTERPRETER, driven from inside the C library. */
static PyObject *py_rev_python(PyObject *m, PyObject *args) {
  (void)m;
  PyObject *cb;
  Py_ssize_t n;
  if (!PyArg_ParseTuple(args, "On", &cb, &n)) return NULL;
  PyObject *prev = g_cb;
  g_cb = cb;
  uint64_t r = ak_reverse_n(trampoline_into_python, 1, (size_t)n);
  g_cb = prev;
  return PyLong_FromUnsignedLongLong(r);
}

/* The same loop with the interpreter re-entry removed but the C->C crossing
 * kept: what the callback costs above a bare function-pointer call. */
static PyObject *py_rev_cfloor(PyObject *m, PyObject *arg) {
  (void)m;
  Py_ssize_t n = PyLong_AsSsize_t(arg);
  if (n == -1 && PyErr_Occurred()) return NULL;
  return PyLong_FromUnsignedLongLong(ak_reverse_n(ak_noop, 1, (size_t)n));
}

/* And the same loop with no crossing at all: the loop's own cost. */
static PyObject *py_rev_floor(PyObject *m, PyObject *arg) {
  (void)m;
  Py_ssize_t n = PyLong_AsSsize_t(arg);
  if (n == -1 && PyErr_Occurred()) return NULL;
  return PyLong_FromUnsignedLongLong(ak_reverse_n_floor(1, (size_t)n));
}

/* --------------------------------------------------------------------------
 * Reverse as a C-API call on a primitive: the design's default.
 *
 * One `op` per loop, dispatched ONCE before the loop, so no row carries a
 * switch.  `op_floor` is the same loop with the body removed.
 * ----------------------------------------------------------------------- */

static const char GUID36[] = "8c8deaf0-3e8d-bdcc-20d5-19afe07cc6c9";
static const char BLOB16[] = "\x01\x02\x03\x04\x05\x06\x07\x08"
                             "\x09\x0a\x0b\x0c\x0d\x0e\x0f\x10";

static PyObject *py_capi(PyObject *m, PyObject *args) {
  const char *op;
  Py_ssize_t n;
  PyObject *obj = Py_None;
  PyObject *key = Py_None;
  if (!PyArg_ParseTuple(args, "sn|OO", &op, &n, &obj, &key)) return NULL;
  akmech_state *st = getstate(m);
  volatile uint64_t sink = 0;

#define AK_OP(NAME) if (strcmp(op, NAME) == 0)

  AK_OP("floor") {
    for (Py_ssize_t i = 0; i < n; i++) sink += (uint64_t)i;
  }
  else AK_OP("PyLong_FromLongLong") {
    /* +2^20 so that every value is outside CPython's small-int cache and the
     * row prices an allocation rather than a table lookup. */
    for (Py_ssize_t i = 0; i < n; i++) {
      PyObject *o = PyLong_FromLongLong((long long)i + (1 << 20));
      sink += (uint64_t)(uintptr_t)o;
      Py_DECREF(o);
    }
  }
  else AK_OP("PyLong_AsLongLong") {
    for (Py_ssize_t i = 0; i < n; i++) sink += (uint64_t)PyLong_AsLongLong(obj);
  }
  else AK_OP("PyUnicode_FromStringAndSize(36)") {
    for (Py_ssize_t i = 0; i < n; i++) {
      PyObject *o = PyUnicode_FromStringAndSize(GUID36, 36);
      sink += (uint64_t)(uintptr_t)o;
      Py_DECREF(o);
    }
  }
  else AK_OP("PyUnicode_DecodeUTF8(36)") {
    for (Py_ssize_t i = 0; i < n; i++) {
      PyObject *o = PyUnicode_DecodeUTF8(GUID36, 36, "strict");
      sink += (uint64_t)(uintptr_t)o;
      Py_DECREF(o);
    }
  }
  else AK_OP("PyUnicode_AsUTF8AndSize") {
    for (Py_ssize_t i = 0; i < n; i++) {
      Py_ssize_t len = 0;
      const char *p = PyUnicode_AsUTF8AndSize(obj, &len);
      sink += (uint64_t)(uintptr_t)p + (uint64_t)len;
    }
  }
  else AK_OP("PyUnicode_AsUTF8String") {
    /* The limited API's alternative before 3.10, and still the only spelling a
     * host reaches for when it wants an owned buffer: it ALLOCATES a bytes
     * object per string where the row above returns a pointer into the str. */
    for (Py_ssize_t i = 0; i < n; i++) {
      PyObject *o = PyUnicode_AsUTF8String(obj);
      sink += (uint64_t)(uintptr_t)o;
      Py_XDECREF(o);
    }
  }
  else AK_OP("PyBytes_FromStringAndSize(16)") {
    for (Py_ssize_t i = 0; i < n; i++) {
      PyObject *o = PyBytes_FromStringAndSize(BLOB16, 16);
      sink += (uint64_t)(uintptr_t)o;
      Py_DECREF(o);
    }
  }
  else AK_OP("PyBytes_AsStringAndSize") {
    for (Py_ssize_t i = 0; i < n; i++) {
      char *p = NULL;
      Py_ssize_t len = 0;
      PyBytes_AsStringAndSize(obj, &p, &len);
      sink += (uint64_t)(uintptr_t)p + (uint64_t)len;
    }
  }
  else AK_OP("PyBool_FromLong") {
    for (Py_ssize_t i = 0; i < n; i++) {
      PyObject *o = PyBool_FromLong((long)(i & 1));
      sink += (uint64_t)(uintptr_t)o;
      Py_DECREF(o);
    }
  }
  else AK_OP("PyObject_GetAttr") {
    for (Py_ssize_t i = 0; i < n; i++) {
      PyObject *o = PyObject_GetAttr(obj, key);
      if (!o) { PyErr_Clear(); continue; }
      sink += (uint64_t)(uintptr_t)o;
      Py_DECREF(o);
    }
  }
  else AK_OP("PyObject_GetAttrString") {
    /* PyUnicode_AsUTF8AndSize is in the limited API from 3.10; the bare
       PyUnicode_AsUTF8 is not, which the abi3 build says by failing to
       declare it. One spelling for both builds. */
    Py_ssize_t klen = 0;
    const char *name = PyUnicode_AsUTF8AndSize(key, &klen);
    if (!name) return NULL;
    for (Py_ssize_t i = 0; i < n; i++) {
      PyObject *o = PyObject_GetAttrString(obj, name);
      if (!o) { PyErr_Clear(); continue; }
      sink += (uint64_t)(uintptr_t)o;
      Py_DECREF(o);
    }
  }
  else AK_OP("PyObject_SetAttr") {
    for (Py_ssize_t i = 0; i < n; i++) {
      if (PyObject_SetAttr(obj, key, Py_None) < 0) PyErr_Clear();
      sink += (uint64_t)i;
    }
  }
  else AK_OP("struct member read") {
    /* The C-extension facade: the shim casts and reads.  README 9.1's "a field
     * read stops being a crossing at all" -- this row is what that sentence is
     * worth in nanoseconds. The type check is the shim's, and it is kept in
     * because a generated shim would have to do it once per object. */
    if (!PyObject_TypeCheck(obj, (PyTypeObject *)st->cresult_type)) {
      PyErr_SetString(PyExc_TypeError, "needs a CResultRaw");
      return NULL;
    }
    CResultRaw *o = (CResultRaw *)obj;
    for (Py_ssize_t i = 0; i < n; i++) sink += (uint64_t)(uintptr_t)o->session_id;
  }
  else AK_OP("struct member read (int64)") {
    if (!PyObject_TypeCheck(obj, (PyTypeObject *)st->cresult_type)) {
      PyErr_SetString(PyExc_TypeError, "needs a CResultRaw");
      return NULL;
    }
    CResultRaw *o = (CResultRaw *)obj;
    for (Py_ssize_t i = 0; i < n; i++) sink += (uint64_t)o->size;
  }
  else AK_OP("PyList_New(4)") {
    for (Py_ssize_t i = 0; i < n; i++) {
      PyObject *o = PyList_New(4);
      sink += (uint64_t)(uintptr_t)o;
      Py_DECREF(o);
    }
  }
  else AK_OP("PyList_SetItem") {
    /* Stable-ABI spelling: bounds-checked, steals a reference. */
    for (Py_ssize_t i = 0; i < n; i++) {
      Py_INCREF(Py_None);
      if (PyList_SetItem(obj, i & 3, Py_None) < 0) PyErr_Clear();
      sink += (uint64_t)i;
    }
  }
#if !AK_ABI3
  else AK_OP("PyList_SET_ITEM") {
    /* The macro.  NOT in the limited API, and that absence is the point of the
     * abi3 column: under abi3 this row does not exist and the row above is what
     * a shim must use. */
    for (Py_ssize_t i = 0; i < n; i++) {
      PyObject *old = PyList_GET_ITEM(obj, i & 3);
      Py_INCREF(Py_None);
      PyList_SET_ITEM(obj, i & 3, Py_None);
      Py_XDECREF(old);
      sink += (uint64_t)i;
    }
  }
#endif
  else AK_OP("PyList_Append") {
    for (Py_ssize_t i = 0; i < n; i++) {
      if (PyList_Append(obj, Py_None) < 0) PyErr_Clear();
      sink += (uint64_t)i;
    }
    /* Leave the list as we found it: the driver reuses it across rounds. */
    if (PyList_SetSlice(obj, 0, PyList_Size(obj), NULL) < 0) PyErr_Clear();
  }
  else AK_OP("PyUnicode_AsUTF8AndSize (uncached: build + read)") {
    /* CPython caches a non-ASCII str's UTF-8 form ON THE OBJECT at the first
       PyUnicode_AsUTF8AndSize, so the row above prices the CACHED path. A
       string that just came off the wire has no cache. This row and the one
       below differ by exactly the read, which is the within-arm delta README
       R4 asks for rather than a ratio to a third arm. */
    Py_ssize_t klen = 0;
    const char *src = PyUnicode_AsUTF8AndSize(obj, &klen);
    if (!src) return NULL;
    char *copy = (char *)PyMem_Malloc((size_t)klen + 1);
    if (!copy) return PyErr_NoMemory();
    memcpy(copy, src, (size_t)klen + 1);
    for (Py_ssize_t i = 0; i < n; i++) {
      PyObject *o = PyUnicode_DecodeUTF8(copy, klen, "strict");
      if (!o) { PyErr_Clear(); continue; }
      Py_ssize_t len = 0;
      const char *p2 = PyUnicode_AsUTF8AndSize(o, &len);
      sink += (uint64_t)(uintptr_t)p2 + (uint64_t)len;
      Py_DECREF(o);
    }
    PyMem_Free(copy);
  }
  else AK_OP("PyUnicode_AsUTF8AndSize (uncached: build only)") {
    Py_ssize_t klen = 0;
    const char *src = PyUnicode_AsUTF8AndSize(obj, &klen);
    if (!src) return NULL;
    char *copy = (char *)PyMem_Malloc((size_t)klen + 1);
    if (!copy) return PyErr_NoMemory();
    memcpy(copy, src, (size_t)klen + 1);
    for (Py_ssize_t i = 0; i < n; i++) {
      PyObject *o = PyUnicode_DecodeUTF8(copy, klen, "strict");
      if (!o) { PyErr_Clear(); continue; }
      sink += (uint64_t)(uintptr_t)o;
      Py_DECREF(o);
    }
    PyMem_Free(copy);
  }
  else AK_OP("Py_BEGIN/END_ALLOW_THREADS") {
    /* README 9.1's last bullet and the GIL hazard: the interesting property of
       a batched crossing is that the pure parse can run with the GIL RELEASED.
       What that costs per release is this row, and it is what says how large a
       batch has to be before releasing is worth doing at all. */
    for (Py_ssize_t i = 0; i < n; i++) {
      Py_BEGIN_ALLOW_THREADS
      sink += (uint64_t)i;
      Py_END_ALLOW_THREADS
    }
  }
  else AK_OP("PyObject_CallNoArgs(type)") {
    /* Constructing one facade element: what a decode shim pays per element
     * before it has written a single field.  PyObject_CallNoArgs is in the
     * limited API from 3.10, so one spelling serves both builds. */
    for (Py_ssize_t i = 0; i < n; i++) {
      PyObject *o = PyObject_CallNoArgs(obj);
      if (!o) { PyErr_Clear(); continue; }
      sink += (uint64_t)(uintptr_t)o;
      Py_DECREF(o);
    }
  }
  else {
    PyErr_Format(PyExc_ValueError, "unknown op %s", op);
    return NULL;
  }
#undef AK_OP

  if (PyErr_Occurred()) return NULL;
  return PyLong_FromUnsignedLongLong((uint64_t)sink);
}

static PyObject *py_available(PyObject *m, PyObject *unused) {
  (void)m;
  (void)unused;
  PyObject *d = PyDict_New();
  if (!d) return NULL;
  PyDict_SetItemString(d, "abi3", AK_ABI3 ? Py_True : Py_False);
#ifdef Py_LIMITED_API
  PyObject *v = PyLong_FromUnsignedLong((unsigned long)Py_LIMITED_API);
#else
  PyObject *v = Py_NewRef(Py_None);
#endif
  PyDict_SetItemString(d, "limited_api", v);
  Py_DECREF(v);
  PyObject *ops = PyList_New(0);
  const char *names[] = {"floor",
                         "PyLong_FromLongLong",
                         "PyLong_AsLongLong",
                         "PyUnicode_FromStringAndSize(36)",
                         "PyUnicode_DecodeUTF8(36)",
                         "PyUnicode_AsUTF8AndSize",
                         "PyUnicode_AsUTF8String",
                         "PyBytes_FromStringAndSize(16)",
                         "PyBytes_AsStringAndSize",
                         "PyBool_FromLong",
                         "PyObject_GetAttr",
                         "PyObject_GetAttrString",
                         "PyObject_SetAttr",
                         "struct member read",
                         "struct member read (int64)",
                         "PyList_New(4)",
                         "PyList_SetItem",
#if !AK_ABI3
                         "PyList_SET_ITEM",
#endif
                         "PyList_Append",
                         "PyUnicode_AsUTF8AndSize (uncached: build + read)",
                         "PyUnicode_AsUTF8AndSize (uncached: build only)",
                         "Py_BEGIN/END_ALLOW_THREADS",
                         "PyObject_CallNoArgs(type)",
                         NULL};
  for (int i = 0; names[i]; i++) {
    PyObject *s = PyUnicode_FromString(names[i]);
    PyList_Append(ops, s);
    Py_DECREF(s);
  }
  PyDict_SetItemString(d, "ops", ops);
  Py_DECREF(ops);
  return d;
}

static PyMethodDef methods[] = {
    {"fwd_noop", py_fwd_noop, METH_O,
     "one forward crossing into libakmech_cabi.so"},
    {"fwd_selfcontained", py_fwd_selfcontained, METH_O,
     "the same entry point with the crossing removed"},
    {"fwd_fast", (PyCFunction)(void (*)(void))py_fwd_fast, METH_FASTCALL,
     "the same, through METH_FASTCALL"},
    {"rev_python", py_rev_python, METH_VARARGS,
     "n reverse calls into the interpreter, driven from C"},
    {"rev_cfloor", py_rev_cfloor, METH_O,
     "n C->C indirect calls, no interpreter"},
    {"rev_floor", py_rev_floor, METH_O, "the loop with no call at all"},
    {"capi", py_capi, METH_VARARGS, "n repetitions of one C-API operation"},
    {"available", py_available, METH_NOARGS, "what this build carries"},
    {NULL, NULL, 0, NULL}};

static int akmech_exec(PyObject *m) {
  akmech_state *st = getstate(m);
  st->cresult_type = PyType_FromSpec(&cresult_spec);
  if (!st->cresult_type) return -1;
  if (PyModule_AddObjectRef(m, "CResultRaw", st->cresult_type) < 0) return -1;
  st->cstamp_type = PyType_FromSpec(&cstamp_spec);
  if (!st->cstamp_type) return -1;
  if (PyModule_AddObjectRef(m, "CStamp", st->cstamp_type) < 0) return -1;
  return 0;
}

static PyModuleDef_Slot akmech_slots[] = {
    {Py_mod_exec, (void *)akmech_exec}, {0, NULL}};

static int akmech_traverse(PyObject *m, visitproc visit, void *arg) {
  akmech_state *st = getstate(m);
  Py_VISIT(st->cresult_type);
  Py_VISIT(st->cstamp_type);
  return 0;
}

static int akmech_clear(PyObject *m) {
  akmech_state *st = getstate(m);
  Py_CLEAR(st->cresult_type);
  Py_CLEAR(st->cstamp_type);
  return 0;
}

static struct PyModuleDef moduledef = {PyModuleDef_HEAD_INIT,
                                       AK_MODNAME_STR,
                                       "mechanism and primitive probe",
                                       sizeof(akmech_state),
                                       methods,
                                       akmech_slots,
                                       akmech_traverse,
                                       akmech_clear,
                                       NULL};

PyMODINIT_FUNC AK_INITFUNC(void) { return PyModuleDef_Init(&moduledef); }
