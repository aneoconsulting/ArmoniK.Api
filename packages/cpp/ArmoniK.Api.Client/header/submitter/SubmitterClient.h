/**
 * @file submitter_client_ext.h
 * @brief This file contains the SubmitterClient class definition.
 */

#pragma once
#include <future>
#include <string>

#include "submitter_common.pb.h"
#include "submitter_service.grpc.pb.h"

namespace armonik {
namespace api {
namespace client {

/**
 * @brief Data structure for task payload
 * @param keys The expected output keys
 * @param payload The task payload
 * @param dependencies The dependencies of the task
 *
 */
struct payload_data {
  std::string keys;
  std::string payload;
  std::vector<std::string> dependencies;
};

/**
 * @brief The SubmitterClientExt class provides methods to create and manage task submissions.
 */
class [[deprecated("Use the Session, Task and Result clients instead")]] SubmitterClient {
private:
  std::unique_ptr<armonik::api::grpc::v1::submitter::Submitter::StubInterface> stub_;

public:
  /**
   * @brief Construct a new Submitter Client object
   *
   */
  SubmitterClient(std::unique_ptr<armonik::api::grpc::v1::submitter::Submitter::StubInterface> stub);

  /**
   * @brief Creates a new session with the control plane.
   * @param default_task_options The default task options.
   * @param partition_ids The partition ids.
   */
  std::string create_session(armonik::api::grpc::v1::TaskOptions default_task_options,
                             const std::vector<std::string> &partition_ids);

  /**
   * @brief Converts task requests into a vector of future large task request objects.
   * @param task_requests The vector of task requests.
   * @param session_id The session ID.
   * @param task_options The task options.
   * @param chunk_max_size The maximum chunk size.
   * @return A vector of future large task request objects.
   */
  static std::vector<std::future<std::vector<armonik::api::grpc::v1::submitter::CreateLargeTaskRequest>>>
  to_request_stream(const std::vector<armonik::api::grpc::v1::TaskRequest> &task_requests, std::string session_id,
                    armonik::api::grpc::v1::TaskOptions task_options, size_t chunk_max_size);

  /**
   * @brief Creates a large task request object with specified parameters.
   * @param task_request The task request.
   * @param is_last Indicates if this is the last task request in the stream.
   * @param chunk_max_size The maximum chunk size.
   * @return A future large task request object.
   */
  static std::future<std::vector<armonik::api::grpc::v1::submitter::CreateLargeTaskRequest>>
  task_chunk_stream(const armonik::api::grpc::v1::TaskRequest &task_request, bool is_last, size_t chunk_max_size);

  /**
   * @brief Creates tasks asynchronously with the specified options and requests.
   * @param session_id The session ID.
   * @param task_options The task options.
   * @param task_requests The vector of task requests.
   * @return A future create task reply object.
   */
  std::future<armonik::api::grpc::v1::submitter::CreateTaskReply>
  create_tasks_async(std::string session_id, armonik::api::grpc::v1::TaskOptions task_options,
                     const std::vector<armonik::api::grpc::v1::TaskRequest> &task_requests);

  /**
   * @brief Submits tasks with dependencies to the session context.
   * @param session_id The session id.
   * @param task_options The task options.
   * @param payloads_with_dependencies A vector of tuples containing the payload, its data, and its dependencies.
   * @param max_retries The maximum number of retries for submitting tasks.
   * @return A vector of submitted task IDs.
   */
  std::pair<std::vector<std::string>, std::vector<std::string>>
  submit_tasks_with_dependencies(std::string session_id, armonik::api::grpc::v1::TaskOptions task_options,
                                 const std::vector<payload_data> &payloads_with_dependencies, int max_retries);

  /**
   * @brief Get result without streaming.
   * @param result_request The vector of result requests.
   * @return A vector containing the data associated to the result
   */
  std::future<std::string> get_result_async(const armonik::api::grpc::v1::ResultRequest &result_request);

  std::map<std::string, armonik::api::grpc::v1::result_status::ResultStatus>
  get_result_status(const std::string &session_id, const std::vector<std::string> &result_ids);
};

} // namespace client
} // namespace api
} // namespace armonik
