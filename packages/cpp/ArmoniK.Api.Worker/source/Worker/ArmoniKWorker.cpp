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
#include "Worker/ArmoniKWorker.h"


using grpc::Channel;
using grpc::ClientContext;
using grpc::Status;

using armonik::api::common::utils::IConfiguration;
using armonik::api::grpc::v1::TaskOptions;

using namespace armonik::api::grpc::v1::worker;
using namespace armonik::api::worker;
using namespace armonik::api::common::utils;

/**
* @brief Implements the Process method of the Worker service.
*
* @param agent The agent object.
* @param request_iterator The request iterator
*
* @return The status of the method.
*/
Status ArmoniKWorker::Process(
    std::unique_ptr<Agent::Stub> agent,
               std::unique_ptr<grpc::ClientReader<ProcessRequest>> request_iterator) {

  logger_.info("Receive new request From C++ Worker");
  TaskHandler taskHandler(std::move(agent), std::move(request_iterator));
  taskHandler.init();
  std::string key;
  std::vector<std::byte> data{'a'};
  auto result = taskHandler.send_result(key, data);

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
