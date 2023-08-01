#pragma once

/**
 * @file EnvConfiguration.h
 * @brief Header file for the EnvConfiguration class
 */

#include "utils/Configuration.h"

namespace API_COMMON_NAMESPACE::utils {
/**
 * @class EnvConfiguration
 * @brief An implementation of Configuration that handles environment variables
 */
class EnvConfiguration : public Configuration {
public:
  /**
   * @brief Default constructor
   */
  EnvConfiguration() { add_env_configuration(); }
};
} // namespace API_COMMON_NAMESPACE::utils
