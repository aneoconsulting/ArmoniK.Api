#ifndef ARMONIK_API_CONTROLPLANE_H
#define ARMONIK_API_CONTROLPLANE_H

#include "utils/IConfiguration.h"

namespace armonik::api::common::options {
class ControlPlane {
public:
  ControlPlane(const utils::IConfiguration &config) {
    endpoint = config.get(EndpointKey);
    user_cert_pem_path = config.get(UserCertKey);
    user_key_pem_path = config.get(UserKeyKey);
    user_p12_path = config.get(UserP12Key);
    ca_cert_perm_path = config.get(CaCertKey);
    sslValidation = config.get(SSLValidationKey) != "disable";
  }

  [[nodiscard]] std::string_view getEndpoint() const { return endpoint; }
  [[nodiscard]] std::string_view getUserCertPemPath() const { return user_cert_pem_path; }
  [[nodiscard]] std::string_view getUserKeyPemPath() const { return user_key_pem_path; }
  [[nodiscard]] std::string_view getUserP12Path() const { return user_p12_path; }
  [[nodiscard]] std::string_view getCaCertPermPath() const { return ca_cert_perm_path; }
  [[nodiscard]] bool isSslValidation() const { return sslValidation; }

  static constexpr char EndpointKey[] = "Grpc__EndPoint";
  static constexpr char UserCertKey[] = "Grpc__ClientCert";
  static constexpr char UserKeyKey[] = "Grpc__ClientKey";
  static constexpr char UserP12Key[] = "Grpc__ClientP12";
  static constexpr char CaCertKey[] = "Grpc__CaCert";
  static constexpr char SSLValidationKey[] = "Grpc__SSLValidation";

private:
  std::string endpoint;
  std::string user_cert_pem_path;
  std::string user_key_pem_path;
  std::string user_p12_path;
  std::string ca_cert_perm_path;
  bool sslValidation;
};
} // namespace armonik::api::common::options

#endif // ARMONIK_API_CONTROLPLANE_H
