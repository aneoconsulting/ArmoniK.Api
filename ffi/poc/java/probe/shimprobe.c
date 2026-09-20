/* What a generated C shim would pay per operation, measured before the arm is built.
 *
 * README 9.1 says the Python binding's shim speaks the CPython API instead of calling
 * back into the interpreter, and that "the C API pays when it REMOVES a crossing, not
 * when it replaces one". The JVM analogue is a shim that reads and writes facade fields
 * through the JNI API instead of upcalling into Java. Whether that can win is arithmetic:
 * the current design pays one cached upcall (about 80 ns here) and then does k field
 * stores in bytecode; the shim pays zero upcalls and k JNI accessor calls. It wins only
 * where k * jni_store < upcall + k * bytecode_store.
 *
 * So each op is timed INSIDE one native call, looping n times, against an empty-loop
 * control at the same n. The entry crossing is amortised away on purpose: the shim is
 * already in C when it pays these.
 *
 * Nothing here is part of the measured binding. It is a calibration, like crossing.log.
 */
#include <jni.h>
#include <stddef.h>

static jclass    g_node;      /* ShimProbe$Node */
static jfieldID  f_a, f_b, f_s;
static jmethodID m_ctor;
static jclass    g_list;      /* java.util.List */
static jmethodID m_set;
static jclass    g_str;

JNIEXPORT void JNICALL Java_ShimProbe_bind(JNIEnv *env, jclass c, jclass node, jclass list) {
  (void) c;
  g_node = (*env)->NewGlobalRef(env, node);
  g_list = (*env)->NewGlobalRef(env, list);
  f_a    = (*env)->GetFieldID(env, g_node, "a", "Ljava/lang/String;");
  f_b    = (*env)->GetFieldID(env, g_node, "b", "Ljava/lang/String;");
  f_s    = (*env)->GetFieldID(env, g_node, "s", "I");
  m_ctor = (*env)->GetMethodID(env, g_node, "<init>", "()V");
  m_set  = (*env)->GetMethodID(env, g_list, "set", "(ILjava/lang/Object;)Ljava/lang/Object;");
  jclass k = (*env)->FindClass(env, "java/lang/String");
  g_str  = (*env)->NewGlobalRef(env, k);
}

/* The control. Everything else is reported as (itself - this) / n. */
JNIEXPORT jlong JNICALL Java_ShimProbe_opNone(JNIEnv *env, jclass c, jobject o, jint n) {
  (void) env; (void) c; (void) o;
  volatile jlong s = 0;
  for (jint i = 0; i < n; i++) s += i;
  return s;
}

JNIEXPORT jlong JNICALL Java_ShimProbe_opGetObj(JNIEnv *env, jclass c, jobject o, jint n) {
  (void) c;
  (*env)->PushLocalFrame(env, 512);
  jlong s = 0;
  /* DeleteLocalRef is NOT in this loop: a read op and a local-ref release are two
   * different costs and the shim can batch the second with a frame. */
  for (jint i = 0; i < n; i++) {
    jobject v = (*env)->GetObjectField(env, o, f_a);
    s += (v != NULL);
    if ((i & 255) == 255) { (*env)->PopLocalFrame(env, NULL); (*env)->PushLocalFrame(env, 512); }
  }
  (*env)->PopLocalFrame(env, NULL);
  return s;
}

JNIEXPORT jlong JNICALL Java_ShimProbe_opSetObj(JNIEnv *env, jclass c, jobject o, jint n) {
  (void) c;
  jobject v = (*env)->GetObjectField(env, o, f_a);
  for (jint i = 0; i < n; i++) (*env)->SetObjectField(env, o, f_b, v);
  (*env)->DeleteLocalRef(env, v);
  return n;
}

JNIEXPORT jlong JNICALL Java_ShimProbe_opSetInt(JNIEnv *env, jclass c, jobject o, jint n) {
  (void) c;
  for (jint i = 0; i < n; i++) (*env)->SetIntField(env, o, f_s, i);
  return n;
}

JNIEXPORT jlong JNICALL Java_ShimProbe_opNewObj(JNIEnv *env, jclass c, jobject o, jint n) {
  (void) c; (void) o;
  jlong s = 0;
  for (jint i = 0; i < n; i++) {
    jobject v = (*env)->NewObject(env, g_node, m_ctor);
    s += (v != NULL);
    (*env)->DeleteLocalRef(env, v);
  }
  return s;
}

JNIEXPORT jlong JNICALL Java_ShimProbe_opAllocObj(JNIEnv *env, jclass c, jobject o, jint n) {
  (void) c; (void) o;
  jlong s = 0;
  for (jint i = 0; i < n; i++) {
    jobject v = (*env)->AllocObject(env, g_node);
    s += (v != NULL);
    (*env)->DeleteLocalRef(env, v);
  }
  return s;
}

/* An upcall on a container method. `set` rather than `add` so the list does not grow
 * while it is being timed; the transition is what is being priced, not the callee. */
JNIEXPORT jlong JNICALL Java_ShimProbe_opListSet(JNIEnv *env, jclass c, jobject list, jint n) {
  (void) c;
  jobject v = (*env)->NewStringUTF(env, "x");
  for (jint i = 0; i < n; i++) {
    jobject old = (*env)->CallObjectMethod(env, list, m_set, 0, v);
    (*env)->DeleteLocalRef(env, old);
  }
  (*env)->DeleteLocalRef(env, v);
  return n;
}

JNIEXPORT jlong JNICALL Java_ShimProbe_opArrSet(JNIEnv *env, jclass c, jobject arr, jint n) {
  (void) c;
  jobjectArray a = (jobjectArray) arr;
  jsize cap = (*env)->GetArrayLength(env, a);
  jobject v = (*env)->GetObjectArrayElement(env, a, 0);
  for (jint i = 0; i < n; i++) (*env)->SetObjectArrayElement(env, a, i % cap, v);
  (*env)->DeleteLocalRef(env, v);
  return n;
}

JNIEXPORT jlong JNICALL Java_ShimProbe_opNewArr(JNIEnv *env, jclass c, jobject o, jint n) {
  (void) c; (void) o;
  jlong s = 0;
  for (jint i = 0; i < n; i++) {
    jobjectArray a = (*env)->NewObjectArray(env, 3, g_str, NULL);
    s += (a != NULL);
    (*env)->DeleteLocalRef(env, a);
  }
  return s;
}

/* A 16-unit string, which is the size class of an ArmoniK id. */
JNIEXPORT jlong JNICALL Java_ShimProbe_opNewStr(JNIEnv *env, jclass c, jobject o, jint n) {
  (void) c; (void) o;
  static const jchar buf[16] = {
    'a','b','c','d','e','f','0','1','2','3','4','5','6','7','8','9' };
  jlong s = 0;
  for (jint i = 0; i < n; i++) {
    jstring v = (*env)->NewString(env, buf, 16);
    s += (v != NULL);
    (*env)->DeleteLocalRef(env, v);
  }
  return s;
}

/* The shape the binding uses TODAY: one cached upcall per run, with the stores done in
 * Java bytecode inside it. Timed at several k so the fixed transition and the per-store
 * cost separate, which is the arithmetic the shim arm has to beat. */
static jclass    g_probe;
static jmethodID m_apply;

JNIEXPORT void JNICALL Java_ShimProbe_bind2(JNIEnv *env, jclass c, jclass probe) {
  (void) c;
  g_probe = (*env)->NewGlobalRef(env, probe);
  m_apply = (*env)->GetStaticMethodID(env, g_probe, "apply", "(LShimProbe$Node;I)V");
}

JNIEXPORT jlong JNICALL Java_ShimProbe_revApply(JNIEnv *env, jclass c, jobject o, jint k, jint n) {
  (void) c;
  for (jint i = 0; i < n; i++) (*env)->CallStaticVoidMethod(env, g_probe, m_apply, o, k);
  return n;
}

/* The same k stores done by the shim itself: no upcall, k JNI accessor calls. Distinct
 * fields and distinct values, matching the callee above store for store. */
static jfieldID f_n[16];
static jobject  g_v[16];

JNIEXPORT void JNICALL Java_ShimProbe_bindFields(JNIEnv *env, jclass c, jobjectArray values) {
  (void) c;
  char name[8];
  for (int i = 0; i < 16; i++) {
    name[0] = 'f';
    if (i < 10) { name[1] = (char) ('0' + i); name[2] = 0; }
    else { name[1] = '1'; name[2] = (char) ('0' + i - 10); name[3] = 0; }
    f_n[i] = (*env)->GetFieldID(env, g_node, name, "Ljava/lang/String;");
    jobject v = (*env)->GetObjectArrayElement(env, values, i);
    g_v[i] = (*env)->NewGlobalRef(env, v);
    (*env)->DeleteLocalRef(env, v);
  }
}

JNIEXPORT jlong JNICALL Java_ShimProbe_shimApply(JNIEnv *env, jclass c, jobject o, jint k, jint n) {
  (void) c;
  for (jint i = 0; i < n; i++)
    for (jint j = 0; j < k; j++) (*env)->SetObjectField(env, o, f_n[j], g_v[j]);
  return n;
}
