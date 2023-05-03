#pragma once
#include <sstream>

/**
 * @brief The armonik namespace contains classes and functions related to the Armonik API.
 */
namespace armonik::api::common::options
{
  /**
   * @brief The ComputePlane class manages the communication addresses for workers and agents.
   */
  class ComputePlane
  {
  public:
    /**
     * @brief Constructs a ComputePlane object with the given configuration.
     * @param configuration The IConfiguration object containing address information.
     */
    ComputePlane(const utils::IConfiguration& configuration)
    {
      set_worker_address(configuration.get("ComputePlane__WorkerChannel__Address"));
      set_agent_address(configuration.get(std::string("ComputePlane__AgentChannel__Address")));
    }

    /**
     * @brief Returns the server address.
     * @return A reference to the server address string.
     */
    std::string& get_server_address()
    {
      return worker_address_;
    }

    /**
     * @brief Sets the worker address with the given socket address.
     * @param socket_address The socket address to set for the worker.
     */
    void set_worker_address(const std::string& socket_address)
    {
      if (socket_address.find("unix:") == std::string::npos)
      {
        std::stringstream out;
        out << "unix:" << socket_address;
        worker_address_ = out.str();
      }
      else
      {
        worker_address_ = socket_address;
      }
    }

    /**
     * @brief Sets the agent address with the given agent address.
     * @param agent_address The agent address to set for the agent.
     */
    void set_agent_address(const std::string& agent_address)
    {
      if (agent_address.find("unix:") == std::string::npos)
      {
        std::stringstream out;
        out << "unix:" << agent_address;
        agent_address_ = out.str();
      }
      else
      {
        agent_address_ = agent_address;
      }
    }

    /**
     * @brief Returns the agent address.
     * @return A reference to the agent address string.
     */
    std::string& get_agent_address()
    {
      return agent_address_;
    }

  private:
    std::string worker_address_; ///< The worker address string.
    std::string agent_address_; ///< The agent address string.
  };
};
