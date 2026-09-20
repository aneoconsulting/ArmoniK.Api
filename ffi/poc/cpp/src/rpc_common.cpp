#include "rpc_common.h"

#include "generated/pb_build.h"

namespace akrpc {

std::atomic<double> g_server_cpu(0.0);

ShapesImpl::ShapesImpl() { pbbuild::payload_p2_2(&resp_); }

grpc::Status ShapesImpl::Fetch(grpc::ServerContext *, const svcns::Empty *,
                               svcns::ListTasksDetailedResponse *out) {
  // The handler's own CPU, so it can be subtracted from the process figure for EVERY cell,
  // measured the same way for all of them. It does not cover the server's transport
  // threads; what it does cover is the deep copy and the serialisation, which is the bulk.
  double a = thread_cpu_ns();
  *out = resp_;
  double d = thread_cpu_ns() - a;
  double cur = g_server_cpu.load(std::memory_order_relaxed);
  while (!g_server_cpu.compare_exchange_weak(cur, cur + d)) {
  }
  return grpc::Status::OK;
}

grpc::Status ShapesImpl::Ping(grpc::ServerContext *, const svcns::Empty *, svcns::Empty *) {
  return grpc::Status::OK;
}

}  // namespace akrpc
