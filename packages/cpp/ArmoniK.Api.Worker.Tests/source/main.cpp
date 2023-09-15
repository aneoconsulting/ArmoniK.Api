#include <iostream>
#include <memory>

#include <grpc++/grpc++.h>

#include "grpcpp/support/sync_stream.h"
#include "objects.pb.h"

#include "utils/WorkerServer.h"

#include "Worker/ArmoniKWorker.h"
#include "Worker/ProcessStatus.h"
#include "Worker/TaskHandler.h"
#include "exceptions/ArmoniKApiException.h"

using grpc::Channel;
using grpc::ClientContext;
using grpc::Status;

using armonik::api::common::utils::Configuration;
using armonik::api::grpc::v1::TaskOptions;

using namespace armonik::api::grpc::v1::worker;
using namespace armonik::api::common::utils;

class TestWorker : public armonik::api::worker::ArmoniKWorker {
public:
  explicit TestWorker(std::unique_ptr<armonik::api::grpc::v1::agent::Agent::Stub> agent)
      : ArmoniKWorker(std::move(agent)) {}

  armonik::api::worker::ProcessStatus Execute(armonik::api::worker::TaskHandler &taskHandler) override {
    std::cout << "Call computer" << std::endl;
    std::cout << "SizePayload : " << taskHandler.getPayload().size()
              << "\nSize DD : " << taskHandler.getDataDependencies().size()
              << "\n Expected results : " << taskHandler.getExpectedResults().size() << std::endl;

    try {
      if (!taskHandler.getExpectedResults().empty()) {
        auto res = taskHandler.send_result(taskHandler.getExpectedResults()[0], taskHandler.getPayload()).get();
        if (res.has_error()) {
          throw armonik::api::common::exceptions::ArmoniKApiException(res.error());
        }
      }

    } catch (const std::exception &e) {
      std::cout << "Error sending result " << e.what() << std::endl;
      return armonik::api::worker::ProcessStatus(e.what());
    }

    return armonik::api::worker::ProcessStatus::Ok;
  }
};

int main(int argc, char **argv) {
  std::cout << "Starting C++ worker..." << std::endl;

  Configuration config;
  config.add_json_configuration("appsettings.json").add_env_configuration();

  config.set("ComputePlane__WorkerChannel__Address", "/cache/armonik_worker.sock");
  config.set("ComputePlane__AgentChannel__Address", "/cache/armonik_agent.sock");

  try {
    armonik::api::worker::WorkerServer::create<TestWorker>(config)->run();
  } catch (const std::exception &e) {
    std::cout << "Error in worker" << e.what() << std::endl;
  }

  std::cout << "Stopping Server..." << std::endl;
  return 0;
}
