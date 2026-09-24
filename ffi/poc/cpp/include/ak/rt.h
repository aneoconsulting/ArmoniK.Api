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

// Which validator the decode path uses. The DFA is the default because it is the one a
// core would ship; the others exist so the policy can be priced against all of them in
// one process rather than across binaries separated by R4's 0.240 drift bar.
#ifndef AK_UTF8
#define AK_UTF8 1
#endif


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

// ---- the two designs ABI v1 section 6 REFUSED, buildable so the refusal has evidence --
//
// Both default to off and neither is in any timed or gated binary. They exist for
// `src/concurrency.cpp` (ABI v1 obligation 12.5), because a concurrency suite that has
// only ever passed has not been seen working, and because section 6's two refusals were
// arguments rather than measurements:
//
//   AK_CONC_PAD     pad the length prefix to the LEARNED width instead of writing the
//                   minimal one and moving. Section 6: "padding to the learned width makes
//                   the encoder's output depend on its own history, which a byte-vector
//                   corpus cannot express". Still per-Enc, so this is a SEQUENTIAL defect
//                   and it needs TWO PAYLOAD SHAPES to be visible at all.
//   AK_CONC_GLOBAL  put the learned-width table in a process-global instead of in the
//                   context. Section 6: "a global table made two encoding threads slower
//                   than one". On its own it is a data race and a throughput defect but
//                   not a byte defect; with AK_CONC_PAD it is section 6's other sentence,
//                   "lets two threads of one process emit two different legal encodings of
//                   the same message", and then it needs CONCURRENCY to be visible.
//
// Two axes, two defects, one visible only in sequence and one only in parallel: which is
// why obligation 12.5 asks for both axes and for more than one shape.
#ifndef AK_CONC_PAD
#define AK_CONC_PAD 0
#endif
#ifndef AK_CONC_GLOBAL
#define AK_CONC_GLOBAL 0
#endif

// A third plant, and the same reason: `src/groupskip.cpp` tests the GROUP skip below, and
// a test that has only ever passed has not been seen working.
//
//   AK_GROUP_PLANT=1  skip a group by counting nesting DEPTH instead of matching the
//                     field number of the tag that opened it. It is the fix everyone
//                     writes first and it ACCEPTS corpus vector `X-group-mismatched-end`,
//                     after which every group in the message is mis-nested.
//   AK_GROUP_PLANT=2  the `case 5:` arm dropped while `case 3:` was being added. That is
//                     not hypothetical: it is what the first run of the SHARED core's
//                     tests caught, and no group test would have noticed it.
#ifndef AK_GROUP_PLANT
#define AK_GROUP_PLANT 0
#endif

namespace ak {

#if AK_CONC_GLOBAL
// Deliberately unsynchronised, which is the point: the refused design has no place to put
// a lock that would not cost more than the table saves.
inline std::vector<uint8_t> &conc_global_widths(std::size_t sites) {
  static std::vector<uint8_t> w;
  if (w.size() < sites) w.resize(sites, 1);
  return w;
}
#endif

const int32_t ERR_MALFORMED = -2;
const int32_t ERR_TRUNCATED = -3;
// AK_ERR_DEPTH of ABI v1 section 5 / `ak_abi.h`. The decode recursion limit, which only
// nested unknown GROUPs can reach in this slice: every other nesting in the description
// is length-delimited and bounded by the description itself.
const int32_t ERR_DEPTH = -4;
const int32_t ERR_TRANSCODE = -6;
const int32_t ERR_CAPACITY = -7;

// protobuf's own default recursion limit, applied to nested groups. Namespace scope and
// `const`, so it has internal linkage, needs no out-of-line definition at C++11, and adds
// nothing to the layout of any installed type.
const uint32_t MAX_GROUP_DEPTH = 100;

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

// The plan's implicit-presence test for a double compares BIT PATTERNS (R-E3), so -0.0
// is written and +0.0 is not.
inline uint64_t f64_bits(double v) {
  uint64_t b;
  std::memcpy(&b, &v, 8);
  return b;
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
  // The learned widths, readable. `src/concurrency.cpp` needs them: whether a pair of
  // payloads can exercise the history surface AT ALL is a property of the pair, and a
  // history test run on a pair that moves no width is a test of nothing.
  const std::vector<uint8_t> &widths() const { return widths_; }
  // Test-only, and off the hot path by construction: it is a setter, not a probe.
  // `src/concurrency.cpp` needs to know WHICH sites a payload writes, and there is no
  // way to tell a site left at width 1 from a site never visited. Priming every width
  // to a value nothing needs and seeing which come back reduced answers it exactly,
  // without a `touched` store in `begin()` that every timed arm would pay for.
  void prime_widths(uint8_t w) {
    for (std::size_t i = 0; i < widths_.size(); ++i) widths_[i] = w;
  }
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
  // fixed32 (plan value codec `fixed32_u32`): 4 bytes little-endian.
  inline void fixed32_field(uint32_t tag, uint32_t v) {
    key(tag, 5);
    u32_raw(v);
  }
  inline void u32_raw(uint32_t v) {
    ensure(4);
    uint8_t *d = &storage_[0] + len_;
    d[0] = (uint8_t)v; d[1] = (uint8_t)(v >> 8); d[2] = (uint8_t)(v >> 16); d[3] = (uint8_t)(v >> 24);
    len_ += 4;
  }
  // One element of a packed double run: the IEEE-754 bits, little-endian.
  inline void f64_raw(double v) {
    uint64_t bits;
    std::memcpy(&bits, &v, 8);
    ensure(8);
    uint8_t *d = &storage_[0] + len_;
    for (int i = 0; i < 8; ++i) d[i] = (uint8_t)(bits >> (8 * i));
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
    m.w = width(site);
    m.hdr = len_;
    ensure(m.w);
    std::memset(&storage_[0] + len_, 0, m.w);
    len_ += m.w;
    return m;
  }

  inline void end(const Mark &m) {
    std::size_t body = len_ - m.hdr - m.w;
    std::size_t need = varint_len((uint64_t)body);
#if AK_CONC_PAD
    // Section 6's refused form: keep the reserved width and write a NON-MINIMAL varint
    // into it. Legal protobuf, a different byte string, and a function of what this
    // encoder happened to encode before.
    if (need < m.w) {
      uint64_t pv = (uint64_t)body;
      uint8_t *pd = &storage_[0] + m.hdr;
      for (std::size_t i = 0; i + 1 < m.w; ++i) { *pd++ = (uint8_t)(pv) | 0x80; pv >>= 7; }
      *pd = (uint8_t)pv;
      return;
    }
#endif
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
  inline uint8_t &width(std::size_t site) {
#if AK_CONC_GLOBAL
    return conc_global_widths(widths_.size())[site];
#else
    return widths_[site];
#endif
  }
  void resize_prefix(const Mark &m, std::size_t body, std::size_t need) {
    width(m.site) = (uint8_t)need;
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
    // Checked form (R-D1): never add to `pos` before comparing.
    if (pos > len || len - pos < 8) { err = ERR_TRUNCATED; return 0.0; }
    uint64_t bits = 0;
    for (int i = 0; i < 8; ++i) bits |= (uint64_t)buf[pos + i] << (8 * i);
    pos += 8;
    double v;
    std::memcpy(&v, &bits, 8);
    return v;
  }
  inline uint32_t fixed32() {
    if (pos > len || len - pos < 4) { err = ERR_TRUNCATED; return 0; }
    uint32_t v = (uint32_t)buf[pos] | ((uint32_t)buf[pos + 1] << 8) |
                 ((uint32_t)buf[pos + 2] << 16) | ((uint32_t)buf[pos + 3] << 24);
    pos += 4;
    return v;
  }
  // (offset, length) into the ONE buffer the host handed in: what `ak_span` is.
  inline void len_body(std::size_t *off, std::size_t *n) {
    // R-D1: `k` comes off the wire, so `pos + k` can wrap past SIZE_MAX and pass the
    // comparison. Compare against the REMAINING length instead, which cannot wrap:
    // `pos <= len` holds here because `varint()` never reads past `len`. The value is
    // also checked before narrowing to size_t, for a 32-bit host.
    uint64_t k64 = varint();
    if (err != 0) { *off = pos; *n = 0; return; }
    if (pos > len || k64 > (uint64_t)(len - pos)) {
      err = ERR_TRUNCATED; *off = pos; *n = 0; return;
    }
    std::size_t k = (std::size_t)k64;
    *off = pos;
    *n = k;
    pos += k;
  }
  // An unknown field: proto3's forward compatibility, and the one decode path a corpus
  // generated from the schema that reads it never executes.
  //
  // `tag` is the field number the wire type arrived with, and it is not decoration. The
  // deprecated GROUP form (wire type 3) carries NO LENGTH, so the only way to find a
  // group's end is to read fields until an END_GROUP whose field number MATCHES the one
  // that opened it. A skipper that counts nesting depth instead accepts a mismatched end
  // tag -- corpus vector `X-group-mismatched-end` -- and then mis-nests every group after
  // it. That is why the tag is a parameter rather than a nesting counter.
  //
  // proto3 cannot express a group, so nothing this slice's generator emits produces one
  // and byte identity against the manifest cannot reach this arm. It is still a message a
  // conformant parser must accept: protobuf C++ and upb both do. Tested by
  // `src/groupskip.cpp` against two planted defects, and by `gen/corpus.py` against the
  // five corpus vectors.
  inline void skip(uint32_t tag, uint32_t wire) {
    std::size_t o, n;
    switch (wire) {
      case 0: varint(); break;
      case 1: pos += 8; break;
      case 2: len_body(&o, &n); break;
      case 3: skip_group(tag, 0); break;
#if AK_GROUP_PLANT == 2
      // PLANTED: the 32-bit arm dropped while the group arm was added.
#else
      case 5: pos += 4; break;
#endif
      // 4 is END_GROUP with nothing open; 6 and 7 are not wire types at all.
      default: err = ERR_MALFORMED; break;
    }
    if (pos > len) err = ERR_TRUNCATED;
  }

  // The GROUP skip. Recursive, because groups nest, and BOUNDED, because a payload of
  // nothing but start tags would otherwise be a stack overflow inside the host's process
  // rather than an error -- which is what ERR_DEPTH is for (ABI v1 section 5).
  //
  // Mutually recursive with `skip` and defined inside the class, which is well-formed at
  // C++11: a member function body is compiled in the complete-class context.
  inline void skip_group(uint32_t tag, uint32_t depth) {
    if (depth >= MAX_GROUP_DEPTH) { err = ERR_DEPTH; return; }
    for (;;) {
      if (err != 0) return;
      if (pos >= len) { err = ERR_TRUNCATED; return; }  // X-group-unterminated
      uint64_t k = varint();
      if (err != 0) return;
      uint32_t t = (uint32_t)(k >> 3), w = (uint32_t)(k & 7);
      if (t == 0) { err = ERR_MALFORMED; return; }
      if (w == 4) {
#if AK_GROUP_PLANT == 1
        // PLANTED: any END_GROUP closes any group, which is the depth counter.
        (void)tag;
#else
        if (t != tag) err = ERR_MALFORMED;
#endif
        return;
      }
      if (w == 3) { skip_group(t, depth + 1); continue; }
      skip(t, w);
    }
  }
};

// ABI v1 open decision 3, decode side: the parser carries the UTF-8 guarantee, because the
// encoder's check bought nothing and the parser cannot trust the wire. One build-time
// choice, never a per-site one. Default here is REJECT, which is what the specification
// settled on and what protobuf C++ itself does (it reports a parse failure on a bad
// `string`). `AK_DEC_LOSSY` builds the other policy so the two can be priced.
// ---- the decode policy's UTF-8 validator ------------------------------------------
//
// Three implementations of one predicate, because the slice's largest single measured
// effect -- 4.5x to 20x on the string path for "check" against "raw" -- turned out to be a
// figure about THIS VALIDATOR rather than about the policy ABI v1 open decision 3 settles.
// 38 ns to validate a 36-byte ASCII string is about 1 ns per byte, and a decision should
// not be priced against the slowest reasonable implementation of the thing being decided.
//
//   utf8_valid_scalar  the original: decode each code point, then range-check it. One
//                      unpredictable branch per byte, and the arithmetic is wasted because
//                      validation never needs the code point's value.
//   utf8_valid_dfa     a 9-state, 12-class table. One table lookup and one add per byte
//                      and no data-dependent branch -- but the state is a SERIAL
//                      DEPENDENCY, one dependent load per byte, and on non-ASCII that
//                      costs more than the branches it removes. Measured, kept, and not
//                      the default: see gen/utf8.sh.
//   utf8_valid_table   the one that wins. Three 256-entry tables indexed by the LEAD byte
//                      give the sequence length and the legal range of the second byte, so
//                      a whole code point is consumed per iteration with no state carried
//                      and no code point computed. C++11, portable, no intrinsics: it is
//                      what a core could actually ship to every host.
//   utf8_valid_protobuf  the CEILING, and it costs nothing to have: protobuf C++'s own
//                      `IsStructurallyValidUTF8`, already linked into every arm of this
//                      slice. R14 says the comparison is against what production runs, and
//                      this is literally the code the incumbent runs on every `string`
//                      field it parses. No fetch, no vendoring, no third-party build.
//
// `utf8_valid` is the one the decode path calls. AK_UTF8 picks which, so the policy can be
// priced against each without two binaries: 0 = scalar, 1 = dfa (default), 2 = range.
bool utf8_valid_scalar(const uint8_t *p, std::size_t n);
bool utf8_valid_dfa(const uint8_t *p, std::size_t n);
bool utf8_valid_table(const uint8_t *p, std::size_t n);
// The INCUMBENT's own validator, which is the one R14 says to compare against: it is
// what protobuf C++ runs on every `string` field it parses, in this very process.
// Declared here so `ak/rt.h` stays the one place the predicate is spelled; defined in
// rt.cpp, which is the only translation unit in the runtime that includes protobuf.
bool utf8_valid_protobuf(const uint8_t *p, std::size_t n);
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

// The same, with the validator named rather than selected, so the string-path table can
// carry one column per implementation IN ONE PROCESS. Pricing a policy against three
// binaries would put R4's 0.240 across-build bar on top of the effect being measured.
inline int32_t decode_str_with(bool (*valid)(const uint8_t *, std::size_t), const uint8_t *p,
                               std::size_t n, std::string *out) {
  if (!valid(p, n)) return ERR_TRANSCODE;
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
