/* R-D9 fault injection: what the bulk-bytes entry point does when the JVM refuses a pin.
 *
 * `GetPrimitiveArrayCritical` returns NULL only after raising OutOfMemoryError, which a
 * JVM run cannot produce on demand. So this links the generated shim against a fake
 * JNIEnv whose function table returns NULL for that one call, and wraps the core's entry
 * point (`-Wl,--wrap`) to record whether the core was entered and with what span.
 *
 * Before the fix the core is entered with (NULL, dlen > 0). After it the entry returns
 * AK_ERR_HOST, the core is not entered, and the frame stack is balanced.
 */
#include <jni.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include "ak_abi.h"

static int g_rel = 0;
static void *JNICALL fake_get_crit(JNIEnv *e, jarray a, jboolean *c) {
  (void) e; (void) a; (void) c; return NULL;
}
static void JNICALL fake_rel_crit(JNIEnv *e, jarray a, void *p, jint m) {
  (void) e; (void) a; (void) p; (void) m; g_rel++;
}

static int g_core = 0;
static const uint8_t *g_ptr = (const uint8_t *) 1;
static size_t g_len = 0;
intptr_t __wrap_ak_encode_UploadResultDataMessage(const void *o, ak_enc_ctx *c,
    const struct ak_evt_UploadResultDataMessage *vt,
    const struct ak_efix_UploadResultDataMessage *fix, const uint8_t *d, size_t n) {
  (void) o; (void) c; (void) vt; (void) fix;
  g_core++; g_ptr = d; g_len = n; return 0;
}

JNIEXPORT jlong JNICALL Java_ak_NativeEntry_encodeDirectUploadResultDataMessage(
    JNIEnv *env, jclass cls, jobject self, jlong ctx, jlong vt, jlong fix,
    jbyteArray data, jint dlen);

int main(void) {
  struct JNINativeInterface_ fns;
  memset(&fns, 0, sizeof fns);
  fns.GetPrimitiveArrayCritical = fake_get_crit;
  fns.ReleasePrimitiveArrayCritical = fake_rel_crit;
  const struct JNINativeInterface_ *tbl = &fns;
  JNIEnv *env = (JNIEnv *) &tbl;
  int bad = 0;
  /* Twice: an unbalanced ak_push would show as the second call seeing depth 1. */
  for (int i = 0; i < 2 * 8 + 1; i++) {
    g_core = 0; g_rel = 0;
    jlong rc = Java_ak_NativeEntry_encodeDirectUploadResultDataMessage(
        env, NULL, (jobject) 1, 0, 0, 0, (jbyteArray) 1, 65536);
    if (i == 0 || i == 16)
      printf("call %2d: rc=%lld core_entered=%d core_span=(%p, %zu) release_calls=%d\n",
             i, (long long) rc, g_core, (const void *) g_ptr, g_len, g_rel);
    if (g_core && g_ptr == NULL) bad = 1;
    if (rc == 0 && !g_core) bad = 1;   /* AK_ERR_INVALID_STATE would mean a leaked frame */
    if (rc != 0 && rc != AK_ERR_HOST) bad = 1;
  }
  printf("%s\n", bad ? "RESULT: DEFECT (core entered with a NULL span, or frame stack leaked)"
                     : "RESULT: ok (NULL pin refused with AK_ERR_HOST, core not entered, "
                       "17 calls past the depth-8 frame stack)");
  return bad;
}
