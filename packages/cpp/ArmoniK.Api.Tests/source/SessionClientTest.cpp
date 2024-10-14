#include <gtest/gtest.h>

#include "common.h"
#include "logger/formatter.h"
#include "logger/logger.h"
#include "logger/writer.h"

#include "sessions/SessionsClient.h"

using Logger = armonik::api::common::logger::Logger;

size_t num_create_session = 0;
size_t num_list_session = 0;

/**
 * Fixture class for session, inherit from MockFixture
 */
class Sessions : public MockFixture {};

TEST_F(Sessions, can_create_session) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;
  init(channel, task_options, log);

  armonik::api::client::SessionsClient client(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel));

  std::string response;
  ASSERT_NO_THROW(response = client.create_session(task_options));
  num_create_session++;
  ASSERT_FALSE(response.empty());
  ASSERT_TRUE(rpcCalled("Sessions", "CreateSession", num_create_session));
}

TEST_F(Sessions, can_cancel_session) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;
  init(channel, task_options, log);

  armonik::api::client::SessionsClient client(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel));

  std::string session_id = client.create_session(task_options);
  num_create_session++;

  armonik::api::grpc::v1::sessions::SessionRaw response;
  ASSERT_NO_THROW(response = client.cancel_session(session_id));
  ASSERT_EQ(response.session_id(), session_id);
  ASSERT_TRUE(rpcCalled("Sessions", "CancelSession"));
}

TEST_F(Sessions, can_get_session) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;
  init(channel, task_options, log);

  armonik::api::client::SessionsClient client(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel));

  std::string session_id = client.create_session(task_options);
  num_create_session++;

  armonik::api::grpc::v1::sessions::SessionRaw response;
  ASSERT_NO_THROW(response = client.get_session(session_id));
  ASSERT_EQ(response.session_id(), session_id);
  ASSERT_TRUE(rpcCalled("Sessions", "GetSession"));
}

TEST_F(Sessions, can_list_sessions) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;
  init(channel, task_options, log);

  auto client = armonik::api::client::SessionsClient(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel));
  std::string session_ids;
  size_t expected_n_sessions = 5;
  for (size_t i = 0; i < expected_n_sessions; i++) {
    ASSERT_NO_THROW(client.create_session(task_options));
    num_create_session++;
  }

  armonik::api::grpc::v1::sessions::Filters filters;
  int total;
  ASSERT_NO_THROW(client.list_sessions(filters, total));
  num_list_session++;
  ASSERT_TRUE(rpcCalled("Sessions", "ListSessions", num_list_session));
}

TEST_F(Sessions, can_list_sessions_small_page) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;
  init(channel, task_options, log);

  auto client = armonik::api::client::SessionsClient(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel));
  std::string session_ids;
  size_t expected_n_sessions = 5;
  for (size_t i = 0; i < expected_n_sessions; i++) {
    ASSERT_NO_THROW(client.create_session(task_options));
    num_create_session++;
  }

  armonik::api::grpc::v1::sessions::Filters filters;
  int total;
  // auto list = client.list_sessions(filters, total, 0, 2);
  ASSERT_NO_THROW(client.list_sessions(filters, total, 0, 2));
  num_list_session++;
  ASSERT_NO_THROW(client.list_sessions(filters, total, -1, 2));
  num_list_session++;
  ASSERT_TRUE(rpcCalled("Sessions", "ListSessions", num_list_session));
}

TEST_F(Sessions, can_pause_session) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;
  init(channel, task_options, log);

  armonik::api::client::SessionsClient client(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel));

  std::string session_id = client.create_session(task_options);
  num_create_session++;

  armonik::api::grpc::v1::sessions::SessionRaw response;
  ASSERT_NO_THROW(response = client.pause_session(session_id));
  ASSERT_EQ(response.session_id(), session_id);
  ASSERT_TRUE(rpcCalled("Sessions", "PauseSession"));
}

TEST_F(Sessions, can_resume_session) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;
  init(channel, task_options, log);

  armonik::api::client::SessionsClient client(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel));

  std::string session_id = client.create_session(task_options);
  num_create_session++;

  ASSERT_NO_THROW(client.pause_session(session_id));

  armonik::api::grpc::v1::sessions::SessionRaw response;
  ASSERT_NO_THROW(response = client.resume_session(session_id));
  ASSERT_EQ(response.session_id(), session_id);
  ASSERT_TRUE(rpcCalled("Sessions", "ResumeSession"));
}

TEST_F(Sessions, can_purge_session) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;
  init(channel, task_options, log);

  armonik::api::client::SessionsClient client(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel));

  std::string session_id = client.create_session(task_options);
  num_create_session++;

  ASSERT_NO_THROW(client.close_session(session_id));

  armonik::api::grpc::v1::sessions::SessionRaw response;
  ASSERT_NO_THROW(response = client.purge_session(session_id));
  ASSERT_EQ(response.session_id(), session_id);
  ASSERT_TRUE(rpcCalled("Sessions", "PurgeSession"));
}

TEST_F(Sessions, can_delete_session) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;
  init(channel, task_options, log);

  armonik::api::client::SessionsClient client(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel));

  std::string session_id = client.create_session(task_options);
  num_create_session++;

  armonik::api::grpc::v1::sessions::SessionRaw response;
  ASSERT_NO_THROW(response = client.delete_session(session_id));
  ASSERT_EQ(response.session_id(), session_id);
  ASSERT_TRUE(rpcCalled("Sessions", "DeleteSession"));
}

TEST_F(Sessions, can_stop_submission) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;
  init(channel, task_options, log);

  armonik::api::client::SessionsClient client(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel));

  std::string session_id = client.create_session(task_options);
  num_create_session++;

  armonik::api::grpc::v1::sessions::SessionRaw response;
  ASSERT_NO_THROW(response = client.stop_submission_session(session_id));
  ASSERT_EQ(response.session_id(), session_id);
  ASSERT_TRUE(rpcCalled("Sessions", "StopSubmission"));
}

TEST_F(Sessions, can_close_session) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;
  init(channel, task_options, log);

  armonik::api::client::SessionsClient client(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel));

  std::string session_id = client.create_session(task_options);
  num_create_session++;

  armonik::api::grpc::v1::sessions::SessionRaw response;
  ASSERT_NO_THROW(response = client.close_session(session_id));
  ASSERT_EQ(response.session_id(), session_id);
  ASSERT_TRUE(rpcCalled("Sessions", "CloseSession", 2));
}

/**
 * This test should be the last to run in the suit, which is why its name is prefixed with "z".
 */
TEST_F(Sessions, z_service_fully_implemented) { ASSERT_TRUE(all_rpc_called("Sessions")); }
