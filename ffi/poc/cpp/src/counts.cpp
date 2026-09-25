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
#include <string>

#include "harness.h"

struct Counts { uint64_t fwd, rev, tc; double per_elem_fwd, per_elem_rev; };

static void show(const char *id, const char *what, const AkCounters &c, double elems) {
  std::printf("  %-6s %-22s forward %6llu  reverse %6llu  transcode %8llu"
              "   per element %6.3f / %6.3f\n",
              id, what, (unsigned long long)c.forward, (unsigned long long)c.reverse,
              (unsigned long long)c.transcode,
              elems > 0 ? c.forward / elems : 0.0, elems > 0 ? c.reverse / elems : 0.0);
}

template <class F, class P>
static void run_case(const char *id, F (*mk)(void), void (*pbmk)(P *),
                     intptr_t (*ffi_enc)(ak_enc_ctx *, const F &, const shapes::ffi::Tcs &),
                     intptr_t (*ffi_enc_z)(ak_enc_ctx *, const F &, const shapes::ffi::Tcs &),
                     intptr_t (*ffi_enc_nb)(ak_enc_ctx *, const F &, const shapes::ffi::Tcs &),
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
  AkCounters c;
  shapes::ffi::Tcs tc = shapes::ffi::tcs_core();
  shapes::ffi::Tcs th = shapes::ffi::tcs_host();

  ak_enc_counters_reset(ctx);
  ffi_enc(ctx, facade, tc);
  ak_enc_counters(ctx, &c);
  show(id, "encode", c, elems);

  ak_enc_counters_reset(ctx);
  ffi_enc_z(ctx, facade, tc);
  ak_enc_counters(ctx, &c);
  show(id, "encode zeroed-fill", c, elems);

  ak_enc_counters_reset(ctx);
  ffi_enc_nb(ctx, facade, tc);
  ak_enc_counters(ctx, &c);
  show(id, "encode UNBATCHED", c, elems);

  ak_enc_counters_reset(ctx);
  ffi_enc(ctx, facade, th);
  ak_enc_counters(ctx, &c);
  // The transcoder in the host is a REVERSE crossing per string; the core's is not a
  // crossing at all. `transcode` counts the invocations either way, so the difference is
  // in where the function lives, and that is what makes the count meaningful.
  show(id, "encode host transcoder", c, elems);

  ak_enc_ctx_free(ctx);

  // Decision 11 rule 6: the context is bound to this payload's root.
  ak_dec_ctx *dctx = shapes::ffi::dec_ctx_new_for<F>();
  F out;
  ak_dec_counters_reset(dctx);
  ffi_dec(dctx, (const uint8_t *)wire.data(), wire.size(), &out);
  ak_dec_counters(dctx, &c);
  show(id, "decode", c, elems);
  if (std::getenv("AK_COUNTS_RETAIN") != NULL) {
    // Requirement 10's arms, counted. Printed only on request, so the committed baseline's
    // rows (the campaign gate's comparison) are unchanged. The two ak_dec_reset_<Root>
    // calls of an armed decode are forward calls the core's counters do not see.
    F r1;
    ak_dec_counters_reset(dctx);
    shapes::ffi::DecRoot<F>::decode_unk(dctx, (const uint8_t *)wire.data(), wire.size(), &r1);
    ak_dec_counters(dctx, &c);
    show(id, "decode retain (+2 rst)", c, elems);
    F r2;
    ak_dec_counters_reset(dctx);
    shapes::ffi::DecRoot<F>::decode_pool(dctx, (const uint8_t *)wire.data(), wire.size(), &r2,
                                         4, 64, NULL);
    ak_dec_counters(dctx, &c);
    show(id, "decode pool (+2 rst)", c, elems);
  }
  ak_dec_ctx_free(dctx);
  (void)nat_dec;
}

// ABI v1 open decision 1, the batching predicate: the DECOMPOSITION finding 4 asks for.
//
// "Batching wins where the crossing count per element explodes" does not survive its own
// table: batching wins on P3.1 (+4.2%) and loses on P1.2 (straddling zero) at an identical
// +1.00 forward crossings per element when unbatched. The crossing count is therefore not
// the variable. What differs is how much of the 32 KB chunk buffer is written before the
// codec reads it: the batched arm makes two passes over it, and the unbatched arm keeps one
// group in L1. This prints the group size, the chunk's element capacity, and how many
// elements of it a payload actually fills, which is the number the sign tracks.
template <class G>
static void chunk_row(const char *id, const char *elem, size_t elems) {
  size_t cap = ak::arena_n(sizeof(G));
  size_t used = elems < cap ? elems : cap;
  std::printf("  %-6s %-16s %8zu %10zu %10zu %12zu %10s\n", id, elem, sizeof(G), cap, used,
              used * sizeof(G), used * sizeof(G) <= 32768 / 2 ? "fits L1?" : "");
}

int main() {
  std::printf("counting build, linkage=%s, -std=%ld\n", AK_LINKAGE, (long)__cplusplus);
  std::printf("Per-element columns are forward / reverse.\n\n");
#define X(id, Root, sroot, pfx, sha, nbytes)                                        \
  run_case<shapes::Root, ns::Root>(                                                 \
      id, &shapes::build::payload_##pfx, &pbbuild::payload_##pfx,                   \
      &shapes::ffi::encode_into_##sroot, &shapes::ffi::encode_into_##sroot##_zeroed,\
      &shapes::ffi::encode_into_##sroot##_nobatch, &shapes::ffi::decode_with_##sroot,\
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
