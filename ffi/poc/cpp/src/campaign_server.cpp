// design/CAMPAIGN.md section 4.2, the RPC grid's SERVER: its own process, pinned by the
// runner to AK_CPU_SERVER (requirement 13), identical work whichever client cell calls.
//
//   Fetch  (direction a)  returns the PRE-SERIALISED P2.2 bytes: built once at start-up with
//                         the incumbent's builder, serialised once, held as a grpc::Slice.
//                         Each call hands out a reference to that slice; nothing is
//                         serialised per call.
//   Push   (direction b)  decodes the P2.2-sized request with protobuf C++ (grpc++'s own
//                         SerializationTraits, the production path), checks it is the P2.2
//                         message (500 tasks), and returns an empty response. The same
//                         decode for every client cell, so B - A and C - B are not moved by
//                         the server.
//
// Both are raw byte methods (grpc++'s generated WithRawCallbackMethod_*), so the codec on the
// server side is fixed and stated rather than chosen per cell.
//
//   campaign_server --port P --transport shipped|pinned
// Prints "READY <port>" on stdout once listening; exits on SIGTERM/SIGINT, printing its own
// CPU time (getrusage RUSAGE_SELF) on stderr as instrumentation for the log header.
#include <grpcpp/grpcpp.h>
#include <grpcpp/impl/codegen/proto_utils.h>
#include <sys/resource.h>

#include <atomic>
#include <csignal>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <thread>

#include "generated/pb_build.h"
#include "shapes_svc.grpc.pb.h"

namespace svcns = armonik::ffi::shapes::v1;

namespace {

std::atomic<bool> g_stop(false);
void on_signal(int) { g_stop = true; }

typedef svcns::Shapes::WithRawCallbackMethod_Fetch<
    svcns::Shapes::WithRawCallbackMethod_Push<svcns::Shapes::Service> > RawBase;

class Svc final : public RawBase {
 public:
  explicit Svc(const std::string &p22) {
    grpc::Slice s(p22.data(), p22.size());
    pre_ = grpc::ByteBuffer(&s, 1);
    grpc::Slice e;
    empty_ = grpc::ByteBuffer(&e, 1);
  }
  grpc::ServerUnaryReactor *Fetch(grpc::CallbackServerContext *ctx, const grpc::ByteBuffer *,
                                  grpc::ByteBuffer *resp) override {
    *resp = pre_;  // a reference to the pre-serialised slice, not a copy of it
    grpc::ServerUnaryReactor *r = ctx->DefaultReactor();
    r->Finish(grpc::Status::OK);
    return r;
  }
  grpc::ServerUnaryReactor *Push(grpc::CallbackServerContext *ctx, const grpc::ByteBuffer *req,
                                 grpc::ByteBuffer *resp) override {
    grpc::ByteBuffer copy(*req);  // Deserialize consumes its argument
    svcns::ListTasksDetailedResponse m;
    grpc::Status st = grpc::SerializationTraits<svcns::ListTasksDetailedResponse>::Deserialize(&copy, &m);
    grpc::ServerUnaryReactor *r = ctx->DefaultReactor();
    if (!st.ok() || m.tasks_size() != 500) {
      r->Finish(grpc::Status(grpc::StatusCode::INVALID_ARGUMENT, "not the P2.2 request"));
      return r;
    }
    *resp = empty_;
    r->Finish(grpc::Status::OK);
    return r;
  }

 private:
  grpc::ByteBuffer pre_;
  grpc::ByteBuffer empty_;
};

}  // namespace

int main(int argc, char **argv) {
  int port = 0;
  std::string transport = "shipped";
  for (int i = 1; i + 1 < argc; i += 2) {
    if (!std::strcmp(argv[i], "--port")) port = std::atoi(argv[i + 1]);
    else if (!std::strcmp(argv[i], "--transport")) transport = argv[i + 1];
  }
  if (transport != "shipped" && transport != "pinned") {
    std::fprintf(stderr, "--transport shipped|pinned\n");
    return 2;
  }
  svcns::ListTasksDetailedResponse p22;
  pbbuild::payload_p2_2(&p22);
  std::string bytes;
  p22.SerializeToString(&bytes);
  Svc svc(bytes);

  grpc::ServerBuilder b;
  int bound = 0;
  b.AddListeningPort("127.0.0.1:" + std::to_string(port), grpc::InsecureServerCredentials(), &bound);
  // Message ceiling: the 2 MiB ArmoniK chunk size in both configurations. `pinned` adds the
  // 4 MiB stream window with BDP probing off (grpc-core has no connection-window argument:
  // logs/cpp/rpcflow.log, C31); `shipped` leaves grpc++'s defaults.
  b.SetMaxReceiveMessageSize(2 * 1024 * 1024);
  b.SetMaxSendMessageSize(2 * 1024 * 1024);
  if (transport == "pinned") {
    b.AddChannelArgument(GRPC_ARG_HTTP2_STREAM_LOOKAHEAD_BYTES, 4 * 1024 * 1024);
    b.AddChannelArgument(GRPC_ARG_HTTP2_BDP_PROBE, 0);
  }
  b.RegisterService(&svc);
  std::unique_ptr<grpc::Server> server = b.BuildAndStart();
  if (!server || bound == 0) {
    std::fprintf(stderr, "server failed to start\n");
    return 1;
  }
  std::signal(SIGTERM, on_signal);
  std::signal(SIGINT, on_signal);
  std::printf("READY %d %zu\n", bound, bytes.size());
  std::fflush(stdout);
  while (!g_stop) std::this_thread::sleep_for(std::chrono::milliseconds(50));
  server->Shutdown();
  struct rusage ru;
  getrusage(RUSAGE_SELF, &ru);
  std::fprintf(stderr, "server cpu_ns %lld (instrumentation)\n",
               (long long)(ru.ru_utime.tv_sec + ru.ru_stime.tv_sec) * 1000000000LL +
                   (long long)(ru.ru_utime.tv_usec + ru.ru_stime.tv_usec) * 1000LL);
  return 0;
}
