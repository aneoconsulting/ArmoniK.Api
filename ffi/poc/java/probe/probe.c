/* The native half of the crossing probe. Nothing here does work; what is timed is the
   transition. `-O2 -fno-inline` on the callback shims would not help: JNI upcalls cannot
   be inlined by either side, which is the whole point. */
#include <jni.h>
#include <stddef.h>

JNIEXPORT jlong JNICALL Java_Probe_fwd(JNIEnv *env, jclass c, jlong x) {
  (void) env; (void) c;
  return x ^ 1;
}

JNIEXPORT jlong JNICALL Java_Probe_fwdBuf(JNIEnv *env, jclass c, jlong addr, jlong x) {
  (void) env; (void) c;
  volatile unsigned char *p = (unsigned char *) (size_t) addr;
  p[0] = (unsigned char) x;
  return x ^ p[0];
}

/* Resolves the class and the method every call: the naive shape. */
JNIEXPORT jlong JNICALL Java_Probe_rev(JNIEnv *env, jclass c, jlong x) {
  jclass k = (*env)->FindClass(env, "Probe");
  jmethodID m = (*env)->GetStaticMethodID(env, k, "cb", "(J)I");
  jint r = (*env)->CallStaticIntMethod(env, k, m, x);
  (*env)->DeleteLocalRef(env, k);
  return r;
}

static jclass g_cls;
static jmethodID g_cb, g_cbObj;

JNIEXPORT jlong JNICALL Java_Probe_revCached(JNIEnv *env, jclass c, jlong x) {
  if (!g_cls) {
    jclass k = (*env)->FindClass(env, "Probe");
    g_cls = (*env)->NewGlobalRef(env, k);
    g_cb = (*env)->GetStaticMethodID(env, g_cls, "cb", "(J)I");
    g_cbObj = (*env)->GetStaticMethodID(env, g_cls, "cbObj", "(Ljava/lang/Object;J)I");
  }
  return (*env)->CallStaticIntMethod(env, g_cls, g_cb, x);
}

JNIEXPORT jlong JNICALL Java_Probe_revObj(JNIEnv *env, jclass c, jobject o, jlong x) {
  if (!g_cls) {
    jclass k = (*env)->FindClass(env, "Probe");
    g_cls = (*env)->NewGlobalRef(env, k);
    g_cb = (*env)->GetStaticMethodID(env, g_cls, "cb", "(J)I");
    g_cbObj = (*env)->GetStaticMethodID(env, g_cls, "cbObj", "(Ljava/lang/Object;J)I");
  }
  return (*env)->CallStaticIntMethod(env, g_cls, g_cbObj, o, x);
}
