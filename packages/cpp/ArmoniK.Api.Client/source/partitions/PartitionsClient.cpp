#include <utility>

#include "exceptions/ArmoniKApiException.h"
#include "partitions/PartitionsClient.h"

using namespace armonik::api::grpc::v1::partitions;

namespace armonik {
namespace api {
namespace client {

std::vector<PartitionRaw> PartitionsClient::list_partitions(Filters filters, int32_t &total, int32_t page,
                                                            int32_t page_size, ListPartitionsRequest::Sort sort) {
  ::grpc::ClientContext context;
  ListPartitionsRequest request;
  ListPartitionsResponse response;

  *request.mutable_filters() = std::move(filters);
  *request.mutable_sort() = std::move(sort);
  request.set_page_size(page_size);

  if (page >= 0) {
    request.set_page(page);
    ::grpc::ClientContext context;
    auto status = stub->ListPartitions(&context, request, &response);
    if (!status.ok()) {
      throw armonik::api::common::exceptions::ArmoniKApiException("Unable to list partitions " +
                                                                  status.error_message());
    }
    total = response.total();
    return {std::make_move_iterator(response.partitions().begin()),
            std::make_move_iterator(response.partitions().end())};
  } else {
    std::vector<PartitionRaw> rawPartitions;
    int current_page = 0;
    do {
      request.set_page(current_page);
      ::grpc::ClientContext context;
      auto status = stub->ListPartitions(&context, request, &response);
      if (!status.ok()) {
        throw armonik::api::common::exceptions::ArmoniKApiException("Unable to list partitions " +
                                                                    status.error_message());
      }
      rawPartitions.insert(rawPartitions.end(), std::make_move_iterator(response.partitions().begin()),
                           std::make_move_iterator(response.partitions().end()));
      if (response.partitions_size() >= page_size) {
        current_page++;
      }

      response.clear_partitions();
    } while ((int32_t)rawPartitions.size() < response.total());

    total = response.total();

    return rawPartitions;
  }
}

PartitionRaw PartitionsClient::get_partition(std::string partition_id) {
  ::grpc::ClientContext context;
  GetPartitionRequest request;
  GetPartitionResponse response;

  *request.mutable_id() = std::move(partition_id);
  auto status = stub->GetPartition(&context, request, &response);
  if (!status.ok()) {
    throw armonik::api::common::exceptions::ArmoniKApiException("Could not get partition : " + status.error_message());
  }

  return std::move(*response.mutable_partition());
}

ListPartitionsRequest::Sort PartitionsClient::default_sort() {
  ListPartitionsRequest::Sort sort;
  sort.set_direction(grpc::v1::sort_direction::SORT_DIRECTION_ASC);
  sort.mutable_field()->mutable_partition_raw_field()->set_field(
      grpc::v1::partitions::PARTITION_RAW_ENUM_FIELD_PRIORITY);
  return sort;
}

} // namespace client
} // namespace api
} // namespace armonik
