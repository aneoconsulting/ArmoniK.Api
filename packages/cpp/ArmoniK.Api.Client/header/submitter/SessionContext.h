#pragma once
#include <iostream>
#include <memory>
#include <string>

#include <grpc++/grpc++.h>

#include "submitter_service.grpc.pb.h"

/**
 * @brief Represents a session context for interacting with the gRPC service.
 */
class SessionContext
{
public:
  /**
   * @brief Constructs a SessionContext object with the given gRPC channel and task options.
   *
   * @param channel A shared pointer to the gRPC channel.
   * @param task_options Task options for the session.
   */
  SessionContext(armonik::api::grpc::v1::TaskOptions task_options);

  /**
   * @brief Gets the session ID for the current session.
   *
   * @return A reference to the session ID string.
   */
  auto get_session_id() -> std::string& { return session_id_; }

  /**
   * @brief Sets the session ID for the current session.
   *
   * @param session_id A string representing the session ID.
   */
  void set_session_id(const std::string& session_id);

  /**
   * @brief Gets the task options for the current session.
   *
   * @return A reference to the task options object.
   */
  auto get_task_options() -> armonik::api::grpc::v1::TaskOptions& { return task_options_; }

private:
  std::string session_id_; ///< The session ID for the current session.
  armonik::api::grpc::v1::TaskOptions task_options_; ///< The task options for the current session.

};
