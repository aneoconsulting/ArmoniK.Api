#pragma once
#include <iostream>
#include <memory>
#include <string>

#include <grpc++/grpc++.h>

#include "grpcpp/support/sync_stream.h"
#include "objects.pb.h"

#include "utils/Configuration.h"
#include "utils/WorkerServer.h"
#include "worker_common.pb.h"
#include "worker_service.grpc.pb.h"

#include "ProcessStatus.h"
#include "Worker/TaskHandler.h"

namespace armonik {
namespace api {
namespace worker {

class ArmoniKWorker : public armonik::api::grpc::v1::worker::Worker::Service {
private:
  armonik::api::common::logger::Logger logger_;
  std::unique_ptr<armonik::api::grpc::v1::agent::Agent::Stub> agent_;

public:
  /**
   * @brief Constructs a ArmoniKWorker object.
   */
  ArmoniKWorker(std::unique_ptr<armonik::api::grpc::v1::agent::Agent::Stub> agent);

  /**
   * @brief Implements the Process method of the Worker service.
   *
   * @param context The ServerContext object.
   * @param reader The request iterator
   * @param response The ProcessReply object.
   *
   * @return The status of the method.
   */
  [[maybe_unused]] ::grpc::Status
  Process(::grpc::ServerContext *context,
          ::grpc::ServerReader<::armonik::api::grpc::v1::worker::ProcessRequest> *reader,
          ::armonik::api::grpc::v1::worker::ProcessReply *response) override;

  /**
   * @brief Function which does the actual work
   * @param taskHandler Task handler
   * @return Process status
   */
  virtual ProcessStatus Execute(TaskHandler &taskHandler) = 0;

  /**
   * @brief Implements the HealthCheck method of the Worker service.
   *
   * @param context The ServerContext object.
   * @param request The Empty object.
   * @param response The HealthCheckReply object.
   *
   * @return The status of the method.
   */
  [[maybe_unused]] ::grpc::Status HealthCheck(::grpc::ServerContext *context,
                                              const ::armonik::api::grpc::v1::Empty *request,
                                              ::armonik::api::grpc::v1::worker::HealthCheckReply *response) override;
};

} // namespace worker
} // namespace api
} // namespace armonik
