#pragma once
/**
 * @file JsonConfiguration.h
 * @brief Definition of a JSON configuration class that inherits from Configuration.
 */
#include "utils/Configuration.h"

namespace armonik::api::common::utils::JsonConfiguration {
void fromPath(Configuration &config, std::string_view filepath);
void fromString(Configuration &config, std::string_view json_string);
inline Configuration fromPath(std::string_view filepath) {
  Configuration config;
  fromPath(config, filepath);
  return config;
}
inline Configuration fromString(std::string_view json_string) {
  Configuration config;
  fromString(config, json_string);
  return config;
}
} // namespace armonik::api::common::utils::JsonConfiguration
