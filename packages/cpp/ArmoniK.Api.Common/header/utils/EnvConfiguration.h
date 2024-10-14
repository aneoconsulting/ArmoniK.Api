#pragma once

/**
 * @file EnvConfiguration.h
 * @brief Header file for the EnvConfiguration class
 */

#include "utils/Configuration.h"

namespace armonik {
namespace api {
namespace common {
namespace utils {
namespace EnvConfiguration {
inline void fromEnv(Configuration &config) { config.add_env_configuration(); }
inline Configuration fromEnv() {
  Configuration config;
  config.add_env_configuration();
  return config;
}
} // namespace EnvConfiguration
} // namespace utils
} // namespace common
} // namespace api
} // namespace armonik
