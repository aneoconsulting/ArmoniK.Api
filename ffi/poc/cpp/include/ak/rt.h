// The codec's runtime support, C++ side. The counterpart of the rust slice's `ak-rt`.
//
// It is runtime support, NOT a codec: every per-message encoder and decoder in this slice
// is generated (README R1). Sharing it between the `core-native-cpp` arm and the payload
// tooling is what makes that arm a control for the C ABI arms rather than a different
// codec -- the buffer, the varints and the learned length widths are the same code.
//
// The learned length-prefix width lives in the Enc, never process-global (ABI v1 section
// 6), and it deliberately survives a reset: that is what makes it learned.
#ifndef AK_RT_H
#define AK_RT_H

#include <cstdint>
#include <cstring>
#include <string>
#include <vector>

// README 5.1: the floor and the target may be DIFFERENT CODE, from one generator with a
// target level, and the wire bytes must be identical across levels (which `conformance`
// checks at every level this slice claims). `AK_FLOOR_IMPL` forces the floor path on at
// the target level, which is README 5.2's arm b: what the floor's missing APIs cost with
// the compiler and the optimiser held constant.
#if !defined(AK_FLOOR_IMPL) && __cplusplus >= 201703L
#define AK_CXX17 1
#else
#define AK_CXX17 0
#endif

namespace ak {

const int32_t ERR_MALFORMED = -2;
const int32_t ERR_TRUNCATED = -3;
const int32_t ERR_TRANSCODE = -6;
const int32_t ERR_CAPACITY = -7;

const uint32_t WIRE_VARINT = 0;
const uint32_t WIRE_I64 = 1;
const uint32_t WIRE_LEN = 2;

// ABI v1 section 7.3: a run is materialised in a fixed-size arena sized as a BYTE BUDGET
// divided by the group size, not an element count.
const std::size_t ARENA_BYTES = 32 * 1024;

// constexpr so a chunk buffer is a real array with a compile-time bound at C++11, which
// is what keeps the arena off the heap and out of the measurement.
constexpr std::size_t arena_n(std::size_t group_size) {
  return ARENA_BYTES / group_size == 0 ? 1 : ARENA_BYTES / group_size;
}

inline std::size_t varint_len(uint64_t v) {
  std::size_t n = 1;
  while (v >= 0x80) { v >>= 7; ++n; }
  return n;
}

struct Mark {
  uint32_t site;
  std::size_t hdr;
  std::size_t w;
};

// The encode buffer, as a RAW CURSOR over a reserved block rather than as a sequence of
// `std::vector::push_back` calls.
//
// This is not a micro-optimisation, it is what makes the no-boundary control a CONTROL.
// Through `std::vector` the compiler reloads the finish and end-of-storage pointers on
// every byte, because a `std::vector<uint8_t>` reached through `Enc*` may alias anything;
// Rust's `&mut Vec<u8>` carries noalias and does not. The first build of this file used
// `push_back` and measured `core-native-cpp` at 1.245 of protobuf C++ on P1.2 encode while
// the SAME codec through the C ABI measured 0.855 -- a control slower than the arm it is
// controlling for, which would have made every subtraction against it meaningless. protobuf
// C++'s own serialiser is a raw cursor for the same reason, so this is also the fair shape
// to compare against. The difference is in this file, not in the generated traversal.
class Enc {
 public:
  explicit Enc(std::size_t sites) : err(0), len_(0), widths_(sites, 1) {
    storage_.resize(4096);
  }

  int32_t err;

  inline std::size_t size() const { return len_; }
  inline const uint8_t *data() const { return storage_.empty() ? NULL : &storage_[0]; }

  void reset() {
    len_ = 0;
    err = 0;
    // The learned widths deliberately SURVIVE a reset.
  }
  void fail(int32_t code) { if (err == 0) err = code; }

  inline void ensure(std::size_t n) {
    if (len_ + n > storage_.size()) grow(len_ + n);
  }

  inline void varint(uint64_t v) {
    ensure(10);
    uint8_t *d = &storage_[0] + len_;
    while (v >= 0x80) { *d++ = (uint8_t)(v) | 0x80; v >>= 7; }
    *d++ = (uint8_t)v;
    len_ = (std::size_t)(d - &storage_[0]);
  }
  inline void key(uint32_t tag, uint32_t wire) { varint(((uint64_t)tag << 3) | wire); }
  inline void varint_field(uint32_t tag, uint64_t v) {
    ensure(20);
    uint8_t *d = &storage_[0] + len_;
    uint64_t k = ((uint64_t)tag << 3) | WIRE_VARINT;
    while (k >= 0x80) { *d++ = (uint8_t)(k) | 0x80; k >>= 7; }
    *d++ = (uint8_t)k;
    while (v >= 0x80) { *d++ = (uint8_t)(v) | 0x80; v >>= 7; }
    *d++ = (uint8_t)v;
    len_ = (std::size_t)(d - &storage_[0]);
  }
  inline void f64_field(uint32_t tag, double v) {
    key(tag, WIRE_I64);
    ensure(8);
    uint64_t bits;
    std::memcpy(&bits, &v, 8);
    std::memcpy(&storage_[0] + len_, &bits, 8);
    len_ += 8;
  }
  inline void blob_field(uint32_t tag, const char *p, std::size_t n) {
    ensure(20 + n);
    uint8_t *d = &storage_[0] + len_;
    uint64_t k = ((uint64_t)tag << 3) | WIRE_LEN;
    while (k >= 0x80) { *d++ = (uint8_t)(k) | 0x80; k >>= 7; }
    *d++ = (uint8_t)k;
    uint64_t m = (uint64_t)n;
    while (m >= 0x80) { *d++ = (uint8_t)(m) | 0x80; m >>= 7; }
    *d++ = (uint8_t)m;
    if (n) std::memcpy(d, p, n);
    len_ = (std::size_t)(d - &storage_[0]) + n;
  }
  inline void blob_field(uint32_t tag, const std::string &s) {
    blob_field(tag, s.data(), s.size());
  }
  inline void raw(const uint8_t *p, std::size_t n) {
    ensure(n);
    std::memcpy(&storage_[0] + len_, p, n);
    len_ += n;
  }

  inline Mark begin(uint32_t tag, uint32_t site) {
    key(tag, WIRE_LEN);
    Mark m;
    m.site = site;
    m.w = widths_[site];
    m.hdr = len_;
    ensure(m.w);
    std::memset(&storage_[0] + len_, 0, m.w);
    len_ += m.w;
    return m;
  }

  inline void end(const Mark &m) {
    std::size_t body = len_ - m.hdr - m.w;
    std::size_t need = varint_len((uint64_t)body);
    if (need != m.w) resize_prefix(m, body, need);
    uint64_t v = (uint64_t)body;
    uint8_t *d = &storage_[0] + m.hdr;
    while (v >= 0x80) { *d++ = (uint8_t)(v) | 0x80; v >>= 7; }
    *d = (uint8_t)v;
  }

 private:
  void grow(std::size_t want) {
    std::size_t n = storage_.size() * 2;
    if (n < want) n = want;
    storage_.resize(n);
  }
  void resize_prefix(const Mark &m, std::size_t body, std::size_t need) {
    widths_[m.site] = (uint8_t)need;
    if (need > m.w) { ensure(need - m.w); len_ += need - m.w; }
    std::memmove(&storage_[0] + m.hdr + need, &storage_[0] + m.hdr + m.w, body);
    if (need < m.w) len_ -= (m.w - need);
  }
  std::vector<uint8_t> storage_;
  std::size_t len_;
  std::vector<uint8_t> widths_;
};

class Dec {
 public:
  Dec(const uint8_t *b, std::size_t n) : buf(b), len(n), pos(0), err(0) {}
  const uint8_t *buf;
  std::size_t len;
  std::size_t pos;
  int32_t err;

  inline bool at_end() const { return pos >= len || err != 0; }
  inline uint64_t varint() {
    uint64_t n = 0;
    uint32_t shift = 0;
    for (;;) {
      if (pos >= len) { err = ERR_TRUNCATED; return 0; }
      uint8_t c = buf[pos++];
      n |= (uint64_t)(c & 0x7f) << shift;
      if ((c & 0x80) == 0) return n;
      shift += 7;
      if (shift > 63) { err = ERR_MALFORMED; return 0; }
    }
  }
  inline double f64() {
    if (pos + 8 > len) { err = ERR_TRUNCATED; return 0.0; }
    uint64_t bits = 0;
    for (int i = 0; i < 8; ++i) bits |= (uint64_t)buf[pos + i] << (8 * i);
    pos += 8;
    double v;
    std::memcpy(&v, &bits, 8);
    return v;
  }
  // (offset, length) into the ONE buffer the host handed in: what `ak_span` is.
  inline void len_body(std::size_t *off, std::size_t *n) {
    std::size_t k = (std::size_t)varint();
    if (pos + k > len) { err = ERR_TRUNCATED; *off = pos; *n = 0; return; }
    *off = pos;
    *n = k;
    pos += k;
  }
  inline void skip(uint32_t wire) {
    std::size_t o, n;
    switch (wire) {
      case 0: varint(); break;
      case 1: pos += 8; break;
      case 2: len_body(&o, &n); break;
      case 5: pos += 4; break;
      default: err = ERR_MALFORMED; break;
    }
    if (pos > len) err = ERR_TRUNCATED;
  }
};

// ABI v1 open decision 3, decode side: the parser carries the UTF-8 guarantee, because the
// encoder's check bought nothing and the parser cannot trust the wire. One build-time
// choice, never a per-site one. Default here is REJECT, which is what the specification
// settled on and what protobuf C++ itself does (it reports a parse failure on a bad
// `string`). `AK_DEC_LOSSY` builds the other policy so the two can be priced.
bool utf8_valid(const uint8_t *p, std::size_t n);

// Both policies exist as named functions so a benchmark can carry BOTH IN ONE BINARY and
// price them as two arms in the same interleaved rounds. `decode_str` is the one the
// generated codec calls and is selected at build time, because a runtime branch per string
// would be a cost of its own.
inline int32_t decode_str_checked(const uint8_t *p, std::size_t n, std::string *out) {
  if (!utf8_valid(p, n)) return ERR_TRANSCODE;
  out->assign((const char *)p, n);
  return 0;
}

inline int32_t decode_str_raw(const uint8_t *p, std::size_t n, std::string *out) {
  out->assign((const char *)p, n);
  return 0;
}

inline int32_t decode_str(const uint8_t *p, std::size_t n, std::string *out) {
#ifdef AK_DEC_LOSSY
  return decode_str_raw(p, n, out);
#else
  return decode_str_checked(p, n, out);
#endif
}

}  // namespace ak
#endif  // AK_RT_H
