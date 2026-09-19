#include <cstdio>
#include <cstdlib>
#include "odr.h"

int main() {
  // Line-buffered: the POSITIVE CONTROL (-DAK_ODR_BREAK) can crash, because a
  // layout disagreement across a linked seam is exactly a memory-safety bug, and a
  // buffered report would be lost with the process.
  setvbuf(stdout, NULL, _IOLBF, 0);
  size_t na = 0, nb = 0;
  const AkOdrFact *a = odr_facts_11(&na);
  const AkOdrFact *b = odr_facts_17(&nb);
  std::printf("TU A: __cplusplus = %ld   TU B: __cplusplus = %ld\n",
              odr_std_11(), odr_std_17());
  if (odr_std_11() == odr_std_17()) {
    std::printf("FAIL: the two translation units are at the SAME standard level, so this\n"
                "      check proves nothing. Build with the CMake target `odrcheck`.\n");
    return 2;
  }
  int bad = 0;
  if (na != nb) {
    std::printf("FAIL: fact counts differ (%zu vs %zu)\n", na, nb);
    return 1;
  }
  for (size_t i = 0; i < na; ++i) {
    if (a[i].value != b[i].value) {
      ++bad;
      std::printf("  MOVED  %-48s c++11=%u  c++17=%u\n", a[i].what, a[i].value, b[i].value);
    }
  }
  std::printf("%zu layout facts compared across the two levels, %d moved\n", na, bad);
  std::fflush(stdout);
  if (bad) {
    std::printf("a moved layout is an ODR violation: the objects below would be misread,\n"
                "so the check stops here rather than demonstrating it by crashing.\n");
    return 1;
  }

  shapes::TaskDetailed t1, t2;
  shapes::Probe p1, p2;
  odr_build_11(&t1, &p1);
  odr_build_17(&t2, &p2);
  bool ok = true;
  if (!odr_read_17(t1, p1)) { ok = false; std::printf("  FAIL: c++17 cannot read what c++11 built\n"); }
  if (!odr_read_11(t2, p2)) { ok = false; std::printf("  FAIL: c++11 cannot read what c++17 built\n"); }
  if (!(t1 == t2) || !(p1 == p2)) { ok = false; std::printf("  FAIL: the two objects differ\n"); }
  std::printf("objects passed across the seam in both directions: %s\n", ok ? "ok" : "FAILED");
  return (bad || !ok) ? 1 : 0;
}
