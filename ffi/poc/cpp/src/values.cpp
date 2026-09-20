#include "ak/values.h"

#include <cstdio>
#include <cstring>

namespace ak {
namespace values {

namespace {

const uint32_t K[64] = {
    0x428a2f98u, 0x71374491u, 0xb5c0fbcfu, 0xe9b5dba5u, 0x3956c25bu, 0x59f111f1u,
    0x923f82a4u, 0xab1c5ed5u, 0xd807aa98u, 0x12835b01u, 0x243185beu, 0x550c7dc3u,
    0x72be5d74u, 0x80deb1feu, 0x9bdc06a7u, 0xc19bf174u, 0xe49b69c1u, 0xefbe4786u,
    0x0fc19dc6u, 0x240ca1ccu, 0x2de92c6fu, 0x4a7484aau, 0x5cb0a9dcu, 0x76f988dau,
    0x983e5152u, 0xa831c66du, 0xb00327c8u, 0xbf597fc7u, 0xc6e00bf3u, 0xd5a79147u,
    0x06ca6351u, 0x14292967u, 0x27b70a85u, 0x2e1b2138u, 0x4d2c6dfcu, 0x53380d13u,
    0x650a7354u, 0x766a0abbu, 0x81c2c92eu, 0x92722c85u, 0xa2bfe8a1u, 0xa81a664bu,
    0xc24b8b70u, 0xc76c51a3u, 0xd192e819u, 0xd6990624u, 0xf40e3585u, 0x106aa070u,
    0x19a4c116u, 0x1e376c08u, 0x2748774cu, 0x34b0bcb5u, 0x391c0cb3u, 0x4ed8aa4au,
    0x5b9cca4fu, 0x682e6ff3u, 0x748f82eeu, 0x78a5636fu, 0x84c87814u, 0x8cc70208u,
    0x90befffau, 0xa4506cebu, 0xbef9a3f7u, 0xc67178f2u};

inline uint32_t rotr(uint32_t x, int n) { return (x >> n) | (x << (32 - n)); }

void block(uint32_t h[8], const uint8_t *p) {
  uint32_t w[64];
  for (int i = 0; i < 16; ++i)
    w[i] = ((uint32_t)p[4 * i] << 24) | ((uint32_t)p[4 * i + 1] << 16) |
           ((uint32_t)p[4 * i + 2] << 8) | (uint32_t)p[4 * i + 3];
  for (int i = 16; i < 64; ++i) {
    uint32_t s0 = rotr(w[i - 15], 7) ^ rotr(w[i - 15], 18) ^ (w[i - 15] >> 3);
    uint32_t s1 = rotr(w[i - 2], 17) ^ rotr(w[i - 2], 19) ^ (w[i - 2] >> 10);
    w[i] = w[i - 16] + s0 + w[i - 7] + s1;
  }
  uint32_t a = h[0], b = h[1], c = h[2], d = h[3], e = h[4], f = h[5], g = h[6], hh = h[7];
  for (int i = 0; i < 64; ++i) {
    uint32_t S1 = rotr(e, 6) ^ rotr(e, 11) ^ rotr(e, 25);
    uint32_t ch = (e & f) ^ (~e & g);
    uint32_t t1 = hh + S1 + ch + K[i] + w[i];
    uint32_t S0 = rotr(a, 2) ^ rotr(a, 13) ^ rotr(a, 22);
    uint32_t mj = (a & b) ^ (a & c) ^ (b & c);
    uint32_t t2 = S0 + mj;
    hh = g; g = f; f = e; e = d + t1; d = c; c = b; b = a; a = t1 + t2;
  }
  h[0] += a; h[1] += b; h[2] += c; h[3] += d;
  h[4] += e; h[5] += f; h[6] += g; h[7] += hh;
}

const char *VOCAB[16] = {"alpha", "bravo", "charlie", "delta", "echo", "foxtrot",
                         "golf", "hotel", "india", "juliet", "kilo", "lima",
                         "mike", "november", "oscar", "papa"};

std::string key_of(const std::string &path, int idx) {
  char b[32];
  std::snprintf(b, sizeof(b), "#%d", idx);
  return path + b;
}

}  // namespace

void sha256(const uint8_t *data, std::size_t n, uint8_t out[32]) {
  uint32_t h[8] = {0x6a09e667u, 0xbb67ae85u, 0x3c6ef372u, 0xa54ff53au,
                   0x510e527fu, 0x9b05688cu, 0x1f83d9abu, 0x5be0cd19u};
  std::size_t i = 0;
  for (; i + 64 <= n; i += 64) block(h, data + i);
  uint8_t tail[128];
  std::size_t rem = n - i;
  std::memcpy(tail, data + i, rem);
  tail[rem] = 0x80;
  std::size_t total = (rem + 1 <= 56) ? 64 : 128;
  std::memset(tail + rem + 1, 0, total - rem - 1);
  uint64_t bits = (uint64_t)n * 8;
  for (int k = 0; k < 8; ++k) tail[total - 1 - k] = (uint8_t)(bits >> (8 * k));
  block(h, tail);
  if (total == 128) block(h, tail + 64);
  for (int k = 0; k < 8; ++k) {
    out[4 * k] = (uint8_t)(h[k] >> 24);
    out[4 * k + 1] = (uint8_t)(h[k] >> 16);
    out[4 * k + 2] = (uint8_t)(h[k] >> 8);
    out[4 * k + 3] = (uint8_t)h[k];
  }
}

std::string sha256_hex(const std::string &s) {
  uint8_t d[32];
  sha256((const uint8_t *)s.data(), s.size(), d);
  static const char *hex = "0123456789abcdef";
  std::string out(64, '0');
  for (int i = 0; i < 32; ++i) {
    out[2 * i] = hex[d[i] >> 4];
    out[2 * i + 1] = hex[d[i] & 15];
  }
  return out;
}

uint64_t h64(const std::string &path, int idx) {
  std::string k = key_of(path, idx);
  uint8_t d[32];
  sha256((const uint8_t *)k.data(), k.size(), d);
  uint64_t v = 0;
  for (int i = 0; i < 8; ++i) v |= (uint64_t)d[i] << (8 * i);  // little endian
  return v;
}

namespace {
ContentSet g_set = kAscii;
}

ContentSet content_set() { return g_set; }
void set_content_set(ContentSet cs) { g_set = cs; }

// The three STRING rules run their result through the current content set. On kAscii
// `recode` returns its input unchanged, so the manifest's bytes are unaffected and every
// gate in this slice sees exactly what it saw before.
std::string guid(const std::string &path, int idx) {
  std::string hx = sha256_hex(key_of(path, idx));
  return recode(hx.substr(0, 8) + "-" + hx.substr(8, 4) + "-" + hx.substr(12, 4) + "-" +
                    hx.substr(16, 4) + "-" + hx.substr(20, 12),
                g_set);
}

std::string word(const std::string &path, int idx) {
  uint64_t h = h64(path, idx);
  char b[16];
  std::snprintf(b, sizeof(b), "%u", (unsigned)(h % 1000));
  return recode(std::string(VOCAB[h % 16]) + b, g_set);
}

std::string sentence(const std::string &path, int idx) {
  uint64_t h = h64(path, idx);
  std::string out;
  for (int i = 0; i < 5; ++i) {
    if (i) out += " ";
    out += VOCAB[(h >> (4 * i)) % 16];
  }
  return recode(out, g_set);
}

std::string blob(const std::string &path, int idx, std::size_t n) {
  std::string out;
  int i = 0;
  char b[32];
  while (out.size() < n) {
    std::snprintf(b, sizeof(b), "#%d#%d", idx, i);
    std::string k = path + b;
    uint8_t d[32];
    sha256((const uint8_t *)k.data(), k.size(), d);
    out.append((const char *)d, 32);
    ++i;
  }
  out.resize(n);
  return out;
}

const std::string &bulk(std::size_t n) {
  static std::string cached;
  static std::size_t cached_n = (std::size_t)-1;
  if (cached_n == n) return cached;
  std::string out;
  int i = 0;
  char b[32];
  while (out.size() < n) {
    std::snprintf(b, sizeof(b), "bulk#%d", i);
    uint8_t d[32];
    sha256((const uint8_t *)b, std::strlen(b), d);
    out.append((const char *)d, 32);
    ++i;
  }
  out.resize(n);
  cached.swap(out);
  cached_n = n;
  return cached;
}

int32_t scalar_i32(const std::string &p, int i) { return (int32_t)(h64(p, i) % 100000); }
int64_t scalar_i64(const std::string &p, int i) {
  return (int64_t)(h64(p, i) % 1000000000000ULL);
}
bool scalar_bool(const std::string &p, int i) { return (h64(p, i) & 1) != 0; }
double scalar_double(const std::string &p, int i) {
  return (double)(h64(p, i) % 1000000) / 1000.0;
}

int32_t enum_value(const std::vector<int32_t> &vals, int idx) {
  return vals[(std::size_t)idx % vals.size()];
}

Stamp timestamp(int idx) {
  Stamp s;
  s.seconds = 1700000000LL + (int64_t)idx * 37;
  s.nanos = (int32_t)(((int64_t)idx * 7919) % 1000000000LL);
  return s;
}

Stamp duration(int idx) {
  Stamp s;
  s.seconds = idx % 3600;
  s.nanos = (int32_t)(((int64_t)idx * 104729) % 1000000000LL);
  return s;
}

std::string recode(const std::string &s, ContentSet cs) {
  if (cs == kAscii) return s;
  std::string out;
  for (std::size_t i = 0; i < s.size(); ++i) {
    unsigned c = (unsigned char)s[i];
    uint32_t cp;
    if (cs == kLatin1) {
      cp = 0xA0 + (c % 0x60);  // U+00A0 .. U+00FF
      out += (char)(0xC0 | (cp >> 6));
      out += (char)(0x80 | (cp & 0x3F));
    } else {
      cp = 0x4E00 + (c % 0x1000);  // well above U+00FF
      out += (char)(0xE0 | (cp >> 12));
      out += (char)(0x80 | ((cp >> 6) & 0x3F));
      out += (char)(0x80 | (cp & 0x3F));
    }
  }
  return out;
}

}  // namespace values
}  // namespace ak
