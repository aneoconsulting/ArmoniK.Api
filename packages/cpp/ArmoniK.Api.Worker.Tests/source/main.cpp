#include <iostream>
#include <memory>
#include <string>

#include <grpc++/grpc++.h>

#include "grpcpp/support/sync_stream.h"
#include "objects.pb.h"

#include "utils/WorkerServer.h"
#include "worker_common.pb.h"
#include "worker_service.grpc.pb.h"

#include "Worker/ArmoniKWorker.h"
#include "Worker/ProcessStatus.h"
#include "Worker/TaskHandler.h"

using grpc::Channel;
using grpc::ClientContext;
using grpc::Status;

using ArmoniK::Api::Common::utils::IConfiguration;
using armonik::api::grpc::v1::TaskOptions;

using namespace armonik::api::grpc::v1::worker;
using namespace ArmoniK::Api::Common::utils;

ArmoniK::Api::Worker::ProcessStatus computer(ArmoniK::Api::Worker::TaskHandler &handler) {
  handler.send_result(handler.getExpectedResults()[0], "test");
  return ArmoniK::Api::Worker::ProcessStatus::OK;
}

int main(int argc, char **argv) {
  std::cout << "Starting C++ worker..." << std::endl;

  std::shared_ptr<IConfiguration> config = std::make_shared<IConfiguration>();

  config->set("ComputePlane__WorkerChannel__Address", "/cache/armonik_worker.sock");
  config->set("ComputePlane__AgentChannel__Address", "/cache/armonik_agent.sock");

  ArmoniK::Api::Worker::WorkerServer::create<ArmoniK::Api::Worker::ArmoniKWorker>(config, &computer)->run();

  std::cout << "Stooping Server..." << std::endl;
  return 0;
}
