#include "utils/ChannelArguments.h"
#include "options/ControlPlane.h"
#include <google/protobuf/util/json_util.h>
#include <sstream>

template <typename T, typename Tin> T saturate_cast(Tin in, T max_value = std::numeric_limits<T>::max()) {
  if (in > max_value) {
    return max_value;
  }
  return static_cast<T>(in);
}

int getMilliseconds(const google::protobuf::Duration &duration) {
  return saturate_cast<int>(duration.seconds() * 1000) + (duration.nanos() / 1000000);
}

std::string
armonik::api::common::utils::getServiceConfigJson(const armonik::api::common::options::ControlPlane &config) {
  std::stringstream ss;
  std::string initialBackoff, maxBackoff, timeout;
  auto status = google::protobuf::util::MessageToJsonString(config.getInitialBackoff(), &initialBackoff);
  if (!status.ok()) {
    throw std::invalid_argument("Initial backoff is invalid" + status.ToString());
  }
  status = google::protobuf::util::MessageToJsonString(config.getMaxBackoff(), &maxBackoff);
  if (!status.ok()) {
    throw std::invalid_argument("Max backoff is invalid" + status.ToString());
  }
  status = google::protobuf::util::MessageToJsonString(config.getRequestTimeout(), &timeout);
  if (!status.ok()) {
    throw std::invalid_argument("Timeout is invalid" + status.ToString());
  }
  ss << R"({ "methodConfig": [{ "name": [{}], )"
     << R"("timeout" : )" << timeout << ',' << R"("retryPolicy" : {)"
     << R"("backoffMultiplier": )" << config.getBackoffMultiplier() << ',' << R"("initialBackoff":)" << initialBackoff
     << ","
     << R"("maxBackoff":)" << maxBackoff << ","
     << R"("maxAttempts":)" << config.getMaxAttempts() << ','
     << R"("retryableStatusCodes": [ "UNAVAILABLE", "ABORTED", "UNKNOWN" ])"
     << "}}]}";
  return ss.str();
}

::grpc::ChannelArguments armonik::api::common::utils::getChannelArguments(const Configuration &config) {
  return getChannelArguments(config.get_control_plane());
}
::grpc::ChannelArguments armonik::api::common::utils::getChannelArguments(const options::ControlPlane &config) {
  ::grpc::ChannelArguments args;
  args.SetInt(GRPC_ARG_KEEPALIVE_TIME_MS, getMilliseconds(config.getKeepAliveTime()));
  args.SetInt(GRPC_ARG_MAX_CONNECTION_IDLE_MS, getMilliseconds(config.getMaxIdleTime()));
  args.SetServiceConfigJSON(getServiceConfigJson(config));
  return args;
}
