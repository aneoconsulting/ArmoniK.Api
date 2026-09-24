// ABI v1 obligation 12.5: the concurrency suite.
//
// 12.5 is "the obligation with the most evidence behind it and the least existence": the
// rust slice found a shared-mutable-client defect BY ACCIDENT, because stage 4 happened to
// ask for 8 calls in flight, and no slice looks for that class on purpose. One thread in
// every codec arm was in this slice's "what is not measured" until this file.
//
// What 12.5 asks for, and what is here:
//
//   at least two payload shapes    four: P1.1 and P1.2 (M1, 858 B and 218 KB), P2.1 and
//                                  P2.2 (M2, 1 KB and 540 KB). Two message types and, more
//                                  to the point, two SIZES per type -- the same
//                                  length-prefix sites reached with different widths, which
//                                  is the surface the learned-width table lives on.
//   threads in sequence AND
//   together                       both, as T2 and T3, with the same work function, so the
//                                  difference between them is only the overlap.
//   every encode asserted
//   against a reference            memcmp against the canonical bytes, which are themselves
//                                  anchored to `manifest.json`'s sha256 once at startup.
//                                  Nothing here counts anything and calls that a check.
//
// And the thing 12.5's last sentence is really about -- "a suite with one shape reports
// zero wrong bytes with a per-thread-state defect present and absent alike" -- is T6, run
// against the PLANTED builds of the two designs ABI v1 section 6 refused, in ak::Enc AND in
// the linked core. See `ak/rt.h` and CMakeLists.txt.
//
// Scope. Each planted build plants BOTH encoders the same way: this slice's own `ak::Enc`
// by define (`ak/rt.h`), and the shared core by LINKING its test-only feature build
// (`pad-widths`, `global-widths`; see CMakeLists.txt and ak-rt's manifest). Until R-D7 the
// planted builds linked the UNPLANTED core, so the ffi arm read 0 in every one of them and
// the positive control never showed that the ffi arm can fail. `conc_a17_corepad` plants
// the core alone, so the ffi arm is seen failing with the native arm at 0.
#ifndef AK_CONC_CORE_PLANT
#define AK_CONC_CORE_PLANT ""
#endif
#include <atomic>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <chrono>
#include <thread>
#include <vector>

#include "harness.h"

// ---------------------------------------------------------------- shapes, type-erased

// One payload, with its root type erased behind four function pointers. The facade object
// is built once on the main thread and then shared CONST by every thread: sharing it is
// deliberate, because a suite where each thread owns its own input cannot see a defect
// that depends on two threads reading one graph.
struct Shape {
  const char *id;
  std::string ref;  // the canonical bytes
  std::vector<uint8_t> widths;  // what this payload leaves the learned table at
  std::vector<uint8_t> need;    // the width each site it WRITES actually needs
  std::vector<bool> writes;     // which sites it visits at all
  void (*enc_native)(ak::Enc *);
  intptr_t (*enc_ffi)(ak_enc_ctx *, const shapes::ffi::Tcs &);
  // A DECODE check: the core decodes the reference and the value is compared with the
  // built object. It used to re-encode the decoded value with `ak::Enc` and memcmp, which
  // made it a second observation of the NATIVE ENCODER (R-D7): every wrong native encode
  // was counted twice, once here, and "44 wrong" was 22 distinct wrong encodes.
  bool (*roundtrip)(ak_dec_ctx *, const uint8_t *, std::size_t);
  // Whether the decoder ACCEPTED the input, whatever it produced: T4's poisoned threads
  // need this and not `roundtrip`, which a truncated-but-accepted decode would also fail.
  bool (*accepts)(ak_dec_ctx *, const uint8_t *, std::size_t);
};

template <class F, class P, F (*MK)(void), void (*PBMK)(P *), void (*NAT)(const F &, ak::Enc *),
          intptr_t (*FFI)(ak_enc_ctx *, const F &, const shapes::ffi::Tcs &),
          int32_t (*DEC)(ak_dec_ctx *, const uint8_t *, std::size_t, F *)>
struct Case {
  static const F &obj() {
    static F o = MK();
    return o;
  }
  // The reference comes from PROTOBUF, not from re-encoding with the codec under test.
  //
  // The first version built it with `ak::Enc`, which is where the plants live -- so on the
  // AK_CONC_PAD+AK_CONC_GLOBAL build the reference itself was wrong and every arm was being
  // compared against a corrupted oracle. The sha anchor caught it, which is what an anchor
  // is for, but the right fix is an oracle no plant can reach. protobuf C++ is a completely
  // independent encoder and it is already the incumbent every other gate in this slice uses.
  static void reference(std::string *out) {
    P pb;
    PBMK(&pb);
    pb_serialize_det(pb, out);
  }
  static void enc_native(ak::Enc *e) { NAT(obj(), e); }
  static intptr_t enc_ffi(ak_enc_ctx *c, const shapes::ffi::Tcs &t) { return FFI(c, obj(), t); }
  static bool roundtrip(ak_dec_ctx *d, const uint8_t *p, std::size_t n) {
    F back;
    if (DEC(d, p, n, &back) < 0) return false;
    return back == obj();
  }
  static bool accepts(ak_dec_ctx *d, const uint8_t *p, std::size_t n) {
    F back;
    return DEC(d, p, n, &back) >= 0;
  }
};

#define SHAPE(Root, sroot, pfx)                                                  \
  Case<shapes::Root, ns::Root, &shapes::build::payload_##pfx,                    \
       &pbbuild::payload_##pfx, &shapes::native::encode_into_##sroot,            \
       &shapes::ffi::encode_into_##sroot, &shapes::ffi::decode_with_##sroot>

typedef SHAPE(ListResultsResponse, list_results_response, p1_1) C11;
typedef SHAPE(ListResultsResponse, list_results_response, p1_2) C12;
typedef SHAPE(ListResultsResponse, list_results_response, p1_3) C13;
typedef SHAPE(ListTasksDetailedResponse, list_tasks_detailed_response, p2_2) C22;

static std::vector<Shape> g_shapes;
// T5 used to take the worst pair T0 found. That coupled a test to a diagnostic, and the
// diagnostic turned out to be measurable only on the SHIPPED build -- AK_CONC_PAD removes
// the very `resize_prefix` call the probe reads. T5 now runs every ordered pair, which
// costs a few hundred milliseconds and cannot pick the wrong one.
static int g_fail = 0;
static int g_checks = 0;

static void check(bool ok, const std::string &what) {
  ++g_checks;
  if (!ok) {
    ++g_fail;
    std::printf("  FAIL  %s\n", what.c_str());
  }
}

template <class C>
static void add(const char *id, const char *sha, std::size_t nbytes) {
  Shape s;
  s.id = id;
  s.enc_native = &C::enc_native;
  s.enc_ffi = &C::enc_ffi;
  s.roundtrip = &C::roundtrip;
  s.accepts = &C::accepts;
  // The reference is the canonical encoding, and it is ANCHORED: memcmp is what the
  // threads do because sha256 of 4 MB per assertion would make the suite a hash benchmark,
  // but the buffer memcmp runs against is checked against manifest.json's sha right here,
  // once, so a wrong reference cannot make the suite pass.
  C::reference(&s.ref);
  // What this payload leaves the learned table at, for T0 only. Encoded on an encoder of
  // its own so one payload's table cannot reach another's -- except under AK_CONC_GLOBAL,
  // where by construction it can, which T0 says rather than hides.
  {
    ak::Enc e(shapes::native::kSites);
    C::enc_native(&e);
    s.widths = e.widths();
  }
  {
    // Which sites this payload writes, and what each one needs. A site left at width 1 and
    // a site never visited are indistinguishable from the table alone, and conflating them
    // is how T5 first picked a pair in which nothing could go wrong: `after P2.2, P1.1`
    // looked like two over-reserved sites and they were both sites P1.1 never touches.
    const uint8_t kPrime = 9;  // no length prefix in this schema needs nine bytes
    ak::Enc e(shapes::native::kSites);
    e.prime_widths(kPrime);
    C::enc_native(&e);
    s.need = e.widths();
    s.writes.assign(s.need.size(), false);
    for (std::size_t i = 0; i < s.need.size(); ++i) s.writes[i] = (s.need[i] != kPrime);
  }
  check(sha_of(s.ref) == std::string(sha),
        std::string(id) + " reference bytes match manifest.json");
  check(s.ref.size() == nbytes, std::string(id) + " reference length matches the manifest");
  g_shapes.push_back(s);
}

// ---------------------------------------------------------------- the worker

// Per-arm, per-test wrong-byte counters. Atomic because several threads write them, and
// the whole suite is worthless if its own bookkeeping races.
struct Counts {
  std::atomic<long> encodes;
  std::atomic<long> wrong_native;
  std::atomic<long> wrong_ffi;
  std::atomic<long> wrong_hosttc;
  std::atomic<long> wrong_roundtrip;   // wrong DECODES, not encodes
  std::atomic<long> spurious_err;
  Counts() { reset(); }
  void reset() {
    encodes = 0; wrong_native = 0; wrong_ffi = 0; wrong_hosttc = 0;
    wrong_roundtrip = 0; spurious_err = 0;
  }
  // Every counter is a DISTINCT operation: one native encode, one encode through the core
  // with its transcoder, one with the host's, one decode. Nothing is observed twice.
  long wrong_encodes() const { return wrong_native + wrong_ffi + wrong_hosttc; }
  long wrong() const { return wrong_encodes() + wrong_roundtrip; }
};

// Whole-run totals of distinct wrong operations, per encoder, so main() can say which
// encoder the plant reached -- and gen/concurrency.sh can require the core's to be > 0.
static std::atomic<long> g_tot_native(0), g_tot_core(0), g_tot_dec(0);
static void tally(const Counts &c) {
  g_tot_native += c.wrong_native;
  g_tot_core += c.wrong_ffi + c.wrong_hosttc;
  g_tot_dec += c.wrong_roundtrip;
}

// `first` and `count` select a window of the shape table, so the SAME worker runs the
// one-shape and the two-shape cases and the difference between them is only the window.
static void worker(int tid, int rounds, std::size_t first, std::size_t count, Counts *c) {
  ak::Enc e(shapes::native::kSites);
  ak_enc_ctx *ectx = ak_enc_ctx_new();
  ak_dec_ctx *dctx = ak_dec_ctx_new();
  shapes::ffi::Tcs core = shapes::ffi::tcs_core();
  shapes::ffi::Tcs host = shapes::ffi::tcs_host();

  for (int r = 0; r < rounds; ++r) {
    for (std::size_t k = 0; k < count; ++k) {
      // Each thread walks the window from a different offset, so no two threads have the
      // same encode HISTORY. A suite where every thread does the same thing in the same
      // order cannot see a defect in state that survives a reset, which the learned-width
      // table deliberately does.
      const Shape &s = g_shapes[first + ((std::size_t)tid + k + (std::size_t)r) % count];
      ++c->encodes;

      e.reset();
      s.enc_native(&e);
      if (e.size() != s.ref.size() || std::memcmp(e.data(), s.ref.data(), e.size()) != 0)
        ++c->wrong_native;

      {
        intptr_t rc = s.enc_ffi(ectx, core);
        const uint8_t *p = NULL;
        std::size_t n = 0;
        ak_enc_take(ectx, &p, &n);
        if (rc < 0 || n != s.ref.size() || std::memcmp(p, s.ref.data(), n) != 0)
          ++c->wrong_ffi;
      }
      {
        intptr_t rc = s.enc_ffi(ectx, host);
        const uint8_t *p = NULL;
        std::size_t n = 0;
        ak_enc_take(ectx, &p, &n);
        if (rc < 0 || n != s.ref.size() || std::memcmp(p, s.ref.data(), n) != 0)
          ++c->wrong_hosttc;
      }
      if (!s.roundtrip(dctx, (const uint8_t *)s.ref.data(), s.ref.size()))
        ++c->wrong_roundtrip;
      if (ak_enc_err(ectx) != 0 || ak_dec_err(dctx) != 0) ++c->spurious_err;
    }
  }
  ak_enc_ctx_free(ectx);
  ak_dec_ctx_free(dctx);
}

static void report(const char *what, const Counts &c) {
  std::printf("  %-26s %5ld iterations; distinct wrong encodes %ld (native %ld, ffi %ld,"
              " ffi-hosttc %ld); wrong decodes %ld; spurious-err %ld\n",
              what, (long)c.encodes, c.wrong_encodes(), (long)c.wrong_native,
              (long)c.wrong_ffi, (long)c.wrong_hosttc, (long)c.wrong_roundtrip,
              (long)c.spurious_err);
}

// ---------------------------------------------------------------- T1: history

// The test a one-shape suite cannot run at all, and the one that catches AK_CONC_PAD.
//
// The learned length-prefix width lives in the context and DELIBERATELY survives a reset
// (ABI v1 section 6) -- that is what makes it learned. So the encoder carries state from
// one message to the next, and the design's claim is that the state changes its SPEED and
// never its BYTES. With one shape that claim is untestable: there is no other history to
// have. With two sizes of one message type it is one memcmp.
// ---------------------------------------------------------------- T0: the surface

// Which pairs of payloads can exercise the history at all, asked of the encoder rather
// than assumed.
//
// This exists because the first version of this suite got its shapes wrong and passed on
// all three PLANTED builds. Two payloads of one message type do not necessarily learn
// different widths -- P1.1 (858 B) and P1.2 (218 KB) learn the SAME table, because the
// length prefixes that vary are per-element and the elements are the same size in both;
// only the element COUNT differs, and the top-level message carries no prefix. And two
// payloads of different message types touch disjoint sites, so one can never over-reserve
// for the other. The pair that works is P1.1 and P1.3: one message type, one of them the
// absent-path payload, so an element body that needs two bytes in P1.1 needs one in P1.3.
//
// Widths only ever GROW within a context (resize_prefix assigns the width it needed), so
// the direction matters: only "A then B, where A left a site wider than B needs" can show
// anything. The table below is printed whether or not it is empty, because an empty one
// would mean T1, T5 and T6 are testing nothing -- which is the state this suite was in
// until it was asked.
static void t0_surface() {
  std::printf("\n-- T0: does this shape set have a history surface at all? --\n");
  for (std::size_t i = 0; i < g_shapes.size(); ++i) {
    int w = 0;
    for (std::size_t k = 0; k < g_shapes[i].writes.size(); ++k) if (g_shapes[i].writes[k]) ++w;
    std::printf("   %s writes %d of %d length-prefix sites\n", g_shapes[i].id, w,
                (int)g_shapes[i].writes.size());
  }
  int total = 0;
  for (std::size_t i = 0; i < g_shapes.size(); ++i) {
    for (std::size_t j = 0; j < g_shapes.size(); ++j) {
      if (i == j) continue;
      int over = 0;
      for (std::size_t k = 0; k < g_shapes[i].widths.size(); ++k)
        if (g_shapes[j].writes[k] && g_shapes[i].widths[k] > g_shapes[j].need[k]) ++over;
      if (over) {
        std::printf("   after %s, %s finds %d site(s) reserved wider than it needs\n",
                    g_shapes[i].id, g_shapes[j].id, over);
        total += over;
      }
    }
  }
#if AK_CONC_GLOBAL
  std::printf("   (AK_CONC_GLOBAL: there is no per-context table to read, so an empty\n"
              "    surface here is the plant and not a bad shape set.)\n");
#else
#if AK_CONC_PAD
  std::printf("   (AK_CONC_PAD: this probe reads `resize_prefix`, which the plant removes on\n"
              "    the narrowing side, so it under-reports here. The surface is the shipped\n"
              "    build's, above.)\n");
#else
  check(total > 0, "the shape set has a non-empty history surface");
  if (total == 0)
    std::printf("   NONE. T1, T5 and T6 below cannot distinguish anything; fix the shapes.\n");
#endif
#endif
}

static void t1_history() {
  std::printf("\n-- T1: the encoder's output does not depend on its own history --\n");
  std::printf("   Sequential, one thread. The learned width survives a reset by design, so\n");
  std::printf("   this asks whether what survives can be seen on the wire. Needs two shapes.\n\n");
  for (std::size_t i = 0; i < g_shapes.size(); ++i) {
    for (std::size_t j = 0; j < g_shapes.size(); ++j) {
      if (i == j) continue;
      const Shape &prior = g_shapes[j];
      const Shape &s = g_shapes[i];
      {
        ak::Enc e(shapes::native::kSites);
        e.reset();
        prior.enc_native(&e);
        e.reset();
        s.enc_native(&e);
        bool ok = e.size() == s.ref.size() &&
                  std::memcmp(e.data(), s.ref.data(), e.size()) == 0;
        if (!ok) ++g_tot_native;
        check(ok, std::string("native: ") + s.id + " after " + prior.id +
                      " is byte-identical to " + s.id + " on a virgin encoder");
      }
      {
        ak_enc_ctx *ctx = ak_enc_ctx_new();
        shapes::ffi::Tcs tc = shapes::ffi::tcs_core();
        const uint8_t *p = NULL;
        std::size_t n = 0;
        prior.enc_ffi(ctx, tc);
        ak_enc_take(ctx, &p, &n);
        s.enc_ffi(ctx, tc);
        ak_enc_take(ctx, &p, &n);
        bool ok = n == s.ref.size() && std::memcmp(p, s.ref.data(), n) == 0;
        if (!ok) ++g_tot_core;
        check(ok, std::string("ffi:    ") + s.id + " after " + prior.id +
                      " is byte-identical to " + s.id + " on a virgin context");
        ak_enc_ctx_free(ctx);
      }
    }
  }
  std::printf("   %d pairs checked on two arms.\n",
              (int)(g_shapes.size() * (g_shapes.size() - 1)));
}

// ---------------------------------------------------------------- T4: the error path

// ABI v1 section 5: "There is no thread-local `ak_last_error()`; the context is the place."
// A thread-local would be a hidden global with a re-entrancy hazard, and the failure mode
// is one thread reading another thread's error -- which shows up as a decode that reports
// failure while producing correct output, or the reverse.
static void t4_error_isolation(int threads, int rounds) {
  std::printf("\n-- T4: a failure on one thread is not visible on another --\n");
  std::printf("   Half the threads feed truncated input and must fail; half feed the\n");
  std::printf("   reference and must succeed with err == 0. Concurrently, %d rounds.\n\n",
              rounds);
  std::atomic<long> bad_ok(0), good_failed(0), leaked(0);
  std::vector<std::thread> ts;
  for (int t = 0; t < threads; ++t) {
    bool poison = (t % 2) == 1;
    ts.push_back(std::thread([t, poison, rounds, &bad_ok, &good_failed, &leaked]() {
      ak_dec_ctx *dctx = ak_dec_ctx_new();
      for (int r = 0; r < rounds; ++r) {
        const Shape &s = g_shapes[(std::size_t)(t + r) % g_shapes.size()];
        if (poison) {
          // Truncated in the middle of a length-delimited body: malformed, not empty.
          std::size_t n = s.ref.size() / 2 + 1;
          bool ok = s.accepts(dctx, (const uint8_t *)s.ref.data(), n);
          if (ok && ak_dec_err(dctx) == 0) ++bad_ok;
          ak_dec_err_reset(dctx);
        } else {
          bool ok = s.roundtrip(dctx, (const uint8_t *)s.ref.data(), s.ref.size());
          if (!ok) ++good_failed;
          if (ak_dec_err(dctx) != 0) ++leaked;
        }
      }
      ak_dec_ctx_free(dctx);
    }));
  }
  for (std::size_t i = 0; i < ts.size(); ++i) ts[i].join();
  check(bad_ok == 0, "every truncated decode failed");
  check(good_failed == 0, "every good decode produced the built value");
  check(leaked == 0, "no good context saw another thread's error");
  std::printf("   truncated-but-accepted %ld, good-but-wrong %ld, error leaked across a"
              " context %ld\n", (long)bad_ok, (long)good_failed, (long)leaked);
}

// ---------------------------------------------------------------- T5: two threads agree

// Section 6's sharpest sentence about the refused design: a padded prefix "lets two threads
// of one process emit two different legal encodings of the same message". Two threads, two
// different histories, one message: their bytes must be equal to each other AND to the
// reference. Equal-to-each-other is the half a manifest comparison alone would not catch if
// both drifted the same way.
static void t5_two_threads_one_message(int rounds) {
  std::printf("\n-- T5: two threads with different histories encode the same message --\n");
  std::printf("   Every ordered pair: thread A encodes the prior and then the target,\n");
  std::printf("   thread B encodes the target alone. Both must produce the reference AND\n");
  std::printf("   agree with each other -- agreement is the half that a manifest comparison\n");
  std::printf("   alone would miss if both drifted the same way, which is exactly what a\n");
  std::printf("   process-global table does.\n\n");
  long tot_disagree = 0, tot_wrong = 0;
  for (std::size_t pi = 0; pi < g_shapes.size(); ++pi) {
    for (std::size_t ti = 0; ti < g_shapes.size(); ++ti) {
      if (pi == ti) continue;
      const Shape &other = g_shapes[pi];
      const Shape &target = g_shapes[ti];
      long disagree = 0, wrong = 0;
      for (int r = 0; r < rounds; ++r) {
        std::string a, b;
        std::thread ta([&a, &target, &other]() {
          ak::Enc e(shapes::native::kSites);
          other.enc_native(&e);   // a history
          e.reset();
          target.enc_native(&e);
          a.assign((const char *)e.data(), e.size());
        });
        std::thread tb([&b, &target]() {
          ak::Enc e(shapes::native::kSites);
          target.enc_native(&e);  // no history
          b.assign((const char *)e.data(), e.size());
        });
        ta.join();
        tb.join();
        if (a != b) ++disagree;
        if (a != target.ref || b != target.ref) ++wrong;
      }
      if (disagree || wrong)
        std::printf("   %s then %s: disagreed %ld, wrong %ld (of %d rounds)\n",
                    other.id, target.id, disagree, wrong, rounds);
      tot_disagree += disagree;
      tot_wrong += wrong;
    }
  }
  check(tot_disagree == 0, "no ordered pair made two threads disagree");
  check(tot_wrong == 0, "no ordered pair made a thread diverge from the reference");
  std::printf("   %d ordered pairs x %d rounds: disagreements %ld, wrong against the"
              " reference %ld\n",
              (int)(g_shapes.size() * (g_shapes.size() - 1)), rounds, tot_disagree, tot_wrong);
}

// ---------------------------------------------------------------- T7: does it scale

// ABI v1 section 6's OTHER claim about the table: "a global table made two encoding
// threads slower than one". That is a timing claim and it was measured in the Java slice
// (1.32 to 2.23 times aggregate throughput at two threads, against a global table), so
// here it is measured again from C++ -- and measured as a SCALING FACTOR rather than as an
// absolute, because the shipped and planted builds are two binaries and R4's across-build
// ratio drift on this machine is 0.240. A scaling factor is a ratio taken inside one
// process, so the comparison between the two builds is a comparison of two such ratios and
// not of two absolutes.
//
// P1.1 on purpose: 858 bytes, so the encode is short and the share of it that touches the
// learned-width table is large. On a 540 KB payload the table is noise.
static void t7_scaling(int threads) {
  std::printf("\n-- T7: does encoding scale with threads? --\n");
  const Shape &s = g_shapes[0];
  const int iters = 20000;

  struct Run {
    static double go(const Shape &sh, int nthreads, int iters) {
      std::vector<std::thread> ts;
      std::chrono::steady_clock::time_point t0 = std::chrono::steady_clock::now();
      for (int t = 0; t < nthreads; ++t)
        ts.push_back(std::thread([&sh, iters]() {
          ak::Enc e(shapes::native::kSites);
          for (int i = 0; i < iters; ++i) { e.reset(); sh.enc_native(&e); }
        }));
      for (std::size_t i = 0; i < ts.size(); ++i) ts[i].join();
      double ns = (double)std::chrono::duration_cast<std::chrono::nanoseconds>(
                      std::chrono::steady_clock::now() - t0).count();
      return (double)nthreads * iters / (ns / 1e9);   // encodes per second, aggregate
    }
    // Thread t alternates two shapes, offset by t, so no two threads agree on what the
    // shared site's width should be at any moment.
    static double go_mixed(int nthreads, int iters) {
      std::vector<std::thread> ts;
      std::chrono::steady_clock::time_point t0 = std::chrono::steady_clock::now();
      for (int t = 0; t < nthreads; ++t)
        ts.push_back(std::thread([t, iters]() {
          ak::Enc e(shapes::native::kSites);
          for (int i = 0; i < iters; ++i) {
            e.reset();
            g_shapes[(std::size_t)(t + i) % 2].enc_native(&e);   // P1.1 and P1.3
          }
        }));
      for (std::size_t i = 0; i < ts.size(); ++i) ts[i].join();
      double ns = (double)std::chrono::duration_cast<std::chrono::nanoseconds>(
                      std::chrono::steady_clock::now() - t0).count();
      return (double)nthreads * iters / (ns / 1e9);
    }
  };

  double one = 0, many = 0;
  for (int rep = 0; rep < 3; ++rep) {          // best of three, each direction
    double a = Run::go(s, 1, iters);
    double b = Run::go(s, threads, iters);
    if (a > one) one = a;
    if (b > many) many = b;
  }
  std::printf("   (a) every thread on %s -- the table is written only on a miss, so after\n"
              "       a few iterations it is read-mostly and shared clean\n", s.id);
  std::printf("       1 thread   %10.0f encodes/s\n", one);
  std::printf("       %d threads  %10.0f encodes/s     scaling %5.2fx\n", threads, many,
              many / one);

  // The case section 6 is really about. When two threads encode payloads that WANT
  // DIFFERENT WIDTHS at the same site, a global table is written on every encode and the
  // line ping-pongs; a per-context table is not shared at all. Leg (a) cannot see that,
  // and measuring only leg (a) would have reported "a global table costs nothing".
  double one_m = 0, many_m = 0;
  for (int rep = 0; rep < 3; ++rep) {
    double a = Run::go_mixed(1, iters);
    double b = Run::go_mixed(threads, iters);
    if (a > one_m) one_m = a;
    if (b > many_m) many_m = b;
  }
  std::printf("   (b) threads alternate between shapes that want DIFFERENT widths at the\n"
              "       same site, so the table is written on every encode\n");
  std::printf("       1 thread   %10.0f encodes/s\n", one_m);
  std::printf("       %d threads  %10.0f encodes/s     scaling %5.2fx\n", threads, many_m,
              many_m / one_m);
  std::printf("   scaling    %6.2fx (a) and %6.2fx (b) on %d threads (%d vCPU)\n",
              many / one, many_m / one_m, threads,
              (int)std::thread::hardware_concurrency());
  std::printf("   A ratio taken inside ONE process, so it is comparable with the same\n");
  std::printf("   number from the AK_CONC_GLOBAL build without carrying R4's 0.240\n");
  std::printf("   across-build bar. Absolutes above are instrumentation, not a figure.\n");
}

// ---------------------------------------------------------------- main

int main(int argc, char **argv) {
  int threads = argc > 1 ? std::atoi(argv[1]) : 4;
  int rounds = argc > 2 ? std::atoi(argv[2]) : 6;

  std::printf("== ABI v1 obligation 12.5: the concurrency suite ==\n");
  std::printf("-std=%ld  impl=%s  guard=%s  linkage=%s  threads=%d  rounds=%d\n",
              (long)__cplusplus,
#if AK_CXX17
              "target",
#else
              "floor",
#endif
#ifdef AK_NO_GUARD
              "off",
#else
              "on",
#endif
              AK_LINKAGE, threads, rounds);
  std::printf("plants: AK_CONC_PAD=%d  AK_CONC_GLOBAL=%d%s\n", AK_CONC_PAD, AK_CONC_GLOBAL,
              (AK_CONC_PAD || AK_CONC_GLOBAL)
                  ? "   <-- in ak::Enc: a REFUSED design from ABI v1 section 6, built on purpose"
                  : "   (the shipped design)");
  std::printf("core plant: %s%s\n", AK_CONC_CORE_PLANT[0] ? AK_CONC_CORE_PLANT : "none",
              AK_CONC_CORE_PLANT[0] ? "   <-- the linked ak-core is a planted feature build"
                                    : "   (the linked ak-core is the shipped build)");
  std::printf("hardware_concurrency=%u\n", std::thread::hardware_concurrency());

  std::printf("\n-- the reference bytes, anchored to manifest.json --\n");
  // The ORDER matters and it is not arbitrary. T6 takes windows of this table, and the
  // two-shape window must be the pair that actually has a history surface -- which is P1.1
  // and P1.3, not P1.1 and P1.2. See T0.
  add<C11>("P1.1", "5df5eb5f3f51ae920f64ed07108d7158e0f559cda2be692f550767a9f39d3206", 858);
  add<C13>("P1.3", "7634c22567e88be4f9e188b98c35d846b4f89735b34c526be31dda65b7921381", 605);
  add<C12>("P1.2", "eed4817ef84c1f7b8d66693a9249fdbb4f75fec436264a922523fafe21124588", 218121);
  add<C22>("P2.2", "8396d3016807731b06575188352df01e1065eba6bdc11e43b4c2c6b450292693", 540422);
  for (std::size_t i = 0; i < g_shapes.size(); ++i)
    std::printf("   %s  %zu B\n", g_shapes[i].id, g_shapes[i].ref.size());

  t0_surface();
  t1_history();

  std::printf("\n-- T2 and T3: the same work, run in sequence and then together --\n");
  std::printf("   %d threads x %d rounds x %d shapes, every encode memcmp'd against the\n",
              threads, rounds, (int)g_shapes.size());
  std::printf("   reference. T2 joins each thread before starting the next, so the two\n");
  std::printf("   differ only by overlap: a defect that shows in both is not a concurrency\n");
  std::printf("   defect, and that distinction is why 12.5 asks for both.\n\n");
  {
    Counts seq;
    for (int t = 0; t < threads; ++t) {
      std::thread th(worker, t, rounds, 0, g_shapes.size(), &seq);
      th.join();
    }
    report("T2 threads in sequence", seq);
    tally(seq);
    check(seq.wrong() == 0, "T2: every encode in sequence matched the reference");
    check(seq.spurious_err == 0, "T2: no context reported an error");

    Counts par;
    std::vector<std::thread> ts;
    for (int t = 0; t < threads; ++t)
      ts.push_back(std::thread(worker, t, rounds, 0, g_shapes.size(), &par));
    for (std::size_t i = 0; i < ts.size(); ++i) ts[i].join();
    report("T3 threads together", par);
    tally(par);
    check(par.wrong() == 0, "T3: every encode in parallel matched the reference");
    check(par.spurious_err == 0, "T3: no context reported an error");
  }

  t4_error_isolation(threads, rounds * 4);
  t5_two_threads_one_message(rounds * 2);

  // ---- T6: 12.5's last sentence, tested rather than quoted --------------------------
  std::printf("\n-- T6: one shape against two, same threads, same rounds --\n");
  std::printf("   12.5: \"a suite with one shape reports zero wrong bytes with a\n");
  std::printf("   per-thread-state defect present and absent alike; two shapes find it in\n");
  std::printf("   twenty encodes out of twenty.\" That is a claim, so it is measured. On the\n");
  std::printf("   shipped design every row below is zero; the three planted builds are where\n");
  std::printf("   the row that matters is not.\n\n");
  {
    // The labels are built from the table, not typed. They were typed, and said
    // "P1.2 alone" and "P1.1 + P1.2" over windows that are P1.3 and P1.1 + P1.3, because
    // the table's order changed (T0) and the strings did not.
    std::size_t first[] = {0, 1, 0, 0};
    std::size_t count[] = {1, 1, 2, 4};
    for (int w = 0; w < 4; ++w) {
      std::string label = count[w] == 1 ? "one shape  (" : count[w] == 2 ? "two shapes ("
                                                                          : "four shapes (";
      for (std::size_t k = 0; k < count[w]; ++k)
        label += std::string(k ? " + " : "") + g_shapes[first[w] + k].id;
      label += ")";
      Counts c;
      std::vector<std::thread> ts;
      for (int t = 0; t < threads; ++t)
        ts.push_back(std::thread(worker, t, rounds, first[w], count[w], &c));
      for (std::size_t i = 0; i < ts.size(); ++i) ts[i].join();
      tally(c);
      std::printf("  %-36s %4ld iterations, %4ld distinct wrong encodes (native %ld, ffi %ld,"
                  " hosttc %ld), wrong decodes %ld\n",
                  label.c_str(), (long)c.encodes, c.wrong_encodes(), (long)c.wrong_native,
                  (long)c.wrong_ffi, (long)c.wrong_hosttc, (long)c.wrong_roundtrip);
    }
  }

  // T7 is a timing. In the setup phase (README 1.1) a container timing is instrumentation,
  // so the gate runs set AK_CONC_NO_T7=1 and their logs carry no figure.
  if (!std::getenv("AK_CONC_NO_T7")) t7_scaling(threads < 2 ? 2 : threads);
  else std::printf("\n-- T7 skipped (AK_CONC_NO_T7): a timing, not a gate --\n");

  // The line gen/concurrency.sh reads. Per encoder, so a plant that reaches only one of
  // them is visible as such (R-D7).
  std::printf("\nwhole run, distinct wrong operations: native-encoder %ld  core-encoder %ld"
              "  decoder %ld\n",
              (long)g_tot_native, (long)g_tot_core, (long)g_tot_dec);
  std::printf("\n%d checks, %d failures\n", g_checks, g_fail);
  if (AK_CONC_PAD || AK_CONC_GLOBAL || AK_CONC_CORE_PLANT[0]) {
    std::printf("This is a PLANTED build. A zero here would mean the suite cannot see the\n"
                "defect class it exists for, so failures above are the expected result and\n"
                "the exit code is inverted by gen/concurrency.sh, not by this binary.\n");
  }
  return g_fail ? 1 : 0;
}
