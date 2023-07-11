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

#include "Worker/ArmoniKWorker.h"
#include "Worker/TaskHandler.h"

using grpc::Channel;
using grpc::ClientContext;
using grpc::Status;

using armonik::api::common::utils::IConfiguration;
using armonik::api::grpc::v1::TaskOptions;

using namespace armonik::api::grpc::v1::worker;
using namespace armonik::api::worker;
using namespace armonik::api::common::utils;

/**
 * @brief Constructs a ArmoniKWorker object.
 */
ArmoniKWorker::ArmoniKWorker(std::unique_ptr<armonik::api::grpc::v1::agent::Agent::Stub> agent,
                             void (*processing_function)(TaskHandler task_handler))
    : logger_(armonik::api::common::serilog::logging_format::SEQ) {
  logger_.info("Build Service ArmoniKWorker");
  logger_.add_property("class", "ArmoniKWorker");
  logger_.add_property("Worker", "ArmoniK.Api.Cpp");
  agent_ = std::move(agent);
  processing_function_ = processing_function;
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
Status ArmoniKWorker::Process(::grpc::ServerContext *context, ::grpc::ServerReader<ProcessRequest> *reader,
                              ::armonik::api::grpc::v1::worker::ProcessReply *response) {

  logger_.info("Receive new request From C++ real Worker");

  auto output = armonik::api::grpc::v1::Output();
  *output.mutable_ok() = armonik::api::grpc::v1::Empty();
  // ProcessRequest req;
  // reader->Read(&req);
  *response->mutable_output() = output;

  std::shared_ptr<grpc::ServerReader<ProcessRequest>> request_iterator =
      std::make_shared<grpc::ServerReader<ProcessRequest>>(*reader);

  TaskHandler task_handler(std::move(agent_), request_iterator);

  task_handler.init();

  logger_.info("Finish call C++");

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
Status ArmoniKWorker::HealthCheck(::grpc::ServerContext *context, const ::armonik::api::grpc::v1::Empty *request,
                                  ::armonik::api::grpc::v1::worker::HealthCheckReply *response) {
  // Implementation of the HealthCheck method
  logger_.info("HealthCheck request OK");

  response->set_status(HealthCheckReply_ServingStatus_SERVING);

  return Status::OK;
}
