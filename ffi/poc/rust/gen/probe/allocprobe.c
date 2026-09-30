/* LD_PRELOAD allocation counter for the stream probe (gen/stream_probe.sh). CONTAINER
 * INSTRUMENTATION ONLY: never loaded by a timed campaign run. Counts every malloc-family call
 * of the process (the host binary and the ak-core cdylib both allocate through libc), and
 * separately the calls of 1 MiB or more (the 2 MiB chunk buffers). Read with akp_counts(). */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdatomic.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>
#include <stdlib.h>
#include <unistd.h>
#include <fcntl.h>
#include <execinfo.h>

/* AKP_BT=N (attribution only): for the first N allocations of >= 1 MiB, write their return
 * addresses to stderr ("AKP_BT size addr ..."), and /proc/self/maps once before the first, so
 * gen/probe/akp_bt.py can resolve them offline. No allocation on that path (write(2) only). */
static _Atomic long bt_left = -1;
static __thread int in_bt;
static void hex(char **p, unsigned long v) {
  char t[20]; int n = 0;
  do { t[n++] = "0123456789abcdef"[v & 15]; v >>= 4; } while (v);
  while (n) *(*p)++ = t[--n];
}
static void bt(size_t n) {
  long left = atomic_load(&bt_left);
  if (left == -1) {
    const char *e = getenv("AKP_BT");
    long want = e ? atol(e) : 0;
    long expect = -1;
    atomic_compare_exchange_strong(&bt_left, &expect, want);
    if (want > 0) {
      int fd = open("/proc/self/maps", O_RDONLY);
      char b[4096]; ssize_t r;
      write(2, "AKP_MAPS_BEGIN\n", 15);
      while (fd >= 0 && (r = read(fd, b, sizeof b)) > 0) write(2, b, r);
      write(2, "AKP_MAPS_END\n", 13);
      if (fd >= 0) close(fd);
    }
    left = atomic_load(&bt_left);
  }
  if (left <= 0 || in_bt) return;
  if (atomic_fetch_sub(&bt_left, 1) <= 0) return;
  in_bt = 1;
  void *a[32]; int k = backtrace(a, 32);
  char line[32 * 20 + 64], *p = line;
  memcpy(p, "AKP_BT ", 7); p += 7; hex(&p, n);
  for (int i = 0; i < k; i++) { *p++ = ' '; hex(&p, (unsigned long)a[i]); }
  *p++ = '\n';
  write(2, line, p - line);
  in_bt = 0;
}

static void *(*r_malloc)(size_t);
static void *(*r_calloc)(size_t, size_t);
static void *(*r_realloc)(void *, size_t);
static void (*r_free)(void *);
static int (*r_posix_memalign)(void **, size_t, size_t);
static void *(*r_aligned_alloc)(size_t, size_t);
static void *(*r_memalign)(size_t, size_t);

static _Atomic uint64_t n_all, n_big, b_big, n_free;
#define BIG (1u << 20)

/* dlsym itself may call calloc before r_calloc is known: serve it from a static arena. */
static char boot[8192];
static size_t boot_used;
static int resolving;

static void init(void) {
  if (r_malloc || resolving) return;
  resolving = 1;
  r_malloc = dlsym(RTLD_NEXT, "malloc");
  r_calloc = dlsym(RTLD_NEXT, "calloc");
  r_realloc = dlsym(RTLD_NEXT, "realloc");
  r_free = dlsym(RTLD_NEXT, "free");
  r_posix_memalign = dlsym(RTLD_NEXT, "posix_memalign");
  r_aligned_alloc = dlsym(RTLD_NEXT, "aligned_alloc");
  r_memalign = dlsym(RTLD_NEXT, "memalign");
  resolving = 0;
}

static inline void count(size_t n) {
  atomic_fetch_add_explicit(&n_all, 1, memory_order_relaxed);
  if (n >= BIG) {
    atomic_fetch_add_explicit(&n_big, 1, memory_order_relaxed);
    atomic_fetch_add_explicit(&b_big, n, memory_order_relaxed);
    if (!in_bt && r_malloc) bt(n);
  }
}

void *malloc(size_t n) { init(); count(n); return r_malloc(n); }
void *calloc(size_t a, size_t b) {
  if (!r_calloc) {
    if (resolving) { size_t n = a * b; void *p = boot + boot_used; boot_used += (n + 15) & ~(size_t)15; memset(p, 0, n); return p; }
    init();
  }
  count(a * b);
  return r_calloc(a, b);
}
void *realloc(void *p, size_t n) { init(); count(n); return r_realloc(p, n); }
void free(void *p) {
  if ((char *)p >= boot && (char *)p < boot + sizeof boot) return;
  init();
  if (p) atomic_fetch_add_explicit(&n_free, 1, memory_order_relaxed);
  r_free(p);
}
int posix_memalign(void **o, size_t al, size_t n) { init(); count(n); return r_posix_memalign(o, al, n); }
void *aligned_alloc(size_t al, size_t n) { init(); count(n); return r_aligned_alloc(al, n); }
void *memalign(size_t al, size_t n) { init(); count(n); return r_memalign(al, n); }

/* out[0] allocation calls, out[1] of them >= 1 MiB, out[2] their bytes, out[3] frees */
void akp_counts(uint64_t out[4]) {
  out[0] = atomic_load(&n_all);
  out[1] = atomic_load(&n_big);
  out[2] = atomic_load(&b_big);
  out[3] = atomic_load(&n_free);
}
