//
// Created by fdenef on 06/03/2024.
//

#include "channel.h"
#include "common.h"
#include "options/ControlPlane.h"
#include "sessions/SessionsClient.h"
#include "utils/Configuration.h"

#include <grpcpp/create_channel.h>
#include <gtest/gtest.h>

namespace grpc_armonik = armonik::api::grpc::v1;

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

TEST(ChannelCredentials, unsecure) {
  armonik::api::common::utils::Configuration configuration;
  configuration.add_json_configuration("appsettings.json").add_env_configuration();

  const auto ctrl_plane = configuration.get_control_plane();
  const std::string server_address{ctrl_plane.getEndpoint().cbegin(), ctrl_plane.getEndpoint().cend()};

  ASSERT_TRUE(ctrl_plane.getEndpoint().substr(0, 4) == "http:");

  const auto credentials = armonik::api::client::create_channel_credentials(ctrl_plane);
  const auto channel = grpc::CreateChannel(server_address, credentials);

  armonik::api::client::SessionsClient client(grpc_armonik::sessions::Sessions::NewStub(channel));

  std::string response;
  ASSERT_NO_THROW(response = client.create_session(default_task_options()));
  ASSERT_FALSE(response.empty());

  ASSERT_TRUE(client.get_session(response).status() == grpc_armonik::session_status::SESSION_STATUS_RUNNING);
}
