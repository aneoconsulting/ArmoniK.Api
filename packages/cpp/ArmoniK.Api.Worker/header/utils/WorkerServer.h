/**
 * @file WorkerServer.h
 * @brief Contains the WorkerServer class, which represents the worker server for ArmoniK API.
 */
#pragma once
#include <functional>
#include <memory>
#include <grpcpp/create_channel.h>
#include <grpcpp/security/credentials.h>

#include "grpcpp/server_builder.h"

#include "agent_service.grpc.pb.h"
#include "agent_common.pb.h"

#include "utils/IConfiguration.h"
#include "options/ComputePlane.h"
#include "serilog/serilog.h"
#include "serilog/SerilogContext.h"

using namespace armonik::api::grpc::v1::agent;

namespace armonik::api::worker
{
  /**
   * @class WorkerServer
   * @brief Represents the worker server for ArmoniK API.
   */
  class WorkerServer
  {
  public:
    common::serilog::serilog logger;

  private:
    ::grpc::ServerBuilder builder_;
    std::shared_ptr<common::utils::IConfiguration> configuration_;

  public:
    /**
     * @brief Constructor for the WorkerServer class.
     * @param configuration A shared pointer to the IConfiguration object.
     */
    WorkerServer(std::shared_ptr<common::utils::IConfiguration> configuration) : configuration_(
      std::move(configuration))
    {
      logger.enrich([&](common::serilog::serilog_context& ctx)
      {
        ctx.add("threadId", std::this_thread::get_id());
      });

      logger.add_property("container", "ArmoniK.Worker");
    }


    /**
     * @brief Create a WorkerServer instance with the given configuration.
     * @tparam Worker The worker class to be used
     * @tparam Collection The collection class to be used
     * @param configuration Shared pointer to the IConfiguration object
     * @param service_configurator Function pointer for the service configurator
     * @return A shared pointer to the created WorkerServer instance
     */
    template <class Worker, class Collection>
    static std::shared_ptr<
      WorkerServer> create(const std::shared_ptr<common::utils::IConfiguration> configuration,
                           [[maybe_unused]] std::function<void(Collection& collection)> service_configurator = nullptr)
    {
      configuration->add_json_configuration("appsetting.json").add_env_configuration();
      auto worker_server = std::make_shared<WorkerServer>(configuration);
      worker_server->logger.info("Creating worker");
      common::options::ComputePlane compute_plane(*configuration);

      worker_server->builder_.AddListeningPort(compute_plane.get_server_address(), ::grpc::InsecureServerCredentials());
      worker_server->builder_.SetMaxReceiveMessageSize(-1);

      worker_server->logger.info("Initialize and register worker");

      // Create a gRPC channel to communicate with the server
      worker_server->channel = CreateChannel(compute_plane.get_agent_address(), ::grpc::InsecureChannelCredentials());

      // Create a stub for the Submitter service
      worker_server->agent_stub = Agent::NewStub(worker_server->channel);

      worker_server->builder_.RegisterService(new Worker());
      worker_server->logger.info("Finish to register new worker");

      return worker_server;
    }


    std::unique_ptr<::grpc::Server> instance_server; ///< Unique pointer to the gRPC server instance
    std::shared_ptr<::grpc::Channel> channel; ///< Shared pointer to the gRPC channel
    std::unique_ptr<Agent::Stub> agent_stub; ///< Proxy to communicate with the agent

    void run()
    {
      instance_server = builder_.BuildAndStart();
      instance_server->Wait();
    }
  };
}
