#pragma once
/**
 * @file JsonConfiguration.h
 * @brief Definition of a JSON configuration class that inherits from Configuration.
 */
#include "utils/Configuration.h"

namespace armonik {
namespace api {
namespace common {
namespace utils {
namespace JsonConfiguration {
void fromPath(Configuration &config, string_view filepath);
void fromString(Configuration &config, string_view json_string);
inline Configuration fromPath(string_view filepath) {
  Configuration config;
  fromPath(config, filepath);
  return config;
}
inline Configuration fromString(string_view json_string) {
  Configuration config;
  fromString(config, json_string);
  return config;
}
} // namespace JsonConfiguration
} // namespace utils
} // namespace common
} // namespace api
} // namespace armonik
