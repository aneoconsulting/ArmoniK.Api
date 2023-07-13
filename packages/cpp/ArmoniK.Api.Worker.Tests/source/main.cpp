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
#include "Worker/TaskHandler.h"

using grpc::Channel;
using grpc::ClientContext;
using grpc::Status;

using armonik::api::common::utils::IConfiguration;
using armonik::api::grpc::v1::TaskOptions;

using namespace armonik::api::grpc::v1::worker;
using namespace armonik::api::worker;
using namespace armonik::api::common::utils;

int main(int argc, char **argv) {
  std::cout << "Starting C++ worker..." << std::endl;

  std::shared_ptr<IConfiguration> config = std::make_shared<IConfiguration>();

  config->set("ComputePlane__WorkerChannel__Address", "/cache/armonik_worker.sock");
  config->set("ComputePlane__AgentChannel__Address", "/cache/armonik_agent.sock");

  config->get_compute_plane();
  WorkerServer::create<ArmoniK::Api::Worker::ArmoniKWorker, bool>(config)->run();

  std::cout << "Stooping Server..." << std::endl;
  return 0;
}
