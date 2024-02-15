#pragma once

#include "versions_common.pb.h"
#include "versions_service.grpc.pb.h"

namespace armonik {
namespace api {
namespace client {

/**
 * Versions Client wrapper
 */
class VersionsClient {
public:
  explicit VersionsClient(std::unique_ptr<armonik::api::grpc::v1::versions::Versions::StubInterface> stub)
      : stub(std::move(stub)){};

  /**
   * Get versions of ArmoniK components
   * @return Mapping between component names and their versions
   */
  std::map<std::string, std::string> list_versions();

private:
  std::unique_ptr<armonik::api::grpc::v1::versions::Versions::StubInterface> stub;
};
} // namespace client
} // namespace api
} // namespace armonik
