// The RPC arm of design/SHAPES.md, re-taken as a GRID, and R9 is why it measures CPU.
//
// The arm this slice published before was a pair: "the host's stack against the core's",
// which moves the codec and the transport at once and cannot say which one paid. SHAPES.md
// now makes it a grid, and this file is that grid:
//
//   | cell | codec          | transport |
//   |------|----------------|-----------|
//   |  A   | protobuf C++   | grpc++    |  the incumbent: what ArmoniK ships today (R14)
//   |  B   | protobuf C++   | the core  |  README 13's OUTCOME 2, priced directly
//   |  C   | the core       | the core  |  outcome 1: the whole proposal
//   |  D   | the core       | grpc++    |  the fourth cell, so the codec difference can be
//   |      |                |           |  taken under each transport and compared
//
// B - A is the transport difference. C - B is the codec difference. D - A is the codec
// difference under the HOST's transport, and whether it agrees with C - B is the question
// nobody has asked: if it does not, the two halves of the proposal are not additive.
//
// **And every figure this branch had for the RPC half was a BLOCKING-mode figure**, because
// `ak_call_unary` was the only delivery the core implemented. Section 9's other two are
// built now, so cells B and C come in each of the three:
//
//   blocking  `ak_call_unary`      2 forward, 0 reverse   a host thread blocks in the core
//   callback  `ak_call_unary_cb`   2 forward, 1 reverse   the core calls out
//   queue     `ak_call_unary_q`    3 forward, 0 reverse   a host thread blocks in the drain
//
// (Those are section 9's numbers. `rpccounts` counts them rather than quoting them, and
// does not get the same answer.)
//
// ABI v1 section 9 asserts the callback "suits C++ and C#" and there is nothing under that
// sentence. C++ is the host where a reverse call is cheapest -- 1.82 ns through a shared
// library, measured by this slice -- so this is where the claim is confirmed or refused.
#include "rpc_common.h"

#include <condition_variable>
#include <mutex>
#include <cmath>
#include <thread>

#include "ak_abi.h"
#include "generated/binding.h"
#include "generated/build.h"
#include "generated/pb_build.h"

using namespace akrpc;

// ---- the two codecs, as one signature ---------------------------------------------------
// Both return an element count, so a cell cannot be optimised into nothing and the sink
// proves each arm decoded the same message.
typedef long (*DecodeFn)(const uint8_t *, size_t);

// The request is an empty `Empty`: zero bytes on the wire, so the grid prices the DECODE
// side and says so rather than implying it covers both directions. A real address and not
// NULL, because the core builds a slice from (ptr, len) and a null pointer is not a valid
// empty slice even at length zero.
static const uint8_t kNoReq[1] = {0};

static long decode_pb(const uint8_t *p, size_t n) {
  svcns::ListTasksDetailedResponse m;
  if (!m.ParseFromArray(p, (int)n)) {
    std::printf("protobuf parse failed\n");
    abort();
  }
  return m.tasks_size();
}

static long decode_core(const uint8_t *p, size_t n) {
  shapes::ListTasksDetailedResponse f;
  ak_dec_ctx *d = ak_dec_ctx_new();
  shapes::ffi::decode_with_list_tasks_detailed_response(d, p, n, &f);
  ak_dec_ctx_free(d);
  return (long)f.tasks.size();
}

// The Ping arm's "codec": there is none, and that is the point. A delivery comparison on a
// 4 ms call cannot see a 0.3 ns reverse crossing whatever it does.
static long decode_none(const uint8_t *p, size_t n) { (void)p; return (long)n + 1; }

static std::atomic<long> g_sink(0);

// ---- cells B and C: the core's transport, in each of section 9's three deliveries -------
//
// One structure for all three, so the delivery is the only thing that differs. Each holds
// `inflight` calls outstanding and retires `calls` of them.
struct CoreArm {
  ak_client *cl;
  DecodeFn dec;
};

// BLOCKING. `inflight` in flight costs `inflight` HOST THREADS, and that is the mode's own
// cost rather than a harness choice: a thread inside `ak_call_unary` is inside the core
// until the response lands, so there is no other way to have eight calls outstanding.
static void run_blocking(CoreArm a, int inflight, int per, const char *path) {
  size_t plen = strlen(path);
  std::vector<std::thread> ts;
  long local = 0;
  std::vector<long> acc((size_t)inflight, 0);
  for (int t = 0; t < inflight; ++t) {
    ts.push_back(std::thread([&, t]() {
      long n = 0;
      for (int i = 0; i < per; ++i) {
        struct ak_bytes out;
        out.ptr = NULL; out.len = 0; out.owner = NULL;
        int32_t rc = ak_call_unary(a.cl, (const uint8_t *)path, plen, kNoReq, 0, &out);
        if (rc != AK_OK) { std::printf("ak_call_unary rc=%d\n", rc); abort(); }
        n += a.dec(out.ptr, out.len);
        ak_bytes_free(&out);
      }
      acc[(size_t)t] = n;
    }));
  }
  for (size_t i = 0; i < ts.size(); ++i) ts[i].join();
  for (size_t i = 0; i < acc.size(); ++i) local += acc[i];
  g_sink += local;
}

// CALLBACK. One host thread keeps a window of `inflight` open. The completion fires on a
// tokio worker -- a thread the host does not own -- and does the MINIMUM: it copies the
// completion into a slot and wakes the waiter. The decode and the release stay on the host
// thread, so this row and the queue row differ in exactly one thing, which is where the
// push happens.
struct CbState {
  std::mutex m;
  std::condition_variable cv;
  std::vector<ak_completion> done;
};

extern "C" void ak_on_complete(void *user, struct ak_completion *c) {
  CbState *s = static_cast<CbState *>(user);
  {
    std::lock_guard<std::mutex> g(s->m);
    s->done.push_back(*c);
  }
  s->cv.notify_one();
}

static void run_callback(CoreArm a, int inflight, int per, const char *path) {
  size_t plen = strlen(path);
  const int total = inflight * per;
  CbState st;
  st.done.reserve((size_t)inflight + 2);
  std::vector<ak_call *> handles((size_t)total, (ak_call *)NULL);
  int submitted = 0, completed = 0;
  long n = 0;
  while (completed < total) {
    while (submitted - completed < inflight && submitted < total) {
      ak_call *h = ak_call_unary_cb(a.cl, (const uint8_t *)path, plen, kNoReq, 0,
                                    ak_on_complete, &st, (uint64_t)submitted);
      if (!h) { std::printf("ak_call_unary_cb returned NULL\n"); abort(); }
      handles[(size_t)submitted] = h;
      ++submitted;
    }
    ak_completion c;
    {
      std::unique_lock<std::mutex> g(st.m);
      st.cv.wait(g, [&] { return !st.done.empty(); });
      c = st.done.back();
      st.done.pop_back();
    }
    if (c.status != AK_OK) { std::printf("callback status=%d\n", c.status); abort(); }
    n += a.dec(c.bytes.ptr, c.bytes.len);
    ak_bytes_free(&c.bytes);
    ak_call_destroy(handles[(size_t)c.tag]);
    ++completed;
  }
  g_sink += n;
}

// CALLBACK, N HOST THREADS, ONE CALL EACH -- the control that separates the DELIVERY from
// the THREAD COUNT.
//
// The callback row above keeps N calls outstanding from one host thread, and blocking needs
// N threads to do the same. So their difference is two things at once, and section 9's
// claim ("a callback suits C++") is about only one of them. This arm has blocking's exact
// shape -- N host threads, each with one call outstanding, each waiting for its own -- and
// takes the completion through `ak_call_unary_cb`. If it lands on the blocking row, the
// delivery is free and the difference is threads. If it lands on the callback row, the
// delivery is doing the work.
static void run_callback_n(CoreArm a, int inflight, int per, const char *path) {
  size_t plen = strlen(path);
  std::vector<std::thread> ts;
  std::vector<long> acc((size_t)inflight, 0);
  for (int t = 0; t < inflight; ++t) {
    ts.push_back(std::thread([&, t]() {
      CbState st;
      st.done.reserve(2);
      long n = 0;
      for (int i = 0; i < per; ++i) {
        ak_call *h = ak_call_unary_cb(a.cl, (const uint8_t *)path, plen, kNoReq, 0,
                                      ak_on_complete, &st, (uint64_t)i);
        if (!h) { std::printf("ak_call_unary_cb returned NULL\n"); abort(); }
        ak_completion c;
        {
          std::unique_lock<std::mutex> g(st.m);
          st.cv.wait(g, [&] { return !st.done.empty(); });
          c = st.done.back();
          st.done.pop_back();
        }
        if (c.status != AK_OK) { std::printf("callback status=%d\n", c.status); abort(); }
        n += a.dec(c.bytes.ptr, c.bytes.len);
        ak_bytes_free(&c.bytes);
        ak_call_destroy(h);
      }
      acc[(size_t)t] = n;
    }));
  }
  for (size_t i = 0; i < ts.size(); ++i) ts[i].join();
  long tot = 0;
  for (size_t i = 0; i < acc.size(); ++i) tot += acc[i];
  g_sink += tot;
}

// QUEUE. The same window, the same one host thread, and no upcall at all: the completion is
// taken out of `ak_queue_next`, which the host thread is blocked inside.
static void run_queue(CoreArm a, int inflight, int per, const char *path) {
  size_t plen = strlen(path);
  const int total = inflight * per;
  ak_queue *q = ak_queue_new();
  std::vector<ak_call *> handles((size_t)total, (ak_call *)NULL);
  int submitted = 0, completed = 0;
  long n = 0;
  while (completed < total) {
    while (submitted - completed < inflight && submitted < total) {
      ak_call *h = ak_call_unary_q(a.cl, (const uint8_t *)path, plen, kNoReq, 0, q,
                                   (uint64_t)submitted);
      if (!h) { std::printf("ak_call_unary_q returned NULL\n"); abort(); }
      handles[(size_t)submitted] = h;
      ++submitted;
    }
    ak_completion c;
    int32_t rc = ak_queue_next(q, &c, (uint64_t)-1);
    if (rc != AK_QUEUE_OK) { std::printf("ak_queue_next rc=%d\n", rc); abort(); }
    if (c.status != AK_OK) { std::printf("queue completion status=%d\n", c.status); abort(); }
    n += a.dec(c.bytes.ptr, c.bytes.len);
    ak_bytes_free(&c.bytes);
    ak_call_destroy(handles[(size_t)c.tag]);
    ++completed;
  }
  ak_queue_shutdown(q);
  ak_queue_destroy(q);
  g_sink += n;
}

// ---- cell A: the incumbent, unchanged ---------------------------------------------------
struct GrpcArm {
  std::vector<std::unique_ptr<svcns::Shapes::Stub> > *stubs;
};

static void run_grpcpp_ping(GrpcArm a, int inflight, int per) {
  std::vector<std::thread> ts;
  std::vector<long> acc((size_t)inflight, 0);
  for (int t = 0; t < inflight; ++t) {
    ts.push_back(std::thread([&, t]() {
      long n = 0;
      for (int i = 0; i < per; ++i) {
        grpc::ClientContext ctx;
        svcns::Empty q, r;
        grpc::Status s = (*a.stubs)[(size_t)t % a.stubs->size()]->Ping(&ctx, q, &r);
        if (!s.ok()) { std::printf("grpc++ ping failed: %s\n", s.error_message().c_str()); abort(); }
        n += 1;
      }
      acc[(size_t)t] = n;
    }));
  }
  for (size_t i = 0; i < ts.size(); ++i) ts[i].join();
  long tot = 0;
  for (size_t i = 0; i < acc.size(); ++i) tot += acc[i];
  g_sink += tot;
}

static void run_grpcpp(GrpcArm a, int inflight, int per) {
  std::vector<std::thread> ts;
  std::vector<long> acc((size_t)inflight, 0);
  for (int t = 0; t < inflight; ++t) {
    ts.push_back(std::thread([&, t]() {
      long n = 0;
      for (int i = 0; i < per; ++i) {
        grpc::ClientContext ctx;
        svcns::Empty q;
        svcns::ListTasksDetailedResponse r;
        grpc::Status s = (*a.stubs)[(size_t)t % a.stubs->size()]->Fetch(&ctx, q, &r);
        if (!s.ok()) { std::printf("grpc++ failed: %s\n", s.error_message().c_str()); abort(); }
        n += r.tasks_size();
      }
      acc[(size_t)t] = n;
    }));
  }
  for (size_t i = 0; i < ts.size(); ++i) ts[i].join();
  long tot = 0;
  for (size_t i = 0; i < acc.size(); ++i) tot += acc[i];
  g_sink += tot;
}

// ---- cell D: grpc++'s transport carrying opaque bytes, decoded by the core ---------------
//
// `SerializationTraits<ByteBuffer>` is grpc++'s own generic path, so the transport is the
// SAME code cell A drives and only the marshaller changes. One thing it does not get for
// free and cell A does: protobuf parses straight off the slice list, while the core's
// decoder needs a contiguous buffer. The concatenation is TIMED and reported separately
// rather than folded into the cell, because it is a property of the integration point and
// not of the codec.
static std::atomic<double> g_dump_ns(0.0);
static std::atomic<long> g_dump_calls(0);
static std::atomic<long> g_dump_multislice(0);

static void run_grpcraw(std::shared_ptr<grpc::Channel> chan, DecodeFn dec, int inflight,
                        int per) {
  std::vector<std::thread> ts;
  std::vector<long> acc((size_t)inflight, 0);
  for (int t = 0; t < inflight; ++t) {
    ts.push_back(std::thread([&, t]() {
      grpc::internal::RpcMethod method(kFetchPath, grpc::internal::RpcMethod::NORMAL_RPC);
      grpc::Slice empty;
      long n = 0;
      std::string flat;
      for (int i = 0; i < per; ++i) {
        grpc::ClientContext ctx;
        grpc::ByteBuffer req(&empty, 1);
        grpc::ByteBuffer resp;
        grpc::Status s = grpc::internal::BlockingUnaryCall<grpc::ByteBuffer, grpc::ByteBuffer>(
            chan.get(), method, &ctx, req, &resp);
        if (!s.ok()) { std::printf("grpc++ raw failed: %s\n", s.error_message().c_str()); abort(); }
        std::vector<grpc::Slice> slices;
        double d0 = thread_cpu_ns();
        grpc::Status ds = resp.Dump(&slices);
        if (!ds.ok()) { std::printf("ByteBuffer::Dump failed\n"); abort(); }
        const uint8_t *p;
        size_t len;
        if (slices.size() == 1) {
          p = slices[0].begin();
          len = slices[0].size();
        } else {
          flat.clear();
          for (size_t k = 0; k < slices.size(); ++k)
            flat.append((const char *)slices[k].begin(), slices[k].size());
          p = (const uint8_t *)flat.data();
          len = flat.size();
          g_dump_multislice.fetch_add(1, std::memory_order_relaxed);
        }
        double dd = thread_cpu_ns() - d0;
        double cur = g_dump_ns.load(std::memory_order_relaxed);
        while (!g_dump_ns.compare_exchange_weak(cur, cur + dd)) {}
        g_dump_calls.fetch_add(1, std::memory_order_relaxed);
        n += dec(p, len);
      }
      acc[(size_t)t] = n;
    }));
  }
  for (size_t i = 0; i < ts.size(); ++i) ts[i].join();
  long tot = 0;
  for (size_t i = 0; i < acc.size(); ++i) tot += acc[i];
  g_sink += tot;
}

// ---- the driver -------------------------------------------------------------------------
struct Result { double cpu_per, wall_per, server_per; };

template <class Fn>
static Result drive(int n_rpcs, Fn body) {
  double c0 = cpu_ns(), w0 = wall_ns(), s0 = server_cpu();
  body();
  Result r;
  double n = (double)n_rpcs;
  r.server_per = (server_cpu() - s0) / n;
  r.cpu_per = (cpu_ns() - c0) / n - r.server_per;
  r.wall_per = (wall_ns() - w0) / n;
  return r;
}

struct Arm {
  const char *name;
  const char *cell;
  const char *delivery;
  const char *host_threads;  // what "N in flight" costs this arm in host threads
  void (*run)(void *, int, int);
  void *ctx;
};

struct CoreCtx { CoreArm arm; int mode; const char *path; };
// mode: 0 blocking, 1 callback, 2 queue, 3 callback with N host threads

static void core_dispatch(void *v, int inflight, int per) {
  CoreCtx *c = static_cast<CoreCtx *>(v);
  if (c->mode == 0) run_blocking(c->arm, inflight, per, c->path);
  else if (c->mode == 1) run_callback(c->arm, inflight, per, c->path);
  else if (c->mode == 3) run_callback_n(c->arm, inflight, per, c->path);
  else run_queue(c->arm, inflight, per, c->path);
}
static void grpcpp_dispatch(void *v, int inflight, int per) {
  run_grpcpp(*static_cast<GrpcArm *>(v), inflight, per);
}
static void grpcpp_ping_dispatch(void *v, int inflight, int per) {
  run_grpcpp_ping(*static_cast<GrpcArm *>(v), inflight, per);
}
struct RawCtx { std::shared_ptr<grpc::Channel> chan; DecodeFn dec; };
static void grpcraw_dispatch(void *v, int inflight, int per) {
  RawCtx *c = static_cast<RawCtx *>(v);
  run_grpcraw(c->chan, c->dec, inflight, per);
}

// ---- the deliveries on a call small enough to see them ----------------------------------
//
// Section 9's "a callback suits C++ and C#" is a claim about the BOUNDARY, and a 540 KB
// response buries the boundary under four milliseconds of codec. `Ping` returns an empty
// message: no codec at all on either side, so what is left is the transport and the three
// deliveries. If the deliveries do not separate HERE they do not separate anywhere in C++,
// and that is a result rather than a shrug.
static int small_main(int calls, int rounds, bool pin) {
  Transport tr = make_transport(true, 50310);
  ShapesImpl svc;
  grpc::ServerBuilder b;
  pin_server(&b, pin);
  b.AddListeningPort(tr.grpc_target, grpc::InsecureServerCredentials());
  b.RegisterService(&svc);
  std::unique_ptr<grpc::Server> server = b.BuildAndStart();
  if (!server) { std::printf("server failed to start on %s\n", tr.grpc_target.c_str()); return 1; }
  grpc::ChannelArguments args = pinned_channel_args(pin);
  std::shared_ptr<grpc::Channel> chan =
      grpc::CreateCustomChannel(tr.grpc_target, grpc::InsecureChannelCredentials(), args);
  std::vector<std::unique_ptr<svcns::Shapes::Stub> > stubs;
  for (int i = 0; i < 16; ++i) stubs.push_back(svcns::Shapes::NewStub(chan));
  ak_runtime *rt = ak_runtime_new(2);
  ak_client_opts opts = pinned_core_opts(pin);
  ak_client *cl = ak_client_new_opts(rt, (const uint8_t *)tr.core_uri.data(),
                                     tr.core_uri.size(), &opts);
  if (!cl) { std::printf("ak_client_new_opts failed\n"); return 1; }

  GrpcArm ga; ga.stubs = &stubs;
  CoreCtx blk; blk.arm.cl = cl; blk.arm.dec = decode_none; blk.mode = 0; blk.path = kPingPath;
  CoreCtx cbk = blk; cbk.mode = 1;
  CoreCtx que = blk; que.mode = 2;
  CoreCtx cbn = blk; cbn.mode = 3;
  Arm arms[] = {
    {"A  grpc++ Ping",    "A", "blocking", "N", grpcpp_ping_dispatch, &ga},
    {"core Ping",         "C", "blocking", "N", core_dispatch,        &blk},
    {"core Ping cb",      "C", "callback", "1", core_dispatch,        &cbk},
    {"core Ping queue",   "C", "queue",    "1", core_dispatch,        &que},
    {"core Ping cbxN",    "C", "cb x N",   "N", core_dispatch,        &cbn},
  };
  const int NARMS = (int)(sizeof(arms) / sizeof(arms[0]));
  for (int i = 0; i < NARMS; ++i) arms[i].run(arms[i].ctx, 1, 20);

  std::printf("configuration: grpc++ %s, UDS, pinning %s, %d rounds, %d RPCs per round per\n"
              "arm per in-flight level. EMPTY request, EMPTY response: no codec on either\n"
              "side, so the boundary is what is left.\n\n",
              grpc::Version().c_str(), pin ? "on" : "off", rounds, calls);
  std::printf("%-18s %-9s %-7s %-9s %13s %6s %13s %10s\n", "arm", "delivery", "inflt",
              "hostthr", "CPU ns/RPC", "spr%", "wall ns/RPC", "CPU/A");
  const int kInflight[] = {1, 8, 16};
  for (int k = 0; k < 3; ++k) {
    int n = kInflight[k];
    int per = calls / n;
    if (per < 5) per = 5;
    const int total = n * per;
    std::vector<double> best((size_t)NARMS, 1e300), worst((size_t)NARMS, 0.0),
        bwall((size_t)NARMS, 0.0);
    std::vector<std::vector<double> > pr((size_t)rounds,
                                         std::vector<double>((size_t)NARMS, 0.0));
    for (int r = 0; r < rounds; ++r)
      for (int j = 0; j < NARMS; ++j) {
        int i = (j + r) % NARMS;
        Result x = drive(total, [&] { arms[i].run(arms[i].ctx, n, per); });
        pr[(size_t)r][(size_t)i] = x.cpu_per;
        if (x.cpu_per < best[(size_t)i]) { best[(size_t)i] = x.cpu_per; bwall[(size_t)i] = x.wall_per; }
        if (x.cpu_per > worst[(size_t)i]) worst[(size_t)i] = x.cpu_per;
      }
    for (int i = 0; i < NARMS; ++i)
      std::printf("%-18s %-9s %-7d %-9s %13.0f %6.1f %13.0f %10.3f\n", arms[i].name,
                  arms[i].delivery, n,
                  strcmp(arms[i].host_threads, "N") ? "1" : (n == 1 ? "1" : "N"),
                  best[(size_t)i],
                  100.0 * (worst[(size_t)i] - best[(size_t)i]) / best[(size_t)i],
                  bwall[(size_t)i], best[(size_t)i] / best[0]);
    struct D { const char *name; int p, q; };
    const D ds[] = {
      {"core - grpc++, blocking",       0, 1},
      {"cb - blocking",                 1, 2},
      {"queue - blocking",              1, 3},
      {"queue - callback",              2, 3},
      {"cb x N - blocking  (control)",  1, 4},
      {"cb x N - cb        (control)",  2, 4},
    };
    std::printf("\n   within-round deltas, %d rounds. A delta that straddles zero has NOT\n"
                "   separated.\n", rounds);
    std::printf("   %-32s %12s %12s %8s %8s %6s\n", "delta", "lo ns", "hi ns", "lo %A",
                "hi %A", "sign");
    for (int d = 0; d < (int)(sizeof(ds) / sizeof(ds[0])); ++d) {
      double lo = 1e300, hi = -1e300, plo = 1e300, phi = -1e300;
      for (int r = 0; r < rounds; ++r) {
        double v = pr[(size_t)r][(size_t)ds[d].q] - pr[(size_t)r][(size_t)ds[d].p];
        double pc = 100.0 * v / pr[(size_t)r][0];
        if (v < lo) lo = v;
        if (v > hi) hi = v;
        if (pc < plo) plo = pc;
        if (pc > phi) phi = pc;
      }
      std::printf("   %-32s %12.0f %12.0f %8.2f %8.2f %6s\n", ds[d].name, lo, hi, plo, phi,
                  (lo > 0 || hi < 0) ? "yes" : "NO");
    }
    std::printf("\n");
  }
  std::printf("sink %ld\n\n", (long)g_sink.load());
  std::printf(
      "The boundary arithmetic this arm is measured against. On this machine a forward\n"
      "crossing is 0.63 ns and a reverse one 0.30 ns, so the deliveries differ by 0.3 ns\n"
      "(callback) or 0.63 ns (queue) of BOUNDARY per call. Against the CPU column above,\n"
      "that is the last digit or beyond it. A delta that separates here is therefore NOT\n"
      "the crossing: it is the delivery's machinery -- a spawned task, a mutex and a\n"
      "condvar, a wakeup -- or the host thread count, and the `cb x N` control is what\n"
      "tells those two apart.\n");
  ak_client_destroy(cl);
  ak_runtime_destroy(rt);
  server->Shutdown();
  server->Wait();
  ::unlink(tr.sock.c_str());
  return 0;
}

int main(int argc, char **argv) {
  akrpc::init_core_or_die();
  int calls = argc > 1 ? atoi(argv[1]) : 40;
  int rounds = argc > 2 ? atoi(argv[2]) : 9;
  const char *which = argc > 3 ? argv[3] : "both";   // uds | tcp | both | small
  bool pin = argc > 4 ? (strcmp(argv[4], "nopin") != 0) : true;
  if (!strcmp(which, "small")) return small_main(calls, rounds, pin);

  std::printf("configuration: grpc++ %s, protobuf C++ %d, -O2 -g -DNDEBUG, shared linkage\n",
              grpc::Version().c_str(), GOOGLE_PROTOBUF_VERSION);
  std::printf("transport pinning: %s", pin ? "ON" : "OFF (each stack's own default)");
  if (pin)
    std::printf("  -- stream window %d, connection window %d, max message %d, BDP probe off\n",
                kStreamWindow, kConnWindow, kMaxMessage);
  else
    std::printf("\n");
  std::printf("rounds %d, %d RPCs per round per arm per in-flight level, best of rounds\n\n",
              rounds, calls);

  int transports[2];
  int ntr = 0;
  if (!strcmp(which, "uds") || !strcmp(which, "both")) transports[ntr++] = 1;
  if (!strcmp(which, "tcp") || !strcmp(which, "both")) transports[ntr++] = 0;

  for (int ti = 0; ti < ntr; ++ti) {
    Transport tr = make_transport(transports[ti] != 0, 50077 + ti);

    ShapesImpl svc;
    grpc::ServerBuilder b;
    pin_server(&b, pin);
    b.AddListeningPort(tr.grpc_target, grpc::InsecureServerCredentials());
    b.RegisterService(&svc);
    std::unique_ptr<grpc::Server> server = b.BuildAndStart();
    if (!server) { std::printf("server failed to start on %s\n", tr.grpc_target.c_str()); return 1; }

    grpc::ChannelArguments args = pinned_channel_args(pin);
    std::shared_ptr<grpc::Channel> chan =
        grpc::CreateCustomChannel(tr.grpc_target, grpc::InsecureChannelCredentials(), args);
    std::vector<std::unique_ptr<svcns::Shapes::Stub> > stubs;
    for (int i = 0; i < 16; ++i) stubs.push_back(svcns::Shapes::NewStub(chan));

    ak_runtime *rt = ak_runtime_new(2);
    ak_client_opts opts = pinned_core_opts(pin);
    ak_client *cl = ak_client_new_opts(rt, (const uint8_t *)tr.core_uri.data(),
                                       tr.core_uri.size(), &opts);
    if (!cl) { std::printf("ak_client_new_opts failed on %s\n", tr.core_uri.c_str()); return 1; }

    GrpcArm ga; ga.stubs = &stubs;
    RawCtx rawc; rawc.chan = chan; rawc.dec = decode_core;
    CoreCtx bB; bB.arm.cl = cl; bB.arm.dec = decode_pb;   bB.mode = 0; bB.path = kFetchPath;
    CoreCtx bBc = bB; bBc.mode = 1;
    CoreCtx bBq = bB; bBq.mode = 2;
    CoreCtx cC; cC.arm.cl = cl; cC.arm.dec = decode_core; cC.mode = 0; cC.path = kFetchPath;
    CoreCtx cCc = cC; cCc.mode = 1;
    CoreCtx cCq = cC; cCq.mode = 2;
    CoreCtx cCn = cC; cCn.mode = 3;

    Arm arms[] = {
      {"A  pb/grpc++",      "A", "blocking", "N",  grpcpp_dispatch,  &ga},
      {"B  pb/core",        "B", "blocking", "N",  core_dispatch,    &bB},
      {"B  pb/core cb",     "B", "callback", "1",  core_dispatch,    &bBc},
      {"B  pb/core queue",  "B", "queue",    "1",  core_dispatch,    &bBq},
      {"C  core/core",      "C", "blocking", "N",  core_dispatch,    &cC},
      {"C  core/core cb",   "C", "callback", "1",  core_dispatch,    &cCc},
      {"C  core/core q",    "C", "queue",    "1",  core_dispatch,    &cCq},
      {"C  core/core cbxN", "C", "cb x N",   "N",  core_dispatch,    &cCn},
      {"D  core/grpc++",    "D", "blocking", "N",  grpcraw_dispatch, &rawc},
    };
    const int NARMS = (int)(sizeof(arms) / sizeof(arms[0]));

    // Warm every path: connection setup, the first allocation, the learned widths, and
    // grpc++'s per-method registration. An unwarmed arm measures the connect.
    for (int i = 0; i < NARMS; ++i) arms[i].run(arms[i].ctx, 1, 4);

    std::printf("== transport: %s (%s), pinning %s ==\n", tr.label,
                tr.uds ? tr.sock.c_str() : tr.grpc_target.c_str(), pin ? "on" : "off");
    std::printf("%-18s %-4s %-9s %-7s %-9s %13s %6s %13s %10s %10s\n", "arm", "cell",
                "delivery", "inflt", "hostthr", "CPU ns/RPC", "spr%", "wall ns/RPC",
                "CPU/A", "wall/A");

    const int kInflight[] = {1, 8, 16};
    for (int k = 0; k < 3; ++k) {
      int n = kInflight[k];
      int per = calls / n;
      if (per < 5) per = 5;
      const int total = n * per;
      // Per LEVEL, so the concatenation figure printed beside D-A is this level's and not a
      // running mean that still carries the warmup.
      g_dump_ns.store(0.0); g_dump_calls.store(0); g_dump_multislice.store(0);
      std::vector<double> best((size_t)NARMS, 1e300);
      std::vector<double> worst((size_t)NARMS, 0.0);
      std::vector<double> bwall((size_t)NARMS, 0.0);
      // Per-round CPU, kept so every difference below can be a WITHIN-ROUND delta. That is
      // this slice's established discipline for decision 1 (R4): a difference between two
      // best-of-nine figures carries the drift of nine rounds, and the drift here is 5 to
      // 23 percent of the quantity being differenced.
      std::vector<std::vector<double> > per_round((size_t)rounds,
                                                  std::vector<double>((size_t)NARMS, 0.0));
      for (int r = 0; r < rounds; ++r) {
        for (int j = 0; j < NARMS; ++j) {
          int i = (j + r) % NARMS;
          Result x = drive(total, [&] { arms[i].run(arms[i].ctx, n, per); });
          per_round[(size_t)r][(size_t)i] = x.cpu_per;
          if (x.cpu_per < best[(size_t)i]) { best[(size_t)i] = x.cpu_per; bwall[(size_t)i] = x.wall_per; }
          if (x.cpu_per > worst[(size_t)i]) worst[(size_t)i] = x.cpu_per;
        }
      }
      for (int i = 0; i < NARMS; ++i) {
        std::printf("%-18s %-4s %-9s %-7d %-9s %13.0f %6.1f %13.0f %10.3f %10.3f\n",
                    arms[i].name, arms[i].cell, arms[i].delivery, n,
                    strcmp(arms[i].host_threads, "N") ? "1" : (n == 1 ? "1" : "N"),
                    best[(size_t)i],
                    100.0 * (worst[(size_t)i] - best[(size_t)i]) / best[(size_t)i],
                    bwall[(size_t)i], best[(size_t)i] / best[0],
                    bwall[(size_t)i] / bwall[0]);
      }

      // Every difference the grid exists to produce, as a WITHIN-ROUND delta, with its
      // range over the rounds and whether lo and hi SHARE A SIGN. A range that straddles
      // zero has not separated, and a range quoted over the subset with the wanted sign is
      // the defect this convention exists to prevent.
      struct Delta { const char *name; int p, q; };
      const Delta deltas[] = {
        {"B-A  transport, blocking",            0, 1},
        {"C-B  codec under the core transport", 1, 4},
        {"D-A  codec under grpc++'s transport", 0, 8},
        {"C-A  both halves together",           0, 4},
        {"cb - blocking, cell C",               4, 5},
        {"queue - blocking, cell C",            4, 6},
        {"queue - callback, cell C",            5, 6},
        {"cb x N - blocking, cell C  (control)", 4, 7},
        {"cb x N - cb, cell C        (control)", 5, 7},
        {"cb - blocking, cell B",               1, 2},
        {"queue - blocking, cell B",            1, 3},
      };
      const int NDELTAS = (int)(sizeof(deltas) / sizeof(deltas[0]));
      std::printf("\n   within-round deltas, %d rounds (R4). 'sign' is whether lo and hi agree;\n"
                  "   a delta that straddles zero has NOT separated and is not quoted as a\n"
                  "   number. %%A is the delta as a percentage of that round's cell A.\n", rounds);
      std::printf("   %-38s %12s %12s %8s %8s %6s\n", "delta", "lo ns", "hi ns", "lo %A",
                  "hi %A", "sign");
      for (int d = 0; d < NDELTAS; ++d) {
        double lo = 1e300, hi = -1e300, plo = 1e300, phi = -1e300;
        for (int r = 0; r < rounds; ++r) {
          double v = per_round[(size_t)r][(size_t)deltas[d].q] -
                     per_round[(size_t)r][(size_t)deltas[d].p];
          double pc = 100.0 * v / per_round[(size_t)r][0];
          if (v < lo) lo = v;
          if (v > hi) hi = v;
          if (pc < plo) plo = pc;
          if (pc > phi) phi = pc;
        }
        std::printf("   %-38s %12.0f %12.0f %8.2f %8.2f %6s\n", deltas[d].name, lo, hi, plo,
                    phi, (lo > 0 || hi < 0) ? "yes" : "NO");
      }
      // The additivity question, also within round: is the codec worth the same under the
      // two transports? (C-B) - (D-A) formed inside each round.
      {
        double lo = 1e300, hi = -1e300;
        for (int r = 0; r < rounds; ++r) {
          const std::vector<double> &x = per_round[(size_t)r];
          double v = (x[4] - x[1]) - (x[8] - x[0]);
          if (v < lo) lo = v;
          if (v > hi) hi = v;
        }
        std::printf("   %-38s %12.0f %12.0f %8s %8s %6s\n", "(C-B) - (D-A)  do the halves add?",
                    lo, hi, "", "", (lo > 0 || hi < 0) ? "yes" : "NO");
      }
      double dump_per = g_dump_calls.load() ? g_dump_ns.load() / (double)g_dump_calls.load() : 0.0;
      std::printf("   cell D also pays %.0f ns/RPC of slice concatenation at this level, which\n"
                  "   cell A does not; D-A with it removed is that much more negative.\n\n",
                  dump_per);
    }

    if (g_dump_calls.load() > 0) {
      std::printf("cell D's slice concatenation at 16 in flight: %.0f ns/RPC over %ld calls, "
                  "%ld of them multi-slice.\n"
                  "   protobuf parses off grpc++'s slice list; the core's decoder needs a\n"
                  "   contiguous buffer, so cell D pays this and cell A does not. It is inside\n"
                  "   D's column above and is printed so it can be taken out.\n\n",
                  g_dump_ns.load() / (double)g_dump_calls.load(), g_dump_calls.load(),
                  g_dump_multislice.load());
      g_dump_ns.store(0.0); g_dump_calls.store(0); g_dump_multislice.store(0);
    }

    // The same response decoded standalone, in THIS process, so the codec half can be had
    // by subtraction as well as from the grid, and the two can be compared (R4: never
    // across binaries).
    {
      svcns::ListTasksDetailedResponse full;
      pbbuild::payload_p2_2(&full);
      std::string wire;
      full.SerializeToString(&wire);
      const int kN = 40;
      for (int i = 0; i < 10; ++i) { decode_pb((const uint8_t *)wire.data(), wire.size());
                                     decode_core((const uint8_t *)wire.data(), wire.size()); }
      double a0 = thread_cpu_ns();
      for (int i = 0; i < kN; ++i) g_sink += decode_pb((const uint8_t *)wire.data(), wire.size());
      double pbdec = (thread_cpu_ns() - a0) / kN;
      double b0 = thread_cpu_ns();
      for (int i = 0; i < kN; ++i) g_sink += decode_core((const uint8_t *)wire.data(), wire.size());
      double codec = (thread_cpu_ns() - b0) / kN;
      std::printf("standalone decode of the same %zu B response, ns of thread CPU:\n"
                  "   protobuf C++ %.0f    the core through the C ABI %.0f    ratio %.3f\n"
                  "   difference %+.0f ns, which is what C-B and D-A above should come to if\n"
                  "   the codec is the only thing that differs between those cells.\n\n",
                  wire.size(), pbdec, codec, codec / pbdec, codec - pbdec);
    }

    ak_client_destroy(cl);
    ak_runtime_destroy(rt);
    server->Shutdown();
    server->Wait();
    if (tr.uds) ::unlink(tr.sock.c_str());
  }

  std::printf("sink %ld\n\n", (long)g_sink.load());
  std::printf(
      "How to read the columns.\n"
      "  `CPU ns/RPC` is getrusage(RUSAGE_SELF) MINUS the server handler's own CPU, both\n"
      "  measured per round and divided by the RPCs in it. It counts every thread in the\n"
      "  process, so the core's tokio workers are in it and grpc++'s transport is too; a\n"
      "  per-thread sum over the threads the harness spawned is NOT like-for-like and\n"
      "  biases the ratio by about 0.2 (C11 in this slice's defect log).\n"
      "  `hostthr` is what N in flight costs the arm in HOST threads. Blocking needs N of\n"
      "  them by construction -- a thread inside `ak_call_unary` is inside the core until\n"
      "  the response lands. The callback and the queue need one. That is the deliveries'\n"
      "  real difference and it is why the wall-clock ratio at 8 and 16 is not a like-for-\n"
      "  like column: one host thread cannot decode 16 responses at once.\n"
      "  `wall/A` is printed beside the CPU column and is never the headline (R9).\n"
      "  `spr%%` is the arm's own round-to-round spread on the CPU column, (max-min)/min.\n"
      "  It is 5 to 25 percent here, which is why every DIFFERENCE below the table is a\n"
      "  within-round delta and not a difference of two best-of-nine figures.\n"
      "\n"
      "What is inside the numbers and is not the thing being measured.\n"
      "  Thread creation. The blocking arms, `cb x N`, cell A and cell D spawn their host\n"
      "  threads per round, so a round of 80 RPCs at 16 in flight pays 16 pthread_create\n"
      "  and join pairs. At a generous 30 us each that is 480 us over 80 RPCs, about 6 us\n"
      "  per RPC against a call of roughly 4,000 us -- 0.15 percent. It is arithmetic and\n"
      "  not a measurement, and it is stated because it lands on exactly the arms the\n"
      "  delivery comparison is between. It is two orders of magnitude below the\n"
      "  differences that separate and cannot account for any of them.\n"
      "  Cell D's slice concatenation, printed per level above: grpc++ hands the generic\n"
      "  path a slice list and the core's decoder needs one buffer, where protobuf parses\n"
      "  straight off the list. Cell A does not pay it and cell D does.\n"
      "  The server. It is in-process and its handler CPU is subtracted from every cell,\n"
      "  but its transport threads are not and cannot be; they are the same work for all\n"
      "  four cells, so they dilute every ratio toward 1 rather than favouring one.\n"
      "\n"
      "What this grid does NOT measure, per R11.\n"
      "  The ENCODE direction. The request is an empty message, so the codec difference\n"
      "  here is a decode difference and nothing else. A grid with a large request would\n"
      "  be a different measurement and this one does not stand in for it.\n"
      "  Allocation per RPC, which design/SHAPES.md's RPC arm asks for beside CPU. Nothing\n"
      "  here counts allocations, and a page-fault or RSS proxy dressed as an allocation\n"
      "  count would be worse than the gap.\n"
      "  Streaming, TLS, retry, deadlines, metadata, the status code as a number, a real\n"
      "  network, failure injection, and the server side. Section 9's case is behavioural\n"
      "  and this arm is not a test of it.\n");
  return 0;
}
