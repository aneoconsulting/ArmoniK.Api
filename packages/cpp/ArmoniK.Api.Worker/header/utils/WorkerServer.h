/**
 * @file WorkerServer.h
 * @brief Contains the WorkerServer class, which represents the worker server for ArmoniK API.
 */
#pragma once
#include <functional>
#include <grpcpp/create_channel.h>
#include <grpcpp/security/credentials.h>
#include <memory>
#include <sstream>
#include <fstream>
#include <thread>

#include "grpcpp/server_builder.h"
#include "grpcpp/security/server_credentials.h"

#include "agent_common.pb.h"
#include "agent_service.grpc.pb.h"

#include "Worker/ProcessStatus.h"
#include "Worker/TaskHandler.h"
#include "exceptions/ArmoniKApiException.h"
#include "logger/formatter.h"
#include "logger/logger.h"
#include "logger/writer.h"
#include "options/ComputePlane.h"
#include "utils/Configuration.h"

using namespace armonik::api::grpc::v1::agent;

namespace armonik {
namespace api {
namespace worker {
/**
 * @class WorkerServer
 * @brief Represents the worker server for ArmoniK API.
 */
class WorkerServer {
public:
  common::logger::Logger logger;

private:
  ::grpc::ServerBuilder builder_;
  std::unique_ptr<::grpc::Server> instance_server; ///< Unique pointer to the gRPC server instance
  std::shared_ptr<::grpc::Channel> channel;        ///< Shared pointer to the gRPC channel

public:
  /**
   * @brief Constructor for the WorkerServer class.
   * @param configuration A shared pointer to the Configuration object.
   */
  explicit WorkerServer(const common::utils::Configuration &configuration)
      : logger(common::logger::writer_console(), common::logger::formatter_clef()) {
    logger.local_context_generator_add("threadId", []() {
      std::stringstream ss;
      ss << std::this_thread::get_id();
      return ss.str();
    });
    logger.global_context_add("container", "ArmoniK.Worker");
    logger.info("Creating worker");
    common::options::ComputePlane compute_plane(configuration);

    logger.info("Worker address : " + compute_plane.get_server_address());
    logger.info("Agent address : " + compute_plane.get_agent_address());

    std::string server_key = readFile("/mnt/c/Users/ddiakite/Programmation/ArmoniK.Extensions.Cpp/certs/server-key.pem"); // The server private key
    std::string server_cert = readFile("/mnt/c/Users/ddiakite/Programmation/ArmoniK.Extensions.Cpp/certs/server-cert.pem"); // The server certificate
    std::string ca_cert = readFile("/mnt/c/Users/ddiakite/Programmation/ArmoniK.Extensions.Cpp/certs/ca-cert.pem"); // The server certificate

    ::grpc::SslServerCredentialsOptions ssl_opts;
  
    ssl_opts.pem_root_certs = ca_cert;
    ssl_opts.pem_key_cert_pairs.push_back({server_key, server_cert});

    ssl_opts.client_certificate_request =
        GRPC_SSL_REQUEST_AND_REQUIRE_CLIENT_CERTIFICATE_AND_VERIFY;

    builder_.AddListeningPort(compute_plane.get_server_address(), ::grpc::SslServerCredentials(ssl_opts)); // 
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
  static std::shared_ptr<WorkerServer> create(common::utils::Configuration configuration, Args &&...args) {
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

  static std::string readFile(const std::string& filename) {
    std::ifstream file(filename);
    if (!file) {
        throw std::runtime_error("Unable to open file: " + filename);
    }
    return std::string((std::istreambuf_iterator<char>(file)),
                       std::istreambuf_iterator<char>());
}
};
} // namespace worker
} // namespace api
} // namespace armonik
