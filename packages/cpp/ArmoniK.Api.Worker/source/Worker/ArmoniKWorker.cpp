#include <iostream>
#include <memory>
#include <string>
#include <utility>

#include <grpc++/grpc++.h>

#include "grpcpp/support/sync_stream.h"
#include "objects.pb.h"

#include "utils/IConfiguration.h"
#include "utils/WorkerServer.h"
#include "worker_common.pb.h"
#include "worker_service.grpc.pb.h"

#include "Worker/ArmoniKWorker.h"
#include "Worker/TaskHandler.h"

using grpc::Channel;
using grpc::ClientContext;
using grpc::Status;

using ArmoniK::Api::Common::utils::IConfiguration;
using armonik::api::grpc::v1::TaskOptions;

using namespace armonik::api::grpc::v1::worker;
using namespace ArmoniK::Api::Common::utils;

/**
 * @brief Constructs a ArmoniKWorker object.
 */
API_WORKER_NAMESPACE::ArmoniKWorker::ArmoniKWorker(std::unique_ptr<armonik::api::grpc::v1::agent::Agent::Stub> agent,
                                                   std::function<ProcessStatus(TaskHandler &)> processing_function)
    : logger_(ArmoniK::Api::Common::serilog::logging_format::SEQ) {
  logger_.info("Build Service ArmoniKWorker");
  logger_.add_property("class", "ArmoniKWorker");
  logger_.add_property("Worker", "ArmoniK.Api.Cpp");
  agent_ = std::move(agent);
  processing_function_ = std::move(processing_function);
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
Status API_WORKER_NAMESPACE::ArmoniKWorker::Process([[maybe_unused]] ::grpc::ServerContext *context,
                                                    ::grpc::ServerReader<ProcessRequest> *reader,
                                                    ::armonik::api::grpc::v1::worker::ProcessReply *response) {

  logger_.debug("Receive new request From C++ Worker");

  // Encapsulate the pointer without deleting it out of scope
  std::shared_ptr<grpc::ServerReader<ProcessRequest>> iterator(reader, [](void *) {});
  std::shared_ptr<armonik::api::grpc::v1::agent::Agent::Stub> agent(agent_.get(), [](void *) {});

  TaskHandler task_handler(agent, iterator);

  task_handler.init();

  ProcessStatus status = processing_function_(task_handler);

  logger_.debug("Finish call C++");

  armonik::api::grpc::v1::Output output;
  if (status.ok()) {
    *output.mutable_ok() = armonik::api::grpc::v1::Empty();
  } else {
    output.mutable_error()->set_details(status.details());
  }

  *response->mutable_output() = output;

  return grpc::Status::OK;
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
Status API_WORKER_NAMESPACE::ArmoniKWorker::HealthCheck([[maybe_unused]] ::grpc::ServerContext *context,
                                                        [[maybe_unused]] const ::armonik::api::grpc::v1::Empty *request,
                                                        ::armonik::api::grpc::v1::worker::HealthCheckReply *response) {
  // Implementation of the HealthCheck method
  logger_.debug("HealthCheck request OK");

  response->set_status(HealthCheckReply_ServingStatus_SERVING);

  return Status::OK;
}
