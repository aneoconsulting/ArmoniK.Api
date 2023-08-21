#pragma once

/**
 * @file EnvConfiguration.h
 * @brief Header file for the EnvConfiguration class
 */

#include "utils/Configuration.h"

namespace API_COMMON_NAMESPACE::utils::EnvConfiguration {
inline void fromEnv(Configuration &config) { config.add_env_configuration(); }
inline Configuration fromEnv() {
  Configuration config;
  config.add_env_configuration();
  return config;
}
} // namespace API_COMMON_NAMESPACE::utils::EnvConfiguration
