#ifndef ARMONIK_API_CONTROLPLANE_H
#define ARMONIK_API_CONTROLPLANE_H

#include "utils/Configuration.h"

namespace armonik::api::common::options {
class ControlPlane {
public:
  ControlPlane(const utils::Configuration &config) {
    endpoint_ = config.get(EndpointKey);
    user_cert_pem_path_ = config.get(UserCertKey);
    user_key_pem_path_ = config.get(UserKeyKey);
    user_p12_path_ = config.get(UserP12Key);
    ca_cert_pem_path_ = config.get(CaCertKey);
    sslValidation_ = config.get(SSLValidationKey) != "disable";
  }

  [[nodiscard]] std::string_view getEndpoint() const { return endpoint_; }
  [[nodiscard]] std::string_view getUserCertPemPath() const { return user_cert_pem_path_; }
  [[nodiscard]] std::string_view getUserKeyPemPath() const { return user_key_pem_path_; }
  [[nodiscard]] std::string_view getUserP12Path() const { return user_p12_path_; }
  [[nodiscard]] std::string_view getCaCertPemPath() const { return ca_cert_pem_path_; }
  [[nodiscard]] bool isSslValidation() const { return sslValidation_; }

  static constexpr char EndpointKey[] = "Grpc__EndPoint";
  static constexpr char UserCertKey[] = "Grpc__ClientCert";
  static constexpr char UserKeyKey[] = "Grpc__ClientKey";
  static constexpr char UserP12Key[] = "Grpc__ClientP12";
  static constexpr char CaCertKey[] = "Grpc__CaCert";
  static constexpr char SSLValidationKey[] = "Grpc__SSLValidation";

private:
  std::string endpoint_;
  std::string user_cert_pem_path_;
  std::string user_key_pem_path_;
  std::string user_p12_path_;
  std::string ca_cert_pem_path_;
  bool sslValidation_;
};
} // namespace armonik::api::common::options

#endif // ARMONIK_API_CONTROLPLANE_H
