#include "channel/ChannelFactory.h"
#include "common.h"
#include "logger/formatter.h"
#include "logger/logger.h"
#include "logger/writer.h"
#include "options/ControlPlane.h"
#include "utils/Configuration.h"
#include <grpcpp/create_channel.h>
#include <gtest/gtest.h>

#include "submitter/SubmitterClient.h"
#include "utils/ChannelArguments.h"

using Logger = armonik::api::common::logger::Logger;

armonik::api::grpc::v1::TaskOptions default_task_options() {
  armonik::api::grpc::v1::TaskOptions default_task_options;
  default_task_options.mutable_options()->insert({"key1", "value1"});
  default_task_options.mutable_options()->insert({"key2", "value2"});
  default_task_options.mutable_max_duration()->set_seconds(3600);
  default_task_options.mutable_max_duration()->set_nanos(0);
  default_task_options.set_max_retries(1);
  default_task_options.set_priority(1);
  default_task_options.set_partition_id("");
  default_task_options.set_application_name("my-app");
  default_task_options.set_application_version("1.0");
  default_task_options.set_application_namespace("my-namespace");
  default_task_options.set_application_service("my-service");
  default_task_options.set_engine_type("Unified");
  return default_task_options;
}

size_t num_create_session_submitter = 0;

/**
 * Fixture class for Options, inherit from MockFixture
 */
class Options : public MockFixture {};

TEST_F(Options, no_options) {
  Logger logger{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  armonik::api::common::utils::Configuration configuration;
  configuration.add_json_configuration("appsettings.json").add_env_configuration();

  std::string server_address = configuration.get("Grpc__EndPoint");
  armonik::api::client::ChannelFactory channel_factory(configuration, logger);

  auto channel = channel_factory.create_channel();
  armonik::api::client::SubmitterClient client(armonik::api::grpc::v1::submitter::Submitter::NewStub(channel));
  ASSERT_NO_THROW(client.create_session(default_task_options(), {}));
  num_create_session_submitter++;
  ASSERT_TRUE(rpcCalled("Submitter", "CreateSession", num_create_session_submitter));
}

TEST_F(Options, default_options) {
  Logger logger{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  armonik::api::common::utils::Configuration configuration;
  configuration.add_json_configuration("appsettings.json").add_env_configuration();

  std::string server_address = configuration.get("Grpc__EndPoint");
  armonik::api::client::ChannelFactory channel_factory(configuration, logger);

  auto channel = channel_factory.create_channel();
  armonik::api::client::SubmitterClient client(armonik::api::grpc::v1::submitter::Submitter::NewStub(channel));
  ASSERT_NO_THROW(client.create_session(default_task_options(), {}));
  num_create_session_submitter++;
  ASSERT_TRUE(rpcCalled("Submitter", "CreateSession", num_create_session_submitter));
}

TEST_F(Options, test_timeout) {
  Logger logger{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  armonik::api::common::utils::Configuration configuration;
  configuration.add_json_configuration("appsettings.json").add_env_configuration();

  std::string server_address = configuration.get("Grpc__EndPoint");
  configuration.set(armonik::api::common::options::ControlPlane::RequestTimeoutKey, "0:0:0.001"); // 1ms, way too short
  armonik::api::client::SubmitterClient client(armonik::api::grpc::v1::submitter::Submitter::NewStub(
      armonik::api::client::ChannelFactory(configuration, logger).create_channel()));
  ASSERT_ANY_THROW(client.create_session(default_task_options(), {}));
  configuration.set(armonik::api::common::options::ControlPlane::RequestTimeoutKey,
                    "0:0:10"); // 10s, should have finished by now
  client = armonik::api::client::SubmitterClient(armonik::api::grpc::v1::submitter::Submitter::NewStub(
      armonik::api::client::ChannelFactory(configuration, logger).create_channel()));
  ASSERT_NO_THROW(client.create_session(default_task_options(), {}));
  num_create_session_submitter++;
  ASSERT_TRUE(rpcCalled("Submitter", "CreateSession", num_create_session_submitter));
}
