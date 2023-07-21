#include <iostream>
#include <memory>
#include <string>

#include <grpc++/grpc++.h>

#include <gmock/gmock.h>
#include <gtest/gtest.h>

#include "SubmitterClientTest.h"

#include "submitter/SubmitterClient.h"
#include "submitter_service.grpc.pb.h"

#include "serilog/serilog.h"
#include "utils/EnvConfiguration.h"
#include "utils/GuuId.h"
#include "utils/StringsUtils.h"

using ArmoniK::Api::Common::utils::IConfiguration;
using armonik::api::grpc::v1::TaskOptions;
using armonik::api::grpc::v1::submitter::CreateSessionReply;
using armonik::api::grpc::v1::submitter::CreateSessionRequest;
using armonik::api::grpc::v1::submitter::Submitter;
using grpc::Channel;
using grpc::ClientContext;
using grpc::Status;
using namespace ArmoniK::Api::Common::utils;

using ::testing::_;
using ::testing::AtLeast;

using namespace ArmoniK::Api::Common::serilog;

/**
 * @brief Initializes task options creates channel with server address
 *
 * @param channel The gRPC channel to communicate with the server.
 * @param default_task_options The default task options.
 */
void init(std::shared_ptr<Channel> &channel, TaskOptions &default_task_options) {

  EnvConfiguration configuration;
  // auto server = std::make_shared<EnvConfiguration>(configuration_t);

  configuration.add_json_configuration("appsetting.json").add_env_configuration();

  std::string server_address = configuration.get("ArmoniK_Client_Server");

  std::cout << " Server address " << server_address << std::endl;

  channel = grpc::CreateChannel(server_address, grpc::InsecureChannelCredentials());

  // stub_ = Submitter::NewStub(channel);

  default_task_options.mutable_options()->insert({"key1", "value1"});
  default_task_options.mutable_options()->insert({"key2", "value2"});
  default_task_options.mutable_max_duration()->set_seconds(3600);
  default_task_options.mutable_max_duration()->set_nanos(0);
  default_task_options.set_max_retries(3);
  default_task_options.set_priority(1);
  default_task_options.set_partition_id("cpp");
  default_task_options.set_application_name("my-app");
  default_task_options.set_application_version("1.0");
  default_task_options.set_application_namespace("my-namespace");
  default_task_options.set_application_service("my-service");
  default_task_options.set_engine_type("Unified");
}

TEST(testMock, createSession) {
  // MockStubInterface stub;
  std::shared_ptr<Channel> channel;

  ClientContext context;
  CreateSessionReply reply;
  CreateSessionRequest request;

  const std::vector<std::string> &partition_ids = {"cpp"};

  TaskOptions task_options;
  init(channel, task_options);

  ASSERT_EQ(task_options.partition_id(), "cpp");

  std::unique_ptr<Submitter::StubInterface> stub = Submitter::NewStub(channel);
  // EXPECT_CALL(*stub, CreateSession(_, _, _)).Times(AtLeast(1));
  ArmoniK::Api::Client::SubmitterClient submitter(std::move(stub));
  std::string session_id = submitter.create_session(task_options, partition_ids);

  std::cout << "create_session response: " << session_id << std::endl;

  ASSERT_FALSE(session_id.empty());
}

TEST(testMock, submitTask) {

  serilog log(logging_format::CONSOLE);

  std::cout << "Serilog closed" << std::endl;

  log.enrich([&](serilog_context &ctx) { ctx.add("threadid", std::this_thread::get_id()); });
  log.enrich([&](serilog_context &ctx) { ctx.add("fieldTestValue", 1); });
  log.add_property("time", time(nullptr));

  ::putenv((char *)"GRPC_DNS_RESOLVER=native");

  std::cout << "Starting client..." << std::endl;

  CreateSessionRequest request;
  TaskOptions task_options;

  std::shared_ptr<Channel> channel;
  init(channel, task_options);

  // MockStubInterface stub;
  std::unique_ptr<Submitter::StubInterface> stub = Submitter::NewStub(channel);

  *request.mutable_default_task_option() = task_options;
  request.add_partition_ids(task_options.partition_id());

  // EXPECT_CALL(*stub, CreateSession(_, _, _)).Times(AtLeast(1));
  // EXPECT_CALL(*stub, GetServiceConfiguration(_, _, _)).Times(AtLeast(1));
  // EXPECT_CALL(*stub, CreateLargeTasksRaw(_, _)).Times(AtLeast(1));

  CreateSessionReply reply;
  grpc::ClientContext context;

  ArmoniK::Api::Client::SubmitterClient submitter(std::move(stub));
  const std::vector<std::string> &partition_ids = {"cpp"};
  std::string session_id = submitter.create_session(task_options, partition_ids);

  ASSERT_FALSE(session_id.empty());

  try {
    std::vector<ArmoniK::Api::Client::payload_data> payloads;

    for (int i = 0; i < 10; i++) {
      ArmoniK::Api::Client::payload_data data;
      data.keys = ArmoniK::Api::Common::utils::GuuId::generate_uuid();
      data.payload = {'a', 'r', 'm', 'o', 'n', 'i', 'k'};
      data.dependencies = {};
      payloads.push_back(data);
    }
    const auto [task_ids, failed_task_ids] =
        submitter.submit_tasks_with_dependencies(session_id, task_options, payloads, 5);
    for (const auto &task_id : task_ids) {
      std::stringstream out;
      out << "Generate task_ids : " << task_id;
      log.info(out.str());
    }
    for (const auto &failed_task_id : failed_task_ids) {
      std::stringstream out;
      out << "Failed task_ids : " << failed_task_id;
      log.info(out.str());
    }
  } catch (std::exception &e) {
    log.error(e.what());
    throw;
  }
  log.info("Stopping client...OK");
}

TEST(testMock, getResult) {
  // MockStubInterface stub;
  std::shared_ptr<Channel> channel;

  CreateSessionReply reply;
  CreateSessionRequest request;

  const std::vector<std::string> &partition_ids = {"cpp"};

  TaskOptions task_options;
  armonik::api::grpc::v1::ResultRequest result_request;

  init(channel, task_options);
  // EXPECT_CALL(*stub, GetServiceConfiguration(_, _, _)).Times(AtLeast(1));
  // EXPECT_CALL(*stub, TryGetResultStreamRaw(_, _)).Times(AtLeast(1));

  std::unique_ptr<Submitter::StubInterface> stub = Submitter::NewStub(channel);
  ArmoniK::Api::Client::SubmitterClient submitter(std::move(stub));

  auto result = submitter.get_result_async(result_request);

  ASSERT_FALSE(result.get().empty());
}
