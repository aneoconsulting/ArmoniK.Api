//
// Created by fdenef on 06/03/2024.
//

#ifndef CHANNEL_H
#define CHANNEL_H

#include <memory>

#include <grpcpp/security/credentials.h>

#include <armonik/common/options/ControlPlane.h>

namespace armonik {
namespace api {
namespace client {

std::shared_ptr<grpc::ChannelCredentials> create_channel_credentials(const common::options::ControlPlane &ctrl_plane);

}
} // namespace api
} // namespace armonik

#endif // CHANNEL_H
