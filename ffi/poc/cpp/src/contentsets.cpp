// The content sets, on WHOLE PAYLOADS rather than on the string path alone.
//
// design/SHAPES.md: "A slice that reports one string-path number without saying which
// content set it came from has reported half a number." This slice had priced the string
// path over all three sets and every whole-payload row over ASCII only, so the whole-payload
// rows were the half-number the sentence is about.
//
// Two things SHAPES.md fixes about how to do it, and both shape this file:
//
//   1. "No manifest oracle covers them, because schema/ emits the ASCII set only. A slice
//      checks a content set by byte identity of all its arms against the INCUMBENT arm,
//      which the manifest validated on ASCII, plus a decode round trip per set." So
//      protobuf is the oracle here, and on ASCII protobuf is itself checked against the
//      manifest first -- otherwise the oracle is unanchored on the one set where it can be
//      anchored.
//   2. "What these sets price is not the same thing in every host, and a column must not be
//      read across." C++ is in Rust's position, not the JVM's: a std::string is already
//      UTF-8, so there is no narrowing transcoder and nothing can fail to be representable.
//      What changes is the byte width of the same character count, and which path the
//      validator takes. These rows price WIDTH AND VALIDATION. They are not comparable with
//      a managed slice's, which price a transcoder.
//
// The content set is applied in the VALUE RULES (ak::values::guid/word/sentence), so both
// construction routes -- the facade builder and the protobuf builder, generated separately
// over two object graphs -- reach it by the same two independent paths they always did.
// `blob` and `bulk` do not honour it: a `bytes` field has no encoding to be in, which is
// C20 stated as code rather than as a comment.
#include <chrono>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

#include "harness.h"

// The same two barriers bench.cpp uses: a register-only one where only the value must
// survive, and a memory clobber where a buffer must.
#if defined(__GNUC__) || defined(__clang__)
#define AK_CS_SINK(x) asm volatile("" : : "r"(x))
#define AK_CS_SINK_MEM(x) asm volatile("" : : "r"(&(x)) : "memory")
#else
#define AK_CS_SINK(x) (void)(x)
#define AK_CS_SINK_MEM(x) (void)(x)
#endif

static int g_fail = 0;
static int g_checks = 0;

static void check(bool ok, const std::string &what) {
  ++g_checks;
  if (!ok) {
    ++g_fail;
    std::printf("  FAIL  %s\n", what.c_str());
  }
}

namespace {

struct Row {
  const char *id;
  std::size_t bytes[3];
  double enc[3];       // ns, min of rounds, the ffi arm as specified (no encode check)
  double encv[3];      // ffi-valtc: the same with the core's VALIDATING transcoder
  double dec[3];
  double pb_enc[3];
  double pb_dec[3];
};

double now_ns() {
  return (double)std::chrono::duration_cast<std::chrono::nanoseconds>(
             std::chrono::steady_clock::now().time_since_epoch())
      .count();
}

template <class F>
double timed(F f, int n) {
  double t0 = now_ns();
  for (int i = 0; i < n; ++i) f();
  return (now_ns() - t0) / n;
}

}  // namespace

// One payload, over all three sets.
template <class Fac, class Pb>
static void run(Row *row, const char *id, Fac (*mk)(void), void (*pbmk)(Pb *),
                intptr_t (*ffi_enc)(ak_enc_ctx *, const Fac &, const shapes::ffi::Tcs &),
                int32_t (*ffi_dec)(ak_dec_ctx *, const uint8_t *, std::size_t, Fac *),
                void (*nat_enc)(const Fac &, ak::Enc *), const char *ascii_sha, int rounds) {
  row->id = id;
  static const ak::values::ContentSet sets[3] = {ak::values::kAscii, ak::values::kLatin1,
                                                 ak::values::kWide};
  static const char *names[3] = {"ascii", "latin1", "wide"};

  ak_enc_ctx *ectx = ak_enc_ctx_new();
  ak_dec_ctx *dctx = shapes::ffi::dec_ctx_new_for<Fac>();
  shapes::ffi::Tcs tc = shapes::ffi::tcs_core();

  for (int s = 0; s < 3; ++s) {
    ak::values::ScopedContentSet scope(sets[s]);
    Fac facade = mk();
    Pb pb;
    pbmk(&pb);

    // The oracle. On ascii it is anchored to the manifest; on the other two sets it is the
    // incumbent and nothing else, which is what SHAPES.md says a slice must do.
    std::string want;
    pb_serialize_det(pb, &want);
    if (s == 0)
      check(sha_of(want) == std::string(ascii_sha),
            std::string(id) + " ascii: the incumbent matches manifest.json, so it is an"
                              " anchored oracle for the other two sets");
    row->bytes[s] = want.size();

    // Every arm, byte-identical against it.
    {
      ak::Enc e(shapes::native::kSites);
      nat_enc(facade, &e);
      check(e.size() == want.size() &&
                std::memcmp(e.data(), want.data(), e.size()) == 0,
            std::string(id) + " " + names[s] + ": native is byte-identical to the incumbent");
    }
    {
      intptr_t rc = ffi_enc(ectx, facade, tc);
      const uint8_t *p = NULL;
      std::size_t n = 0;
      ak_enc_take(ectx, &p, &n);
      check(rc >= 0 && n == want.size() && std::memcmp(p, want.data(), n) == 0,
            std::string(id) + " " + names[s] + ": ffi is byte-identical to the incumbent");
    }
    {
      shapes::ffi::Tcs th = shapes::ffi::tcs_host();
      intptr_t rc = ffi_enc(ectx, facade, th);
      const uint8_t *p = NULL;
      std::size_t n = 0;
      ak_enc_take(ectx, &p, &n);
      check(rc >= 0 && n == want.size() && std::memcmp(p, want.data(), n) == 0,
            std::string(id) + " " + names[s] + ": ffi-hosttc is byte-identical");
    }
    {
      // R-D5: `ffi-valtc` is timed below (the like-for-like encode row) and no gate ran
      // it. On latin1 and wide it is the arm whose validator actually has work to do, so
      // it is the one most worth gating on these sets.
      shapes::ffi::Tcs tv = shapes::ffi::tcs_core_validating();
      intptr_t rc = ffi_enc(ectx, facade, tv);
      const uint8_t *p = NULL;
      std::size_t n = 0;
      int32_t trc = ak_enc_take(ectx, &p, &n);
      check(rc >= 0 && trc == 0 && ak_enc_err(ectx) == 0 && n == want.size() &&
                std::memcmp(p, want.data(), n) == 0,
            std::string(id) + " " + names[s] + ": ffi-valtc is byte-identical");
    }
    // The decode round trip per set, which is the other half of what SHAPES.md asks for.
    {
      Fac back;
      int32_t rc = ffi_dec(dctx, (const uint8_t *)want.data(), want.size(), &back);
      ak::Enc e(shapes::native::kSites);
      nat_enc(back, &e);
      check(rc >= 0 && e.size() == want.size() &&
                std::memcmp(e.data(), want.data(), e.size()) == 0,
            std::string(id) + " " + names[s] + ": decode round trip returns the same bytes");
    }
    {
      Pb back;
      check(back.ParseFromString(want), std::string(id) + " " + names[s] +
                                            ": the incumbent parses its own output");
    }

    // Timing. One process, one build; the three sets are three inputs to the same code, so
    // this is a within-arm comparison and not a cross-build one.
    // A FRESH object per decode, on both arms, exactly as bench.cpp does it. The first
    // version of this file reused one object per arm and the numbers were nonsense: a
    // reused protobuf message keeps its allocations and parses into them, which made the
    // incumbent look 3x faster than it is, while a reused facade ACCUMULATES into its
    // vectors, which made the core look slow and also made its output wrong. That is the
    // same defect the review found in the encode harness (C7), with the sign flipped on
    // one arm and pointing both ways at once.
    std::string out;
    auto e_pb = [&]() { pb_serialize_default(pb, &out); AK_CS_SINK_MEM(out); };
    auto e_ffi = [&]() {
      intptr_t r = ffi_enc(ectx, facade, tc);
      const uint8_t *p = NULL;
      std::size_t n = 0;
      ak_enc_take(ectx, &p, &n);
      AK_CS_SINK(r);
      AK_CS_SINK(p);
      AK_CS_SINK(n);
    };
    // The like-for-like encode row, and on these sets it is the row that matters.
    // protobuf C++ validates UTF-8 when it SERIALISES a `string`; ABI v1 says the core
    // does not, because the encoder's check bought nothing and the parser cannot trust the
    // wire anyway. On ascii that is a small difference. On latin1 and wide it is most of
    // the column, so reporting `ffi` against `pb` alone here would be publishing a check
    // the core skips as if it were codec speed.
    shapes::ffi::Tcs tv = shapes::ffi::tcs_core_validating();
    auto e_ffiv = [&]() {
      intptr_t r = ffi_enc(ectx, facade, tv);
      const uint8_t *p = NULL;
      std::size_t n = 0;
      ak_enc_take(ectx, &p, &n);
      AK_CS_SINK(r);
      AK_CS_SINK(p);
      AK_CS_SINK(n);
    };
    auto d_pb = [&]() { Pb m; m.ParseFromString(want); AK_CS_SINK_MEM(m); };
    auto d_ffi = [&]() {
      Fac o;
      int32_t rc = ffi_dec(dctx, (const uint8_t *)want.data(), want.size(), &o);
      AK_CS_SINK(rc);
      AK_CS_SINK_MEM(o);
    };

    double be = 1e300, bd = 1e300, bpe = 1e300, bpd = 1e300, bev = 1e300;
    for (int r = 0; r < rounds; ++r) {
      double a = timed(e_pb, 3), b = timed(e_ffi, 3), v = timed(e_ffiv, 3);
      double c = timed(d_pb, 3), d = timed(d_ffi, 3);
      if (a < bpe) bpe = a;
      if (b < be) be = b;
      if (v < bev) bev = v;
      if (c < bpd) bpd = c;
      if (d < bd) bd = d;
    }
    row->encv[s] = bev;
    row->enc[s] = be;
    row->dec[s] = bd;
    row->pb_enc[s] = bpe;
    row->pb_dec[s] = bpd;
  }
  ak_enc_ctx_free(ectx);
  ak_dec_ctx_free(dctx);
}

int main(int argc, char **argv) {
  int rounds = argc > 1 ? std::atoi(argv[1]) : 5;
  std::printf("== the content sets, on whole payloads ==\n");
  std::printf("-std=%ld  impl=%s  linkage=%s  rounds=%d\n", (long)__cplusplus,
#if AK_CXX17
              "target",
#else
              "floor",
#endif
              AK_LINKAGE, rounds);
  std::printf("protobuf %d.  Oracle: the INCUMBENT arm, anchored to manifest.json on ascii\n",
              GOOGLE_PROTOBUF_VERSION);
  std::printf("C++ holds UTF-8 already, so these rows price WIDTH AND VALIDATION, never a\n"
              "narrowing transcoder. Not comparable with a managed slice's (SHAPES.md).\n");

  std::vector<Row> rows(5);
  std::printf("\n-- byte identity, every arm against the incumbent, every set --\n");
  run<shapes::ListResultsResponse, ns::ListResultsResponse>(
      &rows[0], "P1.2", &shapes::build::payload_p1_2, &pbbuild::payload_p1_2,
      &shapes::ffi::encode_into_list_results_response,
      &shapes::ffi::decode_with_list_results_response,
      &shapes::native::encode_into_list_results_response,
      "eed4817ef84c1f7b8d66693a9249fdbb4f75fec436264a922523fafe21124588", rounds);
  run<shapes::ListTasksDetailedResponse, ns::ListTasksDetailedResponse>(
      &rows[1], "P2.2", &shapes::build::payload_p2_2, &pbbuild::payload_p2_2,
      &shapes::ffi::encode_into_list_tasks_detailed_response,
      &shapes::ffi::decode_with_list_tasks_detailed_response,
      &shapes::native::encode_into_list_tasks_detailed_response,
      "8396d3016807731b06575188352df01e1065eba6bdc11e43b4c2c6b450292693", rounds);
  run<shapes::ListProbeResponse, ns::ListProbeResponse>(
      &rows[2], "P3.1", &shapes::build::payload_p3_1, &pbbuild::payload_p3_1,
      &shapes::ffi::encode_into_list_probe_response,
      &shapes::ffi::decode_with_list_probe_response,
      &shapes::native::encode_into_list_probe_response,
      "40c414645f0570dc6325b2717327e19aac3bdac730b0e14a8af28fae4fb251a4", rounds);
  run<shapes::ListTaskSummaryResponse, ns::ListTaskSummaryResponse>(
      &rows[3], "P4.1", &shapes::build::payload_p4_1, &pbbuild::payload_p4_1,
      &shapes::ffi::encode_into_list_task_summary_response,
      &shapes::ffi::decode_with_list_task_summary_response,
      &shapes::native::encode_into_list_task_summary_response,
      "1d9c1b4bb7d1978455d3741c3bebccdbb858f0ae16c8cd5f6d64e90961ae3b78", rounds);
  // The control: M6 is packed scalars and one string per batch, so the sets should barely
  // move it. A set of rows where EVERY payload moves by the same factor is a set of rows
  // measuring the harness.
  run<shapes::ListMetricsResponse, ns::ListMetricsResponse>(
      &rows[4], "P6.1", &shapes::build::payload_p6_1, &pbbuild::payload_p6_1,
      &shapes::ffi::encode_into_list_metrics_response,
      &shapes::ffi::decode_with_list_metrics_response,
      &shapes::native::encode_into_list_metrics_response,
      "115d4410844a087c257361aa479921aa7079a68213bcb17a8c335bcc5e929c9b", rounds);

  std::printf("\n  %d checks, %d failures\n", g_checks, g_fail);
  if (g_fail) {
    std::printf("  NOT REPORTING TIMINGS: an arm that is not byte-identical has no speed\n"
                "  worth reporting (R2).\n");
    return 1;
  }
  // AK_CS_GATE_ONLY=1 (run with rounds = 0): the byte-identity gates and nothing else, so
  // a correctness log carries no container timing (README 1.1).
  if (std::getenv("AK_CS_GATE_ONLY")) {
    std::printf("  AK_CS_GATE_ONLY: gates only, no timing reported\n");
    return 0;
  }

  std::printf("\n-- wire size: what the sets cost in bytes --\n");
  std::printf("  %-6s %12s %12s %12s %10s %10s\n", "payload", "ascii B", "latin1 B",
              "wide B", "latin1/a", "wide/a");
  for (std::size_t i = 0; i < rows.size(); ++i)
    std::printf("  %-6s %12zu %12zu %12zu %10.3f %10.3f\n", rows[i].id, rows[i].bytes[0],
                rows[i].bytes[1], rows[i].bytes[2],
                (double)rows[i].bytes[1] / (double)rows[i].bytes[0],
                (double)rows[i].bytes[2] / (double)rows[i].bytes[0]);

  std::printf("\n-- encode and decode, per set, ffi against the incumbent IN THE SAME SET --\n");
  std::printf("  Each ratio has its own denominator: comparing a wide-set core against an\n"
              "  ascii-set protobuf would be comparing two different messages.\n\n");
  std::printf("  %-6s %-7s %11s %11s %7s %11s %7s %11s %11s %7s\n", "payload", "set",
              "pb enc ns", "ffi enc ns", "enc", "ffi-valtc", "valtc", "pb dec ns",
              "ffi dec ns", "dec");
  static const char *names[3] = {"ascii", "latin1", "wide"};
  for (std::size_t i = 0; i < rows.size(); ++i) {
    for (int s = 0; s < 3; ++s)
      std::printf("  %-6s %-7s %11.1f %11.1f %7.3f %11.1f %7.3f %11.1f %11.1f %7.3f\n",
                  rows[i].id, names[s], rows[i].pb_enc[s], rows[i].enc[s],
                  rows[i].enc[s] / rows[i].pb_enc[s], rows[i].encv[s],
                  rows[i].encv[s] / rows[i].pb_enc[s], rows[i].pb_dec[s], rows[i].dec[s],
                  rows[i].dec[s] / rows[i].pb_dec[s]);
  }

  std::printf("\n-- and the same ratios against the ASCII row of the same arm --\n");
  std::printf("  This is the number SHAPES.md's sentence is about: how much of a published\n"
              "  ratio is a fact about the content set it was taken on.\n\n");
  std::printf("  %-6s %-7s %10s %10s %10s %10s %10s\n", "payload", "set", "ffi enc",
              "valtc enc", "pb enc", "ffi dec", "pb dec");
  for (std::size_t i = 0; i < rows.size(); ++i) {
    for (int s = 1; s < 3; ++s)
      std::printf("  %-6s %-7s %10.3f %10.3f %10.3f %10.3f %10.3f\n", rows[i].id, names[s],
                  rows[i].enc[s] / rows[i].enc[0], rows[i].encv[s] / rows[i].encv[0],
                  rows[i].pb_enc[s] / rows[i].pb_enc[0], rows[i].dec[s] / rows[i].dec[0],
                  rows[i].pb_dec[s] / rows[i].pb_dec[0]);
  }
  return 0;
}
