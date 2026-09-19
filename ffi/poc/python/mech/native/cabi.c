/* The plain C shared library: no Python in it at all.
 *
 * This is what a `ctypes` or a `cffi`-ABI host reaches, and it is also the
 * forward target for the C-extension and PyO3 arms, so that every mechanism is
 * measured against the SAME callee. A mechanism arm whose callee differs is a
 * comparison of callees (README R7).
 *
 * Nothing here is generated: it is three no-ops and a loop.
 */
#include <stddef.h>
#include <stdint.h>

#if defined(_WIN32)
#define AK_EXPORT __declspec(dllexport)
#else
#define AK_EXPORT __attribute__((visibility("default")))
#endif

/* The forward target. `volatile` is not used: the call cannot be elided because
 * the symbol is exported from a shared object and the caller is in another
 * translation unit reached through the PLT. */
AK_EXPORT uint64_t ak_noop(uint64_t x) { return x ^ 2u; }

/* A forward call carrying the shape of a real codec entry point: a pointer, a
 * length and an out-parameter, rather than one register. */
AK_EXPORT int32_t ak_noop_buf(const uint8_t *p, size_t n, uint64_t *out) {
  *out = (uint64_t)n ^ (uint64_t)(uintptr_t)p;
  return 0;
}

typedef uint64_t (*ak_cb_fn)(uint64_t);

/* `n` reverse calls inside ONE forward call, so that the forward cost is
 * amortised to nothing and what the driver divides by `n` is the reverse call
 * alone. */
AK_EXPORT uint64_t ak_reverse_n(ak_cb_fn cb, uint64_t x, size_t n) {
  for (size_t i = 0; i < n; i++) x = cb(x);
  return x;
}

/* The same loop with the crossing removed: the floor every reverse figure is
 * quoted above. Without it a reverse number carries the loop in it. */
static uint64_t ak_inline_cb(uint64_t x) { return x ^ 2u; }

AK_EXPORT uint64_t ak_reverse_n_floor(uint64_t x, size_t n) {
  for (size_t i = 0; i < n; i++) x = ak_inline_cb(x);
  return x;
}
