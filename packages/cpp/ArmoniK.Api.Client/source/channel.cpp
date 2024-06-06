//
// Created by fdenef on 06/03/2024.
//

#include "channel.h"

#include <fstream>
#include <sstream>

#include <grpcpp/security/credentials.h>
#include <grpcpp/security/tls_certificate_provider.h>
#include <grpcpp/security/tls_credentials_options.h>

#include "exceptions/ArmoniKApiException.h"

namespace armonik {
namespace api {
namespace client {

static const std::string root_self_signed = R"(-----BEGIN CERTIFICATE REQUEST-----
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

bool is_https(absl::string_view endpoint) noexcept {
  const auto delim = endpoint.find("://");
  if (delim != absl::string_view::npos) {
    const auto endpoint_view = endpoint.substr(0, delim);
    return endpoint_view.back() == 's' || endpoint_view.back() == 'S';
  }
  return false;
}

std::string read_file(absl::string_view path) {
  std::ifstream file(path.data(), std::ios::in | std::ios::binary);
  if (file.is_open()) {
    std::ostringstream sstr;
    sstr << file.rdbuf();
    return sstr.str();
  } else {
    return {};
  }
}

std::shared_ptr<grpc::experimental::CertificateProviderInterface>
create_certificate_provider(std::string rootCertificate, std::string userPublicPem, std::string userPrivatePem) {
  using grpc::experimental::IdentityKeyCertPair;
  using grpc::experimental::StaticDataCertificateProvider;

  if (rootCertificate.empty()) {
    return std::make_shared<StaticDataCertificateProvider>(
        std::vector<IdentityKeyCertPair>{IdentityKeyCertPair{std::move(userPrivatePem), std::move(userPublicPem)}});
  } else if (userPrivatePem.empty() || userPublicPem.empty()) {
    return std::make_shared<StaticDataCertificateProvider>(std::move(rootCertificate));
  } else {
    return std::make_shared<StaticDataCertificateProvider>(
        std::move(rootCertificate),
        std::vector<IdentityKeyCertPair>{IdentityKeyCertPair{std::move(userPrivatePem), std::move(userPublicPem)}});
  }
}

std::shared_ptr<grpc::ChannelCredentials> create_channel_credentials(const common::options::ControlPlane &ctrl_plane) {
  if (is_https(ctrl_plane.getEndpoint())) {
    auto root_cert_pem = read_file(ctrl_plane.getCaCertPemPath());
    auto user_private_pem = read_file(ctrl_plane.getUserKeyPemPath());
    auto user_public_pem = read_file(ctrl_plane.getUserCertPemPath());

    if (!user_private_pem.empty() && !user_public_pem.empty()) {
      if (ctrl_plane.isSslValidation()) {
        return grpc::SslCredentials(grpc::SslCredentialsOptions{std::move(root_cert_pem), std::move(user_private_pem),
                                                                std::move(user_public_pem)});
      } else {
        // FIXME: This part is dead because this code as is does not send the client's certificate.
        /*
        grpc::experimental::TlsChannelCredentialsOptions tls_options;
        tls_options.set_verify_server_certs(false);
        tls_options.set_certificate_provider(
            create_certificate_provider(root_cert_pem, user_public_pem, user_private_pem));
        return grpc::experimental::TlsCredentials(tls_options);
        */
        throw common::exceptions::ArmoniKApiException("mTLS without SSL validation is not supported.");
      }
    } else {
      if (ctrl_plane.isSslValidation()) {
        return grpc::SslCredentials(grpc::SslCredentialsOptions{std::move(root_cert_pem)});
      } else {
        grpc::experimental::TlsChannelCredentialsOptions tls_options;
        tls_options.set_certificate_provider(
            create_certificate_provider(root_self_signed, std::move(user_public_pem), std::move(user_private_pem)));
        tls_options.set_verify_server_certs(false);
        return grpc::experimental::TlsCredentials(tls_options);
      }
    }
  } else {
    return grpc::InsecureChannelCredentials();
  }
}
} // namespace client
} // namespace api
} // namespace armonik
