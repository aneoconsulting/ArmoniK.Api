#pragma once
#include "GrpcSocketType.h"
#include <sstream>
#include <utils/Configuration.h>

/**
 * @brief The armonik namespace contains classes and functions related to the ArmoniK API.
 */
namespace armonik {
namespace api {
namespace common {
namespace options {
/**
 * @brief The ComputePlane class manages the communication addresses for workers and agents.
 */
class ComputePlane {
public:
  /**
   * @brief Constructs a ComputePlane object with the given configuration.
   * @param configuration The Configuration object containing address information.
   */
  ComputePlane(const utils::Configuration &configuration) {
    set_worker_address(configuration.get("ComputePlane__WorkerChannel__Address"));
    set_worker_socket_type(configuration.get("ComputePlane__WorkerChannel__SocketType"));
    set_agent_address(configuration.get(std::string("ComputePlane__AgentChannel__Address")));
    set_agent_socket_type(configuration.get("ComputePlane__AgentChannel__SocketType"));
  }

  /**
   * @brief Returns the server address.
   * @return A reference to the server address string.
   */
  [[nodiscard]] const std::string &get_server_address() const { return worker_address_; }

  /**
   * @brief Returns the server socket type.
   * @return The server socket type as a grpc_socket_type enum value.
   */
  [[nodiscard]] grpc_socket_type get_server_socket_type() const { return worker_socket_type_; }

  static bool starts_with(absl::string_view s, absl::string_view prefix) {
    return s.size() >= prefix.size() && s.substr(0, prefix.size()) == prefix;
  }

  /**
   * @brief Strips http/https schemes and ensures a valid gRPC address format.
   * @param address The address to normalize, provided as an absl::string_view.
   * @return The normalized address.
   */
  static std::string normalize_address(absl::string_view address) {
    if (starts_with(address, "http://")) {
      return std::string(address.substr(7).begin(), address.substr(7).end());
    }
    // No recognized leading scheme: assume unix socket"
    return std::string("unix://") + std::string(address);
  }

  /**
   * @brief Sets the worker address with the given socket address.
   * @param worker_address The socket address to set for the worker.
   */
  void set_worker_address(absl::string_view worker_address) { worker_address_ = normalize_address(worker_address); }

  /**
   * @brief Sets the worker socket type
   * @param socket_type The socket type string from configuration.
   */
  void set_worker_socket_type(absl::string_view socket_type) {
    if (starts_with(socket_type, "tcp")) {
      worker_socket_type_ = grpc_socket_type::tcp;
    } else {
      worker_socket_type_ = grpc_socket_type::UnixDomainSocket;
    }
  }

  /**
   * @brief Sets the agent address with the given agent address.
   * @param agent_address The agent address to set for the agent.
   */
  void set_agent_address(absl::string_view agent_address) { agent_address_ = normalize_address(agent_address); }

  /**
   * @brief Sets the worker socket type
   * @param socket_address The socket type string from configuration.
   */
  void set_agent_socket_type(absl::string_view socket_type) {
    if (starts_with(socket_type, "tcp")) {
      agent_socket_type_ = grpc_socket_type::tcp;
    } else {
      agent_socket_type_ = grpc_socket_type::UnixDomainSocket;
    }
  }

  /**
   * @brief Returns the agent address.
   * @return A reference to the agent address string.
   */
  [[nodiscard]] const std::string &get_agent_address() const { return agent_address_; }

  /**
   * @brief Returns the agent socket type.
   * @return The agent socket type as a grpc_socket_type enum value.
   */
  [[nodiscard]] grpc_socket_type get_agent_socket_type() const { return agent_socket_type_; }

private:
  std::string worker_address_; ///< The worker address string.
  std::string agent_address_;  ///< The agent address string.
  grpc_socket_type worker_socket_type_;
  grpc_socket_type agent_socket_type_;
};
} // namespace options
} // namespace common
} // namespace api
} // namespace armonik
