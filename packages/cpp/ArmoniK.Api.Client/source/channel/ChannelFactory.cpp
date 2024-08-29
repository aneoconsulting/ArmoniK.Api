#include "channel/ChannelFactory.h"

#include "options/ControlPlane.h"
#include "utils/ChannelArguments.h"
#include "exceptions/ArmoniKApiException.h"
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

std::string get_key(const absl::string_view &path) {
  std::ifstream file(path.data(), std::ios::in | std::ios::binary);
  if (file.is_open()) {
    std::ostringstream sstr;
    sstr << file.rdbuf();
    return sstr.str();
  } else {
    return {};
  }
}

bool initialize_protocol_endpoint(const common::options::ControlPlane &controlPlane, std::string &endpoint) {
  absl::string_view endpoint_view = controlPlane.getEndpoint();
  const auto delim = endpoint_view.find("://");
  if (delim != absl::string_view::npos) {
    const auto tmp = endpoint_view.substr(delim + 3);
    endpoint_view = endpoint_view.substr(0, delim);
    endpoint = {tmp.cbegin(), tmp.cend()};
  }
  return endpoint_view.back() == 's' || endpoint_view.back() == 'S';
}

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

std::shared_ptr<grpc::Channel> ChannelFactory::create_channel(){
  auto channel = grpc::CreateCustomChannel(endpoint_, credentials_, common::utils::getChannelArguments(configuration_));
  logger_.log(common::logger::Level::Debug, "Created new channel ");

  if (channel != nullptr) {
    if (ShutdownOnFailure(channel)) {
      logger_.log(common::logger::Level::Debug, "Shutdown unhealthy channel");
    } else {
      logger_.log(common::logger::Level::Debug, "Valid channel");
    }
  }
  return channel;
}

ChannelFactory::ChannelFactory(armonik::api::common::utils::Configuration configuration, common::logger::Logger &logger) : logger_(logger.local()), configuration_(std::move(configuration)){
  const auto control_plane = configuration_.get_control_plane();
  const bool is_https = initialize_protocol_endpoint(control_plane, endpoint_);

  auto root_cert_pem = get_key(control_plane.getCaCertPemPath());
  auto user_private_pem = get_key(control_plane.getUserKeyPemPath());
  auto user_public_pem = get_key(control_plane.getUserCertPemPath());

  if(is_https){
    if(!user_private_pem.empty() && !user_public_pem.empty()){
      if(control_plane.isSslValidation()){
        credentials_ = grpc::SslCredentials(grpc::SslCredentialsOptions{std::move(root_cert_pem), std::move(user_private_pem), std::move(user_public_pem)});
      }else{
        throw common::exceptions::ArmoniKApiException("mTLS without SSL validation is not supported.");
      }
    }else{
      if(control_plane.isSslValidation()){
        credentials_ = grpc::SslCredentials(grpc::SslCredentialsOptions{std::move(root_cert_pem)});
      } else{
        TlsChannelCredentialsOptions tls_options;
        tls_options.set_certificate_provider(create_certificate_provider("", user_public_pem, user_private_pem));
        tls_options.set_verify_server_certs(control_plane.isSslValidation());
        credentials_ = TlsCredentials(tls_options);
      }

    }
    is_secure_ = true;
  }else{
    credentials_ = grpc::InsecureChannelCredentials();
  }
}

bool ChannelFactory::ShutdownOnFailure(std::shared_ptr<grpc::Channel> channel) {
  switch ((*channel).GetState(true)) {
  case GRPC_CHANNEL_CONNECTING:
    // std::cout << "CONNECTING" << std::endl;
    break;
  case GRPC_CHANNEL_IDLE:
    // std::cout << "IDLE" << std::endl;
    break;

  case GRPC_CHANNEL_SHUTDOWN:
    // std::cout << "SHUTDOWN" << std::endl;
    return true;
    break;

  case GRPC_CHANNEL_TRANSIENT_FAILURE:
    // std::cout << "TRANSIENT FAILURE" << std::endl;
    channel.reset();
    return true;
    break;

  case GRPC_CHANNEL_READY:
    // std::cout << "READY" << std::endl;
    break;

  default:
    return false;
    break;
  }
  return false;
}

} // namespace client
} // namespace api
} // namespace armonik
