#include "channel/ChannelFactory.h"

#include "exceptions/ArmoniKApiException.h"
#include "options/ControlPlane.h"
#include "utils/ChannelArguments.h"
#include <grpcpp/create_channel.h>
#include <grpcpp/security/credentials.h>
#include <grpcpp/security/tls_credentials_options.h>

#include <fstream>
#include <sstream>
#include <utility>

namespace armonik {
namespace api {
namespace client {

using namespace grpc::experimental;

/**
 * In TLS without SSL validation, this certificate is used for the function TlsCredentials options when a root
 * certificate is not provided
 */
const std::string root_self_signed = R"(-----BEGIN CERTIFICATE REQUEST-----
MIIEhjCCAm4CAQAwQTELMAkGA1UEBhMCRlIxEzARBgNVBAgMClNvbWUtU3RhdGUx
DjAMBgNVBAcMBVBhcmlzMQ0wCwYDVQQKDARBbmVvMIICIjANBgkqhkiG9w0BAQEF
AAOCAg8AMIICCgKCAgEA3lBl8so+JRen+tfbrytXMmYAvjt/WquctbkbFIN6prdp
uShiRb6kX9jobcOQCleQ08LBLPPoQ7AemymPxT0dq+YPFw33LgrIBpKe0JWYzujB
Ujj39b1EmKonnsx+C6DL2KSkIf7ayoBNdjDgunWkVC4M6hoJE7XYyZ78HKndfuvL
C4zs3o1EizvSpp+O/IzD/y5pnZEBoxMLCRNB8vD7w7mQMhx+6Amx7KkfCDKLOQO4
/K2x8r4Y65+IvxFMyxUsR1Z5XPVv37u7u2akbh3HlUE+m0xzVOk+BmHFYxm/eEAF
4p1Jt3bZWu03eF4f8tmgN31Rv0uV+BRN7na44inXNnyd+2qczaCI1IQmsy23Vu0A
eX61Gu06ifViJAybbcWll3VQjWqj5XtsN2+yr2bGfZw8fpjGXVWTL0+nZSqZPWSo
IYlXMHjcygWyMJXTMVTTN+fV7dd9s1LFVnpdHFFOtmRzY8FlRRSpOoqG8XQXXsk0
pE9904wHaXcwSEe4KtuzgZgNngRCtT61G6k+onhrGa6UVCKpfvMYtS3NEsMNNYsY
I5Hn7Unj/0xBO6IM5Os6PImWWMk8rLSXC3IdtEAHgShS+/xbh2ZVOveSeMXWaecm
u2RIe5wQa5ZXLr03XtkdMB1pebJbdoFrs0ev/sklk1dZfbX06vJSd8eokM9oIIcC
AwEAAaAAMA0GCSqGSIb3DQEBCwUAA4ICAQCr75dBYjypzqDqQ6TiKWuYO8rq6TIh
pdZHw5ystwvD6sn+tPbc7iNbnvDF6GeTgMdKAuwNz0YJMZq9v39hZzTCyMqRLNlT
TU3kYaTWGHDK0HE2O3pHKppHAc2YbAsSxuS8KMHx0wW0abVHiEeudc/nULJppX1/
ObouzLGSJJwZctXEzk/Ye7bD1sneSqVnrdFD1IOBVQVRGoJznAt7WWxvGk9LPW51
+MybzTilL4rk5+ezA4UCIMrQCDwZcI+UCcKqDajDz+7kn81f1K4g1G6dTh+M8qIV
lx6/Bfy3P6DHF1ww0i/hRQht1O9cyUo3mDZzAq20OsIDvkhjNGma/IEbkZ9z0P5C
/5YwAW+GuwG2GrD016y5OjZVrAG/KIfyS6FLQfgN/ww5Y9tK6vO5XkelED7zNPrq
em1zkId2H0Az5dIC2OpnAg3+NuGrehfIXziiY+8MGIivqI/Rulnv7m2l2vjHi66K
GztDm5ohMdfjitFIfPDFYPMH7KES4vivic8zlq9FJYNp8tUYEBR1wW7W03IJPm6e
pUwvXHPjId/qBjlBixZt2ZqC8X4S95wAfVjtS3O33Zsm4oevwlvywfYIK8nTG5SD
bDCNVTg3w/OQLQQdWUl6FunmYinukBgmqnsJnwgrhzBENbmgbgfOZZWGtG5ODENb
wc+KqiSg9c9iqA==
-----END CERTIFICATE REQUEST-----)";

/**
 *
 * @param path
 * @return
 */
std::string read_file(const absl::string_view &path) {
  std::ifstream file(path.data(), std::ios::in | std::ios::binary);
  if (file.is_open()) {
    std::ostringstream sstr;
    sstr << file.rdbuf();
    return sstr.str();
  } else {
    return {};
  }
}

/**
 * @brief Check if it's https connexion
 * @param controlPlane  The control plane object for the current configuration
 * @param endpoint The endpoint
 * @return a boolean on wether http or https connexion
 */
bool initialize_protocol_endpoint(const common::options::ControlPlane &controlPlane, std::string &endpoint) {
  absl::string_view endpoint_view = controlPlane.getEndpoint();
  const auto delim = endpoint_view.find("://");
  const auto http_delim = endpoint_view.find("http://");
  const auto https_delim = endpoint_view.find("https://");
  if (endpoint_view.find("unix") == 0) {
    if (endpoint_view[0] == '/') {
      endpoint.insert(0, "unix://");
    } else {
      endpoint.insert(0, "unix:");
    }
    return false;
  }
  if (https_delim != absl::string_view::npos) {
    const auto tmp = endpoint_view.substr(https_delim + 8);
    endpoint = {tmp.cbegin(), tmp.cend()};
    return true;
  } else {
    if (http_delim != absl::string_view::npos) {
      const auto tmp = endpoint_view.substr(http_delim + 7);
      endpoint = {tmp.cbegin(), tmp.cend()};
    } else {
      endpoint = {endpoint_view.cbegin(), endpoint_view.cend()};
    }
    return false;
  }
}

/**
 *
 * @param rootCertificate The root certificate to validate the server one against
 * @param userPublicPem The client certificate for mTLS
 * @param userPrivatePem The client key for mTLS
 * @return a pointer to a certificate provider interface
 */
std::shared_ptr<CertificateProviderInterface> create_certificate_provider(const std::string &rootCertificate,
                                                                          const std::string &userPublicPem,
                                                                          const std::string &userPrivatePem) {
  if (rootCertificate.empty()) {
    return std::make_shared<StaticDataCertificateProvider>(
        std::vector<IdentityKeyCertPair>{IdentityKeyCertPair{userPrivatePem, userPublicPem}});
  } else if (userPrivatePem.empty() || userPublicPem.empty()) {
    return std::make_shared<StaticDataCertificateProvider>(rootCertificate);
  } else {
    return std::make_shared<StaticDataCertificateProvider>(
        rootCertificate, std::vector<IdentityKeyCertPair>{IdentityKeyCertPair{userPrivatePem, userPublicPem}});
  }
}

std::shared_ptr<grpc::Channel> ChannelFactory::create_channel() {
  auto channel = grpc::CreateCustomChannel(endpoint_, credentials_, common::utils::getChannelArguments(configuration_));
  logger_.log(common::logger::Level::Debug, "Created new channel ");

  return channel;
}

ChannelFactory::ChannelFactory(armonik::api::common::utils::Configuration configuration, common::logger::Logger &logger)
    : logger_(logger.local()), configuration_(std::move(configuration)) {
  const auto control_plane = configuration_.get_control_plane();
  const bool is_https = initialize_protocol_endpoint(control_plane, endpoint_);

  auto root_cert_pem = read_file(control_plane.getCaCertPemPath());
  auto user_private_pem = read_file(control_plane.getUserKeyPemPath());
  auto user_public_pem = read_file(control_plane.getUserCertPemPath());

  if (is_https) {
    if (!user_private_pem.empty() && !user_public_pem.empty()) {
      if (control_plane.isSslValidation()) {
        credentials_ = grpc::SslCredentials(grpc::SslCredentialsOptions{
            std::move(root_cert_pem), std::move(user_private_pem), std::move(user_public_pem)});
      } else {
        throw common::exceptions::ArmoniKApiException("mTLS without SSL validation is not supported.");
      }
    } else {
      if (control_plane.isSslValidation()) {
        credentials_ = grpc::SslCredentials(grpc::SslCredentialsOptions{std::move(root_cert_pem)});
      } else {
        TlsChannelCredentialsOptions tls_options;
        // Set up TLS credentials options by setting root certificate to random certificate
        tls_options.set_certificate_provider(
            create_certificate_provider(root_self_signed, user_public_pem, user_private_pem));
        // Disable SSL certificate validation by setting verify_server to false
        tls_options.set_verify_server_certs(control_plane.isSslValidation());
        // Create TLS credentials with the specified options
        credentials_ = TlsCredentials(tls_options);
      }
    }
    is_secure_ = true;
  } else {
    // Create gRPC insecure credentials
    credentials_ = grpc::InsecureChannelCredentials();
  }
}

bool ChannelFactory::isSecureChannel() const noexcept { return is_secure_; }

} // namespace client
} // namespace api
} // namespace armonik
