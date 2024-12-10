#include <gtest/gtest.h>

#include "common.h"
#include "logger/formatter.h"
#include "logger/logger.h"
#include "logger/writer.h"

#include "events/EventsClient.h"
#include "results/ResultsClient.h"
#include "results_service.grpc.pb.h"
#include "sessions/SessionsClient.h"

using Logger = armonik::api::common::logger::Logger;

/**
 * Fixture class for versions, inherit from MockFixture
 */
class Events : public MockFixture {};

TEST_F(Events, getEvents) {
  GTEST_SKIP() << "Mock server must return something ";
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;
  init(channel, task_options, log);

  armonik::api::client::EventsClient client(armonik::api::grpc::v1::events::Events::NewStub(channel));

  auto session_id = armonik::api::client::SessionsClient(armonik::api::grpc::v1::sessions::Sessions::NewStub(channel))
                        .create_session(task_options);

  auto result_client = armonik::api::client::ResultsClient(armonik::api::grpc::v1::results::Results::NewStub(channel));
  auto payload_id = result_client.create_results(
      session_id, std::vector<std::pair<std::string, std::string>>{{"name", "payload"}})["name"];
  auto result_id = result_client.create_results_metadata(session_id, {"result"})["result"];

  ASSERT_NO_THROW(result_client.upload_result_data(session_id, result_id, "name"));
  ASSERT_NO_THROW(client.wait_for_result_availability(session_id, {result_id, payload_id}));
  ASSERT_EQ(result_client.download_result_data(session_id, result_id), "name");
  ASSERT_TRUE(rpcCalled("Events", "GetEvents"));
}

/**
 * This test should be the last to run in the suit, which is why its name is prefixed with "z".
 */
TEST_F(Events, z_service_fully_implemented) {
  std::vector<std::string> missing_rpcs{"GetEvents"};
  ASSERT_TRUE(all_rpc_called("Events", missing_rpcs));
}
