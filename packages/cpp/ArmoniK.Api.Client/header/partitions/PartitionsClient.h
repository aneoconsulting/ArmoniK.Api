#pragma once

#include "partitions_common.pb.h"
#include "partitions_service.grpc.pb.h"

namespace armonik {
namespace api {
namespace client {

class PartitionsClient {
public:
  explicit PartitionsClient(std::unique_ptr<armonik::api::grpc::v1::partitions::Partitions::StubInterface> stub)
      : stub(std::move(stub)){};
  std::vector<armonik::api::grpc::v1::partitions::PartitionRaw>
  list_partitions(armonik::api::grpc::v1::partitions::Filters filters, int32_t &total, int32_t page = -1,
                  int32_t page_size = 500,
                  armonik::api::grpc::v1::partitions::ListPartitionsRequest::Sort sort = default_sort());

  armonik::api::grpc::v1::partitions::PartitionRaw get_partition(std::string partition_id);

private:
  std::unique_ptr<armonik::api::grpc::v1::partitions::Partitions::StubInterface> stub;
  static armonik::api::grpc::v1::partitions::ListPartitionsRequest::Sort default_sort();
};
} // namespace client
} // namespace api
} // namespace armonik