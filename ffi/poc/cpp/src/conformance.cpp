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
#include "generated/ak_layout_names.h"
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
                     intptr_t (*ffi_enc_unk)(ak_enc_ctx *, const F &, const shapes::ffi::Tcs &),
                     int32_t (*ffi_dec)(ak_dec_ctx *, const uint8_t *, size_t, F *),
                     void (*nat_enc)(const F &, ak::Enc *),
                     int32_t (*nat_dec)(const uint8_t *, size_t, F *),
                     const char *want_sha, size_t want_bytes) {
  F facade = mk();
  P pb;
  pbmk(&pb);

  std::string pb_bytes;
  pb_serialize_det(pb, &pb_bytes);
  // The HEADLINE timing arm calls `SerializeToString`, not the deterministic path, so its
  // output is checked too: identical for every message without a map, and a permutation of
  // the map entries otherwise. A timed arm whose bytes nothing checks is not an arm.
  std::string pb_default;
  pb_serialize_default(pb, &pb_default);
  check(pb_default.size() == pb_bytes.size(),
        std::string(id) + " SerializeToString has the same length as the deterministic form");
  {
    P back;
    check(back.ParseFromString(pb_default),
          std::string(id) + " SerializeToString output parses");
    std::string re;
    pb_serialize_det(back, &re);
    check(sha_of(re) == sha_of(pb_bytes),
          std::string(id) + " SerializeToString output is a permutation of the same message");
  }

  google::protobuf::Arena arena;
  P *pba = google::protobuf::Arena::CreateMessage<P>(&arena);
  pbmk(pba);
  std::string pba_bytes;
  pb_serialize(*pba, &pba_bytes, true);

  ak::Enc e(shapes::native::kSites);
  nat_enc(facade, &e);
  std::string nat_bytes((const char *)e.data(), e.size());
  std::string mem_bytes;
  memcpy_floor(nat_bytes, &mem_bytes);
  check(sha_of(mem_bytes) == sha_of(nat_bytes),
        std::string(id) + " the memcpy floor arm copies the right bytes");

  ak_enc_ctx *ctx = ak_enc_ctx_new();
  shapes::ffi::Tcs tc = shapes::ffi::tcs_core();
  std::string ffi_bytes, ffiz_bytes, ffinb_bytes, ffih_bytes, ffiv_bytes, ffiu_bytes;
#ifndef AK_NO_UNKNOWN_FIELDS
  {
    // Requirement 10 / decision 11: the retain encode (u-groups, empty bags on a payload).
    intptr_t rc = ffi_enc_unk(ctx, facade, tc);
    const uint8_t *p = NULL;
    size_t n = 0;
    ak_enc_take(ctx, &p, &n);
    check(rc >= 0, std::string(id) + " ffi-retain encode rc");
    ffiu_bytes.assign((const char *)p, n);
  }
#else
  (void)ffi_enc_unk;
#endif
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
  {
    // R-D5: `ffi-valtc` is a TIMED arm of bench.cpp and contentsets.cpp -- the like-for-like
    // encode row against protobuf, which validates UTF-8 on serialise -- and until this
    // block no gate ran it. The same entry point with the core's VALIDATING transcoder.
    shapes::ffi::Tcs tv = shapes::ffi::tcs_core_validating();
    intptr_t rc = ffi_enc(ctx, facade, tv);
    const uint8_t *p = NULL;
    size_t n = 0;
    int32_t trc = ak_enc_take(ctx, &p, &n);
    check(rc >= 0 && trc == 0 && ak_enc_err(ctx) == 0,
          std::string(id) + " ffi-valtc encode rc");
    ffiv_bytes.assign((const char *)p, n);
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
    // design/SHAPES.md now records that P2.5 has two valid encodings and that a slice
    // matches either and SAYS WHICH. 19,712 B and 19,632 B are not the same work, so the
    // form the incumbent wrote is named here and the timing rows are taken over it.
    std::printf("  TWO VALID FORMS %s: this incumbent writes %zu B, the manifest records"
                " %zu B (delta %+d). protobuf C++ writes a map entry's value"
                " unconditionally; the canonical form omits an implicit-presence leaf"
                " holding the proto zero.\n",
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
  check(sha_of(ffiv_bytes) == want, std::string(id) + " ffi-valtc sha");
#ifndef AK_NO_UNKNOWN_FIELDS
  check(sha_of(ffiu_bytes) == want, std::string(id) + " ffi-retain sha");
#endif

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
  ak_dec_ctx *dctx = shapes::ffi::dec_ctx_new_for<F>();
  rc = ffi_dec(dctx, (const uint8_t *)pb_bytes.data(), pb_bytes.size(), &fffi);
  check(rc == 0 && ak_dec_err(dctx) == 0, std::string(id) + " ffi decode rc");
#ifndef AK_NO_UNKNOWN_FIELDS
  {
    // Decision 11 on the same bytes and the same bound context: retain everywhere, and the
    // pre-allocated pools refilled in place. A payload has no unknown field, so both must
    // give the built value exactly, and leave nothing allocated behind.
    F fu, fp;
    uint64_t refills = 0;
    int32_t urc = shapes::ffi::DecRoot<F>::decode_unk(dctx, (const uint8_t *)pb_bytes.data(),
                                                      pb_bytes.size(), &fu);
    check(urc == 0 && ak_dec_err(dctx) == 0 && fu == facade,
          std::string(id) + " ffi-retain decode == built value");
    int32_t prc = shapes::ffi::DecRoot<F>::decode_pool(dctx, (const uint8_t *)pb_bytes.data(),
                                                       pb_bytes.size(), &fp, 2, 16, &refills);
    check(prc == 0 && ak_dec_err(dctx) == 0 && fp == facade,
          std::string(id) + " ffi-pool decode == built value");
    check(shapes::ffi::unk_reclaim() == 0, std::string(id) + " no unknown-field buffer left live");
  }
#endif
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
  ak_dec_ctx *dctx = shapes::ffi::dec_ctx_new_for<shapes::DualResponse>();
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
  for (size_t i = 0; i < core.size() && i < n; ++i) {
    if (core[i] != AK_LAYOUT_HOST[i]) {
      ++bad;
      // NAMED, not merely counted: a count says the two sides disagree and a name says
      // about what, which is the difference between a diagnosable bug and a mystery.
      std::printf("  MISMATCH %-46s core=%u host=%u\n", AK_LAYOUT_NAMES[i], core[i],
                  AK_LAYOUT_HOST[i]);
    }
  }
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
    ak_dec_ctx *dctx = shapes::ffi::dec_ctx_new_for<shapes::ListResultsResponse>();
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

  // ---- inside a NESTED message, which README section 10 item 1 names explicitly ----
  //
  // The five vectors above all land at the ROOT, and a decoder's root loop and its nested
  // loop are different code. This splices an unknown field into the body of the FIRST
  // repeated element and rewrites the element's length prefix.
  {
    struct V { const char *name; uint32_t tag; uint32_t wire; std::string body; };
    V vs2[3];
    vs2[0].name = "varint inside ResultRaw"; vs2[0].tag = 800; vs2[0].wire = 0;
    vs2[0].body = std::string("\x2a", 1);
    vs2[1].name = "len inside ResultRaw"; vs2[1].tag = 801; vs2[1].wire = 2;
    vs2[1].body = std::string("\x03" "xyz", 4);
    vs2[2].name = "i32 inside ResultRaw"; vs2[2].tag = 802; vs2[2].wire = 5;
    vs2[2].body = std::string("\x01\x02\x03\x04", 4);

    for (int i = 0; i < 3; ++i) {
      // Rebuild: outer tag 1 (results), its body plus the unknown run, then the rest.
      ak::Dec d((const uint8_t *)clean.data(), clean.size());
      std::string out;
      bool done = false;
      while (!d.at_end()) {
        size_t keypos = d.pos;
        uint64_t k = d.varint();
        uint32_t tag = (uint32_t)(k >> 3), wire = (uint32_t)(k & 7);
        if (tag == 1 && wire == 2 && !done) {
          size_t off, n;
          d.len_body(&off, &n);
          ak::Enc extra(1);
          extra.varint(((uint64_t)vs2[i].tag << 3) | vs2[i].wire);
          std::string body((const char *)d.buf + off, n);
          body.append((const char *)extra.data(), extra.size());
          body += vs2[i].body;
          ak::Enc hdr(1);
          hdr.varint(((uint64_t)1 << 3) | 2);
          hdr.varint(body.size());
          out.append((const char *)hdr.data(), hdr.size());
          out += body;
          done = true;
        } else {
          size_t before = d.pos;
          d.skip(tag, wire);
          out.append(clean, keypos, d.pos - keypos);
          (void)before;
        }
      }
      check(done, std::string("nested vector spliced: ") + vs2[i].name);

      shapes::ListResultsResponse fn, ff;
      int32_t rc = shapes::native::decode_list_results_response(
          (const uint8_t *)out.data(), out.size(), &fn);
      check(rc == 0, std::string("unknown NESTED field skipped, native: ") + vs2[i].name);
      ak_dec_ctx *dctx = shapes::ffi::dec_ctx_new_for<shapes::ListResultsResponse>();
      rc = shapes::ffi::decode_with_list_results_response(
          dctx, (const uint8_t *)out.data(), out.size(), &ff);
      check(rc == 0 && ak_dec_err(dctx) == 0,
            std::string("unknown NESTED field skipped, ffi: ") + vs2[i].name);
      ak_dec_ctx_free(dctx);
      ns::ListResultsResponse pbm;
      check(pbm.ParseFromString(out),
            std::string("unknown NESTED field skipped, pb: ") + vs2[i].name);
      check(fn == ff, std::string("unknown NESTED field: arms agree: ") + vs2[i].name);
    }
  }

  // ---- an unrecognised tag on the message that HAS the oneof ----------------------
  //
  // "A parser cannot tell an unrecognised oneof tag from any other unknown field, since
  // the grouping lives only in the descriptor: the case stays at the last known member and
  // the payload is dropped." The five root vectors were all on `ListResultsResponse`,
  // which has no oneof at all, so the shape was never reached.
  {
    shapes::ListProbeResponse p = shapes::build::payload_p3_1();
    ak::Enc pe(shapes::native::kSites);
    shapes::native::encode_into_list_probe_response(p, &pe);
    std::string base((const char *)pe.data(), pe.size());

    // tag 20 sits past `body`'s members (10..14) and is what a newer schema adding a
    // sixth oneof member would put on the wire.
    ak::Dec d((const uint8_t *)base.data(), base.size());
    std::string out;
    bool done = false;
    while (!d.at_end()) {
      size_t keypos = d.pos;
      uint64_t k = d.varint();
      uint32_t tag = (uint32_t)(k >> 3), wire = (uint32_t)(k & 7);
      if (tag == 1 && wire == 2 && !done) {
        size_t off, n;
        d.len_body(&off, &n);
        std::string body((const char *)d.buf + off, n);
        ak::Enc extra(1);
        extra.varint(((uint64_t)20 << 3) | 2);
        extra.varint(3);
        body.append((const char *)extra.data(), extra.size());
        body += "new";
        ak::Enc hdr(1);
        hdr.varint(((uint64_t)1 << 3) | 2);
        hdr.varint(body.size());
        out.append((const char *)hdr.data(), hdr.size());
        out += body;
        done = true;
      } else {
        d.skip(tag, wire);
        out.append(base, keypos, d.pos - keypos);
      }
    }
    check(done, "unknown oneof-member tag spliced into a Probe");

    shapes::ListProbeResponse fn, ff;
    check(shapes::native::decode_list_probe_response(
              (const uint8_t *)out.data(), out.size(), &fn) == 0,
          "unknown oneof-member tag skipped, native");
    ak_dec_ctx *dctx = shapes::ffi::dec_ctx_new_for<shapes::ListProbeResponse>();
    check(shapes::ffi::decode_with_list_probe_response(
              dctx, (const uint8_t *)out.data(), out.size(), &ff) == 0 &&
              ak_dec_err(dctx) == 0,
          "unknown oneof-member tag skipped, ffi");
    ak_dec_ctx_free(dctx);
    check(fn == ff, "unknown oneof-member tag: arms agree on the value");
    check(fn.probes[0].body.which() == p.probes[0].body.which(),
          "the case STAYS at the last known member and the payload is dropped");
    ns::ListProbeResponse pbm;
    check(pbm.ParseFromString(out), "unknown oneof-member tag skipped, pb");
    std::string pbre;
    pb_serialize_det(pbm, &pbre);
    check(pbre.size() > base.size(), "protobuf C++ RETAINS the unknown oneof-member tag");
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
    ak_dec_ctx *dctx = shapes::ffi::dec_ctx_new_for<shapes::ListResultsResponse>();
    shapes::ListResultsResponse ff;
    int32_t frc = shapes::ffi::decode_with_list_results_response(
        dctx, (const uint8_t *)s.data(), s.size(), &ff);
    // Since the plan layer (poc/codec/gen/plan.py, R-E7, 77f91ee) the CORE validates a
    // `string` on decode under utf8="reject" and refuses with AK_ERR_TRANSCODE before
    // delivering anything; the binding's own check (`s_of` -> ak_fail) is no longer the
    // one that fires. Either path must report -6: the return code, or the sticky slot.
    check(frc == ak::ERR_TRANSCODE || ak_dec_err(dctx) == ak::ERR_TRANSCODE,
          "malformed UTF-8 refused on the ffi decode path (core rc or the sticky slot)");
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
    // The ENCODE side of the same string, which is what makes `ffi-valtc` a different arm
    // from `ffi` rather than the same arm under another name (R-D5). The spec transcoder
    // does not check and must accept; the validating one must refuse with
    // AK_ERR_TRANSCODE. A validating arm that accepted this would be timing a check it
    // does not run. Independent of the decode policy, so outside the #ifdef above.
    {
      ak_enc_ctx *ectx = ak_enc_ctx_new();
      const uint8_t *p = NULL;
      size_t n = 0;
      intptr_t rs = shapes::ffi::encode_into_list_results_response(
          ectx, one, shapes::ffi::tcs_core());
      int32_t ts = ak_enc_take(ectx, &p, &n);
      check(rs >= 0 && ts == 0 && std::string((const char *)p, n) == s,
            "malformed UTF-8 on ENCODE: ffi (spec transcoder, no check) accepts it and"
            " writes the native bytes");
      ak_enc_reset(ectx);
      intptr_t rv = shapes::ffi::encode_into_list_results_response(
          ectx, one, shapes::ffi::tcs_core_validating());
      int32_t tv = ak_enc_take(ectx, &p, &n);
      int32_t ev = ak_enc_err(ectx);
      std::printf("  ffi-valtc on malformed UTF-8: encode rc %ld, take %d, err %d\n",
                  (long)rv, tv, ev);
      check((rv < 0 || tv != 0) && ev == ak::ERR_TRANSCODE,
            "malformed UTF-8 on ENCODE: ffi-valtc refuses it with AK_ERR_TRANSCODE");
      // And the refusal does not survive a reset into the next encode.
      ak_enc_reset(ectx);
      intptr_t rg = shapes::ffi::encode_into_list_results_response(
          ectx, shapes::build::payload_p1_1(), shapes::ffi::tcs_core_validating());
      int32_t tg = ak_enc_take(ectx, &p, &n);
      check(rg >= 0 && tg == 0 && ak_enc_err(ectx) == 0 &&
                sha_of(std::string((const char *)p, n)) ==
                    std::string("5df5eb5f3f51ae920f64ed07108d7158e0f559cda2be692f550767a9f39d3206"),
            "ffi-valtc: a good encode after a refused one on the same context succeeds");
      ak_enc_ctx_free(ectx);
    }
    ns::ListResultsResponse pbm;
    bool pbok = pbm.ParseFromString(s);
    std::printf("  protobuf C++ on malformed UTF-8: ParseFromString -> %s\n",
                pbok ? "true (accepts)" : "false (rejects)");
  }
}

// ---------------------------------------------------------------- decision 11
#ifndef AK_NO_UNKNOWN_FIELDS
//
// ABI v1 decision 11 (implementation rules confirmed 2026-09-25) through the generated
// binding and, where the binding cannot reach a case, the raw C ABI: pools taken in order
// and cleared in the host's struct, grow as the fallback, AK_ERR_CAPACITY without one, the
// host refilling a pool in place between deliveries, one buffer per oneof, discard per
// position, grow's failures, and root-bound contexts.

static void d11_ld(uint32_t tag, const std::string &body, std::string *out) {
  uint64_t k = ((uint64_t)tag << 3) | 2u;
  while (k >= 0x80) { out->push_back((char)(uint8_t)(k | 0x80)); k >>= 7; }
  out->push_back((char)(uint8_t)k);
  uint64_t n = body.size();
  while (n >= 0x80) { out->push_back((char)(uint8_t)(n | 0x80)); n >>= 7; }
  out->push_back((char)(uint8_t)n);
  out->append(body);
}

// One unknown run: field 100, varint `x` (key 800 = a0 06).
static std::string d11_run(uint8_t x) {
  std::string r("\xa0\x06", 2);
  r.push_back((char)x);
  return r;
}

// The host side, instrumented: counts grow calls (and FRESH calls, dst == NULL) through the
// one host pointer, can fail or under-deliver on request, else does what unk_grow does.
struct D11Grow {
  uint32_t calls;
  uint32_t fresh;
  int32_t fail_rc;
  int32_t short_by;
};

static int32_t d11_grow(void *host, int32_t want, uint8_t **dst, int32_t *cap) {
  D11Grow *h = (D11Grow *)host;
  ++h->calls;
  if (*dst == NULL) ++h->fresh;
  if (h->fail_rc != 0) return h->fail_rc;
  int32_t rc = shapes::ffi::unk_grow(host, want, dst, cap);
  if (rc == AK_OK && h->short_by > 0) *cap = want - h->short_by;
  return rc;
}

static struct ak_unk_buf d11_buf(uint32_t cap) {
  struct ak_unk_buf b;
  b.data = std::malloc(cap);
  b.len = 0;
  b.cap = cap;
  // Registered with the binding, so a buffer the decode never delivers is reclaimed by it.
  shapes::ffi::unk_track(b.data);
  return b;
}

static const struct ak_unk_pool kD11NoPool = {NULL, 0, NULL};

// Three ResultRaw elements, each carrying one unknown run.
static std::string d11_results(std::vector<std::string> *runs) {
  std::string b;
  const char *ids[3] = {"aa", "bb", "cc"};
  for (int i = 0; i < 3; ++i) {
    std::string e;
    d11_ld(1, ids[i], &e);
    std::string r = d11_run((uint8_t)(7 + i));
    e += r;
    runs->push_back(r);
    d11_ld(1, e, &b);
  }
  return b;
}

// POOL (rule 1): a batched repeated position with n = 2 pre-allocated buffers: no grow
// for the first two elements that carry unknowns, then one fresh grow; without a grow,
// AK_ERR_CAPACITY. Taken buffers are cleared IN the host's struct.
static void d11_pool() {
  std::vector<std::string> runs;
  std::string b = d11_results(&runs);
  for (int grow = 1; grow >= 0; --grow) {
    struct ak_unk_buf arr[2] = {d11_buf(64), d11_buf(64)};
    D11Grow gc = {0, 0, 0, 0};
    struct ak_dec_ListResultsResponse_opts o;
    std::memset(&o, 0, sizeof(o));
    o.host = &gc;
    o.results.bufs = arr;
    o.results.n = 2;
    o.results.grow = grow ? d11_grow : NULL;
    o.results_created_at = kD11NoPool;
    o.results_completed_at = kD11NoPool;
    ak_dec_ctx *c = ak_dec_ctx_new_ListResultsResponse(NULL);
    shapes::ListResultsResponse v;
    int32_t rc = shapes::ffi::decode_with_list_results_response_opts(
        c, (const uint8_t *)b.data(), b.size(), &v, &o);
    bool cleared = arr[0].data == NULL && arr[1].data == NULL;
    if (grow) {
      bool bags = rc == 0 && v.results.size() == 3;
      for (size_t i = 0; bags && i < 3; ++i) bags = v.results[i].unknown_fields == runs[i];
      std::printf("  pool n=2, grow: rc %d, grow calls %u (fresh %u), entries cleared %d, bags exact %d\n",
                  rc, gc.calls, gc.fresh, (int)cleared, (int)bags);
      check(bags && gc.calls == 1 && gc.fresh == 1 && cleared,
            "d11 pool: two elements from the pool, the third from one fresh grow, entries cleared");
    } else {
      std::printf("  pool n=2, no grow: rc %d (AK_ERR_CAPACITY %d), entries cleared %d, grow calls %u\n",
                  rc, AK_ERR_CAPACITY, (int)cleared, gc.calls);
      check(rc == AK_ERR_CAPACITY && cleared && gc.calls == 0,
            "d11 pool: exhausted with no grow is AK_ERR_CAPACITY, never a partial copy");
    }
    check(shapes::ffi::unk_reclaim() == 0, "d11 pool: nothing left live after the decode");
    ak_dec_ctx_free(c);
  }
}

// IN-PLACE REFILL (rule 1): ListTasksDetailedResponse.tasks is not batched (its element has
// runs of its own), so the host refills the pool in `new_tasks` by writing its own struct;
// the element decoded next must use the refilled buffer. The raw C ABI, because the check
// is the pointer the core placed. Without the refill: AK_ERR_CAPACITY at the second element.
struct D11Refill {
  struct ak_unk_buf *arr;
  bool refill;
  std::vector<void *> placed;
  std::vector<std::pair<void *, std::string> > got;
  int64_t n;
};

static int64_t d11_new(ak_dec_ctx *, void *obj) {
  D11Refill *h = (D11Refill *)obj;
  if (h->refill && h->arr[0].data == NULL) {
    void *p = std::malloc(64);
    h->placed.push_back(p);
    h->arr[0].data = p;
    h->arr[0].len = 0;
    h->arr[0].cap = 64;
  }
  return h->n++;
}

static void d11_apply(ak_dec_ctx *, void *obj, int64_t, const struct ak_dfix_TaskDetailed *f) {
  D11Refill *h = (D11Refill *)obj;
  std::string bytes;
  if (f->unknown.data != NULL) bytes.assign((const char *)f->unknown.data, f->unknown.len);
  h->got.push_back(std::make_pair(f->unknown.data, bytes));
}

static void d11_refill() {
  shapes::ListTasksDetailedResponse v;
  std::vector<std::string> runs;
  for (int i = 0; i < 3; ++i) {
    shapes::TaskDetailed t;
    t.id = std::string("t") + (char)('0' + i);
    t.unknown_fields = d11_run((uint8_t)(20 + i));
    runs.push_back(t.unknown_fields);
    v.tasks.push_back(t);
  }
  // The bytes from the binding's retain encode (the u-groups carry each element's bag).
  std::string b;
  {
    ak_enc_ctx *ec = ak_enc_ctx_new();
    intptr_t erc = shapes::ffi::encode_into_list_tasks_detailed_response_unk(ec, v, shapes::ffi::tcs_core());
    const uint8_t *q = NULL;
    size_t qn = 0;
    ak_enc_take(ec, &q, &qn);
    check(erc >= 0, "d11 refill: retain encode of the input");
    b.assign((const char *)q, qn);
    ak_enc_ctx_free(ec);
  }
  for (int refill = 1; refill >= 0; --refill) {
    struct ak_unk_buf arr[1];
    arr[0].data = std::malloc(64);
    arr[0].len = 0;
    arr[0].cap = 64;
    struct ak_dec_ListTasksDetailedResponse_opts o;
    std::memset(&o, 0, sizeof(o));
    o.tasks.bufs = arr;
    o.tasks.n = 1;
    D11Refill h;
    h.arr = arr;
    h.refill = refill != 0;
    h.placed.push_back(arr[0].data);
    h.n = 0;
    ak_dec_ctx *c = ak_dec_ctx_new_ListTasksDetailedResponse(&o);
    struct ak_dvt_ListTasksDetailedResponse vt;
    std::memset(&vt, 0, sizeof(vt));
    vt.new_tasks = d11_new;
    vt.apply_tasks = d11_apply;
    int32_t rc = ak_decode_ListTasksDetailedResponse(c, &h, (const uint8_t *)b.data(), b.size(), &vt);
    ak_dec_ctx_free(c);
    if (refill) {
      bool ok = rc == 0 && h.got.size() == 3 && h.placed.size() == 3;
      for (size_t i = 0; ok && i < 3; ++i)
        ok = h.got[i].first == h.placed[i] && h.got[i].second == runs[i];
      std::printf("  in-place refill: rc %d, %zu elements, refills %zu, each in the buffer written"
                  " in new_tasks, bags exact: %d\n", rc, h.got.size(), h.placed.size() - 1, (int)ok);
      check(ok, "d11 refill: each element placed in the buffer the host wrote into its struct");
    } else {
      std::printf("  no refill (control): rc %d (AK_ERR_CAPACITY %d)\n", rc, AK_ERR_CAPACITY);
      check(rc == AK_ERR_CAPACITY, "d11 refill control: without the refill, AK_ERR_CAPACITY");
    }
    // Rule 3: the core delivered to this raw host (or failed); every buffer is the host's.
    for (size_t i = 0; i < h.placed.size(); ++i) std::free(h.placed[i]);
  }
}

// ONEOF (rule 4): one position, one buffer. A body that switches message members keeps ONE
// buffer (one fresh grow), emptied on each switch, so the final member's bag is its own run
// only. After a switch to a scalar member the buffer stays in the inactive member's slot
// and the binding frees it; the value has no bag.
static void d11_oneof() {
  struct Seq { const char *name; uint32_t tags[3]; int n; uint8_t last; uint32_t fresh; };
  const Seq seqs[3] = {
      {"stamp -> nothing", {13, 14, 0}, 2, 2, 1},
      {"stamp -> nothing -> stamp", {13, 14, 13}, 3, 3, 1},
      {"stamp -> as_int (scalar)", {13, 10, 0}, 2, 0, 1},
  };
  for (int s = 0; s < 3; ++s) {
    std::string p;
    d11_ld(1, "p", &p);
    for (int i = 0; i < seqs[s].n; ++i) {
      if (seqs[s].tags[i] == 10) {
        p.push_back((char)(10 << 3));  // as_int, varint 5
        p.push_back(5);
      } else {
        d11_ld(seqs[s].tags[i], d11_run((uint8_t)(i + 1)), &p);
      }
    }
    std::string b;
    d11_ld(1, p, &b);
    D11Grow gc = {0, 0, 0, 0};
    struct ak_dec_ListProbeResponse_opts o;
    shapes::ffi::unk_opts_list_probe_response(&o, -1);
    o.host = &gc;
    o.probes_body.grow = d11_grow;
    ak_dec_ctx *c = ak_dec_ctx_new_ListProbeResponse(NULL);
    shapes::ListProbeResponse v;
    int32_t rc = shapes::ffi::decode_with_list_probe_response_opts(
        c, (const uint8_t *)b.data(), b.size(), &v, &o);
    ak_dec_ctx_free(c);
    bool ok = rc == 0 && v.probes.size() == 1 && gc.fresh == seqs[s].fresh;
    std::string bag = "(none)";
    if (ok) {
      shapes::ProbeBody &body = v.probes[0].body;
      if (body.which() == shapes::ProbeBody::kAsStamp) bag = body.mutable_as_stamp().unknown_fields;
      else if (body.which() == shapes::ProbeBody::kAsNothing) bag = body.mutable_as_nothing().unknown_fields;
      else if (body.which() == shapes::ProbeBody::kAsInt) bag = "";
      ok = seqs[s].last ? bag == d11_run(seqs[s].last) : (body.which() == shapes::ProbeBody::kAsInt && bag.empty());
    }
    std::printf("  oneof %-26s rc %d, fresh buffers %u, final bag %s\n", seqs[s].name, rc, gc.fresh,
                ok ? "exact" : "WRONG");
    check(ok, std::string("d11 oneof: ") + seqs[s].name);
  }
}

// DISCARD and grow's failures (rules 2 and 5), on the root position of ListResultsResponse
// with one unknown run at the root.
static void d11_errors() {
  std::string b;
  d11_ld(1, std::string("\x0a\x02zz", 4), &b);   // one element
  std::string root_run = d11_run(9);
  b += root_run;
  struct Case { const char *name; int zero; int32_t fail_rc; int32_t short_by; uint32_t prealloc; bool grow; int32_t want; };
  const Case cs[5] = {
      {"root entry all zero (discard)", 0, 0, 0, 0, false, 0},
      {"root buffer too small, no grow", -1, 0, 0, 2, false, AK_ERR_CAPACITY},
      {"grow returns AK_ERR_HOST", -1, AK_ERR_HOST, 0, 0, true, AK_ERR_HOST},
      {"grow under-delivers", -1, 0, 1, 0, true, AK_ERR_CAPACITY},
      {"root buffer fits, no grow", -1, 0, 0, 16, false, 0},
  };
  for (int i = 0; i < 5; ++i) {
    D11Grow gc = {0, 0, cs[i].fail_rc, cs[i].short_by};
    struct ak_dec_ListResultsResponse_opts o;
    shapes::ffi::unk_opts_list_results_response(&o, cs[i].zero);
    o.host = &gc;
    o.self.grow = cs[i].grow ? d11_grow : NULL;
    if (cs[i].prealloc) o.self.buf = d11_buf(cs[i].prealloc);
    ak_dec_ctx *c = ak_dec_ctx_new_ListResultsResponse(NULL);
    shapes::ListResultsResponse v;
    int32_t rc = shapes::ffi::decode_with_list_results_response_opts(
        c, (const uint8_t *)b.data(), b.size(), &v, &o);
    ak_dec_ctx_free(c);
    bool ok = rc == cs[i].want;
    if (ok && rc == 0) ok = v.unknown_fields == (cs[i].zero == 0 ? std::string() : root_run);
    std::printf("  %-32s rc %d (want %d)%s\n", cs[i].name, rc, cs[i].want,
                rc == 0 ? (ok ? ", bag as expected" : ", bag WRONG") : "");
    check(ok, std::string("d11 errors: ") + cs[i].name);
    check(shapes::ffi::unk_reclaim() == 0, std::string("d11 errors: nothing left live: ") + cs[i].name);
  }
}

// ROOT-BOUND CONTEXTS (rule 6): a context bound to ListResultsResponse refuses another
// root's decode, parse and reset, and the binding's armed decode of another root reports
// the refused reset; its own root still decodes after.
static void d11_wrong_root() {
  ak_dec_ctx *c = ak_dec_ctx_new_ListResultsResponse(NULL);
  const uint8_t *e = (const uint8_t *)"";
  struct ak_dvt_ListTasksDetailedResponse vt;
  std::memset(&vt, 0, sizeof(vt));
  int32_t d = ak_decode_ListTasksDetailedResponse(c, NULL, e, 0, &vt);
  int32_t p = ak_parse_ListTasksDetailedResponse(c, e, 0);
  int32_t r = ak_dec_reset_ListTasksDetailedResponse(c, NULL);
  shapes::ListTasksDetailedResponse t1, t2;
  int32_t bd = shapes::ffi::decode_with_list_tasks_detailed_response(c, e, 0, &t1);
  int32_t bu = shapes::ffi::decode_with_list_tasks_detailed_response_unk(c, e, 0, &t2);
  shapes::ListResultsResponse own;
  int32_t ok_own = shapes::ffi::decode_with_list_results_response_unk(c, e, 0, &own);
  std::printf("  wrong root: decode %d, parse %d, reset %d, binding decode %d, binding armed decode %d"
              " (AK_ERR_INVALID_STATE %d); own root %d\n", d, p, r, bd, bu, AK_ERR_INVALID_STATE, ok_own);
  check(d == AK_ERR_INVALID_STATE && p == AK_ERR_INVALID_STATE && r == AK_ERR_INVALID_STATE &&
            bd == AK_ERR_INVALID_STATE && bu == AK_ERR_INVALID_STATE && ok_own == 0,
        "d11 wrong root: refused with AK_ERR_INVALID_STATE, own root still decodes");
  ak_dec_ctx_free(c);
}

// Retained and re-encoded: the root's and every element's runs come back in place.
static void d11_roundtrip() {
  std::vector<std::string> runs;
  std::string b = d11_results(&runs);
  std::string tail = d11_run(42);
  b += tail;
  ak_dec_ctx *c = ak_dec_ctx_new_ListResultsResponse(NULL);
  shapes::ListResultsResponse v, dv;
  int32_t rc = shapes::ffi::decode_with_list_results_response_unk(c, (const uint8_t *)b.data(), b.size(), &v);
  int32_t drc = shapes::ffi::decode_with_list_results_response(c, (const uint8_t *)b.data(), b.size(), &dv);
  ak_dec_ctx_free(c);
  ak_enc_ctx *ec = ak_enc_ctx_new();
  intptr_t erc = shapes::ffi::encode_into_list_results_response_unk(ec, v, shapes::ffi::tcs_core());
  const uint8_t *p = NULL;
  size_t n = 0;
  ak_enc_take(ec, &p, &n);
  std::string re((const char *)p, n);
  ak_enc_ctx_free(ec);
  bool dropped = drc == 0 && dv.unknown_fields.empty() && dv.results.size() == 3 &&
                 dv.results[0].unknown_fields.empty();
  std::printf("  retain round trip: decode %d, encode %ld, bytes identical %d; drop decode keeps none %d\n",
              rc, (long)erc, (int)(re == b), (int)dropped);
  check(rc == 0 && erc >= 0 && re == b, "d11 retain: decode then retain encode reproduces the input");
  check(dropped, "d11 drop: the same context in drop mode keeps no bag");
}

static void run_decision11() {
  d11_pool();
  d11_refill();
  d11_oneof();
  d11_errors();
  d11_wrong_root();
  d11_roundtrip();
}
#else
// WP5 step 10, the NO-UNKNOWN build: no options, no retain. What remains of decision 11 is
// rule 6 (root-bound contexts, `ak_dec_ctx_new_<Root>(void)`) and the drop of an unknown
// field at the root and inside an element; the layout table above is the variant's (240
// facts) and was compared with this core's export.
static void run_decision11() {
  ak_dec_ctx *c = ak_dec_ctx_new_ListResultsResponse();
  const uint8_t *e = (const uint8_t *)"";
  struct ak_dvt_ListTasksDetailedResponse vt;
  std::memset(&vt, 0, sizeof(vt));
  int32_t d = ak_decode_ListTasksDetailedResponse(c, NULL, e, 0, &vt);
  int32_t p = ak_parse_ListTasksDetailedResponse(c, e, 0);
  shapes::ListTasksDetailedResponse t1;
  int32_t bd = shapes::ffi::decode_with_list_tasks_detailed_response(c, e, 0, &t1);
  // One element with an unknown run, and one at the root: decoded, both dropped.
  std::string b("\x0a\x07\x0a\x02zz\xa0\x06\x07", 9);
  b += std::string("\xa0\x06\x09", 3);
  shapes::ListResultsResponse v;
  int32_t own = shapes::ffi::decode_with_list_results_response(c, (const uint8_t *)b.data(), b.size(), &v);
  bool dropped = own == 0 && v.unknown_fields.empty() && v.results.size() == 1 &&
                 v.results[0].unknown_fields.empty();
  std::printf("  no-unknown build: wrong root decode %d, parse %d, binding decode %d"
              " (AK_ERR_INVALID_STATE %d); own root %d, unknowns dropped %d\n",
              d, p, bd, AK_ERR_INVALID_STATE, own, (int)dropped);
  check(d == AK_ERR_INVALID_STATE && p == AK_ERR_INVALID_STATE && bd == AK_ERR_INVALID_STATE,
        "nounk wrong root: refused with AK_ERR_INVALID_STATE");
  check(dropped, "nounk: unknown fields at the root and in an element are dropped");
  ak_dec_ctx_free(c);
}
#endif

#ifdef AK_NO_UNKNOWN_FIELDS
#define AK_CONF_ENC_UNK(s) NULL
#else
#define AK_CONF_ENC_UNK(s) &shapes::ffi::encode_into_##s##_unk
#endif

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

  std::printf("\n-- byte identity against manifest.json, six arms (native, ffi, ffi-zeroed, ffi-nobatch,"
              " ffi-hosttc, ffi-valtc) --\n");
#define X(id, Root, sroot, pfx, sha, nbytes)                                      \
  run_case<shapes::Root, ns::Root>(                                               \
      id, &shapes::build::payload_##pfx, &pbbuild::payload_##pfx,                 \
      &shapes::ffi::encode_into_##sroot, &shapes::ffi::encode_into_##sroot##_zeroed, \
      &shapes::ffi::encode_into_##sroot##_nobatch, AK_CONF_ENC_UNK(sroot),         \
      &shapes::ffi::decode_with_##sroot,                                          \
      &shapes::native::encode_into_##sroot, &shapes::native::decode_##sroot,      \
      sha, nbytes);
  AK_CASES(X)
#undef X
  run_p71(dir);

  std::printf("\n-- absent, unknown and malformed vectors --\n");
  run_absent_and_unknown();

  std::printf("\n-- ABI v1 decision 11: unknown-field options, pools, oneof, root-bound contexts --\n");
  run_decision11();

  std::printf("\n%d checks, %d failures, %d payloads with two valid encodings\n",
              g_checks, g_fail, g_diverge);
  return g_fail ? 1 : 0;
}
