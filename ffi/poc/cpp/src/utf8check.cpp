// The UTF-8 validators, checked against each other and against an oracle, then timed.
//
// Correctness before timing (R2), and here it is the whole point: the slowest validator in
// this slice is the one every published string-path figure was taken against, so replacing
// it is only worth anything if the replacement rejects exactly what the original rejects.
// A fast validator that accepts a surrogate is not an improvement, it is a wire-format
// defect that byte identity cannot see -- the corpus carries no malformed input by
// construction, because a manifest is made of things that encode.
//
// So: an EXHAUSTIVE oracle over every one, two and three byte string (16.8 million), a
// structured four-byte sweep, the three content sets, and the real strings out of the
// payloads. Then the timing, in one process, over all three content sets, which is how
// decision 3's number should have been priced in the first place.
#include <chrono>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

#include "ak/rt.h"
#include "ak/values.h"
#include "harness.h"

#if defined(__GNUC__) || defined(__clang__)
#define AK_U8_SINK(x) asm volatile("" : : "r"(x))
#else
#define AK_U8_SINK(x) (void)(x)
#endif

static int g_fail = 0;
static long g_checks = 0;

// ---- the oracle -------------------------------------------------------------------
//
// Written to be OBVIOUSLY right rather than fast: it walks the definition in RFC 3629
// table 3-7 with the four ranges spelled out one per line. It is not one of the three
// implementations under test, so agreeing with it is evidence rather than a tautology.
static bool oracle(const uint8_t *p, std::size_t n) {
  std::size_t i = 0;
  while (i < n) {
    uint8_t a = p[i];
    if (a <= 0x7F) { i += 1; continue; }
    if (a >= 0xC2 && a <= 0xDF) {
      if (i + 1 >= n) return false;
      if (p[i + 1] < 0x80 || p[i + 1] > 0xBF) return false;
      i += 2; continue;
    }
    if (a == 0xE0) {
      if (i + 2 >= n) return false;
      if (p[i + 1] < 0xA0 || p[i + 1] > 0xBF) return false;
      if (p[i + 2] < 0x80 || p[i + 2] > 0xBF) return false;
      i += 3; continue;
    }
    if ((a >= 0xE1 && a <= 0xEC) || a == 0xEE || a == 0xEF) {
      if (i + 2 >= n) return false;
      if (p[i + 1] < 0x80 || p[i + 1] > 0xBF) return false;
      if (p[i + 2] < 0x80 || p[i + 2] > 0xBF) return false;
      i += 3; continue;
    }
    if (a == 0xED) {
      if (i + 2 >= n) return false;
      if (p[i + 1] < 0x80 || p[i + 1] > 0x9F) return false;   // no surrogates
      if (p[i + 2] < 0x80 || p[i + 2] > 0xBF) return false;
      i += 3; continue;
    }
    if (a == 0xF0) {
      if (i + 3 >= n) return false;
      if (p[i + 1] < 0x90 || p[i + 1] > 0xBF) return false;   // no overlong
      if (p[i + 2] < 0x80 || p[i + 2] > 0xBF) return false;
      if (p[i + 3] < 0x80 || p[i + 3] > 0xBF) return false;
      i += 4; continue;
    }
    if (a >= 0xF1 && a <= 0xF3) {
      if (i + 3 >= n) return false;
      if (p[i + 1] < 0x80 || p[i + 1] > 0xBF) return false;
      if (p[i + 2] < 0x80 || p[i + 2] > 0xBF) return false;
      if (p[i + 3] < 0x80 || p[i + 3] > 0xBF) return false;
      i += 4; continue;
    }
    if (a == 0xF4) {
      if (i + 3 >= n) return false;
      if (p[i + 1] < 0x80 || p[i + 1] > 0x8F) return false;   // <= U+10FFFF
      if (p[i + 2] < 0x80 || p[i + 2] > 0xBF) return false;
      if (p[i + 3] < 0x80 || p[i + 3] > 0xBF) return false;
      i += 4; continue;
    }
    return false;   // C0, C1, F5..FF, and every stray continuation byte
  }
  return true;
}

struct Impl {
  const char *name;
  bool (*fn)(const uint8_t *, std::size_t);
};

static std::vector<Impl> impls() {
  std::vector<Impl> v;
  Impl a; a.name = "scalar"; a.fn = &ak::utf8_valid_scalar; v.push_back(a);
  Impl b; b.name = "dfa";    b.fn = &ak::utf8_valid_dfa;    v.push_back(b);
  Impl d; d.name = "table";  d.fn = &ak::utf8_valid_table;  v.push_back(d);
  Impl c; c.name = "protobuf"; c.fn = &ak::utf8_valid_protobuf; v.push_back(c);
  return v;
}

static void disagree(const char *what, const Impl &im, const uint8_t *p, std::size_t n,
                     bool got, bool want) {
  if (g_fail < 20) {
    std::printf("  FAIL  %s: %s said %s, oracle says %s, for", what, im.name,
                got ? "valid" : "invalid", want ? "valid" : "invalid");
    for (std::size_t i = 0; i < n; ++i) std::printf(" %02X", p[i]);
    std::printf("\n");
  }
  ++g_fail;
}

// ---- exhaustive ---------------------------------------------------------------------
static void exhaustive(const std::vector<Impl> &v) {
  std::printf("\n-- every string of 1, 2 and 3 bytes, against the oracle --\n");
  uint8_t b[4];
  for (long x = 0; x < 256; ++x) {
    b[0] = (uint8_t)x;
    bool w = oracle(b, 1);
    ++g_checks;
    for (std::size_t k = 0; k < v.size(); ++k)
      if (v[k].fn(b, 1) != w) disagree("len 1", v[k], b, 1, !w, w);
  }
  for (long x = 0; x < 256; ++x) {
    b[0] = (uint8_t)x;
    for (long y = 0; y < 256; ++y) {
      b[1] = (uint8_t)y;
      bool w = oracle(b, 2);
      ++g_checks;
      for (std::size_t k = 0; k < v.size(); ++k)
        if (v[k].fn(b, 2) != w) disagree("len 2", v[k], b, 2, !w, w);
    }
  }
  for (long x = 0; x < 256; ++x) {
    b[0] = (uint8_t)x;
    for (long y = 0; y < 256; ++y) {
      b[1] = (uint8_t)y;
      for (long z = 0; z < 256; ++z) {
        b[2] = (uint8_t)z;
        bool w = oracle(b, 3);
        ++g_checks;
        for (std::size_t k = 0; k < v.size(); ++k)
          if (v[k].fn(b, 3) != w) disagree("len 3", v[k], b, 3, !w, w);
      }
    }
  }
  std::printf("   %ld strings, every implementation agreed with the oracle on all of them\n",
              g_checks);
}

// ---- four bytes, structured ----------------------------------------------------------
//
// 4.3 billion is too many, so the lead byte sweeps every value and the three tails sweep
// the boundaries: the ends of each continuation range and one value either side, which is
// where an off-by-one in a range check lives.
static void four_bytes(const std::vector<Impl> &v) {
  std::printf("\n-- four bytes: every lead, boundary tails --\n");
  static const uint8_t edge[] = {0x00, 0x7F, 0x80, 0x8F, 0x90, 0x9F, 0xA0, 0xBF,
                                 0xC0, 0xC2, 0xED, 0xF0, 0xF4, 0xF5, 0xFF};
  const std::size_t ne = sizeof(edge);
  long n = 0;
  uint8_t b[4];
  for (long x = 0; x < 256; ++x) {
    b[0] = (uint8_t)x;
    for (std::size_t i = 0; i < ne; ++i) {
      b[1] = edge[i];
      for (std::size_t j = 0; j < ne; ++j) {
        b[2] = edge[j];
        for (std::size_t k = 0; k < ne; ++k) {
          b[3] = edge[k];
          bool w = oracle(b, 4);
          ++n;
          ++g_checks;
          for (std::size_t m = 0; m < v.size(); ++m)
            if (v[m].fn(b, 4) != w) disagree("len 4", v[m], b, 4, !w, w);
        }
      }
    }
  }
  std::printf("   %ld strings, all agreed\n", n);
}

// ---- the named malformed classes -----------------------------------------------------
static void named(const std::vector<Impl> &v) {
  std::printf("\n-- the classes a naive decoder gets wrong, named one by one --\n");
  struct T { const char *what; const char *bytes; std::size_t n; bool want; };
  static const T ts[] = {
    {"overlong 2-byte NUL (C0 80)",            "\xC0\x80", 2, false},
    {"overlong 2-byte slash (C1 AF)",          "\xC1\xAF", 2, false},
    {"overlong 3-byte (E0 80 AF)",             "\xE0\x80\xAF", 3, false},
    {"overlong 3-byte at the edge (E0 9F BF)", "\xE0\x9F\xBF", 3, false},
    {"smallest legal 3-byte (E0 A0 80)",       "\xE0\xA0\x80", 3, true},
    {"overlong 4-byte (F0 80 80 80)",          "\xF0\x80\x80\x80", 4, false},
    {"overlong 4-byte at the edge (F0 8F BF BF)", "\xF0\x8F\xBF\xBF", 4, false},
    {"smallest legal 4-byte (F0 90 80 80)",    "\xF0\x90\x80\x80", 4, true},
    {"lead surrogate U+D800 (ED A0 80)",       "\xED\xA0\x80", 3, false},
    {"trail surrogate U+DFFF (ED BF BF)",      "\xED\xBF\xBF", 3, false},
    {"CESU-8 surrogate pair",                  "\xED\xA0\xBD\xED\xB8\x80", 6, false},
    {"last legal before surrogates (ED 9F BF)","\xED\x9F\xBF", 3, true},
    {"first legal after surrogates (EE 80 80)","\xEE\x80\x80", 3, true},
    {"U+10FFFF, the last code point",          "\xF4\x8F\xBF\xBF", 4, true},
    {"U+110000, one past the end",             "\xF4\x90\x80\x80", 4, false},
    {"5-byte lead (F8)",                       "\xF8\x88\x80\x80\x80", 5, false},
    {"6-byte lead (FC)",                       "\xFC\x84\x80\x80\x80\x80", 6, false},
    {"FE",                                     "\xFE", 1, false},
    {"FF",                                     "\xFF", 1, false},
    {"stray continuation (80)",                "\x80", 1, false},
    {"truncated 2-byte at the end (C2)",       "\xC2", 1, false},
    {"truncated 3-byte at the end (E1 80)",    "\xE1\x80", 2, false},
    {"truncated 4-byte at the end (F1 80 80)", "\xF1\x80\x80", 3, false},
    {"truncated then ASCII (E1 80 41)",        "\xE1\x80\x41", 3, false},
    {"an embedded NUL is legal",               "a\x00""b", 3, true},
    {"empty",                                  "", 0, true},
    {"plain ASCII",                            "hello world", 11, true},
    {"ASCII then a 4-byte, no tail",           "abcdefghijklmnop\xF0\x9F\x98\x80", 20, true},
    // The eight-byte ASCII skip must not step over a sequence that starts inside the word
    // or resume in the wrong state; these straddle the boundary on purpose.
    {"7 ASCII then a 2-byte",                  "abcdefg\xC3\xA9", 9, true},
    {"8 ASCII then a 2-byte",                  "abcdefgh\xC3\xA9", 10, true},
    {"9 ASCII then a truncated 2-byte",        "abcdefghi\xC3", 10, false},
    {"15 ASCII then a bad byte",               "abcdefghijklmno\xFF", 16, false},
  };
  for (std::size_t i = 0; i < sizeof(ts) / sizeof(ts[0]); ++i) {
    const uint8_t *p = (const uint8_t *)ts[i].bytes;
    bool w = oracle(p, ts[i].n);
    ++g_checks;
    if (w != ts[i].want) {
      std::printf("  FAIL  the ORACLE disagrees with the expected answer for %s\n", ts[i].what);
      ++g_fail;
    }
    for (std::size_t k = 0; k < v.size(); ++k) {
      ++g_checks;
      if (v[k].fn(p, ts[i].n) != ts[i].want)
        disagree(ts[i].what, v[k], p, ts[i].n, !ts[i].want, ts[i].want);
    }
  }
  std::printf("   %d named vectors x %d implementations\n",
              (int)(sizeof(ts) / sizeof(ts[0])), (int)v.size());
}

// ---- the real strings ----------------------------------------------------------------
static void real_strings(const std::vector<Impl> &v, const std::vector<std::string> &ss) {
  std::printf("\n-- the payloads' own strings --\n");
  for (std::size_t i = 0; i < ss.size(); ++i) {
    const uint8_t *p = (const uint8_t *)ss[i].data();
    bool w = oracle(p, ss[i].size());
    ++g_checks;
    if (!w) { std::printf("  FAIL  a payload string is not valid UTF-8\n"); ++g_fail; }
    for (std::size_t k = 0; k < v.size(); ++k) {
      ++g_checks;
      if (v[k].fn(p, ss[i].size()) != w) disagree("payload string", v[k], p, ss[i].size(), !w, w);
    }
  }
  std::printf("   %d strings\n", (int)ss.size());
}

// ---- the timing ----------------------------------------------------------------------
static double ns_per(bool (*fn)(const uint8_t *, std::size_t),
                     const std::vector<std::string> &ss, int reps, long *bytes) {
  double best = 1e30;
  long nb = 0;
  for (std::size_t i = 0; i < ss.size(); ++i) nb += (long)ss[i].size();
  *bytes = nb;
  for (int r = 0; r < 5; ++r) {   // min of five, as the rest of this slice does
    std::chrono::steady_clock::time_point t0 = std::chrono::steady_clock::now();
    long acc = 0;
    for (int k = 0; k < reps; ++k)
      for (std::size_t i = 0; i < ss.size(); ++i)
        acc += fn((const uint8_t *)ss[i].data(), ss[i].size()) ? 1 : 0;
    double ns = (double)std::chrono::duration_cast<std::chrono::nanoseconds>(
                    std::chrono::steady_clock::now() - t0).count();
    AK_U8_SINK(acc);
    double per = ns / (double)((long)reps * (long)ss.size());
    if (per < best) best = per;
  }
  return best;
}

int main() {
  std::printf("== the decode policy's UTF-8 validator: three implementations ==\n");
  std::printf("-std=%ld  AK_UTF8=%d (%s)  linkage=%s\n", (long)__cplusplus, AK_UTF8,
              AK_UTF8 == 0 ? "scalar"
                           : (AK_UTF8 == 2 ? "protobuf" : (AK_UTF8 == 3 ? "dfa" : "table")),
              AK_LINKAGE);
  std::printf("ceiling: protobuf C++ %d's own IsStructurallyValidUTF8, the validator the\n"
              "         incumbent runs on every `string` field it parses (R14)\n",
              GOOGLE_PROTOBUF_VERSION);

  std::vector<Impl> v = impls();
  std::printf("implementations under test:");
  for (std::size_t i = 0; i < v.size(); ++i) std::printf(" %s", v[i].name);
  std::printf("\n");

  exhaustive(v);
  four_bytes(v);
  named(v);

  // The three content sets, the same ones the string-path table uses.
  std::vector<std::string> ascii, latin1, wide;
  // The same five string fields the string-path table in `bench.cpp` prices, from one
  // definition in harness.h so the two cannot drift.
  p1_2_strings(ak::values::kAscii, &ascii);
  p1_2_strings(ak::values::kLatin1, &latin1);
  p1_2_strings(ak::values::kWide, &wide);
  real_strings(v, ascii);
  real_strings(v, latin1);
  real_strings(v, wide);

  std::printf("\n%ld checks, %d failures\n", g_checks, g_fail);
  if (g_fail) {
    std::printf("NOT TIMING: an implementation that does not agree with the oracle has no\n"
                "speed worth reporting.\n");
    return 1;
  }

  std::printf("\n-- what each costs, one process, per string and per byte --\n");
  const char *setname[] = {"ascii", "latin1", "wide"};
  const std::vector<std::string> *sets[] = {&ascii, &latin1, &wide};
  std::printf("  %-8s %-8s %10s %10s %10s %10s\n", "set", "impl", "ns/string", "ns/byte",
              "vs scalar", "bytes");
  for (int s = 0; s < 3; ++s) {
    double base = 0;
    for (std::size_t k = 0; k < v.size(); ++k) {
      long bytes = 0;
      double per = ns_per(v[k].fn, *sets[s], 200, &bytes);
      double perbyte = per * (double)sets[s]->size() / (double)bytes;
      if (k == 0) base = per;
      std::printf("  %-8s %-8s %10.2f %10.3f %9.2fx %10ld\n", setname[s], v[k].name, per,
                  perbyte, base / per, bytes);
    }
  }
  return 0;
}
