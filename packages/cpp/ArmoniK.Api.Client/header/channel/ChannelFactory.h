#pragma once

#include "logger/logger.h"
#include "logger/writer.h"
#include "utils/Configuration.h"
#include <grpcpp/channel.h>
#include <grpcpp/security/credentials.h>

namespace armonik {
namespace api {
namespace client {
class ChannelFactory {
public:
  /**
   * @brief Creates a channel factory from the given configuration
   * @param configuration The channel configuration
   * @param logger The logger
   */
  explicit ChannelFactory(armonik::api::common::utils::Configuration configuration, common::logger::Logger &logger);

  /**
   * @brief Creates the new gRPC channel
   * @return New channel
   */
  std::shared_ptr<::grpc::Channel> create_channel();

  /**
   *
   * @return A bool on whether the gRPC channel is secure or not
   */
  bool isSecureChannel() const noexcept;

private:
  armonik::api::common::logger::LocalLogger logger_;
  std::shared_ptr<::grpc::ChannelCredentials> credentials_{nullptr};
  std::string endpoint_;
  armonik::api::common::utils::Configuration configuration_;
  bool is_secure_{false};
};
} // namespace client
} // namespace api
} // namespace armonik