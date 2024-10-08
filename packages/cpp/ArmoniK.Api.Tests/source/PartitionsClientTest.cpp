#include <gtest/gtest.h>

#include "common.h"
#include "logger/formatter.h"
#include "logger/logger.h"
#include "logger/writer.h"

#include "partitions/PartitionsClient.h"
#include "sessions/SessionsClient.h"

using Logger = armonik::api::common::logger::Logger;

/**
 * Fixture class for partition, inherit from MockFixture
 */
class Partitions : public MockFixture {};

TEST_F(Partitions, can_get_partition) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;
  init(channel, task_options, log);
  task_options.set_partition_id("default");

  armonik::api::client::PartitionsClient client(armonik::api::grpc::v1::partitions::Partitions::NewStub(channel));

  armonik::api::grpc::v1::partitions::PartitionRaw partition;
  ASSERT_NO_THROW(partition = client.get_partition(task_options.partition_id()));
  ASSERT_TRUE(rpcCalled("Partitions", "GetPartition"));
}

TEST_F(Partitions, can_list_partitions) {
  Logger log{armonik::api::common::logger::writer_console(), armonik::api::common::logger::formatter_plain(true)};
  std::shared_ptr<::grpc::Channel> channel;
  armonik::api::grpc::v1::TaskOptions task_options;
  init(channel, task_options, log);
  task_options.set_partition_id("default");

  armonik::api::client::PartitionsClient client(armonik::api::grpc::v1::partitions::Partitions::NewStub(channel));

  armonik::api::grpc::v1::partitions::Filters filters;
  armonik::api::grpc::v1::partitions::FilterField filter_field;
  filter_field.mutable_field()->mutable_partition_raw_field()->set_field(
      armonik::api::grpc::v1::partitions::PARTITION_RAW_ENUM_FIELD_ID);
  filter_field.mutable_filter_string()->set_value(task_options.partition_id());
  filter_field.mutable_filter_string()->set_operator_(armonik::api::grpc::v1::FILTER_STRING_OPERATOR_EQUAL);
  *filters.mutable_or_()->Add()->mutable_and_()->Add() = filter_field;

  int total;

  std::vector<armonik::api::grpc::v1::partitions::PartitionRaw> partitions;
  ASSERT_NO_THROW(partitions = client.list_partitions(filters, total));
  for (auto &&partition : partitions) {
    std::cout << *partition.mutable_id() << std::endl;
  }
  ASSERT_TRUE(rpcCalled("Partitions", "ListPartitions"));
}

/**
 * This test should be the last to run in the suit, which is why its name is prefixed with "z".
 */
TEST_F(Partitions, z_service_fully_implemented) { all_rpc_called("Partitions"); }
