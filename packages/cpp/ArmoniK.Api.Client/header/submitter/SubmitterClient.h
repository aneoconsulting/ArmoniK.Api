/**
 * @file submitter_client_ext.h
 * @brief This file contains the SubmitterClient class definition.
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
class SubmitterClient
{
private:

  grpc::ClientContext context_;
  //armonik::api::grpc::v1::submitter::Submitter::StubInterface* stub_;
  std::unique_ptr<armonik::api::grpc::v1::submitter::Submitter::StubInterface> stub_;
  


public:

  /**
   * @brief Construct a new Submitter Client object by default
   * 
   */
  SubmitterClient() = default;
  SubmitterClient(std::unique_ptr<armonik::api::grpc::v1::submitter::Submitter::StubInterface> stub);

  /**
   * @brief Construct a new Submitter Client:: Submitter Client object
   * 
   * @param stub the gRPC client stub 
   */
  //SubmitterClient(armonik::api::grpc::v1::submitter::Submitter::StubInterface* stub);

  /**
   * @brief Initializes task options creates channel with server address
   * 
   * @param channel The gRPC channel to communicate with the server.
   * @param task_options The task options.
   */
  static void init(std::shared_ptr<grpc::Channel>& channel, armonik::api::grpc::v1::TaskOptions& task_options);
  
  /**
   * @brief Creates a new session with the control plane.
   * @param default_task_options The default task options.
   * @param partition_ids The partition ids.
   */
  std::string create_session(
      armonik::api::grpc::v1::TaskOptions default_task_options,
                            const std::vector<std::string>& partition_ids);

  /**
   * @brief Cancels a created session.
   * @param session_id The id of the session to be canceled.
   */
  bool cancel_session(std::string_view session_id);



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
   * @param session_id The session ID.
   * @param task_options The task options.
   * @param task_requests The vector of task requests.
   * @return A future create task reply object.
   */
  std::future<armonik::api::grpc::v1::submitter::CreateTaskReply> create_tasks_async(
    std::string& session_id, const armonik::api::grpc::v1::TaskOptions& task_options,
    const std::vector<armonik::api::grpc::v1::TaskRequest>& task_requests);

  /**
   * @brief Submits tasks with dependencies to the session context.
   * @param session_context The session context.
   * @param payloads_with_dependencies A vector of tuples containing the payload, its data, and its dependencies.
   * @param max_retries The maximum number of retries for submitting tasks.
   * @return A vector of submitted task IDs.
   */
  std::vector<std::string> submit_tasks_with_dependencies(SessionContext& session_context,
    std::vector<std::tuple<std::string, std::vector<char>, std::
    vector<std::string>>> payloads_with_dependencies,
    int max_retries);

    /**
   * @brief Get result without streaming.
   * @param result_requests The vector of result requests.
   * @return A vector containing the data associated to the result
   */
  std::future<std::vector<int8_t>> get_result_async(
    const armonik::api::grpc::v1::ResultRequest& result_requests);

};