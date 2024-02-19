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

  armonik::api::client::versions_info versions;
  ASSERT_NO_THROW(versions = client.list_versions());

  std::cout << "API version: " << versions.api << "\n"
            << "Core version: " << versions.core << std::endl;

  ASSERT_NE(versions.api, "Unknown");
  ASSERT_NE(versions.core, "Unknown");
}
