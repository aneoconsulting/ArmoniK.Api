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
  ak::Enc e(shapes::native::kSites);
  nat_enc(facade, &e);
  std::string wire((const char *)(e.buf.empty() ? NULL : &e.buf[0]), e.buf.size());

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

  ak_dec_ctx *dctx = ak_dec_ctx_new();
  F out;
  ak_dec_counters_reset(dctx);
  ffi_dec(dctx, (const uint8_t *)wire.data(), wire.size(), &out);
  ak_dec_counters(dctx, &c);
  show(id, "decode", c, elems);
  ak_dec_ctx_free(dctx);
  (void)pbmk;
  (void)nat_dec;
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
  return 0;
}
