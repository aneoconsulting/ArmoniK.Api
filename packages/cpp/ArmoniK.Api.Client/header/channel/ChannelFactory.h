#pragma once

#include "logger/formatter.h"
#include "logger/logger.h"
#include "logger/writer.h"
#include "utils/Configuration.h"
#include <grpcpp/channel.h>
#include <grpcpp/security/credentials.h>
#include <mutex>
#include <queue>

namespace armonik {
namespace api {
namespace client {
class ChannelFactory {
public:
  explicit ChannelFactory(armonik::api::common::utils::Configuration configuration, common::logger::Logger &logger);

  std::shared_ptr<::grpc::Channel> create_channel();

  static bool ShutdownOnFailure(std::shared_ptr<::grpc::Channel> channel);

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