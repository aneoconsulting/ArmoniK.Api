#include <gmock/gmock.h>
#include <gtest/gtest.h>

#include "common.h"
#include "logger/formatter.h"
#include "logger/logger.h"
#include "logger/writer.h"

#include "results_service.grpc.pb.h"
#include "submitter/ResultsClient.h"
#include "submitter/SubmitterClient.h"

using Logger = armonik::api::common::logger::Logger;

TEST(Results, test_results_created) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;

  init(channel, task_options, log);

  auto client = armonik::api::client::ResultsClient(armonik::api::grpc::v1::results::Results::NewStub(channel));
  auto sub_client =
      armonik::api::client::SubmitterClient(armonik::api::grpc::v1::submitter::Submitter::NewStub(channel));
  auto session_id = sub_client.create_session(task_options, {});
  auto map = client.create_results(session_id, std::vector<std::string>{"0", "1", "2", "3"});
  ASSERT_TRUE(!map.empty());
  ASSERT_EQ(map.size(), 4);
}

TEST(Results, test_results_list) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;

  init(channel, task_options, log);

  auto client = armonik::api::client::ResultsClient(armonik::api::grpc::v1::results::Results::NewStub(channel));
  auto sub_client =
      armonik::api::client::SubmitterClient(armonik::api::grpc::v1::submitter::Submitter::NewStub(channel));
  auto session_id = sub_client.create_session(task_options, {});
  auto map = client.create_results(session_id, std::vector<std::string>{"0", "1", "2", "3"});
  ASSERT_TRUE(!map.empty());
  ASSERT_EQ(map.size(), 4);

  armonik::api::grpc::v1::results::Filters filters;
  armonik::api::grpc::v1::results::FilterField filter_field;
  filter_field.mutable_field()->mutable_result_raw_field()->set_field(
      armonik::api::grpc::v1::results::RESULT_RAW_ENUM_FIELD_SESSION_ID);
  filter_field.mutable_filter_string()->set_value(session_id);
  filter_field.mutable_filter_string()->set_operator_(armonik::api::grpc::v1::FILTER_STRING_OPERATOR_EQUAL);
  *filters.mutable_or_()->Add()->mutable_and_()->Add() = filter_field;
  int total;
  auto list = client.list_results(filters, total);
  ASSERT_EQ(list.size(), 4);
  ASSERT_EQ(list.size(), total);
}

TEST(Results, test_results_list_small_page) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;

  init(channel, task_options, log);

  auto client = armonik::api::client::ResultsClient(armonik::api::grpc::v1::results::Results::NewStub(channel));
  auto sub_client =
      armonik::api::client::SubmitterClient(armonik::api::grpc::v1::submitter::Submitter::NewStub(channel));
  auto session_id = sub_client.create_session(task_options, {});
  auto map = client.create_results(session_id, std::vector<std::string>{"0", "1", "2", "3", "4"});
  ASSERT_TRUE(!map.empty());
  ASSERT_EQ(map.size(), 5);

  armonik::api::grpc::v1::results::Filters filters;
  armonik::api::grpc::v1::results::FilterField filter_field;
  filter_field.mutable_field()->mutable_result_raw_field()->set_field(
      armonik::api::grpc::v1::results::RESULT_RAW_ENUM_FIELD_SESSION_ID);
  filter_field.mutable_filter_string()->set_value(session_id);
  filter_field.mutable_filter_string()->set_operator_(armonik::api::grpc::v1::FILTER_STRING_OPERATOR_EQUAL);
  *filters.mutable_or_()->Add()->mutable_and_()->Add() = filter_field;
  int total;
  auto list = client.list_results(filters, total, 0, 2);
  ASSERT_EQ(list.size(), 2);
  ASSERT_EQ(total, 5);

  list = client.list_results(filters, total, -1, 2);
  ASSERT_EQ(list.size(), 5);
  ASSERT_EQ(total, 5);
}
