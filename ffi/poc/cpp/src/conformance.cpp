// R2: correctness before timing, and byte identity across every arm.
//
// The oracle is `ffi/schema/generated/manifest.json`, validated by the rust slice against
// prost 0.14.4 and a second independent encoder. No arm is checked against another arm
// alone. Two INDEPENDENT construction routes reach the same bytes: the facade builder
// (generated over this slice's own types) and the protobuf builder (generated over
// protoc's types, a different API and a different object graph).
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <iostream>
#include <sstream>

#include "generated/ak_layout.h"
#include "harness.h"

static int g_fail = 0;
static int g_checks = 0;
static int g_diverge = 0;

static void check(bool ok, const std::string &what) {
  ++g_checks;
  if (!ok) {
    ++g_fail;
    std::printf("  FAIL  %s\n", what.c_str());
  }
}

static std::string read_file(const std::string &p) {
  std::ifstream f(p.c_str(), std::ios::binary);
  std::ostringstream ss;
  ss << f.rdbuf();
  return ss.str();
}

// ---------------------------------------------------------------- one payload

template <class F, class P>
static void run_case(const char *id, F (*mk)(void), void (*pbmk)(P *),
                     intptr_t (*ffi_enc)(ak_enc_ctx *, const F &, const shapes::ffi::Tcs &),
                     intptr_t (*ffi_enc_z)(ak_enc_ctx *, const F &, const shapes::ffi::Tcs &),
                     intptr_t (*ffi_enc_nb)(ak_enc_ctx *, const F &, const shapes::ffi::Tcs &),
                     int32_t (*ffi_dec)(ak_dec_ctx *, const uint8_t *, size_t, F *),
                     void (*nat_enc)(const F &, ak::Enc *),
                     int32_t (*nat_dec)(const uint8_t *, size_t, F *),
                     const char *want_sha, size_t want_bytes) {
  F facade = mk();
  P pb;
  pbmk(&pb);

  std::string pb_bytes;
  pb_serialize(pb, &pb_bytes, true);

  google::protobuf::Arena arena;
  P *pba = google::protobuf::Arena::CreateMessage<P>(&arena);
  pbmk(pba);
  std::string pba_bytes;
  pb_serialize(*pba, &pba_bytes, true);

  ak::Enc e(shapes::native::kSites);
  nat_enc(facade, &e);
  std::string nat_bytes((const char *)e.data(), e.size());

  ak_enc_ctx *ctx = ak_enc_ctx_new();
  shapes::ffi::Tcs tc = shapes::ffi::tcs_core();
  std::string ffi_bytes, ffiz_bytes, ffinb_bytes, ffih_bytes;
  {
    intptr_t rc = ffi_enc(ctx, facade, tc);
    const uint8_t *p = NULL;
    size_t n = 0;
    ak_enc_take(ctx, &p, &n);
    check(rc >= 0, std::string(id) + " ffi encode rc");
    ffi_bytes.assign((const char *)p, n);
  }
  {
    intptr_t rc = ffi_enc_z(ctx, facade, tc);
    const uint8_t *p = NULL;
    size_t n = 0;
    ak_enc_take(ctx, &p, &n);
    check(rc >= 0, std::string(id) + " ffi-zeroed encode rc");
    ffiz_bytes.assign((const char *)p, n);
  }
  {
    intptr_t rc = ffi_enc_nb(ctx, facade, tc);
    const uint8_t *p = NULL;
    size_t n = 0;
    ak_enc_take(ctx, &p, &n);
    check(rc >= 0, std::string(id) + " ffi-nobatch encode rc");
    ffinb_bytes.assign((const char *)p, n);
  }
  {
    shapes::ffi::Tcs th = shapes::ffi::tcs_host();
    intptr_t rc = ffi_enc(ctx, facade, th);
    const uint8_t *p = NULL;
    size_t n = 0;
    ak_enc_take(ctx, &p, &n);
    check(rc >= 0, std::string(id) + " ffi-hosttc encode rc");
    ffih_bytes.assign((const char *)p, n);
  }
  ak_enc_ctx_free(ctx);

  std::string want(want_sha);
  // The incumbent is not assumed to agree with the canonical form; it is CHECKED, and
  // where it disagrees the disagreement is reported with its size and proved harmless by
  // value identity rather than waved away. protobuf C++ writes a map entry's key and
  // value unconditionally; the canonical form of `manifest.json` omits an
  // implicit-presence leaf holding the proto zero, and prost does too. The two therefore
  // differ by two bytes per EMPTY MAP VALUE, which only P2.5 has.
  bool pb_canonical = (sha_of(pb_bytes) == want);
  if (!pb_canonical) {
    ++g_diverge;
    std::printf("  DIVERGENCE %s: protobuf C++ writes %zu B, the canonical form is %zu B"
                " (delta %+d)\n",
                id, pb_bytes.size(), want_bytes,
                (int)pb_bytes.size() - (int)want_bytes);
    check(sha_of(pb_bytes) == sha_of(pba_bytes),
          std::string(id) + " pb and pb-arena agree with each other");
  } else {
    check(pb_bytes.size() == want_bytes, std::string(id) + " pb size");
    check(sha_of(pb_bytes) == want, std::string(id) + " pb sha");
    check(sha_of(pba_bytes) == want, std::string(id) + " pb-arena sha");
  }
  check(sha_of(nat_bytes) == want, std::string(id) + " native sha");
  check(sha_of(ffi_bytes) == want, std::string(id) + " ffi sha");
  check(sha_of(ffiz_bytes) == want, std::string(id) + " ffi-zeroed sha");
  check(sha_of(ffinb_bytes) == want, std::string(id) + " ffi-nobatch sha");
  check(sha_of(ffih_bytes) == want, std::string(id) + " ffi-hosttc sha");

  // Decode, then value identity between the two facade decoders and a re-encode.
  {
    P back;
    check(back.ParseFromString(pb_bytes), std::string(id) + " pb decode");
    std::string re;
    pb_serialize(back, &re, true);
    check(sha_of(re) == sha_of(pb_bytes), std::string(id) + " pb round trip");
    // And the incumbent reads the canonical bytes to the same VALUE, which is what makes
    // a divergence a wire-form difference rather than a different message.
    P fromCanon;
    ak::Enc ec(shapes::native::kSites);
    nat_enc(facade, &ec);
    std::string canon((const char *)ec.data(), ec.size());
    check(fromCanon.ParseFromString(canon), std::string(id) + " pb reads canonical bytes");
    std::string re2;
    pb_serialize(fromCanon, &re2, true);
    check(sha_of(re2) == sha_of(pb_bytes),
          std::string(id) + " pb(canonical) re-serialises to pb's own form");
  }
  F fnat;
  int32_t rc = nat_dec((const uint8_t *)pb_bytes.data(), pb_bytes.size(), &fnat);
  check(rc == 0, std::string(id) + " native decode rc");
  F fffi;
  ak_dec_ctx *dctx = ak_dec_ctx_new();
  rc = ffi_dec(dctx, (const uint8_t *)pb_bytes.data(), pb_bytes.size(), &fffi);
  check(rc == 0 && ak_dec_err(dctx) == 0, std::string(id) + " ffi decode rc");
  ak_dec_ctx_free(dctx);
  check(fnat == fffi, std::string(id) + " native/ffi decoded VALUES agree");
  check(fnat == facade, std::string(id) + " decoded value == built value");

  // Re-encode what was decoded: the round trip has to reproduce the manifest.
  ak::Enc e2(shapes::native::kSites);
  nat_enc(fnat, &e2);
  check(sha_of(e2) == want, std::string(id) + " native round trip");

  std::printf("  %-5s %8zu B  ok\n", id, pb_bytes.size());
}

// ---------------------------------------------------------------- P7.1

static void run_p71(const std::string &dir) {
  std::string v = read_file(dir + "/" + AK_P71_VECTOR);
  check(!v.empty(), "P7.1 vector present");
  check(sha_of(v) == std::string(AK_P71_SHA), "P7.1 vector sha");

  shapes::DualResponse f;
  int32_t rc = shapes::native::decode_dual_response((const uint8_t *)v.data(), v.size(), &f);
  check(rc == 0, "P7.1 native decode");
  shapes::DualResponse g;
  ak_dec_ctx *dctx = ak_dec_ctx_new();
  rc = shapes::ffi::decode_with_dual_response(dctx, (const uint8_t *)v.data(), v.size(), &g);
  check(rc == 0 && ak_dec_err(dctx) == 0, "P7.1 ffi decode");
  ak_dec_ctx_free(dctx);
  check(f == g, "P7.1 native/ffi decoded values agree");
  check(f.left.size() == 3 && f.right.size() == 3, "P7.1 3+3 elements");

  // Re-encode contiguously: a permutation of the same (tag, wire, body) triples.
  ak::Enc e(shapes::native::kSites);
  shapes::native::encode_into_dual_response(f, &e);
  std::string re((const char *)e.data(), e.size());
  check(re.size() == v.size(), "P7.1 re-encode same length");
  check(re != v, "P7.1 re-encode is NOT the interleaved form (the whole point)");

  // Same multiset of triples.
  struct Tri { uint32_t tag; uint32_t wire; std::string body; };
  std::vector<std::string> a, b;
  for (int pass = 0; pass < 2; ++pass) {
    const std::string &s = pass ? re : v;
    ak::Dec d((const uint8_t *)s.data(), s.size());
    while (!d.at_end()) {
      uint64_t k = d.varint();
      size_t off, n;
      d.len_body(&off, &n);
      char hdr[32];
      std::snprintf(hdr, sizeof(hdr), "%llu:", (unsigned long long)k);
      (pass ? b : a).push_back(std::string(hdr) + std::string((const char *)d.buf + off, n));
    }
  }
  std::sort(a.begin(), a.end());
  std::sort(b.begin(), b.end());
  check(a == b, "P7.1 re-encode is a permutation of the same triples");
  std::printf("  %-5s %8zu B  ok (decode only)\n", "P7.1", v.size());
}

// ---------------------------------------------------------------- the rest

static void run_layout() {
  // ABI v1 section 10 / obligation 12.3. The two sides genuinely restate the layout here,
  // which the rust slice could not exercise.
  std::vector<uint32_t> core(AK_LAYOUT_FACTS, 0xFFFFFFFFu);
  size_t n = ak_layout_facts(&core[0], core.size());
  check(n == (size_t)AK_LAYOUT_FACTS, "layout fact count agrees");
  int bad = 0;
  for (size_t i = 0; i < core.size() && i < n; ++i)
    if (core[i] != AK_LAYOUT_HOST[i]) ++bad;
  check(bad == 0, "every group layout fact agrees between the core and this header");
  std::printf("  %d layout facts, %d disagreements\n", (int)n, bad);
}

static void run_absent_and_unknown() {
  // A field the reader does not know, one of each wire type, after the known fields AND
  // inside a nested message. A corpus generated from the schema that reads it never
  // executes the unknown-field skip, which is the whole of protobuf's forward
  // compatibility (README section 10).
  shapes::ListResultsResponse base = shapes::build::payload_p1_1();
  ak::Enc e(shapes::native::kSites);
  shapes::native::encode_into_list_results_response(base, &e);
  std::string clean((const char *)e.data(), e.size());

  struct V { const char *name; uint32_t tag; uint32_t wire; std::string body; };
  std::vector<V> vs;
  V a = {"varint at root", 900, 0, std::string("\x2a", 1)};
  V b = {"i64 at root", 901, 1, std::string("\x01\x02\x03\x04\x05\x06\x07\x08", 8)};
  V c = {"len at root", 902, 2, std::string("\x03" "abc", 4)};
  V d = {"i32 at root", 903, 5, std::string("\x01\x02\x03\x04", 4)};
  V f = {"unknown oneof-shaped tag", 15, 0, std::string("\x07", 1)};
  vs.push_back(a); vs.push_back(b); vs.push_back(c); vs.push_back(d); vs.push_back(f);

  for (size_t i = 0; i < vs.size(); ++i) {
    std::string s = clean;
    ak::Enc k(1);
    k.varint(((uint64_t)vs[i].tag << 3) | vs[i].wire);
    s.append((const char *)k.data(), k.size());
    s += vs[i].body;

    shapes::ListResultsResponse fn, ff;
    int32_t rc = shapes::native::decode_list_results_response(
        (const uint8_t *)s.data(), s.size(), &fn);
    check(rc == 0, std::string("unknown field skipped, native: ") + vs[i].name);
    ak_dec_ctx *dctx = ak_dec_ctx_new();
    rc = shapes::ffi::decode_with_list_results_response(
        dctx, (const uint8_t *)s.data(), s.size(), &ff);
    check(rc == 0 && ak_dec_err(dctx) == 0,
          std::string("unknown field skipped, ffi: ") + vs[i].name);
    ak_dec_ctx_free(dctx);
    ns::ListResultsResponse pbm;
    check(pbm.ParseFromString(s), std::string("unknown field skipped, pb: ") + vs[i].name);
    check(fn == ff, std::string("unknown field: arms agree on the value: ") + vs[i].name);
    // Nothing here retains the unknown field (ABI v1 open decision 11), so a re-encode
    // drops it. protobuf C++ DOES retain it, which is the behaviour change.
    ak::Enc e2(shapes::native::kSites);
    shapes::native::encode_into_list_results_response(fn, &e2);
    check(sha_of(e2) == sha_of(clean),
          std::string("core drops the unknown field on re-encode: ") + vs[i].name);
    std::string pbre;
    pb_serialize(pbm, &pbre, true);
    check(pbre.size() > clean.size(),
          std::string("protobuf C++ RETAINS it: ") + vs[i].name);
  }

  // An open enum with an unknown VALUE does round-trip, because the field is known.
  {
    shapes::ListResultsResponse one = shapes::build::payload_p1_1();
    one.results[0].status = shapes::ResultStatus(999);
    ak::Enc e3(shapes::native::kSites);
    shapes::native::encode_into_list_results_response(one, &e3);
    std::string s((const char *)e3.data(), e3.size());
    shapes::ListResultsResponse fn;
    check(shapes::native::decode_list_results_response(
              (const uint8_t *)s.data(), s.size(), &fn) == 0, "enum 999 decode");
    check(fn.results[0].status.v == 999, "enum 999 round-trips losslessly");
    ns::ListResultsResponse pbm;
    check(pbm.ParseFromString(s), "enum 999 pb decode");
    check((int)pbm.results(0).status() == 999, "enum 999 round-trips in protobuf C++ too");
  }

  // A malformed UTF-8 string: proto3 requires a parser to reject, and ABI v1 open
  // decision 3 settles that the DECODER carries the whole guarantee.
  {
    shapes::ListResultsResponse one = shapes::build::payload_p1_1();
    one.results[0].name = std::string("ab\xC3\x28" "cd", 6);
    ak::Enc e4(shapes::native::kSites);
    shapes::native::encode_into_list_results_response(one, &e4);
    std::string s((const char *)e4.data(), e4.size());
    shapes::ListResultsResponse fn;
    int32_t rc = shapes::native::decode_list_results_response(
        (const uint8_t *)s.data(), s.size(), &fn);
#ifdef AK_DEC_LOSSY
    check(rc == 0, "malformed UTF-8: the lossy policy accepts it (and that is the problem)");
#else
    check(rc == ak::ERR_TRANSCODE, "malformed UTF-8 rejected by the native decoder");
    ak_dec_ctx *dctx = ak_dec_ctx_new();
    shapes::ListResultsResponse ff;
    shapes::ffi::decode_with_list_results_response(
        dctx, (const uint8_t *)s.data(), s.size(), &ff);
    check(ak_dec_err(dctx) == ak::ERR_TRANSCODE,
          "malformed UTF-8 reported through ak_fail on the DECODE context");
    // The sticky slot must not poison the next decode (the rust slice's D17).
    ak_dec_err_reset(dctx);
    shapes::ListResultsResponse g2;
    std::string good;
    {
      ak::Enc e5(shapes::native::kSites);
      shapes::native::encode_into_list_results_response(shapes::build::payload_p1_1(), &e5);
      good.assign((const char *)e5.data(), e5.size());
    }
    int32_t rc2 = shapes::ffi::decode_with_list_results_response(
        dctx, (const uint8_t *)good.data(), good.size(), &g2);
    check(rc2 == 0 && ak_dec_err(dctx) == 0,
          "a good decode after a rejected one succeeds (D17 regression)");
    ak_dec_ctx_free(dctx);
#endif
    ns::ListResultsResponse pbm;
    bool pbok = pbm.ParseFromString(s);
    std::printf("  protobuf C++ on malformed UTF-8: ParseFromString -> %s\n",
                pbok ? "true (accepts)" : "false (rejects)");
  }
}

int main(int argc, char **argv) {
  const char *dir = argc > 1 ? argv[1] : "../../schema/generated/payloads";
  std::printf("ak_abi_version = %u, AK_ABI_VERSION = %u\n", ak_abi_version(), AK_ABI_VERSION);
  std::printf("-std=%ld  impl=%s  guard=%s  decode-policy=%s  linkage=%s\n",
              (long)__cplusplus,
#ifdef AK_FLOOR_IMPL
              "floor",
#else
              "target",
#endif
#ifdef AK_NO_GUARD
              "off",
#else
              "on",
#endif
#ifdef AK_DEC_LOSSY
              "lossy",
#else
              "reject",
#endif
              AK_LINKAGE);

  std::printf("\n-- group layout (ABI v1 section 10) --\n");
  run_layout();

  std::printf("\n-- byte identity against manifest.json, five arms --\n");
#define X(id, Root, sroot, pfx, sha, nbytes)                                      \
  run_case<shapes::Root, ns::Root>(                                               \
      id, &shapes::build::payload_##pfx, &pbbuild::payload_##pfx,                 \
      &shapes::ffi::encode_into_##sroot, &shapes::ffi::encode_into_##sroot##_zeroed, \
      &shapes::ffi::encode_into_##sroot##_nobatch, &shapes::ffi::decode_with_##sroot, \
      &shapes::native::encode_into_##sroot, &shapes::native::decode_##sroot,      \
      sha, nbytes);
  AK_CASES(X)
#undef X
  run_p71(dir);

  std::printf("\n-- absent, unknown and malformed vectors --\n");
  run_absent_and_unknown();

  std::printf("\n%d checks, %d failures, %d reported wire divergences of the incumbent\n",
              g_checks, g_fail, g_diverge);
  return g_fail ? 1 : 0;
}
