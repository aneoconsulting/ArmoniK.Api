#pragma once

/**
 * @file EnvConfiguration.h
 * @brief Header file for the EnvConfiguration class
 */

#include "utils/IConfiguration.h"

namespace armonik::api::common::utils {
/**
 * @class EnvConfiguration
 * @brief An implementation of IConfiguration that handles environment variables
 */
class EnvConfiguration : public IConfiguration {
public:
  /**
   * @brief Default constructor
   */
  EnvConfiguration() { add_env_configuration(); }
};
} // namespace armonik::api::common::utils
