#include <gtest/gtest.h>

#include "common.h"
#include "logger/formatter.h"
#include "logger/logger.h"
#include "logger/writer.h"

#include "versions/VersionsClient.h"

using Logger = armonik::api::common::logger::Logger;

TEST(Versions, can_list_versions) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;
  init(channel, task_options, log);

  armonik::api::client::VersionsClient client(armonik::api::grpc::v1::versions::Versions::NewStub(channel));

  std::map<std::string, std::string> versions;
  ASSERT_NO_THROW(versions = client.list_versions());

  std::cout << "API version: " << versions.at("api") << "\n"
            << "Core version: " << versions.at("core") << std::endl;

  ASSERT_NE(versions.at("api"), "Unknown");
  ASSERT_NE(versions.at("core"), "Unknown");
}
