#include <iostream>
#include <memory>
#include <string>

#include <grpc++/grpc++.h>

#include "grpcpp/support/sync_stream.h"
#include "objects.pb.h"

#include "utils/RootConfiguration.h"
#include "utils/WorkerServer.h"
#include "worker_common.grpc.pb.h"
#include "worker_service.grpc.pb.h"

#include "Worker/TaskHandler.h"

class ArmoniKWorker final : public armonik::api::grpc::v1::worker::Worker::Service {
private:
  armonik::api::common::serilog::serilog logger_;

public:
  /**
   * @brief Constructs a ArmoniKWorker object.
   */
  ArmoniKWorker() : logger_(armonik::api::common::serilog::logging_format::SEQ) {
    logger_.info("Build Service ArmoniKWorker");
    logger_.add_property("class", "ArmoniKWorker");
    logger_.add_property("Worker", "ArmoniK.Api.Cpp");
  }

  grpc::Status
  Process(std::unique_ptr<Agent::Stub> agent,
          std::unique_ptr<grpc::ClientReader<armonik::api::grpc::v1::worker::ProcessRequest>> request_iterator);

  grpc::Status HealthCheck(::grpc::ServerContext *context, const ::armonik::api::grpc::v1::Empty *request,
                           ::armonik::api::grpc::v1::worker::HealthCheckReply *response) override;
};
