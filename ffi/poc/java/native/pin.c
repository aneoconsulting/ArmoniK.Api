/* ABI v1 section 9's fourth amendment, made testable.
 *
 * "At least one mode in which the caller waits in the host language. Blocking in a native
 * frame from a virtual thread pins its carrier; what fixes that is parking in Java on a
 * future, which the callback mode already provides. The requirement on the ABI is this
 * weak and this general."
 *
 * That is a claim about the JVM and no slice has measured it. It needs no RPC stack: the
 * question is whether a virtual thread that waits inside a native frame occupies its
 * carrier, and whether the callback delivery mode -- a native thread that upcalls when the
 * work is done, with the Java caller parked on a future -- avoids it.
 *
 * Two entry points, deliberately symmetric in everything but where the waiting happens:
 *
 *   ak_pin_block   waits IN THE NATIVE FRAME, which is the blocking `ak_call_unary`
 *   ak_pin_async   returns at once and completes the host's future from a native thread
 *                  later, which is `ak_call_unary_cb`
 */
#include <jni.h>
#include <pthread.h>
#include <stdlib.h>
#include <time.h>

static void nap(long ms) {
  struct timespec ts;
  ts.tv_sec = ms / 1000;
  ts.tv_nsec = (ms % 1000) * 1000000L;
  nanosleep(&ts, NULL);
}

/* The blocking mode: the host's thread is inside this frame for the whole wait. */
JNIEXPORT void JNICALL Java_ak_Pin_block(JNIEnv *env, jclass c, jlong ms) {
  (void) env; (void) c;
  nap((long) ms);
}

static JavaVM *g_vm;
static jclass g_cls;
static jmethodID g_done;

typedef struct { long ms; jobject fut; } job;

static void *worker(void *p) {
  job *j = (job *) p;
  nap(j->ms);
  JNIEnv *env = NULL;
  /* A thread the host does not own, which is what the callback mode is (ABI v1 section 9):
     "A callback is an upcall onto a thread the host does not own". */
  (*g_vm)->AttachCurrentThreadAsDaemon(g_vm, (void **) &env, NULL);
  (*env)->CallStaticVoidMethod(env, g_cls, g_done, j->fut);
  (*env)->DeleteGlobalRef(env, j->fut);
  (*g_vm)->DetachCurrentThread(g_vm);
  free(j);
  return NULL;
}

JNIEXPORT void JNICALL Java_ak_Pin_bind(JNIEnv *env, jclass c, jclass cls) {
  (void) c;
  (*env)->GetJavaVM(env, &g_vm);
  g_cls = (*env)->NewGlobalRef(env, cls);
  g_done = (*env)->GetStaticMethodID(env, cls, "complete", "(Ljava/lang/Object;)V");
}

/* The callback mode: returns immediately, completes the future from a native thread. */
JNIEXPORT void JNICALL Java_ak_Pin_async(JNIEnv *env, jclass c, jlong ms, jobject fut) {
  (void) c;
  job *j = malloc(sizeof(job));
  j->ms = (long) ms;
  j->fut = (*env)->NewGlobalRef(env, fut);
  pthread_t t;
  pthread_create(&t, NULL, worker, j);
  pthread_detach(t);
}
