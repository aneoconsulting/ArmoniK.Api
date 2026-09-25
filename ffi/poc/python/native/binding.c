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
 *
 * FIX-PLAN WP5 step 5: the generated file is rendered by poc/codec/gen/py_capi.py from the
 * plan; the RPC structs and prototypes below come from the generated `ak_abi.h`, rendered
 * from `plan.rpc` (R-G4: nothing here restates `ak_client_opts` or a section 9 prototype);
 * `ak_init` is rendered once, from `plan.lifecycle`, and called first in `mod_exec` (R-G7);
 * and the two C-API calls newer than the 3.7 floor are under `PY_VERSION_HEX` (R-D4).
 *
 * AK_GEN_DIR selects which generated shim this build wraps (`gen/out` for the payload set,
 * `gen/out/corpus` for the corpus-schema core); the build passes -I for the same directory.
 */
#ifdef AK_RPC
/* This build uses the RPC half, so it does not decline the crypto provider (the plan's
 * default flags are what a CODEC binding passes). */
#define AK_INIT_FLAGS (AK_INIT_NO_PANIC_HOOK)
#endif
#ifdef AK_CORPUS
#include "../gen/out/corpus/binding.c"
#else
#include "../gen/out/binding.c"
#endif

typedef struct {
  PyObject *t[AK_NTYPES];
} mod_state;

static int backend_index(const char *name) {
  for (int i = 0; i < AK_NBACKENDS; i++)
    if (strcmp(AK_BACKENDS[i], name) == 0) return i;
  PyErr_Format(PyExc_ValueError, "unknown backend %s", name);
  return -1;
}

static PyObject *py_encode(PyObject *m, PyObject *args) {
  (void)m;
  const char *backend, *rootname;
  PyObject *root, *acc = NULL;
  int retain = 0;
  if (!PyArg_ParseTuple(args, "ssO|Op", &backend, &rootname, &root, &acc, &retain)) return NULL;
  int b = backend_index(backend);
  int r = root_index(rootname);
  if (b < 0 || r < 0) return NULL;
  if (acc == Py_None) acc = NULL;
  return AK_ENC[b][r](root, acc, retain);
}

static PyObject *py_decode(PyObject *m, PyObject *args) {
  (void)m;
  const char *backend, *rootname;
  PyObject *buf, *types, *acc = NULL;
  int retain = 0;
  unsigned long long zero = 0;
  if (!PyArg_ParseTuple(args, "ssOO|OpK", &backend, &rootname, &buf, &types, &acc, &retain, &zero))
    return NULL;
  if (acc == Py_None) acc = NULL;
  int b = backend_index(backend);
  int r = root_index(rootname);
  if (b < 0 || r < 0) return NULL;
  HostTypes T;
  memset(&T, 0, sizeof T);
  if (types_from_seq(&T, types)) return NULL;
  return AK_DEC[b][r](buf, acc, &T, retain, zero);
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

static PyObject *py_types(PyObject *m, PyObject *unused) {
  (void)m;
  (void)unused;
  PyObject *t = PyTuple_New(AK_NTYPES);
  if (!t) return NULL;
  for (int i = 0; i < AK_NTYPES; i++) {
    PyObject *s = PyUnicode_FromString(AK_TYPE_NAMES[i]);
    if (!s) { Py_DECREF(t); return NULL; }
    PyTuple_SET_ITEM(t, i, s);
  }
  return t;
}

/* Decision 11: the unknown-field positions of one root, as ("name", "msg"|"oneof", "path"),
 * in the order of the decode's `zero` mask bits. */
static PyObject *py_unk_positions(PyObject *m, PyObject *args) {
  (void)m;
  const char *rootname;
  if (!PyArg_ParseTuple(args, "s", &rootname)) return NULL;
  int r = root_index(rootname);
  if (r < 0) return NULL;
  PyObject *out = PyList_New(0);
  if (!out) return NULL;
  for (const char *const *p = AK_UNKPOS[r]; *p; p++) {
    PyObject *s = PyUnicode_FromString(*p);
    if (!s || PyList_Append(out, s) < 0) { Py_XDECREF(s); Py_DECREF(out); return NULL; }
    Py_DECREF(s);
  }
  return out;
}

/* Decision 11 rule 6, the wrong-root control: (reset code, decode code, delivered?). */
static PyObject *py_wrong_root(PyObject *m, PyObject *args) {
  (void)m;
  const char *a, *b;
  if (!PyArg_ParseTuple(args, "ss", &a, &b)) return NULL;
  int ia = root_index(a), ib = root_index(b);
  if (ia < 0 || ib < 0) return NULL;
  int32_t rr = 0, dr = 0;
  ak_py_wrong_root(ia, ib, &rr, &dr);
  return Py_BuildValue("(iiO)", (int)rr, (int)dr, AK_TRAP_DELIVERED ? Py_True : Py_False);
}

/* Buffers freed by the last decode's reclaim (never delivered): 0 after a successful decode. */
static PyObject *py_last_reclaimed(PyObject *m, PyObject *unused) {
  (void)m;
  (void)unused;
  return PyLong_FromUnsignedLong(AK_LAST_RECLAIMED);
}

/* Decision 11 port: decode contexts are per root per thread, created once and reused.
 * The count of contexts created through the thread key: the harness checks it stays at
 * one per (thread, root decoded), so a per-call allocation would show here. */
static PyObject *py_tls_created(PyObject *m, PyObject *unused) {
  (void)m; (void)unused;
  return PyLong_FromUnsignedLong(AK_TLS_CREATED);
}

/* The roots this shim's core carries, in dispatch order. */
static PyObject *py_roots(PyObject *m, PyObject *unused) {
  (void)m;
  (void)unused;
  PyObject *t = PyTuple_New(AK_NROOTS);
  if (!t) return NULL;
  for (int i = 0; i < AK_NROOTS; i++) {
    PyObject *s = PyUnicode_FromString(AK_ROOTS[i]);
    if (!s) { Py_DECREF(t); return NULL; }
    PyTuple_SET_ITEM(t, i, s);
  }
  return t;
}

/* The host's own layout facts, named: [(name, value)], what the import check compared. A
 * harness reads group sizes from here (chunk arithmetic) rather than restating them. */
static PyObject *py_layout_host(PyObject *m, PyObject *unused) {
  (void)m;
  (void)unused;
  PyObject *out = PyList_New(AK_LAYOUT_FACTS);
  if (!out) return NULL;
  for (int i = 0; i < AK_LAYOUT_FACTS; i++) {
    PyObject *t = Py_BuildValue("(sk)", AK_LAYOUT_NAMES[i], (unsigned long)AK_LAYOUT_HOST[i]);
    if (!t) { Py_DECREF(out); return NULL; }
    PyList_SET_ITEM(out, i, t);
  }
  return out;
}

#ifdef AK_RPC
/* ---------------------------------------------------------------------------------
 * ABI v1 section 9: the core's own RPC surface, bound to Python.
 *
 * This is what makes the RPC arm a GRID rather than a pair. `bench.py` moves the codec
 * with the transport held still; cell B below moves the transport with the CODEC held
 * still, by putting `SerializeToString` in front of the core's call. The core's transport
 * never sees a message type -- it moves opaque bytes -- so there is nothing schema-shaped
 * in here and it lives in this file rather than in the generated one.
 *
 * Three deliveries, and the GIL is the whole reason they are not interchangeable here:
 *
 *   blocking   -- `ak_call_unary`, with the GIL RELEASED across it. One host thread per
 *                 call in flight.
 *   queue      -- `ak_call_unary_q` posts and returns; a Python drainer blocks in
 *                 `ak_queue_next` with the GIL released and re-acquires it to hand the
 *                 bytes back. **No upcall at all**, and the drainer is a thread CPython
 *                 already knows.
 *   callback   -- `ak_call_unary_cb`, where the completion arrives on a tokio worker: a
 *                 thread CPython has never seen, which must `PyGILState_Ensure` before it
 *                 can touch a Python object and release it after. That acquisition is the
 *                 cost the queue mode does not pay, and measuring it is the point.
 * --------------------------------------------------------------------------------- */

/* Every type and prototype used below is in the generated ak_abi.h, from plan.rpc. */
typedef struct ak_completion ak_completion_t;

static void cap_rt_free(PyObject *c) { ak_runtime_destroy(PyCapsule_GetPointer(c, "ak_rt")); }
static void cap_cl_free(PyObject *c) { ak_client_destroy(PyCapsule_GetPointer(c, "ak_cl")); }
static void cap_q_free(PyObject *c) { ak_queue_destroy(PyCapsule_GetPointer(c, "ak_q")); }

static PyObject *py_rt_new(PyObject *m, PyObject *args) {
  (void)m;
  unsigned threads = 0;
  if (!PyArg_ParseTuple(args, "|I", &threads)) return NULL;
  void *rt = ak_runtime_new(threads);
  if (!rt) { PyErr_SetString(PyExc_RuntimeError, "ak_runtime_new"); return NULL; }
  return PyCapsule_New(rt, "ak_rt", cap_rt_free);
}

static PyObject *py_client_new(PyObject *m, PyObject *args) {
  (void)m;
  PyObject *rtc;
  const char *uri; Py_ssize_t ulen;
  if (!PyArg_ParseTuple(args, "Os#", &rtc, &uri, &ulen)) return NULL;
  void *rt = PyCapsule_GetPointer(rtc, "ak_rt");
  if (!rt) return NULL;
  void *cl;
  /* Connecting blocks on the tokio runtime; holding the GIL across it would stall every
   * other Python thread, and on a UDS the handshake is short but not free. */
  Py_BEGIN_ALLOW_THREADS
  cl = ak_client_new(rt, (const uint8_t *)uri, (size_t)ulen);
  Py_END_ALLOW_THREADS
  if (!cl) { PyErr_Format(PyExc_RuntimeError, "ak_client_new(%s)", uri); return NULL; }
  return PyCapsule_New(cl, "ak_cl", cap_cl_free);
}

/* The response bytes, copied into a PyBytes and the core's buffer released. The copy is
 * real and is counted: ABI v1 decision 13's borrowed span would remove it, and this arm
 * does not take it, so the RPC table prices the copying transport. */
static PyObject *take_bytes(struct ak_bytes *b) {
  PyObject *r = PyBytes_FromStringAndSize((const char *)b->ptr, (Py_ssize_t)b->len);
  ak_bytes_free(b);
  return r;
}

/* A completion's body, or None when the call FAILED (FIX-PLAN R-D3).
 *
 * The core completes a failed call with a non-zero status and EMPTY bytes (rpc.rs,
 * `empty_ak_bytes`), and `take_bytes` turned those into b"" -- which every decoder here
 * accepts as an empty message. So a caller that forgot the status got a successful decode
 * of nothing, and `rpc.py` did forget it on two of its three deliveries. None cannot be
 * decoded by anything, so a missed status check now fails loudly instead of timing a
 * failure as a cheap success. The bytes are still released on the failure path. */
static PyObject *completion_body(int32_t status, struct ak_bytes *bytesp) {
  if (status != 0) {
    ak_bytes_free(bytesp);
    Py_RETURN_NONE;
  }
  return take_bytes(bytesp);
}

static PyObject *py_call_unary(PyObject *m, PyObject *args) {
  (void)m;
  PyObject *clc;
  const char *path; Py_ssize_t plen;
  const char *req; Py_ssize_t rlen;
  if (!PyArg_ParseTuple(args, "Os#y#", &clc, &path, &plen, &req, &rlen)) return NULL;
  void *cl = PyCapsule_GetPointer(clc, "ak_cl");
  if (!cl) return NULL;
  struct ak_bytes out = {NULL, 0, NULL};
  int32_t rc;
  Py_BEGIN_ALLOW_THREADS
  rc = ak_call_unary(cl, (const uint8_t *)path, (size_t)plen,
                     (const uint8_t *)req, (size_t)rlen, &out);
  Py_END_ALLOW_THREADS
  if (rc != 0) { PyErr_Format(PyExc_RuntimeError, "ak_call_unary -> %d", (int)rc); return NULL; }
  return take_bytes(&out);
}

static PyObject *py_client_new_opts(PyObject *m, PyObject *args) {
  (void)m;
  PyObject *rtc;
  const char *uri; Py_ssize_t ulen;
  struct ak_client_opts o;
  memset(&o, 0, sizeof o);
  unsigned sw = 0, cw = 0, mr = 0, ms = 0;
  int aw = -1, nagle = -1;
  if (!PyArg_ParseTuple(args, "Os#|IIiIIi", &rtc, &uri, &ulen, &sw, &cw, &aw, &mr, &ms,
                        &nagle))
    return NULL;
  o.stream_window = sw; o.connection_window = cw; o.adaptive_window = aw;
  o.max_recv_message = mr; o.max_send_message = ms; o.tcp_nagle = nagle;
  void *rt = PyCapsule_GetPointer(rtc, "ak_rt");
  if (!rt) return NULL;
  void *cl;
  Py_BEGIN_ALLOW_THREADS
  cl = ak_client_new_opts(rt, (const uint8_t *)uri, (size_t)ulen, &o);
  Py_END_ALLOW_THREADS
  if (!cl) { PyErr_Format(PyExc_RuntimeError, "ak_client_new_opts(%s)", uri); return NULL; }
  return PyCapsule_New(cl, "ak_cl", cap_cl_free);
}

static PyObject *py_queue_new(PyObject *m, PyObject *unused) {
  (void)m; (void)unused;
  void *q = ak_queue_new();
  if (!q) { PyErr_SetString(PyExc_RuntimeError, "ak_queue_new"); return NULL; }
  return PyCapsule_New(q, "ak_q", cap_q_free);
}

static PyObject *py_queue_shutdown(PyObject *m, PyObject *qc) {
  (void)m;
  void *q = PyCapsule_GetPointer(qc, "ak_q");
  if (!q) return NULL;
  ak_queue_shutdown(q);
  Py_RETURN_NONE;
}

static PyObject *py_queue_next(PyObject *m, PyObject *args) {
  (void)m;
  PyObject *qc;
  unsigned long long timeout = ~0ULL;
  if (!PyArg_ParseTuple(args, "O|K", &qc, &timeout)) return NULL;
  void *q = PyCapsule_GetPointer(qc, "ak_q");
  if (!q) return NULL;
  ak_completion_t c;
  memset(&c, 0, sizeof c);
  int32_t rc;
  /* THE point of this mode: the drainer is a Python thread that lets the GIL go while it
   * waits and takes it back when there is something to hand over. No thread the core owns
   * ever touches a PyObject. */
  Py_BEGIN_ALLOW_THREADS
  rc = ak_queue_next(q, &c, (uint64_t)timeout);
  Py_END_ALLOW_THREADS
  if (rc == AK_QUEUE_TIMEOUT) Py_RETURN_NONE;
  if (rc == AK_QUEUE_SHUTDOWN) return Py_BuildValue("(KiO)", (unsigned long long)0, -1, Py_None);
  if (rc != 0) { PyErr_Format(PyExc_RuntimeError, "ak_queue_next -> %d", (int)rc); return NULL; }
  PyObject *b = completion_body(c.status, &c.bytes);
  if (!b) return NULL;
  return Py_BuildValue("(KiN)", (unsigned long long)c.tag, (int)c.status, b);
}

static PyObject *py_call_unary_q(PyObject *m, PyObject *args) {
  (void)m;
  PyObject *clc, *qc;
  const char *path; Py_ssize_t plen;
  const char *req; Py_ssize_t rlen;
  unsigned long long tag = 0;
  if (!PyArg_ParseTuple(args, "Os#y#OK", &clc, &path, &plen, &req, &rlen, &qc, &tag))
    return NULL;
  void *cl = PyCapsule_GetPointer(clc, "ak_cl");
  void *q = PyCapsule_GetPointer(qc, "ak_q");
  if (!cl || !q) return NULL;
  void *h = ak_call_unary_q(cl, (const uint8_t *)path, (size_t)plen,
                            (const uint8_t *)req, (size_t)rlen, q, (uint64_t)tag);
  if (!h) { PyErr_SetString(PyExc_RuntimeError, "ak_call_unary_q"); return NULL; }
  /* The handle is only for cancellation, which this arm does not exercise. Destroying it
   * immediately is safe -- it drops an abort handle, not the task -- and keeps the arm
   * from measuring a handle table the design does not require. */
  ak_call_destroy(h);
  Py_RETURN_NONE;
}

/* The callback trampoline. It runs on a TOKIO WORKER, a thread CPython has never seen, so
 * it has to acquire the GIL before it can do anything at all. That acquisition is what
 * this mode costs over the queue, and on this host it is the reason to expect the queue to
 * win -- the prediction the arm exists to check. */
typedef struct { PyObject *fn; } cb_ctx;

static void ak_py_trampoline(void *user, struct ak_completion *comp) {
  cb_ctx *ctx = user;
  PyGILState_STATE g = PyGILState_Ensure();
  PyObject *b = completion_body(comp->status, &comp->bytes);
  if (b) {
    PyObject *r = PyObject_CallFunction(ctx->fn, "KiO", (unsigned long long)comp->tag,
                                        (int)comp->status, b);
    Py_XDECREF(r);
    Py_DECREF(b);
  }
  if (PyErr_Occurred()) PyErr_WriteUnraisable(ctx->fn);
  Py_DECREF(ctx->fn);
  PyMem_Free(ctx);
  PyGILState_Release(g);
}

static PyObject *py_call_unary_cb(PyObject *m, PyObject *args) {
  (void)m;
  PyObject *clc, *fn;
  const char *path; Py_ssize_t plen;
  const char *req; Py_ssize_t rlen;
  unsigned long long tag = 0;
  if (!PyArg_ParseTuple(args, "Os#y#OK", &clc, &path, &plen, &req, &rlen, &fn, &tag))
    return NULL;
  void *cl = PyCapsule_GetPointer(clc, "ak_cl");
  if (!cl) return NULL;
  if (!PyCallable_Check(fn)) {
    PyErr_SetString(PyExc_TypeError, "callback must be callable");
    return NULL;
  }
  cb_ctx *ctx = PyMem_Malloc(sizeof *ctx);
  if (!ctx) return PyErr_NoMemory();
  ctx->fn = AK_NEWREF(fn);
  void *h = ak_call_unary_cb(cl, (const uint8_t *)path, (size_t)plen,
                             (const uint8_t *)req, (size_t)rlen,
                             ak_py_trampoline, ctx, (uint64_t)tag);
  if (!h) {
    Py_DECREF(ctx->fn);
    PyMem_Free(ctx);
    PyErr_SetString(PyExc_RuntimeError, "ak_call_unary_cb");
    return NULL;
  }
  ak_call_destroy(h);
  Py_RETURN_NONE;
}

#endif /* AK_RPC */

static PyMethodDef methods[] = {
    {"encode", py_encode, METH_VARARGS,
     "encode(backend, rootname, obj[, accessors[, retain]]) -> bytes, through the shared core"},
    {"decode", py_decode, METH_VARARGS,
     "decode(backend, rootname, buf, types[, accessors[, retain[, zero_mask]]]) -> facade"},
    {"types", py_types, METH_NOARGS, "the facade type names, in HostTypes order"},
    {"roots", py_roots, METH_NOARGS, "the roots this shim's core carries"},
    {"unk_positions", py_unk_positions, METH_VARARGS, "decision 11: a root's unknown-field positions"},
    {"wrong_root", py_wrong_root, METH_VARARGS, "decision 11 rule 6 control: (reset rc, decode rc, delivered)"},
    {"last_reclaimed", py_last_reclaimed, METH_NOARGS, "buffers the last decode reclaimed undelivered"},
    {"tls_created", py_tls_created, METH_NOARGS, "decode contexts created through the per-thread key"},
    {"layout_host", py_layout_host, METH_NOARGS,
     "the shim's own layout facts, named, as compared with the core's at import"},
    {"layout_facts", py_layout_check, METH_NOARGS,
     "the core's own view of every group layout (ABI v1 section 10)"},
    {"core_counters", py_core_counters, METH_VARARGS, "reserved"},
    {"shim_counts", py_shim_counts, METH_NOARGS, "what the shim did to CPython"},
    {"reset_counts", py_reset_counts, METH_NOARGS, "zero the shim's counters"},
    {"counting", py_counting, METH_NOARGS, "is this the counting build"},
    {"abi_version", py_abi_version, METH_NOARGS, "ak_abi_version() from the core"},
    {"crossing", py_crossing, METH_VARARGS,
     "crossing(n[, 'forward'|'reverse']) -- the boundary, in this process"},
#ifdef AK_RPC
    {"rt_new", py_rt_new, METH_VARARGS, "ak_runtime_new(worker_threads) -> capsule"},
    {"client_new", py_client_new, METH_VARARGS, "ak_client_new(rt, uri) -> capsule"},
    {"client_new_opts", py_client_new_opts, METH_VARARGS,
     "client_new_opts(rt, uri[, stream_window, connection_window, adaptive,"
     " max_recv, max_send, nagle]) -> capsule. The transport PINNED (ABI v1 section 9)"},
    {"call_unary", py_call_unary, METH_VARARGS,
     "call_unary(client, path, req) -> bytes. Blocking, GIL released across the call"},
    {"queue_new", py_queue_new, METH_NOARGS, "ak_queue_new() -> capsule"},
    {"queue_shutdown", py_queue_shutdown, METH_O, "ak_queue_shutdown(queue)"},
    {"queue_next", py_queue_next, METH_VARARGS,
     "queue_next(queue[, timeout_ms]) -> (tag, status, bytes-or-None-on-failure) | None on"
     " timeout. GIL released"},
    {"call_unary_q", py_call_unary_q, METH_VARARGS,
     "call_unary_q(client, path, req, queue, tag). Posts and returns; NO upcall"},
    {"call_unary_cb", py_call_unary_cb, METH_VARARGS,
     "call_unary_cb(client, path, req, fn, tag). Completes on a tokio worker, which must"
     " PyGILState_Ensure first"},
#endif /* AK_RPC */
    {NULL, NULL, 0, NULL}};

static int mod_exec(PyObject *m) {
  /* ABI v1 section 3 (R-G7): ak_init before any other call into the core. */
  if (ak_py_init()) return -1;
  if (ak_py_tls_init()) { PyErr_SetString(PyExc_ImportError, "pthread_key_create failed"); return -1; }
  if (intern_keys()) return -1;
  if (ak_abi_version() != AK_ABI_VERSION) {
    PyErr_Format(PyExc_ImportError,
                 "ABI version mismatch: the core says %u, this shim was generated "
                 "against %u", (unsigned)ak_abi_version(), (unsigned)AK_ABI_VERSION);
    return -1;
  }
  /* ABI v1 section 10: the host's layout facts against the core's, at import. */
  if (ak_py_layout_check()) return -1;
  mod_state *st = (mod_state *)PyModule_GetState(m);
  for (int i = 0; i < AK_NTYPES; i++) {
    st->t[i] = PyType_FromSpec(AK_TYPE_SPECS[i]);
    if (!st->t[i]) return -1;
#if PY_VERSION_HEX >= 0x030A0000
    if (PyModule_AddObjectRef(m, AK_TYPE_NAMES[i], st->t[i]) < 0) return -1;
#else
    /* 3.7 to 3.9: PyModule_AddObject steals a reference only on success. */
    Py_INCREF(st->t[i]);
    if (PyModule_AddObject(m, AK_TYPE_NAMES[i], st->t[i]) < 0) { Py_DECREF(st->t[i]); return -1; }
#endif
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

/* The calling thread's decode contexts and the thread key go with the module; another
 * live thread's block is freed by the key's destructor when that thread exits, or not at
 * all if the process ends first. */
static void mod_free(void *m) { (void)mod_clear((PyObject *)m); ak_py_tls_fini(); }

static struct PyModuleDef moduledef = {
    PyModuleDef_HEAD_INIT, AK_MODNAME_STR,
    "the composed arm: the shared core behind a generated CPython shim",
    sizeof(mod_state), methods, mod_slots, mod_traverse, mod_clear, mod_free};

/* R-D4 at the floor: PyMODINIT_FUNC carries default visibility only from 3.9
 * (Py_EXPORTED_SYMBOL). Before that it is a bare `PyObject *`, so under this build's
 * -fvisibility=hidden a 3.7 shim exported no PyInit_* and could not be imported at all --
 * found by the first build against real 3.7 headers, not visible on 3.9+. */
#if PY_VERSION_HEX < 0x03090000 && defined(__GNUC__)
__attribute__((visibility("default")))
#endif
PyMODINIT_FUNC AK_INITFUNC(void) { return PyModuleDef_Init(&moduledef); }
