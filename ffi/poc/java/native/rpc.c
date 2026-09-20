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
 * The request is pinned rather than copied, for the same reason the pull family's parse
 * is: this call makes no upcall, so a critical section over it is legal and the host's
 * array never has to be staged. */
JNIEXPORT jint JNICALL Java_ak_NativeRpc_callUnary(JNIEnv *env, jclass c, jlong cl,
                                                   jlong pathPtr, jint pathLen,
                                                   jbyteArray req, jint reqOff, jint reqLen,
                                                   jlongArray out) {
  (void) c;
  ak_bytes b = {NULL, 0, NULL};
  void *base = (*env)->GetPrimitiveArrayCritical(env, req, NULL);
  if (base == NULL) return -1;
  int32_t rc = ak_call_unary((void *)(intptr_t) cl,
                             (const uint8_t *)(intptr_t) pathPtr, (size_t) pathLen,
                             (const uint8_t *) base + reqOff, (size_t) reqLen, &b);
  (*env)->ReleasePrimitiveArrayCritical(env, req, base, JNI_ABORT);
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
