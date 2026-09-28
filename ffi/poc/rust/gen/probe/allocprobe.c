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
