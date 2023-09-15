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
void fromPath(Configuration &config, absl::string_view filepath);
void fromString(Configuration &config, absl::string_view json_string);
inline Configuration fromPath(absl::string_view filepath) {
  Configuration config;
  fromPath(config, filepath);
  return config;
}
inline Configuration fromString(absl::string_view json_string) {
  Configuration config;
  fromString(config, json_string);
  return config;
}
} // namespace JsonConfiguration
} // namespace utils
} // namespace common
} // namespace api
} // namespace armonik
