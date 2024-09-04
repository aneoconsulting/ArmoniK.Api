#include <gtest/gtest.h>

#include "common.h"
#include "logger/formatter.h"
#include "logger/logger.h"
#include "logger/writer.h"

#include "results/ResultsClient.h"
#include "results_service.grpc.pb.h"
#include "sessions/SessionsClient.h"

using Logger = armonik::api::common::logger::Logger;

TEST(Results, test_results_created) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;

  init(channel, task_options, log);

  auto client = armonik::api::client::ResultsClient(armonik::api::grpc::v1::results::Results::NewStub(channel));
  auto session_id = armonik::api::client::SessionsClient(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel))
                        .create_session(task_options);
  ASSERT_NO_THROW(client.create_results_metadata(session_id, std::vector<std::string>{"0", "1", "2", "3"}));
  ASSERT_TRUE(rpcCalled("Results", "CreateResultsMetaData"));
}

TEST(Results, test_results_list) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;

  init(channel, task_options, log);

  auto client = armonik::api::client::ResultsClient(armonik::api::grpc::v1::results::Results::NewStub(channel));
  auto session_id = armonik::api::client::SessionsClient(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel))
                        .create_session(task_options);
  ASSERT_NO_THROW(client.create_results_metadata(session_id, std::vector<std::string>{"0", "1", "2", "3"}));

  armonik::api::grpc::v1::results::Filters filters;
  armonik::api::grpc::v1::results::FilterField filter_field;
  filter_field.mutable_field()->mutable_result_raw_field()->set_field(
      armonik::api::grpc::v1::results::RESULT_RAW_ENUM_FIELD_SESSION_ID);
  filter_field.mutable_filter_string()->set_value(session_id);
  filter_field.mutable_filter_string()->set_operator_(armonik::api::grpc::v1::FILTER_STRING_OPERATOR_EQUAL);
  *filters.mutable_or_()->Add()->mutable_and_()->Add() = filter_field;
  int total;
  ASSERT_NO_THROW(client.list_results(filters, total));
  ASSERT_TRUE(rpcCalled("Results", "ListResults"));
}

TEST(Results, test_results_list_small_page) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;

  init(channel, task_options, log);

  auto client = armonik::api::client::ResultsClient(armonik::api::grpc::v1::results::Results::NewStub(channel));
  auto session_id = armonik::api::client::SessionsClient(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel))
                        .create_session(task_options);
  ASSERT_NO_THROW(client.create_results_metadata(session_id, std::vector<std::string>{"0", "1", "2", "3", "4"}));

  armonik::api::grpc::v1::results::Filters filters;
  armonik::api::grpc::v1::results::FilterField filter_field;
  filter_field.mutable_field()->mutable_result_raw_field()->set_field(
      armonik::api::grpc::v1::results::RESULT_RAW_ENUM_FIELD_SESSION_ID);
  filter_field.mutable_filter_string()->set_value(session_id);
  filter_field.mutable_filter_string()->set_operator_(armonik::api::grpc::v1::FILTER_STRING_OPERATOR_EQUAL);
  *filters.mutable_or_()->Add()->mutable_and_()->Add() = filter_field;
  int total;
  ASSERT_NO_THROW(client.list_results(filters, total, 0, 2));

  ASSERT_NO_THROW(client.list_results(filters, total, -1, 2));
  ASSERT_TRUE(rpcCalled("Results", "ListResults"));
}

TEST(Results, test_results_create_with_data_vector) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;

  init(channel, task_options, log);

  auto client = armonik::api::client::ResultsClient(armonik::api::grpc::v1::results::Results::NewStub(channel));
  auto session_id = armonik::api::client::SessionsClient(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel))
                        .create_session(task_options);
  std::vector<std::pair<std::string, std::string>> vec{{"0", "TestPayload"}};
  ASSERT_NO_THROW(client.create_results(session_id, vec));
  ASSERT_TRUE(rpcCalled("Results", "CreateResults"));
}

TEST(Results, test_results_create_with_data_map) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;

  init(channel, task_options, log);

  auto client = armonik::api::client::ResultsClient(armonik::api::grpc::v1::results::Results::NewStub(channel));
  auto session_id = armonik::api::client::SessionsClient(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel))
                        .create_session(task_options);
  std::map<std::string, std::string> name_payload;
  name_payload["0"] = "TestPayload";
  ASSERT_NO_THROW(client.create_results(session_id, std::move(name_payload)));
  ASSERT_TRUE(rpcCalled("Results", "CreateResults"));
}

TEST(Results, test_results_create_with_data_unordered_map) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;

  init(channel, task_options, log);

  auto client = armonik::api::client::ResultsClient(armonik::api::grpc::v1::results::Results::NewStub(channel));
  auto session_id = armonik::api::client::SessionsClient(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel))
                        .create_session(task_options);
  std::unordered_map<std::string, std::string> name_payload;
  name_payload["0"] = "TestPayload";
  ASSERT_NO_THROW(client.create_results(session_id, std::move(name_payload)));
  ASSERT_TRUE(rpcCalled("Results", "CreateResults"));
}

TEST(Results, test_results_create_with_data_string_view) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;

  init(channel, task_options, log);

  auto client = armonik::api::client::ResultsClient(armonik::api::grpc::v1::results::Results::NewStub(channel));
  auto session_id = armonik::api::client::SessionsClient(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel))
                        .create_session(task_options);
  std::vector<std::pair<std::string, absl::string_view>> name_payload;
  std::string fill_str = "TestPayloadTestPayload2";
  name_payload.emplace_back("0", absl::string_view(fill_str.c_str(), 11));
  name_payload.emplace_back("1", absl::string_view(fill_str.c_str() + 11, 12));
  ASSERT_NO_THROW(client.create_results(session_id, name_payload.begin(), name_payload.end()));
  ASSERT_TRUE(rpcCalled("Results", "CreateResults"));
}

TEST(Results, test_results_upload_download) {
  GTEST_SKIP() << "Mock server must return something ";
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;

  init(channel, task_options, log);

  auto client = armonik::api::client::ResultsClient(armonik::api::grpc::v1::results::Results::NewStub(channel));
  auto session_id = armonik::api::client::SessionsClient(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel))
                        .create_session(task_options);
  auto map = client.create_results_metadata(session_id, std::vector<std::string>{"0"});
  ASSERT_EQ(map.size(), 1);
  ASSERT_NO_THROW(map.at("0"));
  ASSERT_NO_THROW(client.upload_result_data(session_id, map.at("0"), "TestPayload"));
  ASSERT_EQ(client.download_result_data(session_id, map.at("0")), "TestPayload");
}

TEST(Results, service_fully_implemented) { ASSERT_TRUE(all_rpc_called("Results")); }
