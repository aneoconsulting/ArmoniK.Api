// The RPC arm of design/SHAPES.md, and R9 is why it measures CPU.
//
// "An RPC arm on P2.2 measures HTTP/2 flow control unless it measures CPU." A 540 KB
// response exceeds the 64 KB default stream window, so a single call in flight spends most
// of its wall clock idle waiting for WINDOW_UPDATE. SHAPES.md asks for CPU per RPC; the
// wall-clock column is printed BESIDE it, never instead of it.
//
// One server, two clients, one process: a grpc++ sync server holding the P2.2 response,
// called by (a) the grpc++ generated stub -- the incumbent -- and (b) the core's
// `ak_call_unary` over tonic. The server's CPU is inside both arms and cancels in the
// ratio; that it is inside at all is stated rather than hidden.
#include <grpcpp/grpcpp.h>
#include <sys/resource.h>

#include <atomic>
#include <cmath>
#include <cstdio>
#include <string>
#include <thread>
#include <vector>

#include "ak_abi.h"
#include "generated/binding.h"
#include "generated/build.h"
#include "generated/pb_build.h"
#include "shapes_svc.grpc.pb.h"

namespace ns = armonik::ffi::shapes::v1;

extern "C" {
typedef struct ak_runtime ak_runtime;
typedef struct ak_client ak_client;
struct ak_bytes { const uint8_t *ptr; size_t len; void *owner; };
ak_runtime *ak_runtime_new(uint32_t worker_threads);
void ak_runtime_destroy(ak_runtime *);
ak_client *ak_client_new(ak_runtime *, const uint8_t *uri, size_t uri_len);
void ak_client_destroy(ak_client *);
int32_t ak_call_unary(ak_client *, const uint8_t *path, size_t path_len,
                      const uint8_t *req, size_t req_len, struct ak_bytes *out);
void ak_bytes_free(struct ak_bytes *);
}

static const char *kPath = "/armonik.ffi.shapes.v1.Shapes/Fetch";

static std::atomic<double> g_server_cpu_atomic(0.0);

static double thread_cpu_ns() {
  struct timespec ts;
  clock_gettime(CLOCK_THREAD_CPUTIME_ID, &ts);
  return (double)ts.tv_sec * 1e9 + (double)ts.tv_nsec;
}

class ShapesImpl final : public ns::Shapes::Service {
 public:
  ShapesImpl() { pbbuild::payload_p2_2(&resp_); }
  grpc::Status Fetch(grpc::ServerContext *, const ns::Empty *,
                     ns::ListTasksDetailedResponse *out) override {
    // The handler's own CPU, so it can be subtracted from the process figure for BOTH
    // arms. It does not cover the server's transport threads, which is stated rather
    // than pretended away: what it does cover is the deep copy and the serialisation,
    // which is the bulk of it.
    double a = thread_cpu_ns();
    *out = resp_;
    double d = thread_cpu_ns() - a;
    double cur = g_server_cpu_atomic.load(std::memory_order_relaxed);
    while (!g_server_cpu_atomic.compare_exchange_weak(cur, cur + d)) {
    }
    return grpc::Status::OK;
  }

 private:
  ns::ListTasksDetailedResponse resp_;
};

static double cpu_ns() {
  struct rusage r;
  getrusage(RUSAGE_SELF, &r);
  return (double)(r.ru_utime.tv_sec + r.ru_stime.tv_sec) * 1e9 +
         (double)(r.ru_utime.tv_usec + r.ru_stime.tv_usec) * 1e3;
}

static double wall_ns() {
  struct timespec ts;
  clock_gettime(CLOCK_MONOTONIC, &ts);
  return (double)ts.tv_sec * 1e9 + (double)ts.tv_nsec;
}


struct Result { double client_cpu_per, proc_cpu_per, server_cpu_per, wall_per; };

// The SERVER's own CPU, measured per thread the way the clients are, so
// `client = process - server` can be stated for BOTH arms rather than asserted for one.
//
// This is the finding that made the first version of this file unsound: summing
// CLOCK_THREAD_CPUTIME_ID over the threads the HARNESS spawned counts the grpc++ stub's
// transport (it runs on the calling thread) and misses the core's (it runs on
// `ak_runtime_new`'s tokio workers). The two arms were being charged differently for the
// same thing. `getrusage(RUSAGE_SELF)` counts every thread in the process, so
// process CPU minus server CPU is a like-for-like client figure for both.
#define g_server_cpu (g_server_cpu_atomic.load(std::memory_order_relaxed))

// Two CPU columns, and the first is the one to read.
//
// `client_cpu_per` sums CLOCK_THREAD_CPUTIME_ID over the CLIENT threads only, so the
// in-process server -- which deep-copies and re-serialises a 540 KB message on every call
// and costs far more than either client -- is excluded. `proc_cpu_per` is the whole
// process and is dominated by that server; it is printed so the dilution is visible rather
// than hidden, because a ratio taken on it is mostly a ratio of the server to itself.
template <class Fn>
static Result drive(int threads, int calls_per_thread, Fn body) {
  double c0 = cpu_ns(), w0 = wall_ns();
  double s0 = g_server_cpu;
  std::vector<double> tcpu((size_t)threads, 0.0);
  std::vector<std::thread> ts;
  for (int t = 0; t < threads; ++t)
    ts.push_back(std::thread([&, t]() {
      double a = thread_cpu_ns();
      for (int i = 0; i < calls_per_thread; ++i) body(t);
      tcpu[(size_t)t] = thread_cpu_ns() - a;
    }));
  for (size_t i = 0; i < ts.size(); ++i) ts[i].join();
  Result r;
  double n = (double)threads * calls_per_thread;
  double sum = 0;
  for (size_t i = 0; i < tcpu.size(); ++i) sum += tcpu[i];
  r.client_cpu_per = sum / n;
  r.proc_cpu_per = (cpu_ns() - c0) / n;
  r.server_cpu_per = (g_server_cpu - s0) / n;
  r.wall_per = (wall_ns() - w0) / n;
  return r;
}

int main(int argc, char **argv) {
  int calls = argc > 1 ? atoi(argv[1]) : 200;
  int rounds = argc > 2 ? atoi(argv[2]) : 9;
  std::string addr = "127.0.0.1:50077";

  ShapesImpl svc;
  grpc::ServerBuilder b;
  b.AddListeningPort(addr, grpc::InsecureServerCredentials());
  b.RegisterService(&svc);
  std::unique_ptr<grpc::Server> server = b.BuildAndStart();
  if (!server) { std::printf("server failed to start\n"); return 1; }
  std::printf("grpc++ %s server on %s, in-process\n", grpc::Version().c_str(), addr.c_str());

  // --- incumbent: the generated grpc++ stub, one per thread (a Channel is shareable).
  auto chan = grpc::CreateChannel(addr, grpc::InsecureChannelCredentials());
  std::vector<std::unique_ptr<ns::Shapes::Stub> > stubs;
  for (int i = 0; i < 16; ++i) stubs.push_back(ns::Shapes::NewStub(chan));

  // --- the core: one runtime, one client handle, shared across threads (ABI v1 section 9
  // says a client handle is usable from many threads at once, and the rust slice found the
  // defect that says so by asking for 8 in flight).
  ak_runtime *rt = ak_runtime_new(2);
  std::string uri = "http://" + addr;
  ak_client *cl = ak_client_new(rt, (const uint8_t *)uri.data(), uri.size());
  if (!cl) { std::printf("ak_client_new failed\n"); return 1; }

  ns::Empty req;
  std::string req_bytes;
  req.SerializeToString(&req_bytes);

  std::atomic<long> bytes_in(0);

  auto grpcpp_call = [&](int t) {
    grpc::ClientContext ctx;
    ns::Empty q;
    ns::ListTasksDetailedResponse r;
    grpc::Status s = stubs[t % stubs.size()]->Fetch(&ctx, q, &r);
    if (!s.ok()) { std::printf("grpc++ call failed: %s\n", s.error_message().c_str()); abort(); }
    bytes_in += r.tasks_size();
  };
  auto core_call = [&](int t) {
    (void)t;
    struct ak_bytes out;
    out.ptr = NULL; out.len = 0; out.owner = NULL;
    int32_t rc = ak_call_unary(cl, (const uint8_t *)kPath, strlen(kPath),
                               (const uint8_t *)req_bytes.data(), req_bytes.size(), &out);
    if (rc != AK_OK) { std::printf("ak_call_unary failed rc=%d\n", rc); abort(); }
    shapes::ListTasksDetailedResponse f;
    ak_dec_ctx *dctx = ak_dec_ctx_new();
    shapes::ffi::decode_with_list_tasks_detailed_response(dctx, out.ptr, out.len, &f);
    ak_dec_ctx_free(dctx);
    bytes_in += (long)f.tasks.size();
    ak_bytes_free(&out);
  };

  // warm both paths (connection setup, the first allocation, the learned widths)
  for (int i = 0; i < 20; ++i) { grpcpp_call(0); core_call(0); }

  std::printf("\nCrossings per RPC: TWO, and it is a property of the code rather than a\n"
              "measurement -- `ak_call_unary` in, `ak_bytes_free` out. The core's RPC module\n"
              "names no message type anywhere, so there is no place a per-field cost could\n"
              "enter; `gen/rpc.sh` greps for that rather than trusting this sentence.\n");
  std::printf("\n%-10s %-8s %14s %14s %14s %14s %9s\n", "arm", "inflight",
              "proc-srv CPU", "harness-thr CPU", "server CPU", "wall ns/RPC",
              "(p-s)/pb");
  const int kInflight[] = {1, 8, 16};
  for (int k = 0; k < 3; ++k) {
    int n = kInflight[k];
    int per = calls / n;
    if (per < 5) per = 5;
    // ROUNDS, and the arms alternate which goes first, because a single shot in a fixed
    // order quoted to three digits is not a measurement.
    Result a, c;
    double ba = 1e300, bc = 1e300;
    for (int r = 0; r < rounds; ++r) {
      Result x, y;
      if (r % 2 == 0) {
        x = drive(n, per, grpcpp_call);
        y = drive(n, per, core_call);
      } else {
        y = drive(n, per, core_call);
        x = drive(n, per, grpcpp_call);
      }
      if (x.proc_cpu_per - x.server_cpu_per < ba) { ba = x.proc_cpu_per - x.server_cpu_per; a = x; }
      if (y.proc_cpu_per - y.server_cpu_per < bc) { bc = y.proc_cpu_per - y.server_cpu_per; c = y; }
    }
    std::printf("%-10s %-8d %14.0f %14.0f %14.0f %14.0f %9s\n", "grpc++", n,
                ba, a.client_cpu_per, a.server_cpu_per, a.wall_per, "1.000");
    std::printf("%-10s %-8d %14.0f %14.0f %14.0f %14.0f %9.3f\n", "core-ffi", n,
                bc, c.client_cpu_per, c.server_cpu_per, c.wall_per, bc / ba);
  }
  // The same decode, standalone, in THIS process, so the transport half can be separated
  // from the codec half by subtraction rather than across processes (R4).
  {
    ns::ListTasksDetailedResponse full;
    pbbuild::payload_p2_2(&full);
    std::string wire;
    full.SerializeToString(&wire);
    const int kN = 40;
    // warm both decoders before either is timed
    for (int i = 0; i < 10; ++i) {
      ns::ListTasksDetailedResponse m;
      m.ParseFromString(wire);
      shapes::ListTasksDetailedResponse f;
      ak_dec_ctx *d = ak_dec_ctx_new();
      shapes::ffi::decode_with_list_tasks_detailed_response(
          d, (const uint8_t *)wire.data(), wire.size(), &f);
      ak_dec_ctx_free(d);
    }
    double a0 = thread_cpu_ns();
    for (int i = 0; i < kN; ++i) {
      ns::ListTasksDetailedResponse m;
      m.ParseFromString(wire);
    }
    double pbdec = (thread_cpu_ns() - a0) / kN;
    double b0 = thread_cpu_ns();
    for (int i = 0; i < kN; ++i) {
      shapes::ListTasksDetailedResponse f;
      ak_dec_ctx *d = ak_dec_ctx_new();
      shapes::ffi::decode_with_list_tasks_detailed_response(
          d, (const uint8_t *)wire.data(), wire.size(), &f);
      ak_dec_ctx_free(d);
    }
    double codec = (thread_cpu_ns() - b0) / kN;
    std::printf("\n-- the same response decoded standalone, in this process, ns of thread CPU --\n");
    std::printf("  protobuf C++ %.0f    the core through the C ABI %.0f    ratio %.3f\n",
                pbdec, codec, codec / pbdec);
    std::printf("  So the client CPU above is mostly the CODEC; what is left after subtracting\n"
                "  it is the transport half, where ABI v1 section 9 spends TWO crossings and\n"
                "  zero per field.\n");
  }

  std::printf("\nsink %ld\n", (long)bytes_in.load());
  std::printf("\nThe idiomatic C++ wait is a blocking call on a thread the host owns, and\n"
              "both arms use it. C++ has no carrier-thread notion to pin, so SHAPES.md's\n"
              "third RPC question is answered trivially here and is a real question only on\n"
              "a runtime with virtual threads.\n");
  std::printf("\n`proc-srv CPU` is getrusage(RUSAGE_SELF) minus the server handler's own\n"
              "CPU, and it is the column to read: it counts EVERY thread in the process,\n"
              "so the core's tokio workers are in it. `harness-thr CPU` sums\n"
              "CLOCK_THREAD_CPUTIME_ID over the threads this harness spawned, which counts\n"
              "the grpc++ stub's transport (it runs on the calling thread) and MISSES the\n"
              "core's (it does not) -- it is printed to show the size of that bias, not to\n"
              "be quoted. Best of %d rounds, arms alternating which goes first.\n", rounds);
  std::printf("\nThe wall-clock column is beside the CPU column and is NOT a throughput\n"
              "figure: 540 KB per response against a 64 KB default stream window means a\n"
              "single call in flight spends most of its wall clock waiting for WINDOW_UPDATE.\n"
              "Both arms' CPU includes the in-process server, which cancels in the ratio.\n");

  ak_client_destroy(cl);
  ak_runtime_destroy(rt);
  server->Shutdown();
  server->Wait();
  return 0;
}
