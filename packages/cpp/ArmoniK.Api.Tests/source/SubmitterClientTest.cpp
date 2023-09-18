#include <iostream>
#include <memory>
#include <sstream>
#include <string>
#include <thread>

#include <grpc++/grpc++.h>

#include <gmock/gmock.h>
#include <gtest/gtest.h>

#include "SubmitterClientTest.h"

#include "submitter/SubmitterClient.h"
#include "submitter_service.grpc.pb.h"

#include "logger/formatter.h"
#include "logger/logger.h"
#include "logger/writer.h"
#include "utils/EnvConfiguration.h"
#include "utils/GuuId.h"

#include "results_common.pb.h"
#include "results_service.grpc.pb.h"
#include "submitter/ResultsClient.h"

using armonik::api::common::utils::Configuration;
using armonik::api::grpc::v1::TaskOptions;
using armonik::api::grpc::v1::submitter::CreateSessionReply;
using armonik::api::grpc::v1::submitter::CreateSessionRequest;
using armonik::api::grpc::v1::submitter::Submitter;
using grpc::Channel;
using grpc::ClientContext;
using grpc::Status;
using namespace armonik::api::common::utils;

using ::testing::_;
using ::testing::AtLeast;

namespace logger = armonik::api::common::logger;

/**
 * @brief Initializes task options creates channel with server address
 *
 * @param channel The gRPC channel to communicate with the server.
 * @param default_task_options The default task options.
 */
void init(std::shared_ptr<Channel> &channel, TaskOptions &default_task_options, logger::ILogger &logger) {

  Configuration configuration;
  // auto server = std::make_shared<EnvConfiguration>(configuration_t);

  configuration.add_json_configuration("appsettings.json").add_env_configuration();

  std::string server_address = configuration.get("Grpc__EndPoint");

  logger.info(" Server address {address}", {{"address", server_address}});

  channel = grpc::CreateChannel(server_address, grpc::InsecureChannelCredentials());

  // stub_ = Submitter::NewStub(channel);

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
}

TEST(testMock, createSession) {
  // MockStubInterface stub;
  std::shared_ptr<Channel> channel;
  logger::Logger log{logger::writer_console(), logger::formatter_plain(true)};

  ClientContext context;
  CreateSessionReply reply;
  CreateSessionRequest request;

  const std::vector<std::string> &partition_ids = {""};

  TaskOptions task_options;
  init(channel, task_options, log);

  ASSERT_EQ(task_options.partition_id(), "");

  std::unique_ptr<Submitter::StubInterface> stub = Submitter::NewStub(channel);
  // EXPECT_CALL(*stub, CreateSession(_, _, _)).Times(AtLeast(1));
  armonik::api::client::SubmitterClient submitter(std::move(stub));
  std::string session_id = submitter.create_session(task_options, partition_ids);

  std::cout << "create_session response: " << session_id << std::endl;

  ASSERT_FALSE(session_id.empty());
}

TEST(testMock, submitTask) {

  logger::Logger log{logger::writer_console(), logger::formatter_plain(true)};

  std::cout << "Serilog closed" << std::endl;

  log.local_context_generator_add("threadid", []() {
    std::stringstream ss;
    ss << std::this_thread::get_id();
    return ss.str();
  });

  log.local_context_generator_add("fieldTestValue", []() { return "1"; });
  log.global_context_add("time", []() {
    std::stringstream ss;
    ss << time(nullptr);
    return ss.str();
  }());

  ::putenv((char *)"GRPC_DNS_RESOLVER=native");

  std::cout << "Starting client..." << std::endl;

  CreateSessionRequest request;
  TaskOptions task_options;

  std::shared_ptr<Channel> channel;
  init(channel, task_options, log);

  // MockStubInterface stub;
  std::unique_ptr<Submitter::StubInterface> stub = Submitter::NewStub(channel);

  *request.mutable_default_task_option() = task_options;
  request.add_partition_ids(task_options.partition_id());

  // EXPECT_CALL(*stub, CreateSession(_, _, _)).Times(AtLeast(1));
  // EXPECT_CALL(*stub, GetServiceConfiguration(_, _, _)).Times(AtLeast(1));
  // EXPECT_CALL(*stub, CreateLargeTasksRaw(_, _)).Times(AtLeast(1));

  CreateSessionReply reply;
  grpc::ClientContext context;

  armonik::api::client::SubmitterClient submitter(std::move(stub));
  const std::vector<std::string> &partition_ids = {""};
  std::string session_id = submitter.create_session(task_options, partition_ids);

  ASSERT_FALSE(session_id.empty());

  armonik::api::client::ResultsClient results(armonik::api::grpc::v1::results::Results::NewStub(channel));
  std::vector<std::string> names;
  names.reserve(10);
  for (int i = 0; i < 10; i++) {
    names.push_back(armonik::api::common::utils::GuuId::generate_uuid());
  }
  auto result_mapping = results.create_results(session_id, names);
  int j = 0;
  for (auto &&kv : result_mapping) {
    names[j++] = kv.second;
  }

  try {
    std::vector<armonik::api::client::payload_data> payloads;

    for (int i = 0; i < 10; i++) {
      armonik::api::client::payload_data data;
      data.keys = names[i];
      data.payload = {'a', 'r', 'm', 'o', 'n', 'i', 'k'};
      data.dependencies = {};
      payloads.push_back(data);
    }
    const auto taskId_failedTaskId = submitter.submit_tasks_with_dependencies(session_id, task_options, payloads, 5);
    for (const auto &task_id : taskId_failedTaskId.first) {
      std::stringstream out;
      out << "Generate task_ids : " << task_id;
      log.info(out.str());
    }

    for (const auto &failed_task_id : taskId_failedTaskId.second) {
      std::stringstream out;
      out << "Failed task_ids : " << failed_task_id;
      log.info(out.str());
      throw;
    }
  } catch (std::exception &e) {
    log.error(e.what());
    throw;
  }
  log.info("Stopping client...OK");
}

TEST(testMock, testWorker) {
  logger::Logger log{logger::writer_console(), logger::formatter_plain(true)};
  std::shared_ptr<Channel> channel;

  CreateSessionReply reply;
  CreateSessionRequest request;

  const std::vector<std::string> &partition_ids = {""};

  TaskOptions task_options;

  init(channel, task_options, log);

  auto stub = armonik::api::grpc::v1::results::Results::NewStub(channel);

  grpc::ClientContext context;

  std::unique_ptr<Submitter::StubInterface> stub_client = Submitter::NewStub(channel);
  armonik::api::client::SubmitterClient submitter(std::move(stub_client));
  std::string session_id = submitter.create_session(task_options, partition_ids);

  auto name = "test";

  armonik::api::grpc::v1::results::CreateResultsMetaDataRequest request_create;
  request_create.set_session_id(session_id);
  armonik::api::client::ResultsClient results(armonik::api::grpc::v1::results::Results::NewStub(channel));
  auto mapping = results.create_results(session_id, {name});
  ASSERT_TRUE(mapping.size() == 1);

  std::vector<armonik::api::client::payload_data> payloads;
  armonik::api::client::payload_data data;
  data.keys = mapping[name];
  data.payload = "armonik";
  data.dependencies = {};
  payloads.push_back(data);
  const auto task_id_failed = submitter.submit_tasks_with_dependencies(session_id, task_options, payloads, 5);

  while (true) {
    auto status = submitter.get_result_status(session_id, {mapping[name]})[mapping[name]];
    if (status == armonik::api::grpc::v1::result_status::RESULT_STATUS_COMPLETED ||
        status == armonik::api::grpc::v1::result_status::RESULT_STATUS_ABORTED) {
      ASSERT_NE(armonik::api::grpc::v1::result_status::RESULT_STATUS_ABORTED, status);
      break;
    }
  }

  armonik::api::grpc::v1::ResultRequest result_request;
  result_request.set_session(session_id);
  result_request.set_result_id(mapping[name]);
  auto result_payload = submitter.get_result_async(result_request).get();
  ASSERT_TRUE(!result_payload.empty());
}

TEST(testMock, getResult) {
  logger::Logger log{logger::writer_console(), logger::formatter_plain(true)};
  // MockStubInterface stub;
  std::shared_ptr<Channel> channel;

  CreateSessionReply reply;
  CreateSessionRequest request;

  const std::vector<std::string> &partition_ids = {""};

  TaskOptions task_options;
  armonik::api::grpc::v1::ResultRequest result_request;

  init(channel, task_options, log);

  auto stub = armonik::api::grpc::v1::results::Results::NewStub(channel);

  grpc::ClientContext context;

  log.info("Creating Client");
  std::unique_ptr<Submitter::StubInterface> stub_client = Submitter::NewStub(channel);
  armonik::api::client::SubmitterClient submitter(std::move(stub_client));
  std::string session_id = submitter.create_session(task_options, partition_ids);
  log.info("Received session id {session_id}", {{"session_id", session_id}});

  auto name = "test";

  armonik::api::grpc::v1::results::CreateResultsMetaDataRequest request_create;
  request_create.set_session_id(session_id);
  armonik::api::client::ResultsClient results(armonik::api::grpc::v1::results::Results::NewStub(channel));
  auto mapping = results.create_results(session_id, {name});
  log.info("Created result {result_id}", {{"result_id", mapping[name]}});
  ASSERT_TRUE(mapping.size() == 1);

  std::string payload = "TestPayload";

  results.upload_result_data(session_id, mapping[name], payload);
  log.info("Uploaded result {result_id}", {{"result_id", mapping[name]}});

  // EXPECT_CALL(*stub, GetServiceConfiguration(_, _, _)).Times(AtLeast(1));
  // EXPECT_CALL(*stub, TryGetResultStreamRaw(_, _)).Times(AtLeast(1));

  result_request.set_result_id(mapping[name]);
  result_request.set_session(session_id);

  auto result = submitter.get_result_async(result_request).get();
  log.info("Received result {result_id}", {{"result_id", mapping[name]}});

  ASSERT_FALSE(result.empty());
  ASSERT_EQ(payload, result);
}
