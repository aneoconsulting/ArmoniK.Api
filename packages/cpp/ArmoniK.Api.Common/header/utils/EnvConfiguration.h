#pragma once

/**
 * @file EnvConfiguration.h
 * @brief Header file for the EnvConfiguration class
 */

#include "utils/Configuration.h"

namespace armonik::api::common::utils::EnvConfiguration {
inline void fromEnv(Configuration &config) { config.add_env_configuration(); }
inline Configuration fromEnv() {
  Configuration config;
  config.add_env_configuration();
  return config;
}
} // namespace armonik::api::common::utils::EnvConfiguration
