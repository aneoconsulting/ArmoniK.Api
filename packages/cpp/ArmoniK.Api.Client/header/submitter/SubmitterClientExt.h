/**
 * @file submitter_client_ext.h
 * @brief This file contains the SubmitterClientExt class definition.
 */

#pragma once
#include <future>
#include <string>

#include "submitter_common.pb.h"
#include "submitter_service.grpc.pb.h"

#include "SessionContext.h"

 /**
  * @brief The SubmitterClientExt class provides methods to create and manage task submissions.
  */
class SubmitterClientExt
{
private:

  grpc::ClientContext context_;
  std::unique_ptr<armonik::api::grpc::v1::submitter::Submitter::Stub> stub_;



public:
  /**
   * @brief Creates a new session with the control plane.
   * @param task_options The task options.
   * @param partition_ids The partition ids.
   */
    std::string create_session(armonik::api::grpc::v1::TaskOptions task_options,
                            const std::vector<std::string>& partition_ids);

  /**
   * @brief Cancels a created session.
   * @param session_id The id of the session to be canceled.
   */
  void cancel_session(const std::string& session_id);



  /**
   * @brief Converts task requests into a vector of future large task request objects.
   * @param task_requests The vector of task requests.
   * @param session_id The session ID.
   * @param task_options The task options.
   * @param chunk_max_size The maximum chunk size.
   * @return A vector of future large task request objects.
   */
  static auto to_request_stream(
    const std::vector<armonik::api::grpc::v1::TaskRequest>& task_requests,
    std::string& session_id,
    const armonik::api::grpc::v1::TaskOptions& task_options,
    size_t chunk_max_size)->std::vector<std::future<std::vector<armonik::api::grpc::v1::submitter::
    CreateLargeTaskRequest>>>;

  /**
   * @brief Creates a large task request object with specified parameters.
   * @param task_request The task request.
   * @param is_last Indicates if this is the last task request in the stream.
   * @param chunk_max_size The maximum chunk size.
   * @return A future large task request object.
   */
  static std::future<std::vector<armonik::api::grpc::v1::submitter::CreateLargeTaskRequest>> task_chunk_stream(
    const armonik::api::grpc::v1::TaskRequest& task_request,
    bool is_last, size_t chunk_max_size);

  /**
   * @brief Creates tasks asynchronously with the specified options and requests.
   * @param channel The gRPC channel to communicate with the server.
   * @param session_id The session ID.
   * @param task_options The task options.
   * @param task_requests The vector of task requests.
   * @return A future create task reply object.
   */
  static std::future<armonik::api::grpc::v1::submitter::CreateTaskReply> create_tasks_async(
    const std::shared_ptr<grpc::Channel>& channel,
    std::string& session_id, const armonik::api::grpc::v1::TaskOptions& task_options,
    const std::vector<armonik::api::grpc::v1::TaskRequest>& task_requests);

  /**
   * @brief Submits tasks with dependencies to the session context.
   * @param session_context The session context.
   * @param payloads_with_dependencies A vector of tuples containing the payload, its data, and its dependencies.
   * @param max_retries The maximum number of retries for submitting tasks.
   * @return A vector of submitted task IDs.
   */
  static std::vector<std::string> submit_tasks_with_dependencies(SessionContext& session_context,
    std::vector<std::tuple<std::string, std::vector<char>, std::
    vector<std::string>>> payloads_with_dependencies,
    int max_retries);

    /**
   * @brief Get  result without streaming.
   * @param channel The gRPC channel to communicate with the server.
   * @param session_id The session ID.
   * @param task_options The task options.
   * @param result_requests The vector of result requests.
   * @return A vector containting the data associated to the result
   */
  static std::future<std::vector<int8_t>> get_result_async(
    const std::shared_ptr<grpc::Channel>& channel,
    std::string& session_id, const armonik::api::grpc::v1::TaskOptions& task_options,
    const armonik::api::grpc::v1::ResultRequest& result_requests);

};
