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

class ShapesImpl final : public ns::Shapes::Service {
 public:
  ShapesImpl() { pbbuild::payload_p2_2(&resp_); }
  grpc::Status Fetch(grpc::ServerContext *, const ns::Empty *,
                     ns::ListTasksDetailedResponse *out) override {
    *out = resp_;
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

struct Result { double cpu_per, wall_per; };

template <class Fn>
static Result drive(int threads, int calls_per_thread, Fn body) {
  double c0 = cpu_ns(), w0 = wall_ns();
  std::vector<std::thread> ts;
  for (int t = 0; t < threads; ++t)
    ts.push_back(std::thread([&, t]() { for (int i = 0; i < calls_per_thread; ++i) body(t); }));
  for (size_t i = 0; i < ts.size(); ++i) ts[i].join();
  Result r;
  double n = (double)threads * calls_per_thread;
  r.cpu_per = (cpu_ns() - c0) / n;
  r.wall_per = (wall_ns() - w0) / n;
  return r;
}

int main(int argc, char **argv) {
  int calls = argc > 1 ? atoi(argv[1]) : 200;
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
  for (int i = 0; i < 5; ++i) { grpcpp_call(0); core_call(0); }

  std::printf("\n%-10s %-8s %14s %14s %10s\n", "arm", "inflight", "CPU ns/RPC",
              "wall ns/RPC", "cpu/pb");
  const int kInflight[] = {1, 8, 16};
  for (int k = 0; k < 3; ++k) {
    int n = kInflight[k];
    int per = calls / n;
    if (per < 5) per = 5;
    Result a = drive(n, per, grpcpp_call);
    Result c = drive(n, per, core_call);
    std::printf("%-10s %-8d %14.0f %14.0f %10s\n", "grpc++", n, a.cpu_per, a.wall_per, "1.000");
    std::printf("%-10s %-8d %14.0f %14.0f %10.3f\n", "core-ffi", n, c.cpu_per, c.wall_per,
                c.cpu_per / a.cpu_per);
  }
  std::printf("\nsink %ld\n", (long)bytes_in.load());
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
