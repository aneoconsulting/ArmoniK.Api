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
#include "utils/Configuration.h"

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
  std::unique_ptr<::grpc::Server> instance_server; ///< Unique pointer to the gRPC server instance
  std::shared_ptr<::grpc::Channel> channel;        ///< Shared pointer to the gRPC channel

public:
  /**
   * @brief Constructor for the WorkerServer class.
   * @param configuration A shared pointer to the Configuration object.
   */
  explicit WorkerServer(const Common::utils::Configuration &configuration) {
    logger.enrich([](Common::serilog::serilog_context &ctx) { ctx.add("threadId", std::this_thread::get_id()); });
    logger.add_property("container", "ArmoniK.Worker");
    logger.info("Creating worker");
    Common::options::ComputePlane compute_plane(configuration);

    builder_.AddListeningPort(compute_plane.get_server_address(), ::grpc::InsecureServerCredentials());
    builder_.SetMaxReceiveMessageSize(-1);

    logger.info("Initialize and register worker");

    // Create a gRPC channel to communicate with the server
    channel = CreateChannel(compute_plane.get_agent_address(), ::grpc::InsecureChannelCredentials());
  }

  /**
   * @brief Create a WorkerServer instance with the given configuration.
   * @tparam Worker The worker class to be used
   * @tparam Args Argument types to construct the worker, apart from the agent stub
   * @param configuration Shared pointer to the Configuration object
   * @param args Arguments to construct the worker, apart from the agent stub
   * @return A shared pointer to the created WorkerServer instance
   */
  template <class Worker, typename... Args>
  static std::shared_ptr<WorkerServer> create(Common::utils::Configuration configuration, Args &&...args) {
    auto worker_server = std::make_shared<WorkerServer>(std::move(configuration));
    worker_server->builder_.RegisterService(
        new Worker(Agent::NewStub(worker_server->channel), static_cast<Args &&>(args)...));
    worker_server->logger.info("Finish to register new worker");

    return worker_server;
  }

  void run() {
    instance_server = builder_.BuildAndStart();
    instance_server->Wait();
  }
};
} // namespace API_WORKER_NAMESPACE
