/* The calibrated delay of the cpp slice's `tax.sh`, lifted rather than re-derived.
 *
 * ABI v1 open decision 1 records the most portable measurement the branch has: with a
 * delay in front of every FORWARD entry-point call, the batched-against-unbatched delta
 * moves monotonically and the crossover falls at a forward crossing of roughly 2 to 4 ns.
 * That number was taken on a host whose crossing is 1.8 ns. JNI's is about 11, far above
 * it, so the prediction is that batching wins decisively here -- and the way to test a
 * crossover is to walk the curve, not to report one point on it.
 *
 * The delay is a dependent chain of integer adds so the compiler cannot hoist it and the
 * processor cannot overlap it with the call. `AK_TAX_N` sets the length; the harness
 * calibrates it to nanoseconds once and prints the mapping.
 */
#define _POSIX_C_SOURCE 200809L   /* clock_gettime under -std=c11 */
#include <jni.h>
#include <stdint.h>
#include <stdlib.h>
#include <time.h>

static int g_n = 0;
static volatile uint64_t g_sink;

__attribute__((constructor)) static void ak_tax_init(void) {
  const char *s = getenv("AK_TAX_N");
  g_n = s ? atoi(s) : 0;
}

void ak_crossing_tax(void) {
  uint64_t x = g_sink;
  for (int i = 0; i < g_n; i++) x = x * 6364136223846793005ULL + 1442695040888963407ULL;
  g_sink = x;
}

/* CAMPAIGN req 21 (amended 2026-09-26, R-H25): the codec suite's CPU is the PROCESS's CPU
 * per round (GC, JIT and helper threads count for every arm), from CLOCK_PROCESS_CPUTIME_ID
 * in ns. Here because tax.c is linked into every shim. */
JNIEXPORT jlong JNICALL Java_ak_Native_processCpuNs(JNIEnv *e, jclass c) {
  (void) e; (void) c;
  struct timespec ts;
  if (clock_gettime(CLOCK_PROCESS_CPUTIME_ID, &ts) != 0) return -1;
  return (jlong) ts.tv_sec * 1000000000LL + (jlong) ts.tv_nsec;
}
