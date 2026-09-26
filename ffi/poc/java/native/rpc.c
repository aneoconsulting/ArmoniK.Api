#define _POSIX_C_SOURCE 200809L   /* clock_gettime under -std=c11 */
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
 * R-G5 (FIX-PLAN WP5 step 3): every struct and prototype below comes from the generated
 * header, rendered from `plan.rpc` by poc/codec/gen/java_abi.py with a static assertion of
 * each struct's size and offsets. Before WP5 this file hand-declared `ak_bytes`,
 * `ak_completion` and every RPC prototype, with handles as `void *`.
 */
#include <jni.h>
#include <stdint.h>
#include <stddef.h>
#include <time.h>

#include "ak_abi.h"

JNIEXPORT jlong JNICALL Java_ak_NativeRpc_runtimeNew(JNIEnv *e, jclass c, jint threads) {
  (void) e; (void) c;
  return (jlong)(intptr_t) ak_runtime_new((uint32_t) threads);
}

JNIEXPORT void JNICALL Java_ak_NativeRpc_runtimeDestroy(JNIEnv *e, jclass c, jlong r) {
  (void) e; (void) c;
  ak_runtime_destroy((ak_runtime *)(intptr_t) r);
}

JNIEXPORT jlong JNICALL Java_ak_NativeRpc_clientNew(JNIEnv *env, jclass c, jlong r,
                                                    jbyteArray uri, jint len) {
  (void) c;
  jbyte *u = (*env)->GetByteArrayElements(env, uri, NULL);
  if (u == NULL) return 0;   /* R-D9 sweep: OutOfMemoryError already pending */
  ak_client *cl = ak_client_new((ak_runtime *)(intptr_t) r, (const uint8_t *) u, (size_t) len);
  (*env)->ReleaseByteArrayElements(env, uri, u, JNI_ABORT);
  return (jlong)(intptr_t) cl;
}

JNIEXPORT void JNICALL Java_ak_NativeRpc_clientDestroy(JNIEnv *e, jclass c, jlong cl) {
  (void) e; (void) c;
  ak_client_destroy((ak_client *)(intptr_t) cl);
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
  int32_t rc = ak_call_unary((ak_client *)(intptr_t) cl,
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
 * blocking mode's two and zero: one more forward crossing per call to stop blocking a host
 * thread in a native frame for the duration of an RPC.
 */

JNIEXPORT jlong JNICALL Java_ak_NativeRpc_queueNew(JNIEnv *e, jclass c) {
  (void) e; (void) c;
  return (jlong)(intptr_t) ak_queue_new();
}

JNIEXPORT void JNICALL Java_ak_NativeRpc_queueShutdown(JNIEnv *e, jclass c, jlong q) {
  (void) e; (void) c;
  ak_queue_shutdown((ak_queue *)(intptr_t) q);
}

JNIEXPORT void JNICALL Java_ak_NativeRpc_queueDestroy(JNIEnv *e, jclass c, jlong q) {
  (void) e; (void) c;
  ak_queue_destroy((ak_queue *)(intptr_t) q);
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
  ak_call *h = ak_call_unary_q((ak_client *)(intptr_t) cl,
                            (const uint8_t *)(intptr_t) pathPtr, (size_t) pathLen,
                            (const uint8_t *) base + reqOff, (size_t) reqLen,
                            (ak_queue *)(intptr_t) q, (uint64_t) tag);
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
  int32_t rc = ak_queue_next((ak_queue *)(intptr_t) q, &comp, (uint64_t) timeoutMs);
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
  if (h != 0) ak_call_destroy((ak_call *)(intptr_t) h);
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
/* R-D2 sweep: `ak_client_opts` comes from the generated header (plan.rpc), which asserts
 * its size and every offset. */

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
  if (u == NULL) return 0;   /* R-D9 sweep: OutOfMemoryError already pending */
  ak_client *cl = ak_client_new_opts((ak_runtime *)(intptr_t) r, (const uint8_t *) u, (size_t) len, &o);
  (*env)->ReleaseByteArrayElements(env, uri, u, JNI_ABORT);
  return (jlong)(intptr_t) cl;
}

/* CAMPAIGN.md req 21: the client process's CPU time for an RPC cell, from
 * CLOCK_PROCESS_CPUTIME_ID (ns). Not getProcessCpuTime: on Linux the JDK reads times(),
 * whose tick is 10 ms. */
JNIEXPORT jlong JNICALL Java_ak_NativeRpc_processCpuNs(JNIEnv *e, jclass c) {
  (void) e; (void) c;
  struct timespec ts;
  if (clock_gettime(CLOCK_PROCESS_CPUTIME_ID, &ts) != 0) return -1;
  return (jlong) ts.tv_sec * 1000000000LL + (jlong) ts.tv_nsec;
}

/* R-H17: cell B parses the core's response in place. A direct ByteBuffer over the core's
 * bytes (no copy); the memory stays the core's and is freed with bytesFree after the parse. */
JNIEXPORT jobject JNICALL Java_ak_NativeRpc_directBuffer(JNIEnv *env, jclass c, jlong ptr, jlong len) {
  (void) c;
  return (*env)->NewDirectByteBuffer(env, (void *)(intptr_t) ptr, len);
}
