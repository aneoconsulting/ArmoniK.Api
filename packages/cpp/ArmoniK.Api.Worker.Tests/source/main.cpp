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
#include "exceptions/ArmoniKApiException.h"

using grpc::Channel;
using grpc::ClientContext;
using grpc::Status;

using ArmoniK::Api::Common::utils::IConfiguration;
using armonik::api::grpc::v1::TaskOptions;

using namespace armonik::api::grpc::v1::worker;
using namespace ArmoniK::Api::Common::utils;

class Computer final : public ArmoniK::Api::Worker::ArmoniKWorker {
public:
  explicit Computer(std::unique_ptr<armonik::api::grpc::v1::agent::Agent::Stub> agent)
      : ArmoniKWorker(std::move(agent)) {}

  ArmoniK::Api::Worker::ProcessStatus Execute(ArmoniK::Api::Worker::TaskHandler &taskHandler) override {
    std::cout << "Call computer" << std::endl;
    std::cout << "SizePayload : " << taskHandler.getPayload().size()
              << "\nSize DD : " << taskHandler.getDataDependencies().size()
              << "\n Expected results : " << taskHandler.getExpectedResults().size() << std::endl;

    try {
      if (!taskHandler.getExpectedResults().empty()) {
        auto res = taskHandler.send_result(taskHandler.getExpectedResults()[0], taskHandler.getPayload()).get();
        if (res.has_error()) {
          throw ArmoniK::Api::Common::exceptions::ArmoniKApiException(res.error());
        }
      }

    } catch (const std::exception &e) {
      std::cout << "Error sending result " << e.what() << std::endl;
      return ArmoniK::Api::Worker::ProcessStatus(e.what());
    }

    return ArmoniK::Api::Worker::ProcessStatus::OK;
  }
};

int main(int argc, char **argv) {
  std::cout << "Starting C++ worker..." << std::endl;

  std::shared_ptr<IConfiguration> config = std::make_shared<IConfiguration>();

  config->set("ComputePlane__WorkerChannel__Address", "/cache/armonik_worker.sock");
  config->set("ComputePlane__AgentChannel__Address", "/cache/armonik_agent.sock");

  try {
    ArmoniK::Api::Worker::WorkerServer::create<Computer>(config)->run();
  } catch (const std::exception &e) {
    std::cout << "Error in worker" << e.what() << std::endl;
  }

  std::cout << "Stopping Server..." << std::endl;
  return 0;
}
