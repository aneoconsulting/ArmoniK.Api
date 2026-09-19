// The one non-inline piece of the runtime: the UTF-8 validator the decode policy uses.
// Scalar, deliberately: the rust slice priced a SIMD validator separately and that is a
// validator question rather than an ABI one.
#include "ak/rt.h"

namespace ak {

bool utf8_valid(const uint8_t *p, std::size_t n) {
  std::size_t i = 0;
  while (i < n) {
    uint8_t c = p[i];
    if (c < 0x80) { ++i; continue; }
    std::size_t need;
    uint32_t cp;
    if ((c & 0xE0) == 0xC0) { need = 1; cp = c & 0x1F; }
    else if ((c & 0xF0) == 0xE0) { need = 2; cp = c & 0x0F; }
    else if ((c & 0xF8) == 0xF0) { need = 3; cp = c & 0x07; }
    else { return false; }
    if (i + need >= n) return false;
    for (std::size_t k = 1; k <= need; ++k) {
      uint8_t d = p[i + k];
      if ((d & 0xC0) != 0x80) return false;
      cp = (cp << 6) | (d & 0x3F);
    }
    if (need == 1 && cp < 0x80) return false;
    if (need == 2 && cp < 0x800) return false;
    if (need == 3 && cp < 0x10000) return false;
    if (cp > 0x10FFFF) return false;
    if (cp >= 0xD800 && cp <= 0xDFFF) return false;
    i += need + 1;
  }
  return true;
}

}  // namespace ak
