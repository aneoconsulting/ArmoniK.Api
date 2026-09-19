// The timing harness. R4: every ratio is formed inside ONE process on one runtime, and a
// question that can be asked as a delta between two arms in the same interleaved rounds is
// asked that way.
//
// Arms, all in this process:
//   pb          protobuf C++, non-arena, deterministic serialisation (needed for R2)
//   pb-arena    the same on a google::protobuf::Arena
//   native      the generated codec emitted into C++: the no-boundary control (R3)
//   ffi         the same codec through the C ABI
//   ffi-zeroed  ABI v1 open decision 9's candidate element fill
//   ffi-nobat   the host declines to batch: one element per call
//   ffi-hosttc  the transcoder lives in the HOST, so every string costs a reverse crossing
//   groupfill   the by-value group's host-side fill ALONE, no codec at all
//
// The last three and `groupfill` are ABI v1 open decision 1: the price of the group, of
// the string-as-data form and of the batching predicate, in C++, measured rather than
// argued.
#include <algorithm>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

#include "harness.h"

#if defined(__GNUC__)
#define AK_BARRIER(x) asm volatile("" : : "r,m"(x) : "memory")
#else
#define AK_BARRIER(x) (void)(x)
#endif

static double now_ns() {
  struct timespec ts;
  clock_gettime(CLOCK_MONOTONIC, &ts);
  return (double)ts.tv_sec * 1e9 + (double)ts.tv_nsec;
}

static int g_rounds = 5;

struct Row {
  std::string payload;
  std::string dir;
  std::string arm;
  std::vector<double> ns;   // one per round
};

static std::vector<Row> g_rows;

static Row *row(const char *p, const char *d, const char *a) {
  for (size_t i = 0; i < g_rows.size(); ++i)
    if (g_rows[i].payload == p && g_rows[i].dir == d && g_rows[i].arm == a)
      return &g_rows[i];
  Row r;
  r.payload = p;
  r.dir = d;
  r.arm = a;
  g_rows.push_back(r);
  return &g_rows.back();
}

// Calibrate so every arm runs for roughly the same wall time per round.
template <class Fn>
static double timed(Fn f, int iters) {
  double t0 = now_ns();
  for (int i = 0; i < iters; ++i) f();
  return (now_ns() - t0) / iters;
}

template <class Fn>
static int calibrate(Fn f) {
  double one = timed(f, 1);
  if (one <= 0) one = 1;
  int n = (int)(20e6 / one);          // ~20 ms
  if (n < 3) n = 3;
  if (n > 2000000) n = 2000000;
  return n;
}

// ---------------------------------------------------------------- one payload

template <class F, class P>
static void run_case(const char *id, F (*mk)(void), void (*pbmk)(P *),
                     intptr_t (*ffi_enc)(ak_enc_ctx *, const F &, const shapes::ffi::Tcs &),
                     intptr_t (*ffi_enc_z)(ak_enc_ctx *, const F &, const shapes::ffi::Tcs &),
                     intptr_t (*ffi_enc_nb)(ak_enc_ctx *, const F &, const shapes::ffi::Tcs &),
                     int32_t (*ffi_dec)(ak_dec_ctx *, const uint8_t *, size_t, F *),
                     void (*nat_enc)(const F &, ak::Enc *),
                     int32_t (*nat_dec)(const uint8_t *, size_t, F *)) {
  F facade = mk();
  P pb;
  pbmk(&pb);
  google::protobuf::Arena arena;
  P *pba = google::protobuf::Arena::CreateMessage<P>(&arena);
  pbmk(pba);

  std::string wire;
  pb_serialize(pb, &wire, true);

  ak::Enc e(shapes::native::kSites);
  ak_enc_ctx *ctx = ak_enc_ctx_new();
  ak_dec_ctx *dctx = ak_dec_ctx_new();
  shapes::ffi::Tcs tc = shapes::ffi::tcs_core();
  shapes::ffi::Tcs th = shapes::ffi::tcs_host();
  std::string scratch;

  // --- encode
  struct { const char *name; int iters; } enc_arms[] = {
      {"pb", 0}, {"pb-arena", 0}, {"native", 0}, {"ffi", 0},
      {"ffi-zeroed", 0}, {"ffi-nobat", 0}, {"ffi-hosttc", 0}, {"groupfill", 0}};
  (void)enc_arms;

  auto f_pb = [&]() { pb_serialize(pb, &scratch, true); AK_BARRIER(scratch); };
  auto f_pba = [&]() { pb_serialize(*pba, &scratch, true); AK_BARRIER(scratch); };
  auto f_nat = [&]() { nat_enc(facade, &e); AK_BARRIER(e.buf); };
  auto f_ffi = [&]() { intptr_t r = ffi_enc(ctx, facade, tc); AK_BARRIER(r); };
  auto f_ffiz = [&]() { intptr_t r = ffi_enc_z(ctx, facade, tc); AK_BARRIER(r); };
  auto f_ffinb = [&]() { intptr_t r = ffi_enc_nb(ctx, facade, tc); AK_BARRIER(r); };
  auto f_ffih = [&]() { intptr_t r = ffi_enc(ctx, facade, th); AK_BARRIER(r); };

  int i_pb = calibrate(f_pb), i_pba = calibrate(f_pba), i_nat = calibrate(f_nat);
  int i_ffi = calibrate(f_ffi), i_ffiz = calibrate(f_ffiz), i_ffinb = calibrate(f_ffinb);
  int i_ffih = calibrate(f_ffih);

  // --- decode
  F dst;
  auto f_pbd = [&]() { P m; m.ParseFromString(wire); AK_BARRIER(m); };
  auto f_pbad = [&]() {
    google::protobuf::Arena a;
    P *m = google::protobuf::Arena::CreateMessage<P>(&a);
    m->ParseFromString(wire);
    AK_BARRIER(m);
  };
  auto f_natd = [&]() {
    F o;
    nat_dec((const uint8_t *)wire.data(), wire.size(), &o);
    AK_BARRIER(o);
  };
  auto f_ffid = [&]() {
    F o;
    ffi_dec(dctx, (const uint8_t *)wire.data(), wire.size(), &o);
    AK_BARRIER(o);
  };
  int i_pbd = calibrate(f_pbd), i_pbad = calibrate(f_pbad);
  int i_natd = calibrate(f_natd), i_ffid = calibrate(f_ffid);

  for (int r = 0; r < g_rounds; ++r) {
    row(id, "enc", "pb")->ns.push_back(timed(f_pb, i_pb));
    row(id, "enc", "pb-arena")->ns.push_back(timed(f_pba, i_pba));
    row(id, "enc", "native")->ns.push_back(timed(f_nat, i_nat));
    row(id, "enc", "ffi")->ns.push_back(timed(f_ffi, i_ffi));
    row(id, "enc", "ffi-zeroed")->ns.push_back(timed(f_ffiz, i_ffiz));
    row(id, "enc", "ffi-nobat")->ns.push_back(timed(f_ffinb, i_ffinb));
    row(id, "enc", "ffi-hosttc")->ns.push_back(timed(f_ffih, i_ffih));

    row(id, "dec", "pb")->ns.push_back(timed(f_pbd, i_pbd));
    row(id, "dec", "pb-arena")->ns.push_back(timed(f_pbad, i_pbad));
    row(id, "dec", "native")->ns.push_back(timed(f_natd, i_natd));
    row(id, "dec", "ffi")->ns.push_back(timed(f_ffid, i_ffid));
  }
  ak_enc_ctx_free(ctx);
  ak_dec_ctx_free(dctx);
  std::printf("  %-5s %8zu B  done\n", id, wire.size());
}

// ---------------------------------------------------------------- group fill alone

// ABI v1 open decision 1, the group half: what the by-value group costs the HOST, on its
// own, in ns per element -- not as a subtraction between two whole encodes. The chunk
// buffer is the one the loop callback would use and the result is barriered, so the fill
// cannot be dead-coded away.
template <class Elem, class Group>
static void group_fill(const char *id, const std::vector<Elem> &src,
                       Group (*make)(const Elem &, const shapes::ffi::Tcs &)) {
  shapes::ffi::Tcs tc = shapes::ffi::tcs_core();
  const size_t kChunk = ak::arena_n(sizeof(Group));
  std::vector<Group> chunk(kChunk);
  auto f = [&]() {
    size_t i = 0;
    for (size_t k = 0; k < src.size(); ++k) {
      chunk[i] = make(src[k], tc);
      if (++i == kChunk) i = 0;
    }
    AK_BARRIER(chunk[0]);
  };
  int iters = calibrate(f);
  for (int r = 0; r < g_rounds; ++r) row(id, "enc", "groupfill")->ns.push_back(timed(f, iters));
}

// ---------------------------------------------------------------- the crossing itself

static uint64_t rev_cb(uint64_t x) { return x ^ 1; }

static void price_the_boundary() {
  uint64_t x = 1;
  auto f_fwd = [&]() { x = ak_noop(x); AK_BARRIER(x); };
  auto f_rev = [&]() { x = ak_noop_reverse(rev_cb, x); AK_BARRIER(x); };
  const int kInner = 1000;
  auto g_fwd = [&]() { for (int i = 0; i < kInner; ++i) f_fwd(); };
  auto g_rev = [&]() { for (int i = 0; i < kInner; ++i) f_rev(); };
  int n = calibrate(g_fwd);
  std::printf("\n-- the crossing itself, same process and build, linkage=%s --\n", AK_LINKAGE);
  for (int r = 0; r < g_rounds; ++r) {
    double a = timed(g_fwd, n) / kInner;
    double b = timed(g_rev, n) / kInner;
    std::printf("  forward %.3f ns   forward+reverse %.3f ns   reverse alone %.3f ns\n",
                a, b, b - a);
  }
}

// ---------------------------------------------------------------- report

static double med(std::vector<double> v) {
  std::sort(v.begin(), v.end());
  return v[v.size() / 2];
}

static void report() {
  std::printf("\n%-6s %-4s %-11s %10s %10s %8s %8s\n", "payload", "dir", "arm",
              "ns/op med", "ns/op min", "r/pb lo", "r/pb hi");
  for (size_t i = 0; i < g_rows.size(); ++i) {
    Row &r = g_rows[i];
    Row *base = row(r.payload.c_str(), r.dir.c_str(), "pb");
    double lo = 1e18, hi = -1e18;
    for (size_t k = 0; k < r.ns.size() && k < base->ns.size(); ++k) {
      double q = r.ns[k] / base->ns[k];
      if (q < lo) lo = q;
      if (q > hi) hi = q;
    }
    std::printf("%-6s %-4s %-11s %10.1f %10.1f %8.3f %8.3f\n", r.payload.c_str(),
                r.dir.c_str(), r.arm.c_str(), med(r.ns),
                *std::min_element(r.ns.begin(), r.ns.end()), lo, hi);
  }
}

int main(int argc, char **argv) {
  if (argc > 1) g_rounds = atoi(argv[1]);
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
              "lossy",
#else
              "reject",
#endif
              AK_LINKAGE, g_rounds);
  std::printf("protobuf %d, deterministic serialisation ON (R2 needs it)\n",
              GOOGLE_PROTOBUF_VERSION);

  price_the_boundary();

  std::printf("\n-- arms --\n");
#define X(id, Root, sroot, pfx, sha, nbytes)                                        \
  run_case<shapes::Root, ns::Root>(                                                 \
      id, &shapes::build::payload_##pfx, &pbbuild::payload_##pfx,                   \
      &shapes::ffi::encode_into_##sroot, &shapes::ffi::encode_into_##sroot##_zeroed,\
      &shapes::ffi::encode_into_##sroot##_nobatch, &shapes::ffi::decode_with_##sroot,\
      &shapes::native::encode_into_##sroot, &shapes::native::decode_##sroot);
  AK_CASES(X)
#undef X

  // The group fill alone, on the two element shapes that matter.
  {
    shapes::ListResultsResponse a = shapes::build::payload_p1_2();
    group_fill("P1.2", a.results, &shapes::ffi::make_result_raw);
    shapes::ListResultsResponse b = shapes::build::payload_p1_3();
    group_fill("P1.3", b.results, &shapes::ffi::make_result_raw);
    shapes::ListTasksDetailedResponse c = shapes::build::payload_p2_2();
    group_fill("P2.2", c.tasks, &shapes::ffi::make_task_detailed);
    shapes::ListTasksDetailedResponse d = shapes::build::payload_p2_5();
    group_fill("P2.5", d.tasks, &shapes::ffi::make_task_detailed);
  }

  // The incumbent's deterministic-serialisation cost, on the only payload family with a
  // map. Reported beside the headline rather than hidden inside it.
  {
    shapes::ListTasksDetailedResponse f = shapes::build::payload_p2_2();
    (void)f;
    ns::ListTasksDetailedResponse pb;
    pbbuild::payload_p2_2(&pb);
    std::string s;
    auto det = [&]() { pb_serialize(pb, &s, true); AK_BARRIER(s); };
    auto nod = [&]() { pb_serialize(pb, &s, false); AK_BARRIER(s); };
    int n = calibrate(det);
    std::printf("\n-- protobuf C++ deterministic serialisation, P2.2 (2,000 map entries) --\n");
    for (int r = 0; r < g_rounds; ++r)
      std::printf("  deterministic %.0f ns   non-deterministic %.0f ns   ratio %.3f\n",
                  timed(det, n), timed(nod, n), timed(det, n) / timed(nod, n));
  }

  report();
  return 0;
}
