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
#define _GNU_SOURCE               /* clock_gettime, dirfd, sockets, RUSAGE_THREAD under -std=c11 */
#define _POSIX_C_SOURCE 200809L
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

/* CAMPAIGN req 17 as amended 2026-10-01 (D10): Nagle off on every client socket, READ BACK on
 * the live sockets. Every open fd of this process that is a TCP socket connected to
 * 127.0.0.1:port is checked with getsockopt(TCP_NODELAY); grpc-java's (Netty) and the core's
 * (tonic) sockets alike, since both live in this process. Returns (sockets << 32) | (sockets
 * with TCP_NODELAY set), or -1 when /proc/self/fd cannot be read. */
#include <arpa/inet.h>
#include <dirent.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <sys/socket.h>
JNIEXPORT jlong JNICALL Java_ak_Native_tcpNodelay(JNIEnv *e, jclass c, jint port) {
  (void) e; (void) c;
  DIR *d = opendir("/proc/self/fd");
  if (!d) return -1;
  long long n = 0, on = 0;
  struct dirent *de;
  while ((de = readdir(d)) != NULL) {
    int fd = atoi(de->d_name);
    if (de->d_name[0] < '0' || de->d_name[0] > '9' || fd == dirfd(d)) continue;
    struct sockaddr_in pa;
    socklen_t pl = sizeof pa;
    int type = 0;
    socklen_t tl = sizeof type;
    if (getsockopt(fd, SOL_SOCKET, SO_TYPE, &type, &tl) != 0 || type != SOCK_STREAM) continue;
    if (getpeername(fd, (struct sockaddr *) &pa, &pl) != 0 || pa.sin_family != AF_INET) continue;
    if (ntohs(pa.sin_port) != port || pa.sin_addr.s_addr != htonl(INADDR_LOOPBACK)) continue;
    int v = 0;
    socklen_t vl = sizeof v;
    n++;
    if (getsockopt(fd, IPPROTO_TCP, TCP_NODELAY, &v, &vl) == 0 && v) on++;
  }
  closedir(d);
  return (jlong) ((n << 32) | on);
}

/* CAMPAIGN req 25 / D9 as amended (owner, 2026-10-03): the allocator mode this process really
 * runs. One 16 MiB malloc through glibc (the allocator the core and its transport use in this
 * process), with mallinfo2's count of mmapped blocks read before and after: glibc's default
 * mmap threshold (128 KiB, dynamic up to 32 MiB) serves it with mmap -> 1 ("mmapped"); the
 * pinned pass (GLIBC_TUNABLES mmap_threshold=33554432) serves it from the heap -> 0 ("heap").
 * -1 when the allocation fails. */
#include <malloc.h>
#include <string.h>
JNIEXPORT jint JNICALL Java_ak_Native_allocProbe(JNIEnv *e, jclass c) {
  (void) e; (void) c;
  struct mallinfo2 before = mallinfo2();
  void *volatile p = malloc((size_t) 16 << 20);
  if (!p) return -1;
  memset(p, 1, 4096);
  struct mallinfo2 after = mallinfo2();
  int mmapped = after.hblks > before.hblks;
  /* An mmapped block is NOT freed: freeing it would raise glibc's dynamic mmap threshold to
   * 16 MiB for the rest of the process (malloc.c, free of an mmapped chunk above the
   * threshold), changing the default mode this probe checks; a later probe then reads "heap"
   * (seen: a second check in one JMH fork). Kept mapped, it costs 16 MiB of address space and
   * the one touched page. A heap block is freed (no threshold effect). */
  if (!mmapped) free(p);
  return mmapped ? 1 : 0;
}

/* Req 25 as amended (mechanics, owner 2026-10-03): the minor faults over a measured span, for
 * every sample: getrusage(RUSAGE_SELF).ru_minflt, the whole process. */
#include <sys/resource.h>
JNIEXPORT jlong JNICALL Java_ak_Native_minorFaults(JNIEnv *e, jclass c) {
  (void) e; (void) c;
  struct rusage u;
  if (getrusage(RUSAGE_SELF, &u) != 0) return -1;
  return (jlong) u.ru_minflt;
}
