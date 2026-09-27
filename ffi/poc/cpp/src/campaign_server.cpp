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
//   Upload (direction c)  decodes the P5.3 / P5.4 request (M5) with protobuf C++, refuses one
//                         without data, and returns an empty response (req. 14, 2026-09-27).
//   UploadStream (d)      client streaming: every message is hashed (SHA-256 of its bytes as
//                         received) and decoded with protobuf C++ as M5, the ids required on
//                         the first; the answer is an UploadAck with the data byte count and
//                         the digest, which the client checks against its own (req. 18).
//
// All are raw byte methods (grpc++'s generated WithRawCallbackMethod_*), so the codec on the
// server side is fixed and stated rather than chosen per cell.
//
// One server process and configuration per launch (req. 13, amended 2026-09-26): it serves
// every cell of both builds on two Unix domain sockets (req. 17), one per transport
// configuration, because grpc++'s window settings are per server: `shipped` (grpc++'s
// defaults) and `pinned` (4 MiB stream window, BDP off). The runner warms it with
// `campaign_rpc --warm-server N` from each client transport before any client runs.
//
//   campaign_server --uds-shipped PATH --uds-pinned PATH
// Prints "READY <shipped-path> <pinned-path> <P2.2 bytes> <threads>" once listening; exits
// on SIGTERM/SIGINT, printing on stderr its CPU time (getrusage RUSAGE_SELF, instrumentation),
// the calls it served per method and its thread count (req. 4).
#include <grpcpp/grpcpp.h>
#include <grpcpp/impl/codegen/proto_utils.h>
#include <sys/resource.h>

#include <atomic>
#include <fstream>
#include <csignal>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <thread>
#include <vector>

#include "generated/pb_build.h"
#include "sha256.h"
#include "shapes_svc.grpc.pb.h"

namespace svcns = armonik::ffi::shapes::v1;

namespace {

std::atomic<bool> g_stop(false);
std::atomic<long> g_fetch(0), g_push(0), g_upload(0), g_stream(0);
int proc_threads() {
  std::ifstream f("/proc/self/status");
  std::string line;
  while (std::getline(f, line))
    if (line.compare(0, 8, "Threads:") == 0) return std::atoi(line.c_str() + 8);
  return -1;
}
void on_signal(int) { g_stop = true; }

typedef svcns::Shapes::WithRawCallbackMethod_Fetch<svcns::Shapes::WithRawCallbackMethod_Push<
    svcns::Shapes::WithRawCallbackMethod_Upload<svcns::Shapes::WithRawCallbackMethod_UploadStream<
        svcns::Shapes::Service> > > > RawBase;

// Direction d's reader: one per call, deleted when grpc++ is done with it.
class UploadReader final : public grpc::ServerReadReactor<grpc::ByteBuffer> {
 public:
  explicit UploadReader(grpc::ByteBuffer *resp) : resp_(resp) { StartRead(&msg_); }
  void OnReadDone(bool ok) override {
    if (!ok) {  // the client ended the request stream: answer
      svcns::UploadAck a;
      a.set_data_bytes(total_);
      uint8_t d[32];
      sha_.final(d);
      a.set_sha256(d, 32);
      std::string w;
      a.SerializeToString(&w);
      grpc::Slice sl(w.data(), w.size());
      *resp_ = grpc::ByteBuffer(&sl, 1);
      Finish(first_ ? grpc::Status(grpc::StatusCode::INVALID_ARGUMENT, "an upload with no message")
                    : grpc::Status::OK);
      return;
    }
    std::vector<grpc::Slice> slices;
    if (!msg_.Dump(&slices).ok()) return fail("unreadable message");
    for (size_t i = 0; i < slices.size(); ++i) sha_.update(slices[i].begin(), slices[i].size());
    svcns::UploadResultDataMessage m;
    if (!grpc::SerializationTraits<svcns::UploadResultDataMessage>::Deserialize(&msg_, &m).ok() || !m.has_upload())
      return fail("not an M5 message");
    if (first_ && (m.upload().session_id().empty() || m.upload().result_id().empty()))
      return fail("the first message carries no ids");
    first_ = false;
    total_ += m.upload().data_chunk().size();
    msg_.Clear();
    StartRead(&msg_);
  }
  void OnDone() override { delete this; }

 private:
  void fail(const char *why) { Finish(grpc::Status(grpc::StatusCode::INVALID_ARGUMENT, why)); }
  grpc::ByteBuffer *resp_;
  grpc::ByteBuffer msg_;
  aksha::Sha256 sha_;
  uint64_t total_ = 0;
  bool first_ = true;
};

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
    ++g_fetch;
    *resp = pre_;  // a reference to the pre-serialised slice, not a copy of it
    grpc::ServerUnaryReactor *r = ctx->DefaultReactor();
    r->Finish(grpc::Status::OK);
    return r;
  }
  grpc::ServerUnaryReactor *Push(grpc::CallbackServerContext *ctx, const grpc::ByteBuffer *req,
                                 grpc::ByteBuffer *resp) override {
    ++g_push;
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
  grpc::ServerUnaryReactor *Upload(grpc::CallbackServerContext *ctx, const grpc::ByteBuffer *req,
                                   grpc::ByteBuffer *resp) override {
    ++g_upload;
    grpc::ByteBuffer copy(*req);
    svcns::UploadResultDataMessage m;
    grpc::Status st = grpc::SerializationTraits<svcns::UploadResultDataMessage>::Deserialize(&copy, &m);
    grpc::ServerUnaryReactor *r = ctx->DefaultReactor();
    if (!st.ok() || m.upload().data_chunk().empty()) {
      r->Finish(grpc::Status(grpc::StatusCode::INVALID_ARGUMENT, "not an M5 upload with data"));
      return r;
    }
    *resp = empty_;
    r->Finish(grpc::Status::OK);
    return r;
  }
  grpc::ServerReadReactor<grpc::ByteBuffer> *UploadStream(grpc::CallbackServerContext *,
                                                          grpc::ByteBuffer *resp) override {
    ++g_stream;
    return new UploadReader(resp);
  }

 private:
  grpc::ByteBuffer pre_;
  grpc::ByteBuffer empty_;
};

}  // namespace

const int kServerMaxMessage = 8 * 1024 * 1024;

std::unique_ptr<grpc::Server> start(const std::string &path, bool pinned, Svc *svc) {
  grpc::ServerBuilder b;
  b.AddListeningPort("unix:" + path, grpc::InsecureServerCredentials());
  // Message ceiling (D44, req. 14 as amended 2026-09-27): 8 MiB in both configurations, so
  // P5.4 (4,194,390 B) and a 2 MiB chunk with its M5 framing are accepted. `pinned` adds the
  // 4 MiB stream window with BDP probing off (grpc-core has no connection-window argument:
  // logs/cpp/rpcflow.log, C31); `shipped` leaves grpc++'s other defaults.
  b.SetMaxReceiveMessageSize(kServerMaxMessage);
  b.SetMaxSendMessageSize(kServerMaxMessage);
  if (pinned) {
    b.AddChannelArgument(GRPC_ARG_HTTP2_STREAM_LOOKAHEAD_BYTES, 4 * 1024 * 1024);
    b.AddChannelArgument(GRPC_ARG_HTTP2_BDP_PROBE, 0);
  }
  b.RegisterService(svc);
  return b.BuildAndStart();
}

int main(int argc, char **argv) {
  std::string shipped, pinned;
  for (int i = 1; i + 1 < argc; i += 2) {
    if (!std::strcmp(argv[i], "--uds-shipped")) shipped = argv[i + 1];
    else if (!std::strcmp(argv[i], "--uds-pinned")) pinned = argv[i + 1];
  }
  if (shipped.empty() || pinned.empty()) {
    std::fprintf(stderr, "usage: campaign_server --uds-shipped PATH --uds-pinned PATH\n");
    return 2;
  }
  svcns::ListTasksDetailedResponse p22;
  pbbuild::payload_p2_2(&p22);
  std::string bytes;
  p22.SerializeToString(&bytes);
  Svc svc_s(bytes), svc_p(bytes);
  std::unique_ptr<grpc::Server> ss = start(shipped, false, &svc_s);
  std::unique_ptr<grpc::Server> sp = start(pinned, true, &svc_p);
  if (!ss || !sp) {
    std::fprintf(stderr, "server failed to start\n");
    return 1;
  }
  std::signal(SIGTERM, on_signal);
  std::signal(SIGINT, on_signal);
  std::printf("READY %s %s %zu %d\n", shipped.c_str(), pinned.c_str(), bytes.size(), proc_threads());
  std::fflush(stdout);
  while (!g_stop) std::this_thread::sleep_for(std::chrono::milliseconds(50));
  int threads = proc_threads();
  ss->Shutdown();
  sp->Shutdown();
  struct rusage ru;
  getrusage(RUSAGE_SELF, &ru);
  std::fprintf(stderr, "server cpu_ns %lld (instrumentation); served Fetch %ld Push %ld Upload %ld UploadStream %ld; threads %d\n",
               (long long)(ru.ru_utime.tv_sec + ru.ru_stime.tv_sec) * 1000000000LL +
                   (long long)(ru.ru_utime.tv_usec + ru.ru_stime.tv_usec) * 1000LL,
               g_fetch.load(), g_push.load(), g_upload.load(), g_stream.load(), threads);
  return 0;
}
