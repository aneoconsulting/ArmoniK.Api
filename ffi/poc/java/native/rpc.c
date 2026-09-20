/* ABI v1 section 9's RPC half, reached from the JVM.
 *
 * This is the arm the branch's outcome 2 actually proposes: the RPC layer lives in the
 * core, so a host calls `ak_call_unary` and tonic does the HTTP/2. An arm that keeps
 * grpc-java and swaps only the marshaller measures the CODEC inside somebody else's
 * transport, which `RunR14` already did without a server; it is not the architecture.
 *
 * Two crossings per call and none per field, which is the property section 9 claims and
 * the rust slice measured. They are kept two here rather than folded into one:
 *
 *   1. `ak_call_unary` -- returns a borrowed {ptr, len, owner} the core still owns;
 *   2. `ak_bytes_free` -- the host is done with the bytes.
 *
 * The response copy in between is done by `Unsafe.copyMemory` on the Java side, which is
 * an intrinsic and NOT a crossing. Folding the copy into the call would have made this one
 * crossing and flattered the arm against its own specification.
 *
 * Declared here rather than in `ak_abi.h`: the shared header does not carry section 9
 * either, for the same reason it does not carry the pull family. Filed as a request.
 */
#include <jni.h>
#include <stdint.h>
#include <stddef.h>

typedef struct {
  const uint8_t *ptr;
  size_t len;
  void *owner;
} ak_bytes;

void   *ak_runtime_new(uint32_t worker_threads);
void    ak_runtime_destroy(void *r);
void   *ak_client_new(void *r, const uint8_t *uri, size_t uri_len);
void    ak_client_destroy(void *c);
int32_t ak_call_unary(void *c, const uint8_t *path, size_t path_len,
                      const uint8_t *req, size_t req_len, ak_bytes *out);
void    ak_bytes_free(ak_bytes *b);

JNIEXPORT jlong JNICALL Java_ak_NativeRpc_runtimeNew(JNIEnv *e, jclass c, jint threads) {
  (void) e; (void) c;
  return (jlong)(intptr_t) ak_runtime_new((uint32_t) threads);
}

JNIEXPORT void JNICALL Java_ak_NativeRpc_runtimeDestroy(JNIEnv *e, jclass c, jlong r) {
  (void) e; (void) c;
  ak_runtime_destroy((void *)(intptr_t) r);
}

JNIEXPORT jlong JNICALL Java_ak_NativeRpc_clientNew(JNIEnv *env, jclass c, jlong r,
                                                    jbyteArray uri, jint len) {
  (void) c;
  jbyte *u = (*env)->GetByteArrayElements(env, uri, NULL);
  void *cl = ak_client_new((void *)(intptr_t) r, (const uint8_t *) u, (size_t) len);
  (*env)->ReleaseByteArrayElements(env, uri, u, JNI_ABORT);
  return (jlong)(intptr_t) cl;
}

JNIEXPORT void JNICALL Java_ak_NativeRpc_clientDestroy(JNIEnv *e, jclass c, jlong cl) {
  (void) e; (void) c;
  ak_client_destroy((void *)(intptr_t) cl);
}

/* Crossing one. `out` receives {ptr, len, owner}; the core still owns the bytes.
 *
 * THE REQUEST IS COPIED, NOT PINNED, AND THE REASON IS A DEADLOCK.
 *
 * The first version held `GetPrimitiveArrayCritical` across the whole call, by analogy
 * with the pull family's parse: that call makes no upcall, so a critical section over it
 * is legal. The analogy is false for a BLOCKING call. Making no upcall is what makes a
 * critical section legal under JNI's rules; it does not make it safe when the call cannot
 * complete until other Java threads make progress. A critical section blocks GC, the peer
 * is a grpc-java server in this same process, and that server must allocate to produce a
 * response -- so a GC needed inside the window waits for a critical section that waits for
 * the server that waits for the GC. It hung on the first small payload, having survived a
 * large one by luck of timing.
 *
 * The rule generalises past this arm and past RPC: a host must not pin a Java array across
 * an ABI call whose completion depends on another Java thread. `GetByteArrayElements` with
 * a copy is correct here, and the request is small by construction -- it is a request. */
JNIEXPORT jint JNICALL Java_ak_NativeRpc_callUnary(JNIEnv *env, jclass c, jlong cl,
                                                   jlong pathPtr, jint pathLen,
                                                   jbyteArray req, jint reqOff, jint reqLen,
                                                   jlongArray out) {
  (void) c;
  ak_bytes b = {NULL, 0, NULL};
  jbyte *base = (*env)->GetByteArrayElements(env, req, NULL);
  if (base == NULL) return -1;
  int32_t rc = ak_call_unary((void *)(intptr_t) cl,
                             (const uint8_t *)(intptr_t) pathPtr, (size_t) pathLen,
                             (const uint8_t *) base + reqOff, (size_t) reqLen, &b);
  (*env)->ReleaseByteArrayElements(env, req, base, JNI_ABORT);
  if (rc == 0) {
    jlong v[3];
    v[0] = (jlong)(intptr_t) b.ptr;
    v[1] = (jlong) b.len;
    v[2] = (jlong)(intptr_t) b.owner;
    (*env)->SetLongArrayRegion(env, out, 0, 3, v);
  }
  return (jint) rc;
}

/* Crossing two, and the only other one. */
JNIEXPORT void JNICALL Java_ak_NativeRpc_bytesFree(JNIEnv *e, jclass c, jlong ptr,
                                                   jlong len, jlong owner) {
  (void) e; (void) c;
  ak_bytes b;
  b.ptr = (const uint8_t *)(intptr_t) ptr;
  b.len = (size_t) len;
  b.owner = (void *)(intptr_t) owner;
  ak_bytes_free(&b);
}

/* ---- section 9's completion queue ---------------------------------------------------
 *
 * The mode section 9 says a managed host should want: no upcall at all, and no thread the
 * host does not own. The drainer is a host thread that enters `ak_queue_next` and comes
 * back out, so there is nothing for the JVM to attach and nothing to pin -- which is the
 * claim this slice is placed to test, because its own `pinning.log` is where the blocking
 * mode's carrier pinning was measured.
 *
 * Three forward crossings per call (submit, next, free) and zero reverse, against the
 * blocking mode's two and zero. On this machine a forward crossing is 11.9 to 12.9 ns, so
 * the queue spends about 12 ns more per call to stop blocking a host thread in a native
 * frame for the duration of an RPC.
 */
typedef struct {
  uint64_t tag;
  int32_t  status;
  ak_bytes bytes;
} ak_completion;

void   *ak_queue_new(void);
void   *ak_call_unary_q(void *c, const uint8_t *path, size_t path_len,
                        const uint8_t *req, size_t req_len, void *q, uint64_t tag);
int32_t ak_queue_next(void *q, ak_completion *out, uint64_t timeout_ms);
void    ak_queue_shutdown(void *q);
void    ak_queue_destroy(void *q);
void    ak_call_cancel(void *h);
void    ak_call_destroy(void *h);

JNIEXPORT jlong JNICALL Java_ak_NativeRpc_queueNew(JNIEnv *e, jclass c) {
  (void) e; (void) c;
  return (jlong)(intptr_t) ak_queue_new();
}

JNIEXPORT void JNICALL Java_ak_NativeRpc_queueShutdown(JNIEnv *e, jclass c, jlong q) {
  (void) e; (void) c;
  ak_queue_shutdown((void *)(intptr_t) q);
}

JNIEXPORT void JNICALL Java_ak_NativeRpc_queueDestroy(JNIEnv *e, jclass c, jlong q) {
  (void) e; (void) c;
  ak_queue_destroy((void *)(intptr_t) q);
}

/* Crossing one: submit and return. The request is copied, not pinned, for the deadlock
 * reason above -- and here it matters even more, because the submitting thread goes on to
 * do other work while the call is in flight. */
JNIEXPORT jlong JNICALL Java_ak_NativeRpc_callUnaryQ(JNIEnv *env, jclass c, jlong cl,
                                                     jlong pathPtr, jint pathLen,
                                                     jbyteArray req, jint reqOff, jint reqLen,
                                                     jlong q, jlong tag) {
  (void) c;
  jbyte *base = (*env)->GetByteArrayElements(env, req, NULL);
  if (base == NULL) return 0;
  void *h = ak_call_unary_q((void *)(intptr_t) cl,
                            (const uint8_t *)(intptr_t) pathPtr, (size_t) pathLen,
                            (const uint8_t *) base + reqOff, (size_t) reqLen,
                            (void *)(intptr_t) q, (uint64_t) tag);
  (*env)->ReleaseByteArrayElements(env, req, base, JNI_ABORT);
  return (jlong)(intptr_t) h;
}

/* Crossing two: the downcall the host blocks in. `out` receives
 * {status, tag, ptr, len, owner}; the rc is the queue status, not the call's. */
JNIEXPORT jint JNICALL Java_ak_NativeRpc_queueNext(JNIEnv *env, jclass c, jlong q,
                                                   jlong timeoutMs, jlongArray out) {
  (void) c;
  ak_completion comp;
  comp.tag = 0;
  comp.status = 0;
  comp.bytes.ptr = NULL;
  comp.bytes.len = 0;
  comp.bytes.owner = NULL;
  int32_t rc = ak_queue_next((void *)(intptr_t) q, &comp, (uint64_t) timeoutMs);
  if (rc == 0) {
    jlong v[5];
    v[0] = (jlong) comp.status;
    v[1] = (jlong) comp.tag;
    v[2] = (jlong)(intptr_t) comp.bytes.ptr;
    v[3] = (jlong) comp.bytes.len;
    v[4] = (jlong)(intptr_t) comp.bytes.owner;
    (*env)->SetLongArrayRegion(env, out, 0, 5, v);
  }
  return (jint) rc;
}

JNIEXPORT void JNICALL Java_ak_NativeRpc_callDestroy(JNIEnv *e, jclass c, jlong h) {
  (void) e; (void) c;
  if (h != 0) ak_call_destroy((void *)(intptr_t) h);
}

/* ---- ak_client_new_opts: pin the transport on the CORE side too -----------------------
 *
 * Cells B and C were dialling with `ak_client_new`, which passes null options, so they ran
 * on hyper's defaults (2 MiB stream, 5 MiB connection) while cells A and D ran on the
 * 4 MiB the grpc-java channel was given. The grid's "pinned" configuration pinned half of
 * itself, and B minus A -- the transport delta, the one claim with a clean sign -- was
 * comparing two transports at two different window sizes. Neither window is smaller than
 * the 540 KB response, so no arm stalled; that is a reason it went unnoticed, not a reason
 * it was sound.
 */
typedef struct {
  uint32_t stream_window;
  uint32_t connection_window;
  int32_t  adaptive_window;
  uint32_t max_recv_message;
  uint32_t max_send_message;
  int32_t  tcp_nagle;
} ak_client_opts;

void *ak_client_new_opts(void *r, const uint8_t *uri, size_t uri_len,
                         const ak_client_opts *opts);

JNIEXPORT jlong JNICALL Java_ak_NativeRpc_clientNewOpts(JNIEnv *env, jclass c, jlong r,
                                                        jbyteArray uri, jint len,
                                                        jint streamWin, jint connWin,
                                                        jint adaptive, jint maxRecv,
                                                        jint maxSend, jint nagle) {
  (void) c;
  ak_client_opts o;
  o.stream_window = (uint32_t) streamWin;
  o.connection_window = (uint32_t) connWin;
  o.adaptive_window = (int32_t) adaptive;
  o.max_recv_message = (uint32_t) maxRecv;
  o.max_send_message = (uint32_t) maxSend;
  o.tcp_nagle = (int32_t) nagle;
  jbyte *u = (*env)->GetByteArrayElements(env, uri, NULL);
  void *cl = ak_client_new_opts((void *)(intptr_t) r, (const uint8_t *) u, (size_t) len, &o);
  (*env)->ReleaseByteArrayElements(env, uri, u, JNI_ABORT);
  return (jlong)(intptr_t) cl;
}
