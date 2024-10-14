#pragma once

#include "events_common.pb.h"
#include "events_service.grpc.pb.h"

namespace armonik {
namespace api {
namespace client {
class EventsClient {
public:
  explicit EventsClient(std::unique_ptr<armonik::api::grpc::v1::events::Events::StubInterface> stub)
      : stub(std::move(stub)) {}

  void wait_for_result_availability(std::string session_id, std::vector<std::string> result_ids);

private:
  std::unique_ptr<armonik::api::grpc::v1::events::Events::StubInterface> stub;
};
} // namespace client
} // namespace api
} // namespace armonik
