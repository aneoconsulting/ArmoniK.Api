#include <iostream>
#include <memory>
#include <string>
#include <utility>

#include <grpc++/grpc++.h>

#include "grpcpp/support/sync_stream.h"
#include "objects.pb.h"

#include "utils/Configuration.h"
#include "utils/WorkerServer.h"
#include "worker_common.pb.h"
#include "worker_service.grpc.pb.h"

#include "Worker/ArmoniKWorker.h"
#include "Worker/TaskHandler.h"

using grpc::Channel;
using grpc::ClientContext;
using grpc::Status;

using armonik::api::common::utils::Configuration;
using armonik::api::grpc::v1::TaskOptions;

using namespace armonik::api::grpc::v1::worker;
using namespace armonik::api::common::utils;

/**
 * @brief Constructs a ArmoniKWorker object.
 */
armonik::api::worker::ArmoniKWorker::ArmoniKWorker(std::unique_ptr<armonik::api::grpc::v1::agent::Agent::Stub> agent)
    : logger_(armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_clef()) {
  logger_.info("Build Service ArmoniKWorker");
  logger_.global_context_add("class", "ArmoniKWorker");
  logger_.global_context_add("Worker", "ArmoniK.Api.Cpp");
  agent_ = std::move(agent);
}

/**
 * @brief Implements the Process method of the Worker service.
 *
 * @param context The ServerContext object.
 * @param reader The request iterator
 * @param response The ProcessReply object.
 *
 * @return The status of the method.
 */
[[maybe_unused]] Status
armonik::api::worker::ArmoniKWorker::Process([[maybe_unused]] ::grpc::ServerContext *context,
                                             ::grpc::ServerReader<ProcessRequest> *reader,
                                             ::armonik::api::grpc::v1::worker::ProcessReply *response) {

  logger_.debug("Receive new request From C++ Worker");

  TaskHandler task_handler(*agent_, *reader);

  task_handler.init();
  try {
    ProcessStatus status = Execute(task_handler);

    logger_.debug("Finish call C++");

    armonik::api::grpc::v1::Output output;
    if (status.ok()) {
      *output.mutable_ok() = armonik::api::grpc::v1::Empty();
    } else {
      output.mutable_error()->set_details(std::move(status).details());
    }
    *response->mutable_output() = std::move(output);
  } catch (const std::exception &e) {
    return {::grpc::StatusCode::UNAVAILABLE, "Error processing task", e.what()};
  }

  return ::grpc::Status::OK;
}

/**
 * @brief Implements the HealthCheck method of the Worker service.
 *
 * @param context The ServerContext object.
 * @param request The Empty object.
 * @param response The HealthCheckReply object.
 *
 * @return The status of the method.
 */
[[maybe_unused]] Status
armonik::api::worker::ArmoniKWorker::HealthCheck([[maybe_unused]] ::grpc::ServerContext *context,
                                                 [[maybe_unused]] const ::armonik::api::grpc::v1::Empty *request,
                                                 ::armonik::api::grpc::v1::worker::HealthCheckReply *response) {
  // Implementation of the HealthCheck method
  logger_.debug("HealthCheck request OK");

  response->set_status(HealthCheckReply_ServingStatus_SERVING);

  return Status::OK;
}
