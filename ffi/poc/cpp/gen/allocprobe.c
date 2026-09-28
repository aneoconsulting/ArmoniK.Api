/* R-1 / HG-5 probe (2026-09-28): an LD_PRELOAD allocator shim that counts the allocations of at
 * least AK_PROBE_MIN bytes (default 1 MiB) made through malloc, calloc, realloc (a growing one),
 * posix_memalign, aligned_alloc and memalign, process-wide. campaign_rpc --alloc-probe N reads
 * the counter through dlsym(RTLD_DEFAULT, "akprobe_big_allocs"). Not linked into any timed build.
 *   gcc -O2 -shared -fPIC -o allocprobe.so gen/allocprobe.c -ldl */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdatomic.h>
#include <stdint.h>
#include <stdlib.h>

static atomic_ulong big;
static size_t min_sz = 1 << 20;
static void *(*r_malloc)(size_t), *(*r_calloc)(size_t, size_t), *(*r_realloc)(void *, size_t);
static int (*r_pma)(void **, size_t, size_t);
static void *(*r_aa)(size_t, size_t), *(*r_ma)(size_t, size_t);
static char boot[1 << 16];
static size_t boot_used;

__attribute__((constructor)) static void init(void) {
  const char *e = getenv("AK_PROBE_MIN");
  if (e) min_sz = (size_t)strtoull(e, 0, 10);
}
static void resolve(void) {
  if (r_malloc) return;
  r_calloc = dlsym(RTLD_NEXT, "calloc");
  r_realloc = dlsym(RTLD_NEXT, "realloc");
  r_pma = dlsym(RTLD_NEXT, "posix_memalign");
  r_aa = dlsym(RTLD_NEXT, "aligned_alloc");
  r_ma = dlsym(RTLD_NEXT, "memalign");
  r_malloc = dlsym(RTLD_NEXT, "malloc");
}
unsigned long akprobe_big_allocs(void) { return atomic_load(&big); }
void *malloc(size_t n) {
  resolve();
  if (n >= min_sz) atomic_fetch_add(&big, 1);
  return r_malloc(n);
}
void *calloc(size_t a, size_t b) {
  if (!r_calloc) { /* dlsym itself calls calloc: serve it from a static block */
    size_t n = (a * b + 15) & ~(size_t)15;
    void *p = boot + boot_used; boot_used += n; return p;
  }
  if (a * b >= min_sz) atomic_fetch_add(&big, 1);
  return r_calloc(a, b);
}
void *realloc(void *p, size_t n) {
  resolve();
  if (n >= min_sz) atomic_fetch_add(&big, 1);
  return r_realloc(p, n);
}
int posix_memalign(void **p, size_t al, size_t n) {
  resolve();
  if (n >= min_sz) atomic_fetch_add(&big, 1);
  return r_pma(p, al, n);
}
void *aligned_alloc(size_t al, size_t n) {
  resolve();
  if (n >= min_sz) atomic_fetch_add(&big, 1);
  return r_aa(al, n);
}
void *memalign(size_t al, size_t n) {
  resolve();
  if (n >= min_sz) atomic_fetch_add(&big, 1);
  return r_ma(al, n);
}
static void (*r_free)(void *);
void free(void *p) {
  if ((char *)p >= boot && (char *)p < boot + sizeof boot) return;
  if (!r_free) r_free = dlsym(RTLD_NEXT, "free");
  r_free(p);
}
