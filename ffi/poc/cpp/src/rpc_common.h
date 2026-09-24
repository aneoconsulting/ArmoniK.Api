// What the three RPC binaries share: the C ABI declarations, the one server every cell is
// measured against, and the transport configuration `design/SHAPES.md` says every arm pins
// and states.
//
// Three binaries and not one, deliberately:
//   `rpcbench`  the timed grid              (the core WITHOUT --features count)
//   `rpccounts` R5's crossing counts        (the core WITH --features count)
//   `rpcflow`   the flow-control probe      (no timing at all)
// A counting core in a timed binary is exactly what R5 forbids, and a trace-parsing probe
// in a timed binary would perturb it for nothing.
#ifndef AK_RPC_COMMON_H
#define AK_RPC_COMMON_H

#include <grpcpp/grpcpp.h>
#include <grpcpp/impl/rpc_method.h>
#include <grpcpp/impl/client_unary_call.h>
#include <sys/resource.h>
#include <unistd.h>

#include <atomic>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <vector>

#include "ak_abi.h"
#include "generated/binding.h"
#include "shapes_svc.grpc.pb.h"

namespace svcns = armonik::ffi::shapes::v1;

// ---- the C ABI ----------------------------------------------------------------------
// Every RPC struct, handle, constant and prototype of ABI v1 section 9 comes from the
// generated `ak_abi.h`, rendered from `plan.rpc` by the shared C++ backend (R-G5, FIX-PLAN
// WP5 step 2). They used to be hand-declared here: `ak_client_opts` with 3 fields while the
// core read 6 (R-D2), and `ak_bytes`, `ak_completion`, `ak_queue_next` and the rest as a
// third copy of declarations the core had twice.
//
// The counting build's RPC crossing counters (`ak_rpc_counting`, `ak_rpc_counters`,
// `ak_rpc_counters_reset`, `struct ak_rpc_counters`) are in the plan too since WP5 step 6
// (`plan.FIXED.rpc_counting`, R-G13) and come from the generated header: nothing of the
// ABI is declared by hand in this file any more.

namespace akrpc {

// R-G7: ABI v1 section 3's `ak_init`, rendered from plan.lifecycle into the binding
// (`shapes::ffi::ak_init_once`), called by every RPC binary before its first RPC call.
inline int init_core_or_die() {
  int32_t rc = shapes::ffi::ak_init_once();
  if (rc != AK_OK) {
    std::fprintf(stderr, "ak_init refused: %d\n", rc);
    std::exit(2);
  }
  return 0;
}

static const char *const kFetchPath = "/armonik.ffi.shapes.v1.Shapes/Fetch";
static const char *const kPingPath = "/armonik.ffi.shapes.v1.Shapes/Ping";

// ---- the pinned transport ------------------------------------------------------------
//
// design/SHAPES.md: "every arm pins the same configuration, which is ArmoniK's rather than
// the stack's -- 2 MiB chunking for upload and download, and a 4 MiB stream window". The
// 2 MiB figure is the application-level chunk size and bounds the largest message; this arm
// carries one 540 KB message, so what it configures is the message-size ceiling. The 4 MiB
// is the HTTP/2 stream window, and it is the one that decides whether a wall-clock column
// is measuring the codec or measuring WINDOW_UPDATE.
//
// Whether these are one setting or two, and whether pinning one turns auto-tuning off, is
// NOT inherited from the SHAPES.md table: `rpcflow` establishes it from each stack's own
// behaviour, because the table has that answer for .NET and grpc-java and not for these two.
static const int kStreamWindow = 4 * 1024 * 1024;
static const int kConnWindow = 4 * 1024 * 1024;
static const int kMaxMessage = 2 * 1024 * 1024;

inline grpc::ChannelArguments pinned_channel_args(bool pin) {
  grpc::ChannelArguments a;
  a.SetMaxReceiveMessageSize(kMaxMessage);
  a.SetMaxSendMessageSize(kMaxMessage);
  if (pin) {
    a.SetInt(GRPC_ARG_HTTP2_STREAM_LOOKAHEAD_BYTES, kStreamWindow);
    a.SetInt(GRPC_ARG_HTTP2_BDP_PROBE, 0);
  }
  return a;
}

inline void pin_server(grpc::ServerBuilder *b, bool pin) {
  b->SetMaxReceiveMessageSize(kMaxMessage);
  b->SetMaxSendMessageSize(kMaxMessage);
  if (pin) {
    b->AddChannelArgument(GRPC_ARG_HTTP2_STREAM_LOOKAHEAD_BYTES, kStreamWindow);
    b->AddChannelArgument(GRPC_ARG_HTTP2_BDP_PROBE, 0);
  }
}

// R-D2's layout guard, on the host side of every RPC binary. The header asserts the
// layout; this asserts that the code BELOW sets every field it has, so a seventh field
// added to the core fails here rather than arriving uninitialised.
static_assert(sizeof(ak_client_opts) == 24, "ak_client_opts is 24 bytes (6 x 4)");
static_assert(AK_CLIENT_OPTS_FIELDS == 6,
              "ak_client_opts gained a field: set it explicitly in core_opts() below");

// Every field set explicitly, at every call site (R-D2). `max_*_message` match what the
// grpc++ cells set in both pinned and unpinned configurations (kMaxMessage), and Nagle is
// OFF, stated rather than defaulted: ArmoniK ships `tcp_nagle_algorithm = false`, and
// grpc++ sets TCP_NODELAY by default, so both transports agree.
inline ak_client_opts core_opts(uint32_t stream_window, uint32_t connection_window,
                                int32_t adaptive_window) {
  ak_client_opts o;
  std::memset(&o, 0, sizeof o);
  o.stream_window = stream_window;
  o.connection_window = connection_window;
  o.adaptive_window = adaptive_window;
  o.max_recv_message = (uint32_t)kMaxMessage;
  o.max_send_message = (uint32_t)kMaxMessage;
  o.tcp_nagle = 0;
  return o;
}

inline ak_client_opts pinned_core_opts(bool pin) {
  return core_opts(pin ? (uint32_t)kStreamWindow : 0u, pin ? (uint32_t)kConnWindow : 0u,
                   pin ? 0 : -1);
}

// ---- clocks ---------------------------------------------------------------------------
inline double thread_cpu_ns() {
  struct timespec ts;
  clock_gettime(CLOCK_THREAD_CPUTIME_ID, &ts);
  return (double)ts.tv_sec * 1e9 + (double)ts.tv_nsec;
}

inline double cpu_ns() {
  struct rusage r;
  getrusage(RUSAGE_SELF, &r);
  return (double)(r.ru_utime.tv_sec + r.ru_stime.tv_sec) * 1e9 +
         (double)(r.ru_utime.tv_usec + r.ru_stime.tv_usec) * 1e3;
}

inline double wall_ns() {
  struct timespec ts;
  clock_gettime(CLOCK_MONOTONIC, &ts);
  return (double)ts.tv_sec * 1e9 + (double)ts.tv_nsec;
}

// ---- the server every cell is measured against -----------------------------------------
//
// ONE server for all four cells, which is what makes B - A a transport difference: the
// server side is byte-for-byte the same work whichever client asked. Its own CPU is
// measured the way the clients' is and subtracted from both, which is C11 in the defect
// log and the reason the ratio is trustworthy.
extern std::atomic<double> g_server_cpu;

class ShapesImpl final : public svcns::Shapes::Service {
 public:
  ShapesImpl();
  grpc::Status Fetch(grpc::ServerContext *, const svcns::Empty *,
                     svcns::ListTasksDetailedResponse *out) override;
  grpc::Status Ping(grpc::ServerContext *, const svcns::Empty *,
                    svcns::Empty *) override;
  const svcns::ListTasksDetailedResponse &payload() const { return resp_; }

 private:
  svcns::ListTasksDetailedResponse resp_;
};

inline double server_cpu() { return g_server_cpu.load(std::memory_order_relaxed); }

// A transport the whole grid shares. `uds` decides which, and both the grpc++ target and
// the core's URI are derived from the same string so the two arms cannot end up on
// different sockets.
struct Transport {
  bool uds;
  std::string sock;        // the UDS path, empty for TCP
  std::string grpc_target; // what grpc++ dials and listens on
  std::string core_uri;    // what ak_client_new_opts dials
  const char *label;
};

inline Transport make_transport(bool uds, int port) {
  Transport t;
  t.uds = uds;
  if (uds) {
    char p[256];
    std::snprintf(p, sizeof p, "/tmp/ak-rpc-%d.sock", (int)getpid());
    ::unlink(p);
    t.sock = p;
    t.grpc_target = std::string("unix:") + p;
    t.core_uri = std::string("unix:") + p;
    t.label = "uds";
  } else {
    char a[64];
    std::snprintf(a, sizeof a, "127.0.0.1:%d", port);
    t.grpc_target = a;
    t.core_uri = std::string("http://") + a;
    t.label = "tcp";
  }
  return t;
}

}  // namespace akrpc

#endif  // AK_RPC_COMMON_H
