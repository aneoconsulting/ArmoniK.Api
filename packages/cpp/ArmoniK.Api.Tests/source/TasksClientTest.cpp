#include <gtest/gtest.h>
#include <numeric>

#include "common.h"
#include "logger/formatter.h"
#include "logger/logger.h"
#include "logger/writer.h"

#include "objects/Task.h"
#include "results/ResultsClient.h"
#include "sessions/SessionsClient.h"
#include "tasks/TasksClient.h"
#include "tasks_service.grpc.pb.h"

using Logger = armonik::api::common::logger::Logger;

armonik::api::grpc::v1::tasks::Filters get_session_id_filter(std::string session_id) {
  armonik::api::grpc::v1::tasks::Filters filters;
  armonik::api::grpc::v1::tasks::FilterField filter_field;
  filter_field.mutable_field()->mutable_task_summary_field()->set_field(
      armonik::api::grpc::v1::tasks::TASK_SUMMARY_ENUM_FIELD_SESSION_ID);
  *filter_field.mutable_filter_string()->mutable_value() = std::move(session_id);
  filter_field.mutable_filter_string()->set_operator_(armonik::api::grpc::v1::FILTER_STRING_OPERATOR_EQUAL);
  *filters.mutable_or_()->Add()->mutable_and_()->Add() = filter_field;
  return filters;
}

/**
 * Fixture class for task, inherit from MockFixture
 */
class Tasks : public MockFixture {};

TEST_F(Tasks, submit_tasks_test) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;

  init(channel, task_options, log);
  auto task_options_submit = task_options;
  task_options_submit.set_priority(task_options.priority() + 1);
  auto task_options_unique = task_options;
  task_options_unique.set_priority(task_options.priority() + 2);

  auto session_id = armonik::api::client::SessionsClient(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel))
                        .create_session(task_options);
  auto result_client = armonik::api::client::ResultsClient(armonik::api::grpc::v1::results::Results::NewStub(channel));
  auto payload_id = result_client.create_results(
      session_id, std::vector<std::pair<std::string, std::string>>{{"name", "payload"}})["name"];
  auto result_id = result_client.create_results_metadata(session_id, {"result"})["result"];

  auto client = armonik::api::client::TasksClient(armonik::api::grpc::v1::tasks::Tasks::NewStub(channel));

  std::vector<armonik::api::common::TaskInfo> tasks_simple;
  ASSERT_NO_THROW(tasks_simple =
                      client.submit_tasks(session_id, {armonik::api::common::TaskCreation{payload_id, {{result_id}}}}));
  // ASSERT_EQ(tasks_simple.size(), 1);

  std::vector<armonik::api::common::TaskInfo> tasks_submit_override;
  ASSERT_NO_THROW(tasks_submit_override =
                      client.submit_tasks(session_id, {armonik::api::common::TaskCreation{payload_id, {{result_id}}}},
                                          task_options_submit));
  // ASSERT_EQ(tasks_submit_override.size(), 1);

  std::vector<armonik::api::common::TaskInfo> tasks_submit_unique_override;
  ASSERT_NO_THROW(tasks_submit_unique_override = client.submit_tasks(
                      session_id,
                      {armonik::api::common::TaskCreation{payload_id, {{result_id}}, {}, task_options_unique}},
                      task_options_submit));
  ASSERT_TRUE(rpcCalled("Tasks", "SubmitTasks", 3));
}

TEST_F(Tasks, count_tasks_test) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;

  init(channel, task_options, log);

  auto session_id = armonik::api::client::SessionsClient(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel))
                        .create_session(task_options);
  auto result_client = armonik::api::client::ResultsClient(armonik::api::grpc::v1::results::Results::NewStub(channel));
  auto payload_id = result_client.create_results(
      session_id, std::vector<std::pair<std::string, std::string>>{{"name", "payload"}})["name"];
  auto result_id = result_client.create_results_metadata(session_id, {"result"})["result"];
  auto client = armonik::api::client::TasksClient(armonik::api::grpc::v1::tasks::Tasks::NewStub(channel));

  auto filters = get_session_id_filter(session_id);

  std::map<armonik::api::grpc::v1::task_status::TaskStatus, int32_t> status_count;
  ASSERT_NO_THROW(status_count = client.count_tasks_by_status(filters));

  client.submit_tasks(session_id, {armonik::api::common::TaskCreation{payload_id, {{result_id}}}});

  ASSERT_NO_THROW(status_count = client.count_tasks_by_status(filters));
  ASSERT_TRUE(rpcCalled("Tasks", "CountTasksByStatus", 2));
}

TEST_F(Tasks, get_result_ids_test) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;

  init(channel, task_options, log);

  auto session_id = armonik::api::client::SessionsClient(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel))
                        .create_session(task_options);
  auto result_client = armonik::api::client::ResultsClient(armonik::api::grpc::v1::results::Results::NewStub(channel));
  auto payload_id = result_client.create_results(
      session_id, std::vector<std::pair<std::string, std::string>>{{"name", "payload"}})["name"];
  auto result_id = result_client.create_results_metadata(session_id, {"result"})["result"];
  auto client = armonik::api::client::TasksClient(armonik::api::grpc::v1::tasks::Tasks::NewStub(channel));

  auto task_id = client.submit_tasks(session_id, {armonik::api::common::TaskCreation{payload_id, {result_id}}});

  std::map<std::string, std::vector<std::string>> tid_rids;
  ASSERT_NO_THROW(tid_rids = client.get_result_ids({"task_id"}));
  ASSERT_TRUE(rpcCalled("Tasks", "GetResultIds"));
}

TEST_F(Tasks, get_task_test) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;

  init(channel, task_options, log);

  auto session_id = armonik::api::client::SessionsClient(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel))
                        .create_session(task_options);
  auto result_client = armonik::api::client::ResultsClient(armonik::api::grpc::v1::results::Results::NewStub(channel));
  auto payload_id = result_client.create_results(
      session_id, std::vector<std::pair<std::string, std::string>>{{"name", "payload"}})["name"];
  auto result_id = result_client.create_results_metadata(session_id, {"result"})["result"];
  auto client = armonik::api::client::TasksClient(armonik::api::grpc::v1::tasks::Tasks::NewStub(channel));

  auto task_id = client.submit_tasks(session_id, {armonik::api::common::TaskCreation{payload_id, {result_id}}});

  armonik::api::grpc::v1::tasks::TaskDetailed details;
  ASSERT_NO_THROW(details = client.get_task("task_id"));
  ASSERT_TRUE(rpcCalled("Tasks", "GetTask"));
}

TEST_F(Tasks, cancel_tasks_test) {
  GTEST_SKIP() << "Core bug #523";
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;

  init(channel, task_options, log);

  auto session_id = armonik::api::client::SessionsClient(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel))
                        .create_session(task_options);
  auto result_client = armonik::api::client::ResultsClient(armonik::api::grpc::v1::results::Results::NewStub(channel));
  auto payload_id = result_client.create_results(
      session_id, std::vector<std::pair<std::string, std::string>>{{"name", "payload"}})["name"];
  auto result_id = result_client.create_results_metadata(session_id, {"result"})["result"];
  auto dd_id = result_client.create_results_metadata(session_id, {"DD"})["DD"];
  auto client = armonik::api::client::TasksClient(armonik::api::grpc::v1::tasks::Tasks::NewStub(channel));

  auto task_id =
      client.submit_tasks(session_id, {armonik::api::common::TaskCreation{payload_id, {result_id}, {dd_id}}})[0]
          .task_id;

  ASSERT_NE(client.get_task(task_id).status(), armonik::api::grpc::v1::task_status::TASK_STATUS_CANCELLED);

  ASSERT_EQ(client.cancel_tasks({task_id}).at(0).status(), armonik::api::grpc::v1::task_status::TASK_STATUS_CANCELLED);
}

TEST_F(Tasks, list_tasks_test) {
  GTEST_SKIP() << "Mock must return something ";
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;

  init(channel, task_options, log);

  auto session_id = armonik::api::client::SessionsClient(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel))
                        .create_session(task_options);
  auto result_client = armonik::api::client::ResultsClient(armonik::api::grpc::v1::results::Results::NewStub(channel));
  auto payload_id = result_client.create_results(
      session_id, std::vector<std::pair<std::string, std::string>>{{"name", "payload"}})["name"];
  auto result_id = result_client.create_results_metadata(session_id, {"result"})["result"];
  auto client = armonik::api::client::TasksClient(armonik::api::grpc::v1::tasks::Tasks::NewStub(channel));

  client.submit_tasks(session_id, {{payload_id, {result_id}}});

  int total;
  ASSERT_EQ(client.list_tasks(get_session_id_filter(session_id), total).size(), 1);
  ASSERT_EQ(total, 1);
  ASSERT_TRUE(rpcCalled("Tasks", "ListTasks"));
}

TEST_F(Tasks, list_tasks_detailed_test) {
  GTEST_SKIP() << "Mock must return something ";
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;

  init(channel, task_options, log);

  auto session_id = armonik::api::client::SessionsClient(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel))
                        .create_session(task_options);
  auto result_client = armonik::api::client::ResultsClient(armonik::api::grpc::v1::results::Results::NewStub(channel));
  auto payload_id = result_client.create_results(
      session_id, std::vector<std::pair<std::string, std::string>>{{"name", "payload"}})["name"];
  auto result_id = result_client.create_results_metadata(session_id, {"result"})["result"];
  auto client = armonik::api::client::TasksClient(armonik::api::grpc::v1::tasks::Tasks::NewStub(channel));

  client.submit_tasks(session_id, {{payload_id, {result_id}}});

  int total;
  ASSERT_EQ(client.list_tasks_detailed(get_session_id_filter(session_id), total).size(), 1);
  ASSERT_EQ(total, 1);
  ASSERT_TRUE(rpcCalled("Tasks", "ListTasksDetailed"));
}

/**
 * This test should be the last to run in the suit, which is why its name is prefixed with "z".
 */
TEST_F(Tasks, z_service_fully_implemented) {
  std::vector<std::string> missing_rpcs{"CancelTasks", "ListTasks", "ListTasksDetailed"};
  ASSERT_TRUE(all_rpc_called("Tasks", missing_rpcs));
}
