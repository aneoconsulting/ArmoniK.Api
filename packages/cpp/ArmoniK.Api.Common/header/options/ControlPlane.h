#ifndef ARMONIK_API_CONTROLPLANE_H
#define ARMONIK_API_CONTROLPLANE_H

#include "utils/Configuration.h"
#include <cmath>
#include <google/protobuf/duration.pb.h>

namespace armonik {
namespace api {
namespace common {
namespace options {
class ControlPlane {
public:
  ControlPlane(const utils::Configuration &config);

  absl::string_view getEndpoint() const { return endpoint_; }
  absl::string_view getUserCertPemPath() const { return user_cert_pem_path_; }
  absl::string_view getUserKeyPemPath() const { return user_key_pem_path_; }
  absl::string_view getUserP12Path() const { return user_p12_path_; }
  absl::string_view getCaCertPemPath() const { return ca_cert_pem_path_; }
  bool isSslValidation() const { return sslValidation_; }
  const google::protobuf::Duration &getKeepAliveTime() const { return keep_alive_time_; }
  const google::protobuf::Duration &getKeepAliveTimeInterval() const { return keep_alive_time_interval_; }
  const google::protobuf::Duration &getMaxIdleTime() const { return max_idle_time_; }
  int getMaxAttempts() const { return max_attempts_; }
  double getBackoffMultiplier() const { return backoff_multiplier_; }
  const google::protobuf::Duration &getInitialBackoff() const { return initial_backoff_; }
  const google::protobuf::Duration &getMaxBackoff() const { return max_backoff_; }
  const google::protobuf::Duration &getRequestTimeout() const { return request_timeout_; }

  static constexpr char EndpointKey[] = "Grpc__EndPoint";
  static constexpr char UserCertKey[] = "Grpc__ClientCert";
  static constexpr char UserKeyKey[] = "Grpc__ClientKey";
  static constexpr char UserP12Key[] = "Grpc__ClientP12";
  static constexpr char CaCertKey[] = "Grpc__CaCert";
  static constexpr char SSLValidationKey[] = "Grpc__SSLValidation";
  static constexpr char KeepAliveTimeKey[] = "Grpc__KeepAliveTime";
  static constexpr char KeepAliveTimeIntervalKey[] = "Grpc__KeepAliveTimeInterval";
  static constexpr char MaxIdleTimeKey[] = "Grpc__MaxIdleTime";
  static constexpr char MaxAttemptsKey[] = "Grpc__MaxAttempts";
  static constexpr char BackoffMultiplierKey[] = "Grpc__BackoffMultiplier";
  static constexpr char InitialBackOffKey[] = "Grpc__InitialBackOff";
  static constexpr char MaxBackOffKey[] = "Grpc__MaxBackOff";
  static constexpr char RequestTimeoutKey[] = "Grpc__RequestTimeout";

private:
  std::string endpoint_;
  std::string user_cert_pem_path_;
  std::string user_key_pem_path_;
  std::string user_p12_path_;
  std::string ca_cert_pem_path_;
  ::google::protobuf::Duration keep_alive_time_;
  ::google::protobuf::Duration keep_alive_time_interval_;
  ::google::protobuf::Duration max_idle_time_;
  int max_attempts_{};
  double backoff_multiplier_{};
  ::google::protobuf::Duration initial_backoff_;
  ::google::protobuf::Duration max_backoff_;
  ::google::protobuf::Duration request_timeout_;
  bool sslValidation_;
};
} // namespace options
} // namespace common
} // namespace api
} // namespace armonik

#endif // ARMONIK_API_CONTROLPLANE_H
