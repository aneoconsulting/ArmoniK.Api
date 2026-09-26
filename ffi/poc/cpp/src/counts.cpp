// R5: count the crossings, do not infer them, and prove the boundary exists.
//
// The counters live in the CORE and in the contexts, so a call the optimiser removed is
// not counted. That is necessary and NOT sufficient: with a statically linked core and
// -flto, an entry point can be inlined into the host WITH ITS COUNTING CODE, and the
// counts keep reporting the right number while the arm has quietly become the no-boundary
// control. `gen/boundary.sh` answers that from the built artifact; this binary answers
// "how many", and the `counts_a17_static_lto` build exists so the artifact check can be
// seen firing.
#include <cstdio>
#include <cstdlib>
#include <fstream>
#include <iterator>
#include <sstream>
#include <string>

#include "harness.h"

struct Counts { uint64_t fwd, rev, tc; double per_elem_fwd, per_elem_rev; };

// CAMPAIGN req 19 (amended 2026-09-26, R-H31): every exported entry point the timed loop
// calls, resets included. `core` is what the core's own counters see (the codec entry
// points, their loops and reverse calls); `host` is what they cannot see, counted by the
// binding in this build (AK_HOST_CALL): ak_enc_reset inside every encode_into_*, BEFORE
// the encode entry point, and the two ak_dec_reset_<Root> of an armed decode, one BEFORE
// the decode (arms the options) and one AFTER it (disarms); `take` is the timed loop's own
// ak_enc_take after an encode. forward = core + host + take. A drop-mode decode resets
// nothing (its context is bound in drop mode once, outside the loop).
static void show(const char *id, const char *what, const AkCounters &c, uint64_t host, int take,
                 double elems) {
  unsigned long long fwd = (unsigned long long)(c.forward + host + (uint64_t)take);
  std::printf("  %-6s %-22s forward %6llu  reverse %6llu  transcode %8llu"
              "   per element %6.3f / %6.3f   (core %llu + host %llu + take %d)\n",
              id, what, fwd, (unsigned long long)c.reverse, (unsigned long long)c.transcode,
              elems > 0 ? fwd / elems : 0.0, elems > 0 ? c.reverse / elems : 0.0,
              (unsigned long long)c.forward, (unsigned long long)host, take);
}

template <class F>
static void count_enc(const char *id, const char *what, ak_enc_ctx *ctx,
                      intptr_t (*enc)(ak_enc_ctx *, const F &, const shapes::ffi::Tcs &), const F &v,
                      const shapes::ffi::Tcs &t, double elems) {
  AkCounters c;
  ak_enc_counters_reset(ctx);
  shapes::ffi::host_calls_take();
  enc(ctx, v, t);
  const uint8_t *p = NULL;
  size_t n = 0;
  ak_enc_take(ctx, &p, &n);  // the timed loop's own call
  uint64_t host = shapes::ffi::host_calls_take();
  ak_enc_counters(ctx, &c);
  show(id, what, c, host, 1, elems);
}

template <class F>
static void count_dec(const char *id, const char *what, ak_dec_ctx *dctx,
                      int32_t (*dec)(ak_dec_ctx *, const uint8_t *, size_t, F *), const std::string &b,
                      double elems, F *out) {
  AkCounters c;
  ak_dec_counters_reset(dctx);
  shapes::ffi::host_calls_take();
  dec(dctx, (const uint8_t *)b.data(), b.size(), out);
  uint64_t host = shapes::ffi::host_calls_take();
  ak_dec_counters(dctx, &c);
  show(id, what, c, host, 0, elems);
}

template <class F, class P>
static void run_case(const char *id, F (*mk)(void), void (*pbmk)(P *),
                     intptr_t (*ffi_enc)(ak_enc_ctx *, const F &, const shapes::ffi::Tcs &),
                     intptr_t (*ffi_enc_z)(ak_enc_ctx *, const F &, const shapes::ffi::Tcs &),
                     intptr_t (*ffi_enc_nb)(ak_enc_ctx *, const F &, const shapes::ffi::Tcs &),
                     intptr_t (*ffi_enc_unk)(ak_enc_ctx *, const F &, const shapes::ffi::Tcs &),
                     int32_t (*ffi_dec)(ak_dec_ctx *, const uint8_t *, size_t, F *),
                     void (*nat_enc)(const F &, ak::Enc *),
                     int32_t (*nat_dec)(const uint8_t *, size_t, F *),
                     double elems) {
  F facade = mk();
  // The SAME bytes `bench` decodes. Built with protobuf's deterministic serialiser rather
  // than with the native encoder, because on P2.5 the two differ by 80 B (protobuf writes
  // an empty map value, the canonical form omits it) and a crossing count taken over
  // different bytes than the timing is a count of different work.
  P pbm;
  pbmk(&pbm);
  std::string wire;
  pb_serialize_det(pbm, &wire);
  ak::Enc e(shapes::native::kSites);
  nat_enc(facade, &e);

  ak_enc_ctx *ctx = ak_enc_ctx_new();
  shapes::ffi::Tcs tc = shapes::ffi::tcs_core();
  shapes::ffi::Tcs th = shapes::ffi::tcs_host();
  count_enc<F>(id, "encode", ctx, ffi_enc, facade, tc, elems);
  count_enc<F>(id, "encode zeroed-fill", ctx, ffi_enc_z, facade, tc, elems);
  count_enc<F>(id, "encode UNBATCHED", ctx, ffi_enc_nb, facade, tc, elems);
  // The transcoder in the host is a REVERSE crossing per string; the core's is not a
  // crossing at all. `transcode` counts the invocations either way, so the difference is
  // in where the function lives, and that is what makes the count meaningful.
  count_enc<F>(id, "encode host transcoder", ctx, ffi_enc, facade, th, elems);
  if (ffi_enc_unk) count_enc<F>(id, "encode retain", ctx, ffi_enc_unk, facade, tc, elems);

  // Decision 11 rule 6: the context is bound to this payload's root.
  ak_dec_ctx *dctx = shapes::ffi::dec_ctx_new_for<F>();
  F out;
  count_dec<F>(id, "decode", dctx, ffi_dec, wire, elems, &out);
#ifndef AK_NO_UNKNOWN_FIELDS
  // Retain mode (req 19): every position armed, no pre-placed buffer, and unk_grow
  // allocates exactly the size the core requests. The payloads carry no unknown field.
  F r1;
  count_dec<F>(id, "decode retain", dctx, &shapes::ffi::DecRoot<F>::decode_unk, wire, elems, &r1);
#endif
  ak_dec_ctx_free(dctx);
  ak_enc_ctx_free(ctx);
  (void)nat_dec;
}

// The corpus's U-* rows at the shapes roots (req 7's 92), through the counting core: decode
// and re-encode in drop mode, and (full build) in retain mode, where every unknown run is
// copied into a buffer the core asks `grow` for (reverse crossings), exact-size.
template <class F>
static void run_row(const std::string &id, const std::string &v,
                    int32_t (*dec)(ak_dec_ctx *, const uint8_t *, size_t, F *),
                    intptr_t (*enc)(ak_enc_ctx *, const F &, const shapes::ffi::Tcs &),
                    intptr_t (*enc_unk)(ak_enc_ctx *, const F &, const shapes::ffi::Tcs &)) {
  ak_dec_ctx *dctx = shapes::ffi::dec_ctx_new_for<F>();
  ak_enc_ctx *ctx = ak_enc_ctx_new();
  shapes::ffi::Tcs tc = shapes::ffi::tcs_core();
  F d;
  count_dec<F>(id.c_str(), "decode", dctx, dec, v, 0, &d);
  count_enc<F>(id.c_str(), "encode", ctx, enc, d, tc, 0);
#ifndef AK_NO_UNKNOWN_FIELDS
  F r;
  count_dec<F>(id.c_str(), "decode retain", dctx, &shapes::ffi::DecRoot<F>::decode_unk, v, 0, &r);
  if (enc_unk) count_enc<F>(id.c_str(), "encode retain", ctx, enc_unk, r, tc, 0);
#else
  (void)enc_unk;
#endif
  ak_dec_ctx_free(dctx);
  ak_enc_ctx_free(ctx);
}

template <class G>
static void chunk_row(const char *id, const char *elem, size_t elems) {
  size_t cap = ak::arena_n(sizeof(G));
  size_t used = elems < cap ? elems : cap;
  std::printf("  %-6s %-16s %8zu %10zu %10zu %12zu %10s\n", id, elem, sizeof(G), cap, used,
              used * sizeof(G), used * sizeof(G) <= 32768 / 2 ? "fits L1?" : "");
}

#ifdef AK_NO_UNKNOWN_FIELDS
#define AK_COUNT_ENC_UNK(s) NULL
#else
#define AK_COUNT_ENC_UNK(s) &shapes::ffi::encode_into_##s##_unk
#endif

int main(int argc, char **argv) {
  std::string corpus, rows;
  for (int i = 1; i + 1 < argc; i += 2) {
    if (std::string(argv[i]) == "--corpus") corpus = argv[i + 1];
    else if (std::string(argv[i]) == "--rows") rows = argv[i + 1];
  }
  std::printf("counting build, linkage=%s, -std=%ld\n", AK_LINKAGE, (long)__cplusplus);
  std::printf("Per-element columns are forward / reverse. forward = core + host + take (req 19):\n"
              "host = ak_enc_reset inside each encode_into_* (before the encode entry point) and the\n"
              "two ak_dec_reset_<Root> of an armed (retain) decode (before and after it); take = the\n"
              "timed loop's ak_enc_take after an encode. Retain: no pre-placed buffer, exact-size grow.\n\n");
#define X(id, Root, sroot, pfx, sha, nbytes)                                        \
  run_case<shapes::Root, ns::Root>(                                                 \
      id, &shapes::build::payload_##pfx, &pbbuild::payload_##pfx,                   \
      &shapes::ffi::encode_into_##sroot, &shapes::ffi::encode_into_##sroot##_zeroed,\
      &shapes::ffi::encode_into_##sroot##_nobatch, AK_COUNT_ENC_UNK(sroot),         \
      &shapes::ffi::decode_with_##sroot,                                            \
      &shapes::native::encode_into_##sroot, &shapes::native::decode_##sroot,        \
      AK_ELEMS_##pfx);
#define AK_ELEMS_p1_1 4
#define AK_ELEMS_p1_2 1000
#define AK_ELEMS_p1_3 300
#define AK_ELEMS_p2_1 1
#define AK_ELEMS_p2_2 500
#define AK_ELEMS_p2_3 125
#define AK_ELEMS_p2_4 80
#define AK_ELEMS_p2_5 20
#define AK_ELEMS_p3_1 200
#define AK_ELEMS_p4_1 200
#define AK_ELEMS_p5_1 1
#define AK_ELEMS_p5_2 1
#define AK_ELEMS_p5_3 1
#define AK_ELEMS_p5_4 1
#define AK_ELEMS_p6_1 200
  AK_CASES(X)
#undef X

  if (!corpus.empty() && !rows.empty()) {
    std::printf("\n-- the corpus's U-* rows at the shapes roots (req 7, req 19) --\n");
    std::ifstream rf(rows.c_str());
    std::string line;
    while (std::getline(rf, line)) {
      std::istringstream is(line);
      std::string id, root, file;
      std::getline(is, id, '\t');
      std::getline(is, root, '\t');
      std::getline(is, file, '\t');
      std::ifstream vf((corpus + "/" + file).c_str(), std::ios::binary);
      std::string v((std::istreambuf_iterator<char>(vf)), std::istreambuf_iterator<char>());
#define R(Root, sroot)                                                                    \
      if (root == #Root)                                                                  \
        run_row<shapes::Root>(id, v, &shapes::ffi::decode_with_##sroot,                   \
                              &shapes::ffi::encode_into_##sroot, AK_COUNT_ENC_UNK(sroot));
      AK_ROOTS(R)
#undef R
    }
  }

  std::printf("\n-- the batching predicate's decomposition (ABI v1 decision 1) --\n");
  std::printf("  %-6s %-16s %8s %10s %10s %12s\n", "payload", "element group",
              "sizeof", "chunk cap", "used", "chunk bytes");
  chunk_row<struct ak_efix_ResultRaw>("P1.1", "ResultRaw", 4);
  chunk_row<struct ak_efix_ResultRaw>("P1.2", "ResultRaw", 1000);
  chunk_row<struct ak_efix_ResultRaw>("P1.3", "ResultRaw", 300);
  chunk_row<struct ak_efix_TaskDetailed>("P2.1", "TaskDetailed", 1);
  chunk_row<struct ak_efix_TaskDetailed>("P2.2", "TaskDetailed", 500);
  chunk_row<struct ak_efix_TaskDetailed>("P2.3", "TaskDetailed", 125);
  chunk_row<struct ak_efix_TaskDetailed>("P2.4", "TaskDetailed", 80);
  chunk_row<struct ak_efix_TaskDetailed>("P2.5", "TaskDetailed", 20);
  chunk_row<struct ak_efix_Probe>("P3.1", "Probe", 200);
  chunk_row<struct ak_efix_TaskSummary>("P4.1", "TaskSummary", 200);
  chunk_row<struct ak_efix_MetricsBatch>("P6.1", "MetricsBatch", 200);
  std::printf("  P2.3 and P2.4 are the two payloads where UNBATCHING adds 125 and 311\n"
              "  forward crossings per element rather than 1, and they are the two where\n"
              "  batching wins decisively. Everywhere else the delta tracks the chunk\n"
              "  bytes touched, not the crossing count.\n");
  return 0;
}
