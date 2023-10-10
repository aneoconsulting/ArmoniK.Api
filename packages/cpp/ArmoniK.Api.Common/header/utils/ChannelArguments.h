#pragma once

#include "utils/Configuration.h"
#include <grpcpp/support/channel_arguments.h>

namespace armonik {
namespace api {
namespace common {
namespace utils {
/**
 * Get custom channel arguments for channel creation
 * @param config Configuration
 * @return Channel arguments
 */
::grpc::ChannelArguments getChannelArguments(const Configuration &config);

/**
 * Get custom channel arguments for channel creation
 * @param config Control Plane configuration
 * @return Channel arguments
 */
::grpc::ChannelArguments getChannelArguments(const options::ControlPlane &config);

/**
 * Generate the service config for the channel arguments
 * @param config Control Plane configuration
 * @return Json of the service
 */
std::string getServiceConfigJson(const armonik::api::common::options::ControlPlane &config);
} // namespace utils
} // namespace common
} // namespace api
} // namespace armonik
