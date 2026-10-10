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
// binding in this build (AK_HOST_CALL): since FIX-PLAN D27 (the core resets every context
// in the first call of an operation) no ak_enc_reset, and ak_dec_reset_<Root> only where the
// options pointer is set or changed (a retain decode after a drop one arms the context once
// and leaves it armed; a drop decode after a retain one disarms it); every decode count is
// taken warm (the same decode once before on the same context), so a steady loop's resets,
// none, are what is counted; plus ak_dec_err after a decode; `take` is the timed loop's own
// ak_enc_take after an encode. forward = core + host + take.
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
  // FIX-PLAN D27: the count is taken WARM, as the timed loop runs (the same decode once
  // before on the same context): the binding arms a context once and the core re-arms on
  // every decode entry, so a retain decode in the loop makes no reset; the arming is the
  // warm call's.
  { F warm; dec(dctx, (const uint8_t *)b.data(), b.size(), &warm); }
  ak_dec_counters_reset(dctx);
  shapes::ffi::host_calls_take();
  dec(dctx, (const uint8_t *)b.data(), b.size(), out);
  uint64_t host = shapes::ffi::host_calls_take();
  ak_dec_counters(dctx, &c);
  show(id, what, c, host, 0, elems);
}

// X-2 (2026-09-28): the pull family. ABI v1 7.1's invariant: pull makes NO reverse call and
// writes exactly one record where push makes one, so records == push's reverse count; the
// host counts the records its replay dispatches (pull_records_take). A retain pull's reverse
// calls are decision 11's `grow` only (none on a payload, which carries no unknown field).
template <class F>
static uint64_t push_reverse(ak_dec_ctx *dctx, int32_t (*dec)(ak_dec_ctx *, const uint8_t *, size_t, F *),
                             const std::string &b) {
  AkCounters c;
  F out;
  ak_dec_counters_reset(dctx);
  dec(dctx, (const uint8_t *)b.data(), b.size(), &out);
  ak_dec_counters(dctx, &c);
  return c.reverse;
}
template <class F>
static void count_pull(const char *id, const char *what, ak_dec_ctx *dctx,
                       int32_t (*pull)(ak_dec_ctx *, const uint8_t *, size_t, F *), const std::string &b,
                       uint64_t want_records) {
  AkCounters c;
  F out;
  { F warm; pull(dctx, (const uint8_t *)b.data(), b.size(), &warm); }  // D27: warm, as count_dec
  ak_dec_counters_reset(dctx);
  shapes::ffi::host_calls_take();
  shapes::ffi::pull_records_take();
  pull(dctx, (const uint8_t *)b.data(), b.size(), &out);
  uint64_t host = shapes::ffi::host_calls_take();
  uint64_t recs = shapes::ffi::pull_records_take();
  ak_dec_counters(dctx, &c);
  unsigned long long fwd = (unsigned long long)(c.forward + host);
  std::printf("  %-6s %-22s forward %6llu  reverse %6llu  records %6llu  (core %llu + host %llu)  %s\n", id, what,
              fwd, (unsigned long long)c.reverse, (unsigned long long)recs, (unsigned long long)c.forward,
              (unsigned long long)host, recs == want_records ? "records = push reverse" : "RECORDS != PUSH REVERSE");
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
  // Retain mode (req 19): every position armed, no pre-placed buffer, and unk_grow grows
  // geometrically (decision 11 rule 8), as in the timed build. The payloads carry no
  // unknown field.
  F r1;
  count_dec<F>(id, "decode retain", dctx, &shapes::ffi::DecRoot<F>::decode_unk, wire, elems, &r1);
#endif
  {
    const uint64_t pr = push_reverse<F>(dctx, ffi_dec, wire);
    count_pull<F>(id, "decode pull", dctx, &shapes::ffi::DecRoot<F>::pull, wire, pr);
    count_pull<F>(id, "decode pull drained", dctx, &shapes::ffi::DecRoot<F>::pull_drain, wire, pr);
#ifndef AK_NO_UNKNOWN_FIELDS
    count_pull<F>(id, "decode pull retain", dctx, &shapes::ffi::DecRoot<F>::pull_unk, wire, pr);
#endif
  }
  shapes::ffi::dec_ctx_free(dctx);
  ak_enc_ctx_free(ctx);
  (void)nat_dec;
}

// The corpus's U-* rows at the shapes roots (req 7's 92), through the counting core: decode
// and re-encode in drop mode, and (full build) in retain mode, where every unknown run is
// copied into a buffer the core asks `grow` for (reverse crossings), grown geometrically.
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
  shapes::ffi::dec_ctx_free(dctx);
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

#ifdef AK_COUNT_GRID
// D18 (CAMPAIGN section 4.0) and req. 19: one count row per timed core-ffi row of the core grid,
// exactly the calls that row's timed loop makes. Encode: end state (ii), the transport-ready form:
// encode_into_* (retain in the full build; the no-unknown build's own), ak_enc_take_owned (the
// buffer moved to the transport) and ak_bytes_free when the transport drops it: `take` here is
// those two RPC-layer entries, from the core's RPC counters (this binary links the rpc,count
// campaign core). Decode: decode_with_*_unk (full) / decode_with_* (no-unknown) from the
// contiguous buffer, then the read (host code, no crossing). Rows: the 16 shapes in ASCII (P7.1
// decode only), P2.2 Latin-1 and wide, and the 7 U-* rows of section 4.0.
template <class F>
static void count_enc_transport(const char *id, const char *what, ak_enc_ctx *ctx,
                                intptr_t (*enc)(ak_enc_ctx *, const F &, const shapes::ffi::Tcs &), const F &v,
                                double elems) {
  AkCounters c;
  ak_enc_counters_reset(ctx);
  ak_rpc_counters_reset();
  shapes::ffi::host_calls_take();
  enc(ctx, v, shapes::ffi::tcs_core());
  struct ak_bytes b;
  b.ptr = NULL; b.len = 0; b.owner = NULL;
  ak_enc_take_owned(ctx, &b);
  ak_bytes_free(&b);
  uint64_t host = shapes::ffi::host_calls_take();
  struct ak_rpc_counters rc;
  ak_rpc_counters(&rc);
  ak_enc_counters(ctx, &c);
  show(id, what, c, host, (int)rc.forward, elems);
}
#ifdef AK_NO_UNKNOWN_FIELDS
#define AK_GRID_ENC(s) &shapes::ffi::encode_into_##s
#define AK_GRID_DEC(F) &shapes::ffi::DecRoot<F>::decode
#define AK_GRID_ENC_WHAT "encode transport"
#define AK_GRID_DEC_WHAT "decode"
#else
#define AK_GRID_ENC(s) &shapes::ffi::encode_into_##s##_unk
#define AK_GRID_DEC(F) &shapes::ffi::DecRoot<F>::decode_unk
#define AK_GRID_ENC_WHAT "encode retain transport"
#define AK_GRID_DEC_WHAT "decode retain"
#endif
template <class F, class P>
static void grid_case(const std::string &id, F (*mk)(void), void (*pbmk)(P *),
                      intptr_t (*enc)(ak_enc_ctx *, const F &, const shapes::ffi::Tcs &), double elems, bool encode) {
  F facade = mk();
  P pbm;
  pbmk(&pbm);
  std::string wire;
  pb_serialize_det(pbm, &wire);
  ak_enc_ctx *ctx = ak_enc_ctx_new();
  ak_dec_ctx *dctx = shapes::ffi::dec_ctx_new_for<F>();
  if (encode) count_enc_transport<F>(id.c_str(), AK_GRID_ENC_WHAT, ctx, enc, facade, elems);
  F out;
  count_dec<F>(id.c_str(), AK_GRID_DEC_WHAT, dctx, AK_GRID_DEC(F), wire, elems, &out);
  shapes::ffi::dec_ctx_free(dctx);
  ak_enc_ctx_free(ctx);
}
template <class F>
static void grid_row(const std::string &id, const std::string &v,
                     intptr_t (*enc)(ak_enc_ctx *, const F &, const shapes::ffi::Tcs &)) {
  ak_enc_ctx *ctx = ak_enc_ctx_new();
  ak_dec_ctx *dctx = shapes::ffi::dec_ctx_new_for<F>();
  F d;
  count_dec<F>(id.c_str(), AK_GRID_DEC_WHAT, dctx, AK_GRID_DEC(F), v, 0, &d);
  count_enc_transport<F>(id.c_str(), AK_GRID_ENC_WHAT, ctx, enc, d, 0);
  shapes::ffi::dec_ctx_free(dctx);
  ak_enc_ctx_free(ctx);
}
static int grid_main(const std::string &corpus) {
  std::printf("counting build (D18 core grid), linkage=%s, -std=%ld\n", AK_LINKAGE, (long)__cplusplus);
  std::printf("One row per timed core-ffi row of CAMPAIGN 4.0's grid. forward = core + host + take: host = the binding's\n"
              "calls the core does not count (D27: no ak_enc_reset, no per-decode reset; counted warm); take = ak_enc_take_owned + ak_bytes_free\n"
              "(the transport-ready encode), from the core's RPC counters.\n\n");
  static const ak::values::ContentSet sets[3] = {ak::values::kAscii, ak::values::kLatin1, ak::values::kWide};
  static const char *setname[3] = {"", "/latin1", "/wide"};
#define X(id, Root, sroot, pfx, sha, nbytes)                                                        \
  for (int cs = 0; cs < (std::string(id) == "P2.2" ? 3 : 1); ++cs) {                              \
    ak::values::ScopedContentSet scope(sets[cs]);                                                  \
    grid_case<shapes::Root, ns::Root>(std::string(id) + setname[cs], &shapes::build::payload_##pfx, \
                                      &pbbuild::payload_##pfx, AK_GRID_ENC(sroot), AK_ELEMS_##pfx, true); \
  }
  AK_CASES(X)
#undef X
  // P7.1: decode only, from its committed vector.
  {
    std::ifstream vf((std::string("payloads/") + AK_P71_VECTOR).c_str(), std::ios::binary);
    std::string v((std::istreambuf_iterator<char>(vf)), std::istreambuf_iterator<char>());
    if (!v.empty()) {
      ak_dec_ctx *dctx = shapes::ffi::dec_ctx_new_for<shapes::DualResponse>();
      shapes::DualResponse out;
      count_dec<shapes::DualResponse>("P7.1", AK_GRID_DEC_WHAT, dctx, AK_GRID_DEC(shapes::DualResponse), v, 0, &out);
      shapes::ffi::dec_ctx_free(dctx);
    } else {
      std::printf("  P7.1   (vector payloads/P7.1.bin not found: run from ffi/schema/generated)\n");
    }
  }
  static const char *const kU[7][3] = {
      {"U-nested-before", "ListResultsResponse", ""}, {"U-deep-u-repeated", "ListTasksDetailedResponse", ""},
      {"U-oneof-u-repeated", "ListProbeResponse", ""},
      {"U-wire-ListTaskSummaryResponse-tasks-as-wt5", "ListTaskSummaryResponse", ""},
      {"U-wire-UploadResultDataMessage-upload-as-wt5", "UploadResultDataMessage", ""},
      {"U-wire-ListMetricsResponse-batches-as-wt0", "ListMetricsResponse", ""},
      {"U-wire-DualResponse-left-as-wt5", "DualResponse", ""}};
  std::printf("\n-- the 7 U-* rows of CAMPAIGN 4.0 --\n");
  for (int u = 0; u < 7; ++u) {
    // the row's vector file, as gen/u_rows.py resolves it: vectors/<id>.bin under the corpus
    std::ifstream vf((corpus + "/vectors/" + kU[u][0] + ".bin").c_str(), std::ios::binary);
    std::string v((std::istreambuf_iterator<char>(vf)), std::istreambuf_iterator<char>());
    if (v.empty()) { std::printf("  %s: vector not found\n", kU[u][0]); return 2; }
    const std::string root = kU[u][1], id = kU[u][0];
#define R(Root, sroot) \
    if (root == #Root) grid_row<shapes::Root>(id, v, AK_GRID_ENC(sroot));
    AK_ROOTS(R)
#undef R
  }
  return 0;
}
#endif

int main(int argc, char **argv) {
  std::string corpus, rows;
  for (int i = 1; i + 1 < argc; i += 2) {
    if (std::string(argv[i]) == "--corpus") corpus = argv[i + 1];
    else if (std::string(argv[i]) == "--rows") rows = argv[i + 1];
  }
#ifdef AK_COUNT_GRID
  return grid_main(corpus);
#endif
  std::printf("counting build, linkage=%s, -std=%ld\n", AK_LINKAGE, (long)__cplusplus);
  std::printf("Per-element columns are forward / reverse. forward = core + host + take (req 19):\n"
              "host = the binding's calls the core does not count (D27: no ak_enc_reset, no per-decode\n"
              "reset; counted warm); take = the timed loop's\n"
              "ak_enc_take after an encode. Retain: no pre-placed buffer, geometric grow (rule 8).\n\n");
#define X(id, Root, sroot, pfx, sha, nbytes)                                        \
  run_case<shapes::Root, ns::Root>(                                                 \
      id, &shapes::build::payload_##pfx, &pbbuild::payload_##pfx,                   \
      &shapes::ffi::encode_into_##sroot, &shapes::ffi::encode_into_##sroot##_zeroed,\
      &shapes::ffi::encode_into_##sroot##_nobatch, AK_COUNT_ENC_UNK(sroot),         \
      &shapes::ffi::decode_with_##sroot,                                            \
      &shapes::native::encode_into_##sroot, &shapes::native::decode_##sroot,        \
      AK_ELEMS_##pfx);
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
