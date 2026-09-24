// The timing harness. R4: every ratio is formed inside ONE process on one runtime, and a
// question that can be asked as a delta between two arms in the same interleaved rounds is
// asked that way.
//
// Encode arms, all in this process:
//   pb           protobuf C++ `SerializeToString` -- the incumbent, and what a C++
//                consumer actually writes. Every ratio is against this.
//   pb-det       the same forced to deterministic map ordering, which byte identity needs
//                on any message with a map. Its own row; the sort is real work.
//   pb-arena     `SerializeToString` on a google::protobuf::Arena
//   memcpy       R2's floor: one copy of the finished payload into a reused buffer.
//                Nothing can encode faster than copying the answer.
//   native       the generated codec emitted into C++: the no-boundary control (R3)
//   ffi          the same codec through the C ABI, spec transcoders (no UTF-8 check)
//   ffi-valtc    the same with a VALIDATING transcoder, which is the check protobuf C++
//                does on serialize. The like-for-like row against `pb`.
//   ffi-zeroed   ABI v1 open decision 9's candidate element fill
//   ffi-nobat    the host declines to batch: one element per call
//   ffi-hosttc   the transcoder lives in the HOST: what a string-as-a-CALL form costs
//   groupfill    the by-value group's host-side fill ALONE, no codec at all
//
// Decode arms: pb, pb-arena, native, ffi.
//
// **The arm order rotates every round.** The rust slice's `findings/rust.md` records the
// same source measuring 0.778 and 0.88 depending only on which build ran first, and fixed
// it with a rotating order. The first version of this file ran a fixed order and its four
// FFI variants came out monotone decreasing in execution order on P5.2 and P5.4.
#include <algorithm>
#include <cstdio>
#include <cstring>
#include <functional>
#include <string>
#include <vector>

#include "harness.h"

// A REGISTER-ONLY barrier. The first version used a full "memory" clobber per iteration,
// which forces a spill and reload and is the likely whole of the gap between this
// harness's crossing figure (1.85 ns) and the rust harness's on the same machine and the
// same .so (1.5 ns). R13 compares those two numbers, so they have to be measured the same
// way. `AK_SINK` is used where only the value must survive; `AK_SINK_MEM` where a buffer
// must not be dead-stored.
#if defined(__GNUC__)
#define AK_SINK(x) asm volatile("" : : "r"(x))
#define AK_SINK_MEM(x) asm volatile("" : : "r,m"(x) : "memory")
#else
#define AK_SINK(x) (void)(x)
#define AK_SINK_MEM(x) (void)(x)
#endif

#ifdef AK_PERTURB
// A semantically neutral LAYOUT PERTURBATION. The rust slice's R4 technique: build twice,
// shifting code and data addresses without changing what runs, and publish the
// identical-source rows as an across-build control. This slice needs it because two of its
// conclusions -- the floor costing nothing, and the guard not being measurable -- are
// smaller than the drift that was observed between logs on code a build flag could not
// reach (`native` moved 5 to 8 percent between bench_a17_shared and bench_a17_noguard,
// and AK_NO_GUARD does not reach core_native.cpp at all).
static volatile char ak_perturb_pad[7919] = {1};
extern "C" int ak_perturb_fn_a(int x) { return x + (int)ak_perturb_pad[0]; }
extern "C" int ak_perturb_fn_b(int x) { return ak_perturb_fn_a(x) * 3; }
extern "C" int ak_perturb_fn_c(int x) { return ak_perturb_fn_b(x) - 1; }
#endif

// ---- finding 5: the crossing tax -------------------------------------------------
// A calibrated delay in front of every forward entry-point call, so the batching verdict
// can be reported as a CROSSOVER rather than as a fact about a 1.85 ns crossing.
#ifdef AK_CROSSING_TAX
static volatile uint64_t g_tax_sink = 0;
static uint32_t g_tax_iters = 0;
extern "C" void ak_crossing_tax() {
  uint64_t x = g_tax_sink;
  for (uint32_t i = 0; i < g_tax_iters; ++i) x = x * 6364136223846793005ULL + 1442695040888963407ULL;
  g_tax_sink = x;
}
#endif

static bool wanted(const char *id);

static double now_ns() {
  struct timespec ts;
  clock_gettime(CLOCK_MONOTONIC, &ts);
  return (double)ts.tv_sec * 1e9 + (double)ts.tv_nsec;
}

static int g_rounds = 9;

struct Row {
  std::string payload;
  std::string dir;
  std::string arm;
  double per_elem;          // elements this payload has, for the per-element columns
  bool per_message;         // true when "per element" would be a lie (P5.x, one element)
  std::vector<double> ns;
};

static std::vector<Row> g_rows;
static int g_borrow_fail = 0;

static double elems_of(const std::string &p) {
  if (p == "P1.1") return 4;
  if (p == "P1.2") return 1000;
  if (p == "P1.3") return 300;
  if (p == "P2.1") return 1;
  if (p == "P2.2") return 500;
  if (p == "P2.3") return 125;
  if (p == "P2.4") return 80;
  if (p == "P2.5") return 20;
  if (p == "P3.1") return 200;
  if (p == "P4.1") return 200;
  if (p == "P6.1") return 200;
  return 1;              // P5.x: ONE element. See `per_message`.
}

// P5.1 to P5.4 carry one element, so "ns per element" is "ns per message" there and the
// report says so instead of printing a column that means two different things.
static bool per_message_of(const std::string &p) { return p.compare(0, 3, "P5.") == 0; }

static Row *row(const std::string &p, const std::string &d, const std::string &a) {
  for (size_t i = 0; i < g_rows.size(); ++i)
    if (g_rows[i].payload == p && g_rows[i].dir == d && g_rows[i].arm == a) return &g_rows[i];
  Row r;
  r.payload = p;
  r.dir = d;
  r.arm = a;
  r.per_elem = elems_of(p);
  r.per_message = per_message_of(p);
  g_rows.push_back(r);
  return &g_rows.back();
}

// One measurement = the MINIMUM of five sub-batches, which is the uncontended cost on a
// shared container.
template <class Fn>
static double timed(Fn f, int iters) {
  int per = iters / 5;
  if (per < 1) per = 1;
  double best = 1e300;
  for (int b = 0; b < 5; ++b) {
    double t0 = now_ns();
    for (int i = 0; i < per; ++i) f();
    double d = (now_ns() - t0) / per;
    if (d < best) best = d;
  }
  return best;
}

template <class Fn>
static int calibrate(Fn f) {
  double one = timed(f, 1);
  if (one <= 0) one = 1;
  int n = (int)(40e6 / one);
  if (n < 5) n = 5;
  if (n > 2000000) n = 2000000;
  return n;
}

// An arm, as data, so the ORDER can be rotated per round rather than fixed in the source.
struct Arm {
  const char *name;
  std::function<void()> fn;
  int iters;
  // R-D5: what makes this arm's output correct, run ONCE before any timing. The first
  // version of this file sank every return code into AK_SINK and never looked at it, so an
  // arm whose encode refused (a validating transcoder meeting a bad string, a rejected
  // group) would have been timed as if it were fast. An arm whose gate fails is REMOVED
  // before calibration and the process exits non-zero: its row never exists.
  std::function<bool()> gate;
};

static int g_gate_fail = 0;
// AK_BENCH_GATE_ONLY=1: run every arm's gate and nothing else -- no calibration, no timing.
// What a correctness log of the bench is taken with, so it carries no container figure.
static bool g_gate_only = false;

// Runs every arm's gate, drops the ones that fail, and says which. Every arm must carry a
// gate: an arm without one is itself a gate failure, so a new arm cannot be timed by
// forgetting to write one.
static void gate_arms(const char *id, const char *dir, std::vector<Arm> &arms) {
  std::vector<Arm> kept;
  for (size_t i = 0; i < arms.size(); ++i) {
    bool ok = arms[i].gate ? arms[i].gate() : false;
    if (ok) {
      kept.push_back(arms[i]);
    } else {
      ++g_gate_fail;
      std::printf("  %-5s %s %-11s GATE FAILED%s -- NOT TIMED\n", id, dir, arms[i].name,
                  arms[i].gate ? "" : " (the arm has no gate)");
    }
  }
  std::printf("  %-5s %s gate: %zu of %zu arms pass\n", id, dir, kept.size(), arms.size());
  arms.swap(kept);
}

#ifdef AK_GATE_PLANT
// R-D5's plant: a "validating" transcoder that refuses every string. `ffi-valtc` built
// over it returns an error on every payload that carries a string, which is what the gate
// has to catch. Never in a timed binary; bench_a17_gateplant only.
extern "C" int32_t ak_gate_plant_refuse(const void *, size_t, uint8_t *, int32_t,
                                        ak_grow_fn, void *) {
  return -6;  // AK_ERR_TRANSCODE
}
#endif

static void run_round(const char *id, const char *dir, std::vector<Arm> &arms, int round) {
  size_t n = arms.size();
  for (size_t k = 0; k < n; ++k) {
    Arm &a = arms[(k + (size_t)round) % n];     // rotate the starting point every round
    row(id, dir, a.name)->ns.push_back(timed(a.fn, a.iters));
  }
}

static void calibrate_all(std::vector<Arm> &arms) {
  for (size_t i = 0; i < arms.size(); ++i) arms[i].iters = calibrate(arms[i].fn);
}

// ---------------------------------------------------------------- one payload

template <class F, class P, class B>
static void run_case(const char *id, F (*mk)(void), void (*pbmk)(P *),
                     int32_t (*bor_dec)(ak_dec_ctx *, const uint8_t *, size_t, B *),
                     intptr_t (*bor_enc)(ak_enc_ctx *, const B &, const shapes_borrow::ffi::Tcs &),
                     intptr_t (*ffi_enc)(ak_enc_ctx *, const F &, const shapes::ffi::Tcs &),
                     intptr_t (*ffi_enc_z)(ak_enc_ctx *, const F &, const shapes::ffi::Tcs &),
                     intptr_t (*ffi_enc_nb)(ak_enc_ctx *, const F &, const shapes::ffi::Tcs &),
                     int32_t (*ffi_dec)(ak_dec_ctx *, const uint8_t *, size_t, F *),
                     void (*nat_enc)(const F &, ak::Enc *),
                     int32_t (*nat_dec)(const uint8_t *, size_t, F *),
                     const char *want_sha) {
  if (!wanted(id)) return;
  F facade = mk();
  P pb;
  pbmk(&pb);
  google::protobuf::Arena arena;
  P *pba = google::protobuf::Arena::CreateMessage<P>(&arena);
  pbmk(pba);

  std::string wire;
  pb_serialize_det(pb, &wire);

  ak::Enc e(shapes::native::kSites);
  ak_enc_ctx *ctx = ak_enc_ctx_new();
  ak_dec_ctx *dctx = ak_dec_ctx_new();
  shapes::ffi::Tcs tc = shapes::ffi::tcs_core();
  shapes::ffi::Tcs tv = shapes::ffi::tcs_core_validating();
#ifdef AK_GATE_PLANT
  tv.utf8 = &ak_gate_plant_refuse;
#endif
  shapes::ffi::Tcs th = shapes::ffi::tcs_host();
  std::string s1, s2, s3, s4;
  const std::string want(want_sha);
  // One encode through the C ABI, its return code and its bytes both checked.
  auto ffi_ok = [&](intptr_t rc) {
    const uint8_t *p = NULL;
    size_t n = 0;
    int32_t trc = ak_enc_take(ctx, &p, &n);
    bool ok = rc >= 0 && trc == 0 && ak_enc_err(ctx) == 0 &&
              sha_of(std::string((const char *)p, n)) == want;
    ak_enc_reset(ctx);  // a refused gate must not leave its error on the next arm's gate
    return ok;
  };

  std::vector<Arm> enc;
  {
    Arm a;
    // The pb arms are gated against the deterministic form `wire`: equal for pb-det and
    // memcpy, a permutation of equal length for the default-order ones (conformance checks
    // the permutation; here the length is enough to catch a failed serialise).
    a.name = "pb";        a.fn = [&]() { pb_serialize_default(pb, &s1); AK_SINK_MEM(s1); };
    a.gate = [&]() { s1.clear(); pb_serialize_default(pb, &s1); return s1.size() == wire.size(); };
    enc.push_back(a);
    a.name = "pb-det";    a.fn = [&]() { pb_serialize_det(pb, &s2); AK_SINK_MEM(s2); };
    a.gate = [&]() { s2.clear(); pb_serialize_det(pb, &s2); return s2 == wire; };
    enc.push_back(a);
    a.name = "pb-arena";  a.fn = [&]() { pb_serialize_default(*pba, &s3); AK_SINK_MEM(s3); };
    a.gate = [&]() { s3.clear(); pb_serialize_default(*pba, &s3); return s3.size() == wire.size(); };
    enc.push_back(a);
    a.name = "memcpy";    a.fn = [&]() { memcpy_floor(wire, &s4); AK_SINK_MEM(s4); };
    a.gate = [&]() { s4.clear(); memcpy_floor(wire, &s4); return s4 == wire; };
    enc.push_back(a);
    // The codec arms are gated against the MANIFEST's canonical sha, as conformance is.
    a.name = "native";    a.fn = [&]() { nat_enc(facade, &e); AK_SINK_MEM(e); };
    a.gate = [&]() { nat_enc(facade, &e); return sha_of(e) == want; };
    enc.push_back(a);
    a.name = "ffi";       a.fn = [&]() { intptr_t r = ffi_enc(ctx, facade, tc); AK_SINK(r); };
    a.gate = [&]() { return ffi_ok(ffi_enc(ctx, facade, tc)); };
    enc.push_back(a);
    a.name = "ffi-valtc"; a.fn = [&]() { intptr_t r = ffi_enc(ctx, facade, tv); AK_SINK(r); };
    a.gate = [&]() { return ffi_ok(ffi_enc(ctx, facade, tv)); };
    enc.push_back(a);
    a.name = "ffi-zeroed";a.fn = [&]() { intptr_t r = ffi_enc_z(ctx, facade, tc); AK_SINK(r); };
    a.gate = [&]() { return ffi_ok(ffi_enc_z(ctx, facade, tc)); };
    enc.push_back(a);
    a.name = "ffi-nobat"; a.fn = [&]() { intptr_t r = ffi_enc_nb(ctx, facade, tc); AK_SINK(r); };
    a.gate = [&]() { return ffi_ok(ffi_enc_nb(ctx, facade, tc)); };
    enc.push_back(a);
    a.name = "ffi-hosttc";a.fn = [&]() { intptr_t r = ffi_enc(ctx, facade, th); AK_SINK(r); };
    a.gate = [&]() { return ffi_ok(ffi_enc(ctx, facade, th)); };
    enc.push_back(a);
  }
  std::vector<Arm> dec;
  {
    Arm a;
    a.name = "pb";
    a.fn = [&]() { P m; m.ParseFromString(wire); AK_SINK_MEM(m); };
    a.gate = [&]() { P m; return m.ParseFromString(wire); };
    dec.push_back(a);
    a.name = "pb-arena";
    a.fn = [&]() {
      google::protobuf::Arena ar;
      P *m = google::protobuf::Arena::CreateMessage<P>(&ar);
      m->ParseFromString(wire);
      AK_SINK_MEM(m);
    };
    a.gate = [&]() {
      google::protobuf::Arena ar;
      P *m = google::protobuf::Arena::CreateMessage<P>(&ar);
      return m->ParseFromString(wire);
    };
    dec.push_back(a);
    a.name = "native";
    a.fn = [&]() { F o; nat_dec((const uint8_t *)wire.data(), wire.size(), &o); AK_SINK_MEM(o); };
    a.gate = [&]() {
      F o;
      return nat_dec((const uint8_t *)wire.data(), wire.size(), &o) == 0 && o == facade;
    };
    dec.push_back(a);
    a.name = "ffi";
    a.fn = [&]() { F o; ffi_dec(dctx, (const uint8_t *)wire.data(), wire.size(), &o); AK_SINK_MEM(o); };
    a.gate = [&]() {
      F o;
      int32_t rc = ffi_dec(dctx, (const uint8_t *)wire.data(), wire.size(), &o);
      bool ok = rc == 0 && ak_dec_err(dctx) == 0 && o == facade;
      ak_dec_err_reset(dctx);
      return ok;
    };
    dec.push_back(a);
    // The BORROWED-facade arm. Same ABI, same entry point, same UTF-8 validation; the
    // only difference is that every string field is an `ak::StringView` over the input
    // buffer instead of an owned `std::string`. It isolates THE STRING COPY -- vectors,
    // maps and message children are still constructed -- and it is a measurement arm, not
    // a proposal: the views are valid only while the input buffer lives.
    a.name = "ffi-borrow";
    a.fn = [&]() { B o; bor_dec(dctx, (const uint8_t *)wire.data(), wire.size(), &o); AK_SINK_MEM(o); };
    // BYTE IDENTITY GATES THE ARM (R2): decode into the borrowed facade and re-encode.
    // Against the MANIFEST's canonical form, not against `wire`. The timed input is the
    // incumbent's bytes, and on P2.5 those carry two explicit empty map values that the
    // canonical form omits (design/SHAPES.md: two valid encodings). Re-encoding either
    // facade from them produces the canonical form, which is the same thing the owned
    // arm does and what `conformance` already checks. It used to run AFTER the timing;
    // it is now the arm's gate, so a failure means the row is never taken.
    a.gate = [&]() {
      B o;
      int32_t rc = bor_dec(dctx, (const uint8_t *)wire.data(), wire.size(), &o);
      ak_enc_ctx *c2 = ak_enc_ctx_new();
      shapes_borrow::ffi::Tcs bt = shapes_borrow::ffi::tcs_core();
      intptr_t n = bor_enc(c2, o, bt);
      const uint8_t *p = NULL;
      size_t len = 0;
      ak_enc_take(c2, &p, &len);
      std::string back((const char *)p, len);
      ak_enc_ctx_free(c2);
      bool ok = rc == 0 && n >= 0 && sha_of(back) == want;
      if (!ok) ++g_borrow_fail;
      ak_dec_err_reset(dctx);
      return ok;
    };
    dec.push_back(a);
  }
  gate_arms(id, "enc", enc);
  gate_arms(id, "dec", dec);
  if (g_gate_only) {
    ak_enc_ctx_free(ctx);
    ak_dec_ctx_free(dctx);
    return;
  }
  calibrate_all(enc);
  calibrate_all(dec);
  for (int r = 0; r < g_rounds; ++r) {
    run_round(id, "enc", enc, r);
    run_round(id, "dec", dec, r);
  }
  ak_enc_ctx_free(ctx);
  ak_dec_ctx_free(dctx);
  std::printf("  %-5s %8zu B  done\n", id, wire.size());
}

// ---------------------------------------------------------------- group fill alone

// ABI v1 open decision 1, the group half. Two variants, because the first version measured
// only the indirect one and came out LARGER than the total gap it is a component of:
// calling `make` through a function-pointer parameter defeats inlining and returns the
// group through `sret`, where the real loop calls `make_result_raw` directly in the same
// translation unit. The direct variant is the one to read; the indirect one is kept so the
// difference between them is visible rather than argued.
template <class Elem, class Group, Group (*Make)(const Elem &, const shapes::ffi::Tcs &)>
static void group_fill_arms(const char *id, const std::vector<Elem> &src,
                            Group (*make_ptr)(const Elem &, const shapes::ffi::Tcs &),
                            std::vector<Arm> *out, std::vector<Group> *chunk) {
  static shapes::ffi::Tcs tc = shapes::ffi::tcs_core();
  const size_t kChunk = ak::arena_n(sizeof(Group));
  chunk->resize(kChunk);
  Arm a;
  a.name = "groupfill";
  a.fn = [&src, chunk, kChunk]() {
    size_t i = 0;
    for (size_t k = 0; k < src.size(); ++k) {
      (*chunk)[i] = Make(src[k], tc);            // direct call, same TU, inlinable
      if (++i == kChunk) i = 0;
    }
    AK_SINK_MEM((*chunk)[0]);
  };
  out->push_back(a);
  a.name = "groupfill-ind";
  a.fn = [&src, chunk, kChunk, make_ptr]() {
    size_t i = 0;
    for (size_t k = 0; k < src.size(); ++k) {
      (*chunk)[i] = make_ptr(src[k], tc);        // through a pointer, as the first build did
      if (++i == kChunk) i = 0;
    }
    AK_SINK_MEM((*chunk)[0]);
  };
  out->push_back(a);
}

// ---------------------------------------------------------------- the crossing itself

static uint64_t rev_cb(uint64_t x) { return x ^ 1; }

static void price_the_boundary() {
  uint64_t x = 1;
  const int kInner = 1000;
  auto g_fwd = [&]() {
    for (int i = 0; i < kInner; ++i) { x = ak_noop(x); AK_SINK(x); }
  };
  auto g_rev = [&]() {
    for (int i = 0; i < kInner; ++i) { x = ak_noop_reverse(rev_cb, x); AK_SINK(x); }
  };
  int n = calibrate(g_fwd);
  std::printf("\n-- the crossing itself, same process and build, linkage=%s --\n", AK_LINKAGE);
  std::printf("   register-only barrier, to match the rust harness R13 compares against\n");
  for (int r = 0; r < g_rounds; ++r) {
    double a = timed(g_fwd, n) / kInner;
    double b = timed(g_rev, n) / kInner;
    std::printf("  forward %.3f ns   forward+reverse %.3f ns   reverse alone %.3f ns\n",
                a, b, b - a);
  }
  AK_SINK_MEM(x);
}

// ---------------------------------------------------------------- report

static double med(std::vector<double> v) {
  std::sort(v.begin(), v.end());
  return v[v.size() / 2];
}

// R4 as sharpened. Every row of the table is printed, with whether lo and hi SHARE A SIGN,
// so a range cannot be quoted over the subset that has the wanted sign. The first version
// of this slice quoted "1.1 to 3.9 points" over ten of fifteen rows and dropped a row whose
// sign was the opposite.
static void deltas(const char *title, const char *a, const char *b) {
  std::printf("\n-- %s: (%s) - (%s), within-round --\n", title, a, b);
  std::printf("%-6s %-4s %6s %12s %12s %12s %10s %s\n", "payload", "dir", "unit",
              "lo", "median", "hi", "% of pb", "sign");
  for (size_t i = 0; i < g_rows.size(); ++i) {
    Row &r = g_rows[i];
    if (r.arm != a) continue;
    Row *o = row(r.payload, r.dir, b);
    Row *p = row(r.payload, r.dir, "pb");
    if (o->ns.empty()) continue;
    std::vector<double> d, pct;
    for (size_t k = 0; k < r.ns.size() && k < o->ns.size(); ++k) {
      d.push_back((r.ns[k] - o->ns[k]) / r.per_elem);
      pct.push_back(100.0 * (r.ns[k] - o->ns[k]) / p->ns[k]);
    }
    std::sort(d.begin(), d.end());
    const char *sign = (d.front() > 0 && d.back() > 0) ? "+"
                       : (d.front() < 0 && d.back() < 0) ? "-" : "STRADDLES ZERO";
    std::printf("%-6s %-4s %6s %12.2f %12.2f %12.2f %10.2f %s\n", r.payload.c_str(),
                r.dir.c_str(), r.per_message ? "ns/msg" : "ns/el",
                d.front(), d[d.size() / 2], d.back(), med(pct), sign);
  }
}

static void report() {
  std::printf("\n%-6s %-4s %-12s %10s %10s %8s %8s %6s %s\n", "payload", "dir", "arm",
              "ns/op med", "ns/op min", "r/pb lo", "r/pb hi", "spr%", "per-round r/pb");
  for (size_t i = 0; i < g_rows.size(); ++i) {
    Row &r = g_rows[i];
    Row *base = row(r.payload, r.dir, "pb");
    double lo = 1e18, hi = -1e18;
    std::vector<double> q;
    for (size_t k = 0; k < r.ns.size() && k < base->ns.size(); ++k) {
      double v = r.ns[k] / base->ns[k];
      q.push_back(v);
      if (v < lo) lo = v;
      if (v > hi) hi = v;
    }
    // The DENOMINATOR's own spread, so a row whose incumbent moved 11 percent between
    // rounds is not read beside one whose incumbent moved 0.5 percent.
    double bl = *std::min_element(base->ns.begin(), base->ns.end());
    double bh = *std::max_element(base->ns.begin(), base->ns.end());
    std::printf("%-6s %-4s %-12s %10.1f %10.1f %8.3f %8.3f %6.1f ", r.payload.c_str(),
                r.dir.c_str(), r.arm.c_str(), med(r.ns),
                *std::min_element(r.ns.begin(), r.ns.end()), lo, hi,
                100.0 * (bh - bl) / bl);
    // Every per-round ratio, so an outlier ROUND is visible as an outlier and not read as
    // a legitimate bound. P1.2 decode has one round in nine about 34 percent high in every
    // log this slice has produced.
    for (size_t k = 0; k < q.size(); ++k) std::printf("%.3f%s", q[k], k + 1 < q.size() ? "," : "");
    std::printf("\n");
  }
}

// AK_BENCH_ONLY=P2.2,P1.2 restricts the run. The interleaving and the ratio are still
// formed inside one process over exactly the cases that remain, so a filtered run is as
// valid as a full one for the payloads it keeps. The rust harness has the same switch.
static std::vector<std::string> g_only;

static bool wanted(const char *id) {
  if (g_only.empty()) return true;
  for (size_t i = 0; i < g_only.size(); ++i)
    if (g_only[i] == id) return true;
  return false;
}

int main(int argc, char **argv) {
  if (argc > 1) g_rounds = atoi(argv[1]);
  if (const char *o = getenv("AK_BENCH_ONLY")) {
    std::string t(o), cur;
    for (size_t i = 0; i <= t.size(); ++i) {
      if (i == t.size() || t[i] == ',') { if (!cur.empty()) g_only.push_back(cur); cur.clear(); }
      else if (t[i] != ' ') cur += t[i];
    }
    std::printf("# filtered: AK_BENCH_ONLY=%s\n", o);
  }
  std::printf("-std=%ld  impl=%s  guard=%s  decode-policy=%s  linkage=%s  rounds=%d\n",
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
              "no-utf8-check",
#else
              "reject",
#endif
              AK_LINKAGE, g_rounds);
  std::printf("protobuf %d.  pb = SerializeToString (what a C++ consumer writes).\n",
              GOOGLE_PROTOBUF_VERSION);
  std::printf("protobuf validates UTF-8 on SERIALIZE; the spec says the core does not, so\n"
              "`ffi-valtc` is the like-for-like row and `ffi` is the specified behaviour.\n");
#ifdef NDEBUG
  std::printf("NDEBUG is defined: protobuf's GOOGLE_DCHECKs are compiled out.\n");
#else
  std::printf("WARNING: NDEBUG is NOT defined; the incumbent carries debug assertions.\n");
#endif

#ifdef AK_CROSSING_TAX
  {
    // Calibrate the tax against the clock, then report what one taxed crossing costs.
    const char *e = getenv("AK_TAX_NS");
    double want = e ? atof(e) : 0.0;
    g_tax_iters = 0;
    auto one = [&]() { for (int i = 0; i < 1000; ++i) ak_crossing_tax(); };
    double base = timed(one, calibrate(one)) / 1000.0;
    // one multiply-add chain step is ~1 cycle; step until the measured cost matches.
    while (want > 0) {
      g_tax_iters++;
      double t = timed(one, 20000) / 1000.0;
      if (t - base >= want || g_tax_iters > 4000) break;
    }
    double t = timed(one, 20000) / 1000.0;
    std::printf("CROSSING TAX: %u iterations = %.2f ns per forward entry-point call"
                " (asked for %.1f). Every forward call in the binding pays it, so the\n"
                "crossing is priced up and the batching verdict can be read as a"
                " crossover rather than as a fact about a 1.85 ns crossing.\n",
                g_tax_iters, t - base, want);
  }
#endif

  g_gate_only = getenv("AK_BENCH_GATE_ONLY") != NULL;
  if (g_gate_only) std::printf("# AK_BENCH_GATE_ONLY: gates only, nothing is timed\n");
  if (!g_gate_only) price_the_boundary();

  std::printf("\n-- arms --\n");
#define X(id, Root, sroot, pfx, sha, nbytes)                                        \
  run_case<shapes::Root, ns::Root, shapes_borrow::Root>(                            \
      id, &shapes::build::payload_##pfx, &pbbuild::payload_##pfx,                   \
      &shapes_borrow::ffi::decode_with_##sroot,                                     \
      &shapes_borrow::ffi::encode_into_##sroot,                                     \
      &shapes::ffi::encode_into_##sroot, &shapes::ffi::encode_into_##sroot##_zeroed,\
      &shapes::ffi::encode_into_##sroot##_nobatch, &shapes::ffi::decode_with_##sroot,\
      &shapes::native::encode_into_##sroot, &shapes::native::decode_##sroot, sha);
  AK_CASES(X)
#undef X
  if (g_gate_only) {
    std::printf("\ngate-only run: %d arm(s) failed their gate%s\n", g_gate_fail,
                g_gate_fail ? " and would NOT have been timed" : "");
    return g_gate_fail ? 1 : 0;
  }

  // The group fill alone, interleaved in its own rounds beside a `pb` row taken in the
  // same rounds.
  // The first version ran it after every payload had finished and then paired its round k
  // with `pb`'s round k, which paired measurements taken minutes apart.
  if (g_only.empty())
  {
    shapes::ListResultsResponse p12 = shapes::build::payload_p1_2();
    shapes::ListResultsResponse p13 = shapes::build::payload_p1_3();
    shapes::ListTasksDetailedResponse p22 = shapes::build::payload_p2_2();
    shapes::ListTasksDetailedResponse p25 = shapes::build::payload_p2_5();
    ns::ListResultsResponse q12, q13;
    ns::ListTasksDetailedResponse q22, q25;
    pbbuild::payload_p1_2(&q12);
    pbbuild::payload_p1_3(&q13);
    pbbuild::payload_p2_2(&q22);
    pbbuild::payload_p2_5(&q25);
    std::vector<struct ak_efix_ResultRaw> c12, c13;
    std::vector<struct ak_efix_TaskDetailed> c22, c25;
    std::string sink;

    struct GF {
      const char *id;
      std::vector<Arm> arms;
    } sets[4];
    sets[0].id = "P1.2";
    sets[1].id = "P1.3";
    sets[2].id = "P2.2";
    sets[3].id = "P2.5";
    group_fill_arms<shapes::ResultRaw, struct ak_efix_ResultRaw, shapes::ffi::make_result_raw>(
        "P1.2", p12.results, &shapes::ffi::make_result_raw, &sets[0].arms, &c12);
    group_fill_arms<shapes::ResultRaw, struct ak_efix_ResultRaw, shapes::ffi::make_result_raw>(
        "P1.3", p13.results, &shapes::ffi::make_result_raw, &sets[1].arms, &c13);
    group_fill_arms<shapes::TaskDetailed, struct ak_efix_TaskDetailed, shapes::ffi::make_task_detailed>(
        "P2.2", p22.tasks, &shapes::ffi::make_task_detailed, &sets[2].arms, &c22);
    group_fill_arms<shapes::TaskDetailed, struct ak_efix_TaskDetailed, shapes::ffi::make_task_detailed>(
        "P2.5", p25.tasks, &shapes::ffi::make_task_detailed, &sets[3].arms, &c25);
    google::protobuf::MessageLite *qs[4] = {&q12, &q13, &q22, &q25};
    for (int i = 0; i < 4; ++i) {
      Arm a;
      a.name = "pb";
      google::protobuf::MessageLite *m = qs[i];
      a.fn = [m, &sink]() { pb_serialize_default(*m, &sink); AK_SINK_MEM(sink); };
      sets[i].arms.push_back(a);
      calibrate_all(sets[i].arms);
    }
    for (int r = 0; r < g_rounds; ++r)
      for (int i = 0; i < 4; ++i) run_round(sets[i].id, "gfill", sets[i].arms, r);
  }

  // The incumbent's deterministic-serialisation cost, on the only payload family with a
  // map.
  // Both measurements are hoisted into locals: the first version called timed() four times
  // and printed a ratio of two FRESH measurements beside two others.
  if (g_only.empty())
  {
    ns::ListTasksDetailedResponse pb;
    pbbuild::payload_p2_2(&pb);
    std::string s;
    auto det = [&]() { pb_serialize_det(pb, &s); AK_SINK_MEM(s); };
    auto nod = [&]() { pb_serialize_default(pb, &s); AK_SINK_MEM(s); };
    int n = calibrate(det);
    std::printf("\n-- protobuf C++ deterministic serialisation, P2.2 (2,000 map entries) --\n");
    for (int r = 0; r < g_rounds; ++r) {
      double a = timed(det, n);
      double b = timed(nod, n);
      std::printf("  deterministic %.0f ns   default %.0f ns   ratio %.4f\n", a, b, a / b);
    }
  }

  // Where the C ABI's encode advantage goes: the two-pass blob write that ABI v1 section
  // 4's removal of the declared expansion bound forces.
  if (g_only.empty())
  {
    shapes::ListResultsResponse m = shapes::build::payload_p1_2();
    std::vector<const std::string *> ss;
    for (size_t i = 0; i < m.results.size(); ++i) {
      ss.push_back(&m.results[i].session_id);
      ss.push_back(&m.results[i].name);
      ss.push_back(&m.results[i].owner_task_id);
      ss.push_back(&m.results[i].result_id);
      ss.push_back(&m.results[i].created_by);
      ss.push_back(&m.results[i].opaque_id);
    }
    ak::Enc e1(4), e2(4);
    auto one_pass = [&]() {
      e1.reset();
      for (size_t i = 0; i < ss.size(); ++i) e1.blob_field(1, *ss[i]);
      AK_SINK_MEM(e1);
    };
    auto two_pass = [&]() {
      e2.reset();
      for (size_t i = 0; i < ss.size(); ++i) {
        ak::Mark mk = e2.begin(1, 0);
        e2.raw((const uint8_t *)ss[i]->data(), ss[i]->size());
        e2.end(mk);
      }
      AK_SINK_MEM(e2);
    };
    int n = calibrate(one_pass);
    std::printf("\n-- the two-pass blob write, %zu strings of P1.2, ns per string --\n",
                ss.size());
    for (int r = 0; r < g_rounds; ++r) {
      double a = timed(one_pass, n) / ss.size();
      double b = timed(two_pass, n) / ss.size();
      std::printf("  one pass (length known) %.3f   open-prefix-then-resolve %.3f"
                  "   delta %+.3f\n", a, b, b - a);
    }
  }

  // ABI v1 decision 3's decode half, priced the way the rust slice priced it: on the
  // string path ALONE, as two arms in ONE process and ONE binary, over all three content
  // sets. The first version compared two whole-payload arms in two different binaries,
  // which is a difference of two ratios across the across-build drift, ASCII only, and it
  // did not reconcile with the logs it was derived from.
  if (g_only.empty())
  {
    shapes::ListResultsResponse m = shapes::build::payload_p1_2();
    const char *setname[3] = {"ascii", "latin1", "wide"};
    ak::values::ContentSet sets[3] = {ak::values::kAscii, ak::values::kLatin1,
                                      ak::values::kWide};
    std::printf("\n-- ABI v1 decision 3, decode side: the string path ALONE, one process --\n");
    std::printf("   %d strings of P1.2 per set: the FIVE `string` fields of each element.\n"
                "   `ResultRaw.opaque_id` is a `bytes` field and the policy does not apply\n"
                "   to it -- including it was C20, and in the ASCII set its 1,000 values are\n"
                "   arbitrary bytes that the check arm rejected on the first bad byte, so\n"
                "   the row understated the cost rather than overstating it.\n"
                "   `check` validates UTF-8 and rejects; `raw` copies the bytes and does not\n"
                "   look at them -- which is what a C++ non-validating arm IS. A C++\n"
                "   std::string holds arbitrary bytes, so there is no lossy-substituting\n"
                "   path to compare against, unlike Rust's from_utf8_lossy.\n"
                "   Three validators, one process (C20): `scalar` is what every earlier\n"
                "   figure in this slice was taken against, `table` is what the codec calls\n"
                "   now, and `protobuf` is the INCUMBENT'S OWN IsStructurallyValidUTF8 --\n"
                "   the validator it runs on every `string` field it parses, which is the\n"
                "   comparison R14 asks for.\n", 5000);
    std::printf("%-8s %10s %-9s %10s %10s %9s %10s\n", "set", "bytes", "validator",
                "raw ns/str", "chk ns/str", "delta", "chk/raw");
    for (int si = 0; si < 3; ++si) {
      std::vector<std::string> ss;
      p1_2_strings(sets[si], &ss);
      size_t total = 0;
      for (size_t i = 0; i < ss.size(); ++i) total += ss[i].size();
      std::string out;
      auto raw = [&]() {
        for (size_t i = 0; i < ss.size(); ++i) {
          out.assign(ss[i].data(), ss[i].size());
          AK_SINK_MEM(out);
        }
      };
      // One lambda per validator, all in this process, so the three are separated by
      // interleaved rounds rather than by three binaries and R4's 0.240 drift bar.
      struct V { const char *name; bool (*fn)(const uint8_t *, std::size_t); };
      static const V vs[3] = {{"scalar", &ak::utf8_valid_scalar},
                              {"table", &ak::utf8_valid_table},
                              {"protobuf", &ak::utf8_valid_protobuf}};
      bool (*vf)(const uint8_t *, std::size_t) = NULL;
      auto chk = [&]() {
        for (size_t i = 0; i < ss.size(); ++i) {
          int32_t rc = ak::decode_str_with(vf, (const uint8_t *)ss[i].data(), ss[i].size(),
                                           &out);
          AK_SINK(rc);
          AK_SINK_MEM(out);
        }
      };
      int n = calibrate(raw);
      double brw = 0, best[3] = {0, 0, 0}, rlo[3] = {1e300, 1e300, 1e300},
             rhi[3] = {-1e300, -1e300, -1e300};
      for (int r = 0; r < g_rounds; ++r) {
        double a = timed(raw, n) / ss.size();
        if (r == 0 || a < brw) brw = a;
        for (int k = 0; k < 3; ++k) {
          vf = vs[k].fn;
          double b = timed(chk, n) / ss.size();
          if (b / a < rlo[k]) rlo[k] = b / a;
          if (b / a > rhi[k]) rhi[k] = b / a;
          if (r == 0 || b < best[k]) best[k] = b;
        }
      }
      for (int k = 0; k < 3; ++k)
        std::printf("%-8s %10zu %-9s %10.3f %10.3f %+9.3f %5.3f-%.3f\n", setname[si], total,
                    vs[k].name, brw, best[k], best[k] - brw, rlo[k], rhi[k]);

    }
  }

  // README 5.2's arms b and c, measured INSIDE one process.
  //
  // Two reasons the a/b/c table of three separate binaries cannot answer this. First, the
  // across-build ratio drift measured by `gen/drift.sh` is larger than the effect. Second,
  // `AK_CXX17` reaches exactly 11 sites in the whole emitted tree, all on the decode side,
  // so every encode row and most decode rows of arm b compile IDENTICAL SOURCE -- two of
  // the three cells were the same code measured twice. The two constructs the switch
  // actually selects are benchmarked here directly, as two arms in the same rounds, over
  // the real data.
  // Guarded on the LANGUAGE level rather than on AK_CXX17: the point is to run both
  // constructs side by side, which needs a compiler that has both, so this section exists
  // only in the C++17 binaries. The c++11 and c++14 binaries cannot compile the target
  // construct at all, which is the constraint being priced.
#if __cplusplus >= 201703L
  if (g_only.empty())
  {
    shapes::ListTasksDetailedResponse p22 = shapes::build::payload_p2_2();
    std::vector<std::pair<std::string, std::string> > entries;
    for (size_t i = 0; i < p22.tasks.size(); ++i)
      if (p22.tasks[i].options.has_value())
        for (std::map<std::string, std::string>::const_iterator it =
                 p22.tasks[i].options->options.begin();
             it != p22.tasks[i].options->options.end(); ++it)
          entries.push_back(*it);
    std::map<std::string, std::string> mm;
    auto floor_map = [&]() {
      mm.clear();
      for (size_t i = 0; i < entries.size(); ++i) mm[entries[i].first] = entries[i].second;
      AK_SINK_MEM(mm);
    };
    auto target_map = [&]() {
      mm.clear();
      for (size_t i = 0; i < entries.size(); ++i) {
        std::string k = entries[i].first, v = entries[i].second;
        mm.insert_or_assign(std::move(k), std::move(v));
      }
      AK_SINK_MEM(mm);
    };
    shapes::ListTasksDetailedResponse p23 = shapes::build::payload_p2_3();
    std::vector<const std::string *> strs;
    for (size_t i = 0; i < p23.tasks.size(); ++i)
      for (size_t j = 0; j < p23.tasks[i].parent_task_ids.size(); ++j)
        strs.push_back(&p23.tasks[i].parent_task_ids[j]);
    std::vector<std::string> vv;
    auto floor_app = [&]() {
      vv.clear();
      for (size_t i = 0; i < strs.size(); ++i) {
        vv.push_back(std::string());
        vv.back().assign(strs[i]->data(), strs[i]->size());
      }
      AK_SINK_MEM(vv);
    };
    auto target_app = [&]() {
      vv.clear();
      for (size_t i = 0; i < strs.size(); ++i)
        vv.emplace_back().assign(strs[i]->data(), strs[i]->size());
      AK_SINK_MEM(vv);
    };
    int n1 = calibrate(floor_map), n2 = calibrate(floor_app);
    std::printf("\n-- README 5.2 arm b, INSIDE one process: what the C++11 floor's missing\n"
                "   APIs cost. These two constructs are the WHOLE of the AK_CXX17\n"
                "   divergence (11 sites, all on the decode side). --\n");
    std::printf("%-26s %10s %12s %12s %10s\n", "construct", "items", "floor ns",
                "target ns", "t/f");
    for (int r = 0; r < g_rounds; ++r) {
      double a = timed(floor_map, n1), b = timed(target_map, n1);
      double c = timed(floor_app, n2), d = timed(target_app, n2);
      std::printf("%-26s %10zu %12.0f %12.0f %10.4f\n",
                  "map[k]=v vs insert_or_assign", entries.size(), a, b, b / a);
      std::printf("%-26s %10zu %12.0f %12.0f %10.4f\n",
                  "push_back+back vs emplace_back", strs.size(), c, d, d / c);
    }
  }
#endif

  deltas("the BORROWED-string facade: how much of decode is the COPY", "ffi-borrow", "ffi");
  deltas("the STRING-AS-DATA form (decision 1)", "ffi-hosttc", "ffi");
  deltas("the BATCHING predicate (decision 1)", "ffi-nobat", "ffi");
  deltas("decision 9's zeroed element fill", "ffi-zeroed", "ffi");
  deltas("encode-side UTF-8 validation, which protobuf does and the spec does not",
         "ffi-valtc", "ffi");
  deltas("the boundary: ffi against the no-boundary control", "ffi", "native");
  deltas("the group fill: direct call against a call through a pointer",
         "groupfill-ind", "groupfill");

  report();
  if (g_gate_fail) {
    std::printf("\nGATE: %d arm(s) failed their correctness gate and were NOT timed; the"
                " rows above omit them\n", g_gate_fail);
    return 1;
  }
  std::printf("\ngate: every timed codec arm passed its correctness gate before timing\n");
  if (g_borrow_fail) {
    std::printf("\nBORROWED FACADE: %d payloads failed byte identity -- its rows are void\n",
                g_borrow_fail);
    return 1;
  }
  std::printf("\nborrowed facade: byte identity holds on every payload\n");
  return 0;
}
