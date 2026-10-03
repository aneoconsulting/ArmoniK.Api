/* CAMPAIGN req 21 as amended 2026-10-01 (FIX-PLAN WP12, WP13): the RPC client's CPU is perf
 * task-clock of the WHOLE process, softirq time included, which CLOCK_PROCESS_CPUTIME_ID misses
 * on a kernel with CONFIG_IRQ_TIME_ACCOUNTING=y (findings/physical-probe.md section 2).
 *
 * A JVM creates its threads (VM thread, GC, JIT compilers, then every Java thread) long before
 * a JNI library is loaded, and a perf counter with `inherit` covers only threads created after
 * it. So this file is a JVM AGENT (-agentpath:libaktc.so): Agent_OnLoad runs on the JVM's main
 * thread during JVM creation, before those threads exist, and opens ONE task-clock counter on
 * that thread with inherit=1, so every later thread (and theirs) is an inherited child whose
 * count the kernel sums into this counter on every read (live children and exited ones). The
 * java launcher's primordial thread, which only waits for the JVM, is not counted.
 *
 * The same library is then System.load-ed by ak.TaskClock so its JNI functions read the
 * counter (dlopen returns the agent's handle: one copy of g_fd). The Rust slice opens the same
 * counter first thing in main (stream_probe AK_PROBE_TASKCLOCK). */
#define _GNU_SOURCE
#include <jni.h>
#include <errno.h>
#include <linux/perf_event.h>
#include <stdint.h>
#include <string.h>
#include <sys/syscall.h>
#include <unistd.h>

static int g_fd = -1;
static int g_err = 0;

static int open_task_clock(void) {
  struct perf_event_attr a;
  memset(&a, 0, sizeof a);
  a.type = PERF_TYPE_SOFTWARE;
  a.size = sizeof a;
  a.config = PERF_COUNT_SW_TASK_CLOCK;
  a.inherit = 1;
  return (int) syscall(SYS_perf_event_open, &a, 0, -1, -1, PERF_FLAG_FD_CLOEXEC);
}

JNIEXPORT jint JNICALL Agent_OnLoad(JavaVM *vm, char *options, void *reserved) {
  (void) vm; (void) options; (void) reserved;
  g_fd = open_task_clock();
  if (g_fd < 0) g_err = errno;
  return 0;   /* never fail the JVM: ak.TaskClock reports the error and the harness refuses */
}

/* The counter in ns, or -1 when it is not open. */
JNIEXPORT jlong JNICALL Java_ak_TaskClock_ns(JNIEnv *e, jclass c) {
  (void) e; (void) c;
  uint64_t v;
  if (g_fd < 0 || read(g_fd, &v, sizeof v) != (ssize_t) sizeof v) return -1;
  return (jlong) v;
}

/* >= 0: the counter's fd (opened by the agent); otherwise -errno of perf_event_open, or
 * -1000 when the agent never ran (no -agentpath). */
JNIEXPORT jint JNICALL Java_ak_TaskClock_status(JNIEnv *e, jclass c) {
  (void) e; (void) c;
  if (g_fd >= 0) return g_fd;
  return g_err ? -g_err : -1000;
}
