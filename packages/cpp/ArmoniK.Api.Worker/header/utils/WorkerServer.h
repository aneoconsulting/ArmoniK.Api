/**
 * @file WorkerServer.h
 * @brief Contains the WorkerServer class, which represents the worker server for ArmoniK API.
 */
#pragma once
#include <functional>
#include <grpcpp/create_channel.h>
#include <grpcpp/security/credentials.h>
#include <memory>

#include "grpcpp/server_builder.h"

#include "agent_common.pb.h"
#include "agent_service.grpc.pb.h"

#include "Worker/ProcessStatus.h"
#include "Worker/TaskHandler.h"
#include "options/ComputePlane.h"
#include "serilog/SerilogContext.h"
#include "serilog/serilog.h"
#include "utils/IConfiguration.h"

using namespace armonik::api::grpc::v1::agent;

namespace API_WORKER_NAMESPACE {
/**
 * @class WorkerServer
 * @brief Represents the worker server for ArmoniK API.
 */
class WorkerServer {
public:
  Common::serilog::serilog logger;

private:
  ::grpc::ServerBuilder builder_;
  Common::utils::IConfiguration configuration_;
  std::unique_ptr<::grpc::Server> instance_server; ///< Unique pointer to the gRPC server instance
  std::shared_ptr<::grpc::Channel> channel;        ///< Shared pointer to the gRPC channel
  std::unique_ptr<Agent::Stub> agent_stub;         ///< Proxy to communicate with the agent

public:
  /**
   * @brief Constructor for the WorkerServer class.
   * @param configuration A shared pointer to the IConfiguration object.
   */
  WorkerServer(const ArmoniK::Api::Common::utils::IConfiguration &configuration) : configuration_(configuration) {
    logger.enrich([&](Common::serilog::serilog_context &ctx) { ctx.add("threadId", std::this_thread::get_id()); });
    logger.add_property("container", "ArmoniK.Worker");
    logger.info("Creating worker");
    Common::options::ComputePlane compute_plane(configuration);

    builder_.AddListeningPort(std::string(compute_plane.get_server_address()), ::grpc::InsecureServerCredentials());
    builder_.SetMaxReceiveMessageSize(-1);

    logger.info("Initialize and register worker");

    // Create a gRPC channel to communicate with the server
    channel = CreateChannel(std::string(compute_plane.get_agent_address()), ::grpc::InsecureChannelCredentials());

    // Create a stub for the Submitter service
    agent_stub = Agent::NewStub(channel);
  }

  /**
   * @brief Create a WorkerServer instance with the given configuration.
   * @tparam Worker The worker class to be used
   * @tparam Args Argument types to construct the worker, apart from the agent stub
   * @param configuration Shared pointer to the IConfiguration object
   * @param args Arguments to construct the worker, apart from the agent stub
   * @return A shared pointer to the created WorkerServer instance
   */
  template <class Worker, typename... Args>
  static std::shared_ptr<WorkerServer> create(const Common::utils::IConfiguration &configuration, Args... args) {
    auto worker_server = std::make_shared<WorkerServer>(configuration);
    worker_server->builder_.RegisterService(new Worker(std::move(worker_server->agent_stub), args...));
    worker_server->logger.info("Finish to register new worker");

    return worker_server;
  }

  /**
   * @brief Create a WorkerServer instance with the given configuration.
   * @tparam Worker The worker class to be used
   * @param configuration Shared pointer to the IConfiguration object
   * @return A shared pointer to the created WorkerServer instance
   */
  template <class Worker>
  static std::shared_ptr<WorkerServer> create(const Common::utils::IConfiguration &configuration) {
    auto worker_server = std::make_shared<WorkerServer>(configuration);
    worker_server->builder_.RegisterService(new Worker(std::move(worker_server->agent_stub)));
    worker_server->logger.info("Finish to register new worker");

    return worker_server;
  }

  void run() {
    instance_server = builder_.BuildAndStart();
    instance_server->Wait();
  }
};
} // namespace API_WORKER_NAMESPACE
