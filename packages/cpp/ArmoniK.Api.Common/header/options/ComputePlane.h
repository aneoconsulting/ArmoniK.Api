#pragma once
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
    set_agent_address(configuration.get(std::string("ComputePlane__AgentChannel__Address")));
  }

  /**
   * @brief Returns the server address.
   * @return A reference to the server address string.
   */
  [[nodiscard]] const std::string &get_server_address() const { return worker_address_; }

  static bool starts_with(absl::string_view s, absl::string_view prefix) {
  return s.size() >= prefix.size() && s.substr(0, prefix.size()) == prefix;
  }

  /**
   * @brief Strips http/https schemes and ensures a valid gRPC address format.
   * @param address The address to normalize.
   * @return The normalized address.
   */
   static std::string normalize_address(std::string address) {
     absl::string_view av(address);
     if (starts_with(av, "https://")) {
       return std::string(av.substr(8).begin(), av.substr(8).end());
     }
     if (starts_with(av, "http://")) {
       return std::string(av.substr(7).begin(), av.substr(7).end());
     }
     // No recognized leading scheme: assume unix socket"
     return std::string("unix://") + address;
   }

  /**
   * @brief Sets the worker address with the given socket address.
   * @param socket_address The socket address to set for the worker.
   */
  void set_worker_address(std::string socket_address) {
      worker_address_ = normalize_address(socket_address);
  }

  /**
   * @brief Sets the agent address with the given agent address.
   * @param agent_address The agent address to set for the agent.
   */
  void set_agent_address(std::string agent_address) {
      agent_address_ = normalize_address(agent_address);
  }
  /**
   * @brief Returns the agent address.
   * @return A reference to the agent address string.
   */
  [[nodiscard]] const std::string &get_agent_address() const { return agent_address_; }

private:
  std::string worker_address_; ///< The worker address string.
  std::string agent_address_;  ///< The agent address string.
};
} // namespace options
} // namespace common
} // namespace api
} // namespace armonik
