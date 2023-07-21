#pragma once
/**
 * @file JsonConfiguration.h
 * @brief Definition of a JSON configuration class that inherits from IConfiguration.
 */
#include "utils/IConfiguration.h"

namespace API_COMMON_NAMESPACE::utils {
/**
 * @class JsonConfiguration
 * @brief JSON configuration class that inherits from IConfiguration.
 */
class JsonConfiguration : public IConfiguration {
private:
  JsonConfiguration() = default;

public:
  /**
   * @brief Constructor that takes a JSON file path.
   * @param filepath JSON file path to be used for configuration.
   */
  explicit JsonConfiguration(const std::string &filepath);

  static void fromPath(IConfiguration &config, std::string_view filepath);
  static JsonConfiguration fromString(const std::string &json_string);
  static void fromString(IConfiguration &config, const std::string &json_string);
};
} // namespace API_COMMON_NAMESPACE::utils
