/**
 * @file Configuration.h
 * @brief Interface for a configuration class that stores and manages key-value pairs.
 */

#pragma once

#include <absl/strings/string_view.h>
#include <map>
#include <memory>
#include <set>
#include <string>
#include <vector>

namespace armonik {
namespace api {
namespace common {
namespace options {
class ComputePlane;
class ControlPlane;
} // namespace options

namespace utils {
/**
 * @class Configuration
 * @brief Interface for a configuration class that stores and manages key-value pairs.
 */
class Configuration {
public:
  Configuration() noexcept = default;
  Configuration(const Configuration &) = default;
  Configuration(Configuration &&) noexcept = default;

  Configuration &operator=(const Configuration &) = default;
  Configuration &operator=(Configuration &&) noexcept = default;
  ~Configuration() = default;
  /**
   * @brief Get the value associated with the given key.
   * @param string Key to look up.
   * @return The value associated with the key, as a string.
   */
  [[nodiscard]] std::string get(const std::string &string) const;

  /**
   * @brief Set the value associated with the given key.
   * @param string Key to set the value for.
   * @param value Value to set for the key.
   */
  void set(const std::string &string, const std::string &value);

  /**
   * @brief Set the values from another Configuration object.
   * @param other Configuration object to copy values from.
   */
  void set(const Configuration &other);

  /**
   * @brief List defined values of this configuration.
   * @note Does not include environment variables
   */
  [[nodiscard]] const std::map<std::string, std::string> &list() const;

  /**
   * @brief Add JSON configuration from a file.
   * @param file_path Path to the JSON file.
   * @return Reference to the current Configuration object.
   */
  Configuration &add_json_configuration(absl::string_view file_path);

  /**
   * @brief Add environment variable configuration.
   * @return Reference to the current Configuration object.
   */
  Configuration &add_env_configuration();

  /**
   * @brief Get the current ComputePlane configuration.
   * @return A ComputePlane object representing the current configuration.
   */
  [[nodiscard]] options::ComputePlane get_compute_plane() const;

  /**
   * @brief Get the current ControlPlane configuration
   * @return A ControlPlane object
   */
  [[nodiscard]] options::ControlPlane get_control_plane() const;

private:
  /**
   * @brief Storage for the key-value pairs.
   */
  std::map<std::string, std::string> options_;
  std::set<std::string> above_env_keys_;
  bool use_environment_ = false;
};
} // namespace utils
} // namespace common
} // namespace api
} // namespace armonik
