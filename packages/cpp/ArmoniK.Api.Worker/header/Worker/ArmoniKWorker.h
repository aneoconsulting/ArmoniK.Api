#include <iostream>
#include <memory>
#include <string>

#include <grpc++/grpc++.h>

#include "grpcpp/support/sync_stream.h"
#include "objects.pb.h"

#include "utils/IConfiguration.h"
#include "utils/WorkerServer.h"
#include "worker_common.pb.h"
#include "worker_service.grpc.pb.h"

#include "Worker/TaskHandler.h"

namespace API_WORKER_NAMESPACE {

class ArmoniKWorker final : public armonik::api::grpc::v1::worker::Worker::Service {
private:
  armonik::api::common::serilog::serilog logger_;
  std::unique_ptr<armonik::api::grpc::v1::agent::Agent::Stub> agent_;
  void (*processing_function_)(TaskHandler taskHandler);

public:
  /**
   * @brief Constructs a ArmoniKWorker object.
   */
  ArmoniKWorker(std::unique_ptr<armonik::api::grpc::v1::agent::Agent::Stub> agent,
                void (*processing_function)(TaskHandler task_handler));

  /**
   * @brief Implements the Process method of the Worker service.
   *
   * @param context The ServerContext object.
   * @param reader The request iterator
   * @param response The ProcessReply object.
   *
   * @return The status of the method.
   */
  grpc::Status Process(::grpc::ServerContext *context,
                       ::grpc::ServerReader<::armonik::api::grpc::v1::worker::ProcessRequest> *reader,
                       ::armonik::api::grpc::v1::worker::ProcessReply *response) override;

  /**
   * @brief Implements the HealthCheck method of the Worker service.
   *
   * @param context The ServerContext object.
   * @param request The Empty object.
   * @param response The HealthCheckReply object.
   *
   * @return The status of the method.
   */
  grpc::Status HealthCheck(::grpc::ServerContext *context, const ::armonik::api::grpc::v1::Empty *request,
                           ::armonik::api::grpc::v1::worker::HealthCheckReply *response) override;
};

} // namespace API_WORKER_NAMESPACE
