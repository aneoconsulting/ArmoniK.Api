#pragma once

/**
 * @namespace armonik::api::common::options
 * @brief This namespace contains common options for the armonik API.
 */
namespace armonik {
namespace api {
namespace common {
namespace options {
/**
 * @enum grpc_socket_type
 * @brief Enumerates the types of gRPC sockets supported by armonik API.
 */
enum grpc_socket_type {
  tcp = 1,             /**< @brief TCP/IP socket type */
  UnixDomainSocket = 2 /**< @brief Unix domain socket type */
};
} // namespace options
} // namespace common
} // namespace api
} // namespace armonik
