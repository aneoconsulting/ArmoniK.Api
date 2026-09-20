// The one non-inline piece of the runtime: the UTF-8 validator the decode policy uses.
//
// Three of them now. ABI v1 open decision 3 asks what the decode-side UTF-8 check costs,
// and this slice's answer -- 4.5x to 20x on the string path -- was a figure about the
// implementation below rather than about the policy. A decision is not priced against the
// slowest reasonable implementation of the thing being decided, so the policy is now
// measured against a validator a core would actually ship.
#include "ak/rt.h"

#include <google/protobuf/stubs/common.h>

namespace ak {

// ---- 1. the original: decode, then range-check ------------------------------------
//
// Kept, not deleted: every string-path figure this slice published before today was taken
// against it, and a row that cannot be reproduced is a row that has to be withdrawn rather
// than corrected. It is also the honest baseline for "what does a validator written the
// obvious way cost".
bool utf8_valid_scalar(const uint8_t *p, std::size_t n) {
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

// ---- 2. the table-driven one -------------------------------------------------------
//
// A 9-state machine over 12 byte classes. Validation never needs a code point's VALUE, so
// this computes none: one class lookup, one transition lookup, per byte.
//
// The states and the transitions are written out below rather than transcribed as a magic
// table, because the whole value of a validator is that it rejects what it should. The
// three constraints that a naive decoder gets wrong are each one state:
//
//   S_E0  after E0, the next byte must be A0..BF   -- an overlong three-byte form
//   S_ED  after ED, the next byte must be 80..9F   -- a UTF-16 surrogate, D800..DFFF
//   S_F0  after F0, the next byte must be 90..BF   -- an overlong four-byte form
//   S_F4  after F4, the next byte must be 80..8F   -- a code point above U+10FFFF
//
// and C0, C1 and F5..FF are rejected at the lead byte because no valid sequence begins
// with them. `gen/utf8_test.py`-shaped differential coverage lives in `src/utf8check.cpp`:
// every implementation must agree with every other on every vector, which is the only
// reason to trust a fast validator over a slow one.
namespace {

enum { S_ACC = 0, S_1, S_2, S_3, S_E0, S_ED, S_F0, S_F4, S_REJ, N_STATE };
enum { C_ASCII = 0, C_80_8F, C_90_9F, C_A0_BF, C_C2_DF, C_E1_EF, C_F1_F3,
       C_BAD, C_E0, C_ED, C_F0, C_F4, N_CLASS };

struct Tables {
  uint8_t cls[256];
  uint8_t next[N_STATE * N_CLASS];

  Tables() {
    for (int b = 0; b < 256; ++b) {
      if (b < 0x80) cls[b] = C_ASCII;
      else if (b < 0x90) cls[b] = C_80_8F;
      else if (b < 0xA0) cls[b] = C_90_9F;
      else if (b < 0xC0) cls[b] = C_A0_BF;
      else if (b < 0xC2) cls[b] = C_BAD;       // C0, C1: overlong two-byte lead
      else if (b < 0xE0) cls[b] = C_C2_DF;
      else if (b == 0xE0) cls[b] = C_E0;
      else if (b == 0xED) cls[b] = C_ED;
      else if (b < 0xF0) cls[b] = C_E1_EF;
      else if (b == 0xF0) cls[b] = C_F0;
      else if (b < 0xF4) cls[b] = C_F1_F3;
      else if (b == 0xF4) cls[b] = C_F4;
      else cls[b] = C_BAD;                     // F5..FF: above the Unicode range
    }
    for (int i = 0; i < N_STATE * N_CLASS; ++i) next[i] = S_REJ;
    set(S_ACC, C_ASCII, S_ACC);
    set(S_ACC, C_C2_DF, S_1);
    set(S_ACC, C_E1_EF, S_2);
    set(S_ACC, C_F1_F3, S_3);
    set(S_ACC, C_E0, S_E0);
    set(S_ACC, C_ED, S_ED);
    set(S_ACC, C_F0, S_F0);
    set(S_ACC, C_F4, S_F4);
    // A continuation byte is any of the three 80..BF classes.
    cont(S_1, S_ACC);
    cont(S_2, S_1);
    cont(S_3, S_2);
    set(S_E0, C_A0_BF, S_1);                   // E0 A0..BF
    set(S_ED, C_80_8F, S_1);                   // ED 80..9F
    set(S_ED, C_90_9F, S_1);
    set(S_F0, C_90_9F, S_2);                   // F0 90..BF
    set(S_F0, C_A0_BF, S_2);
    set(S_F4, C_80_8F, S_2);                   // F4 80..8F
    // S_REJ is absorbing: every row was initialised to S_REJ, including its own.
  }
  void set(int st, int c, int to) { next[st * N_CLASS + c] = (uint8_t)to; }
  void cont(int st, int to) {
    set(st, C_80_8F, to);
    set(st, C_90_9F, to);
    set(st, C_A0_BF, to);
  }
};

const Tables &tab() {
  static Tables t;
  return t;
}

}  // namespace

bool utf8_valid_dfa(const uint8_t *p, std::size_t n) {
  const Tables &t = tab();
  std::size_t i = 0;
  uint32_t st = S_ACC;
  while (i < n) {
    // ASCII is the overwhelming majority of every real payload, and it is the only stretch
    // that can be skipped without consulting the machine -- but only from the accepting
    // state, or a partial sequence would be dropped. Eight bytes per test.
    if (st == S_ACC) {
      while (i + 8 <= n) {
        uint64_t w;
        std::memcpy(&w, p + i, 8);
        if (w & 0x8080808080808080ULL) break;
        i += 8;
      }
      if (i >= n) break;
    }
    st = t.next[st * N_CLASS + t.cls[p[i]]];
    if (st == S_REJ) return false;
    ++i;
  }
  return st == S_ACC;
}

// ---- 3. the lead-byte table ----------------------------------------------------------
//
// The DFA above removes every branch and pays for it with a serial dependency: the next
// state is a load whose address depends on the previous load, one per byte, and on latin1
// and wide content that is measurably WORSE than the scalar version it replaces. The
// scalar version's branches are near-perfectly predicted when the content is uniform,
// which real payload fields are.
//
// So: keep the scalar shape -- consume a whole sequence per iteration -- and remove what
// is actually wasted in it, which is the code-point arithmetic. Validation never needs the
// value, only the ranges, and every range constraint in UTF-8 is a function of the LEAD
// byte alone:
//
//   kLen[c]  how many bytes the sequence is, or 0 if c cannot begin one
//   kLo[c]   the smallest legal SECOND byte, and
//   kHi[c]   the largest -- which is where overlong forms, surrogates and everything above
//            U+10FFFF are rejected, because each of those is exactly a constraint on the
//            second byte given the first.
//
// The third and fourth bytes have no lead-dependent constraint: they are 80..BF or the
// sequence is invalid.
namespace {

// ONE table, not three. Three parallel byte arrays are three dependent loads per code
// point off three cache lines; packing them into a single u32 makes it one load, and on
// the latin1 set -- two-byte sequences, so the table is consulted every other byte -- that
// is most of the difference between 1.07x and something worth shipping.
//   bits  0..7   the sequence length, 0 if this byte cannot begin one
//   bits  8..15  the smallest legal SECOND byte
//   bits 16..23  the largest
struct Lead {
  uint32_t info[256];
  Lead() {
    uint8_t len[256], lo[256], hi[256];
    for (int c = 0; c < 256; ++c) { len[c] = 0; lo[c] = 0x80; hi[c] = 0xBF; }
    for (int c = 0x00; c <= 0x7F; ++c) len[c] = 1;
    for (int c = 0xC2; c <= 0xDF; ++c) len[c] = 2;
    for (int c = 0xE0; c <= 0xEF; ++c) len[c] = 3;
    for (int c = 0xF0; c <= 0xF4; ++c) len[c] = 4;
    lo[0xE0] = 0xA0;              // E0 80..9F would be an overlong three-byte form
    hi[0xED] = 0x9F;              // ED A0..BF is a UTF-16 surrogate, U+D800..U+DFFF
    lo[0xF0] = 0x90;              // F0 80..8F would be an overlong four-byte form
    hi[0xF4] = 0x8F;              // F4 90..BF is above U+10FFFF
    // C0, C1 and F5..FF keep len 0: no valid sequence begins with them.
    for (int c = 0; c < 256; ++c)
      info[c] = (uint32_t)len[c] | ((uint32_t)lo[c] << 8) | ((uint32_t)hi[c] << 16);
  }
};

const Lead &lead() {
  static Lead l;
  return l;
}

}  // namespace

bool utf8_valid_table(const uint8_t *p, std::size_t n) {
  const Lead &L = lead();
  std::size_t i = 0;
  while (i < n) {
    if (p[i] < 0x80) {
      // Eight ASCII bytes per test. Real payload fields are ids and names, so this is the
      // loop that runs, and it is why the ASCII column moves at all.
      while (i + 8 <= n) {
        uint64_t w;
        std::memcpy(&w, p + i, 8);
        if (w & 0x8080808080808080ULL) break;
        i += 8;
      }
      while (i < n && p[i] < 0x80) ++i;
      continue;
    }
    uint32_t info = L.info[p[i]];
    std::size_t k = info & 0xFF;
    if (k < 2 || i + k > n) return false;
    uint32_t b1 = p[i + 1];
    if (b1 < ((info >> 8) & 0xFF) || b1 > ((info >> 16) & 0xFF)) return false;
    if (k > 2 && (p[i + 2] & 0xC0) != 0x80) return false;
    if (k > 3 && (p[i + 3] & 0xC0) != 0x80) return false;
    i += k;
  }
  return true;
}

// ---- 4. the ceiling, for free -------------------------------------------------------
//
// protobuf C++'s own validator. R14: the baseline is what ArmoniK actually runs, and on
// the decode side of a `string` field what it runs is exactly this. It is already linked
// into every arm here, so the ceiling costs a declaration rather than a build system --
// which is also why it is a better ceiling than a fetched SIMD library would be: nobody
// has to believe a claim about how it was configured.
bool utf8_valid_protobuf(const uint8_t *p, std::size_t n) {
  return google::protobuf::internal::IsStructurallyValidUTF8((const char *)p, (int)n);
}

// ---- 5. unused ------------------------------------------------------------------
#if AK_HAVE_UTF8RANGE
bool utf8_valid_range(const uint8_t *p, std::size_t n) {
  return utf8_range_IsValid((const char *)p, n) != 0;
}
#endif

// ---- what the decode path calls --------------------------------------------------
bool utf8_valid(const uint8_t *p, std::size_t n) {
#if AK_UTF8 == 0
  return utf8_valid_scalar(p, n);
#elif AK_UTF8 == 2
  return utf8_valid_protobuf(p, n);
#elif AK_UTF8 == 3
  return utf8_valid_dfa(p, n);
#else
  return utf8_valid_table(p, n);
#endif
}

}  // namespace ak
