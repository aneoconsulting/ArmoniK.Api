#pragma once

#include "logger/logger.h"
#include "objects.pb.h"
#include <grpcpp/channel.h>
#include <memory>

/**
 * @brief Initializes task options creates channel with server address
 *
 * @param channel The gRPC channel to communicate with the server.
 * @param default_task_options The default task options.
 */
void init(std::shared_ptr<grpc::Channel> &channel, armonik::api::grpc::v1::TaskOptions &task_options,
          armonik::api::common::logger::Logger &logger);
