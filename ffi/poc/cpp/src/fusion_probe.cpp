// A fixture whose only job is to make `gen/boundary.sh`'s fusion check fail on demand.
//
// C18: half two of the boundary check used to ask whether a control function was LARGER
// than the largest timing closure in the image, and take that as evidence the loop was not
// carrying a copy of it. Two problems. It is indirect -- size is a proxy for fusion, not a
// test of it -- and its positive control was `-flto`, which fires only if the optimiser
// happens to fuse something. After W10 moved the core the largest closure shrank from
// 1433 B to 911 B, the 1155 B control landed on the other side of the line, and the
// control stopped firing. The check reported the pass as unproven, which was right, but a
// control that depends on the optimiser's mood is a control that goes quiet without anyone
// deciding it should.
//
// So the check now counts CALL SITES to the symbol, which is the question itself, and this
// file is its control: one function that must be called and one that must be fused, both
// guaranteed by construction rather than by optimisation level.
//
//   ak_probe_called  noinline, and its address is taken through a volatile pointer, so the
//                    compiler cannot devirtualise the call. There MUST be a call site.
//   ak_probe_fused   always_inline and static, so there is no out-of-line body to call.
//                    There MUST NOT be one.
//
// If `boundary.sh` ever reports a call site for `fused`, or none for `called`, the counter
// itself is broken and every other answer it gives is worthless.
#include <cstdio>
#include <cstdlib>

#if defined(__GNUC__) || defined(__clang__)
#define AK_NOINLINE __attribute__((noinline))
#define AK_ALWAYS_INLINE __attribute__((always_inline)) inline
#else
#define AK_NOINLINE
#define AK_ALWAYS_INLINE inline
#endif

extern "C" AK_NOINLINE long ak_probe_called(long x) {
  long a = 0;
  for (int i = 0; i < 7; ++i) a += (x ^ (long)i) * 31 + (a >> 3);
  return a;
}

static AK_ALWAYS_INLINE long ak_probe_fused_body(long x) {
  long a = 0;
  for (int i = 0; i < 7; ++i) a += (x ^ (long)i) * 31 + (a >> 3);
  return a;
}

// Take the address through a volatile pointer so no amount of optimisation can turn the
// indirect call into a direct one and then inline it.
static long (*volatile g_called)(long) = &ak_probe_called;

extern "C" long ak_probe_loop_called(long n) {
  long acc = 0;
  for (long i = 0; i < n; ++i) acc += g_called(i);
  return acc;
}

extern "C" long ak_probe_loop_fused(long n) {
  long acc = 0;
  for (long i = 0; i < n; ++i) acc += ak_probe_fused_body(i);
  return acc;
}

int main(int argc, char **argv) {
  long n = argc > 1 ? std::atol(argv[1]) : 1000;
  std::printf("%ld %ld\n", ak_probe_loop_called(n), ak_probe_loop_fused(n));
  return 0;
}
