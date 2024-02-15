#include "versions/VersionsClient.h"
#include "exceptions/ArmoniKApiException.h"

namespace armonik {
namespace api {
namespace client {

std::map<std::string, std::string> VersionsClient::list_versions() {
  ::grpc::ClientContext context;
  armonik::api::grpc::v1::versions::ListVersionsRequest request;
  armonik::api::grpc::v1::versions::ListVersionsResponse response;

  std::map<std::string, std::string> mapping;

  auto status = stub->ListVersions(&context, request, &response);
  if (!status.ok()) {
    throw armonik::api::common::exceptions::ArmoniKApiException("Could not get list versions : " +
                                                                status.error_message());
  }

  mapping.insert({"api", response.api()});
  mapping.insert({"core", response.core()});

  return mapping;
}
} // namespace client
} // namespace api
} // namespace armonik