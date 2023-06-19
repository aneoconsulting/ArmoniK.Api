/**
 * @file IConfiguration.h
 * @brief Interface for a configuration class that stores and manages key-value pairs.
 */

#pragma once

#include <memory>
#include <string>
#include <unordered_map>

namespace armonik::api::common::options
{
  class ComputePlane;
}

namespace armonik::api::common::utils
{
  /**
   * @class IConfiguration
   * @brief Interface for a configuration class that stores and manages key-value pairs.
   */
  class IConfiguration
  {
  public:
    /**
     * @brief Default constructor.
     */
    IConfiguration()
    {
    }

    /**
     * @brief Default virtual destructor.
     */
    virtual ~IConfiguration() = default;

    /**
     * @brief Get the value associated with the given key.
     * @param string Key to look up.
     * @return The value associated with the key, as a string.
     */
    [[nodiscard]] virtual std::string get(const std::string& string) const = 0;

    /**
     * @brief Set the value associated with the given key.
     * @param string Key to set the value for.
     * @param value Value to set for the key.
     */
    virtual void set(const std::string& string, const std::string& value) = 0;

    /**
     * @brief Set the values from another IConfiguration object.
     * @param other IConfiguration object to copy values from.
     */
    virtual void set(const IConfiguration& other) = 0;

    /**
     * @brief Add JSON configuration from a file.
     * @param file_path Path to the JSON file.
     * @return Reference to the current IConfiguration object.
     */
    IConfiguration& add_json_configuration(const std::string& file_path);

    /**
     * @brief Add environment variable configuration.
     * @return Reference to the current IConfiguration object.
     */
    IConfiguration& add_env_configuration();

    /**
     * @brief Get the current ComputePlane configuration.
     * @return A ComputePlane object representing the current configuration.
     */
    options::ComputePlane get_compute_plane();
  };
}
