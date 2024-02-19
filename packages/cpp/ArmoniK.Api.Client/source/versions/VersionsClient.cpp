#include "versions/VersionsClient.h"
#include "exceptions/ArmoniKApiException.h"

using armonik::api::grpc::v1::versions::ListVersionsRequest;
using armonik::api::grpc::v1::versions::ListVersionsResponse;
using namespace armonik::api::grpc::v1::versions;

namespace armonik {
namespace api {
namespace client {

versions_info VersionsClient::list_versions() {
  ::grpc::ClientContext context;
  ListVersionsRequest request;
  ListVersionsResponse response;

  auto status = stub->ListVersions(&context, request, &response);
  if (!status.ok()) {
    throw armonik::api::common::exceptions::ArmoniKApiException("Could not get list versions : " +
                                                                status.error_message());
  }

  versions_info mapping = {response.api(), response.core()};

  return mapping;
}
} // namespace client
} // namespace api
} // namespace armonik
