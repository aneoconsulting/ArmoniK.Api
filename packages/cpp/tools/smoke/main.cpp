#include <iostream>

#include "channel/ChannelFactory.h"
#include "exceptions/ArmoniKApiException.h"
#include "logger/formatter.h"
#include "logger/logger.h"
#include "logger/writer.h"
#include "utils/Configuration.h"
#include "versions/VersionsClient.h"

/**
 * Calls ListVersions on a port with no server behind it, the call is expected to fail with UNAVAILABLE.
 */
int main() {
  armonik::api::common::logger::Logger logger{armonik::api::common::logger::writer_console(),
                                              armonik::api::common::logger::formatter_plain(true)};
  armonik::api::common::utils::Configuration configuration;
  configuration.set("GrpcClient__Endpoint", "http://127.0.0.1:1");

  armonik::api::client::ChannelFactory channel_factory(configuration, logger);
  armonik::api::client::VersionsClient client(
      armonik::api::grpc::v1::versions::Versions::NewStub(channel_factory.create_channel()));

  try {
    client.list_versions();
  } catch (const armonik::api::common::exceptions::ArmoniKApiException &e) {
    std::cout << "Expected failure: " << e.what() << std::endl;
    return 0;
  }
  std::cerr << "ListVersions unexpectedly succeeded" << std::endl;
  return 1;
}
