#include "common.h"
#include "utils/Configuration.h"
#include <grpcpp/create_channel.h>
#include "channel/ChannelFactory.h"

/**
 * @brief Initializes task options creates channel with server address
 *
 * @param channel The gRPC channel to communicate with the server.
 * @param default_task_options The default task options.
 */
void init(std::shared_ptr<::grpc::Channel> &channel, armonik::api::grpc::v1::TaskOptions &default_task_options,
          armonik::api::common::logger::Logger &logger) {

  armonik::api::common::utils::Configuration configuration;
  // auto server = std::make_shared<EnvConfiguration>(configuration_t);

  configuration.add_json_configuration("appsettings.json").add_env_configuration();

  // std::string server_address = configuration.get("Grpc__EndPoint");

  armonik::api::client::ChannelFactory channel_factory(configuration, logger);

  channel = channel_factory.create_channel();

  logger.info(" Server address {address}", {{"address", configuration.get("Grpc__EndPoint")}});

  // channel = ::grpc::CreateChannel(server_address, grpc::InsecureChannelCredentials());

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
