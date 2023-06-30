#pragma once
#include <future>
#include <string>

#include "agent_common.pb.h"
#include "agent_service.grpc.pb.h"

#include "worker_common.pb.h"
#include "worker_service.grpc.pb.h"

// #include "SessionContext.h"

/**
 * @brief The TaskHandler class provides methods to create and handle tasks
 *
 */
class TaskHandler {

private:
  grpc::ClientContext context_;
  std::unique_ptr<armonik::api::grpc::v1::agent::Agent::Stub> stub_;
  std::unique_ptr<grpc::ClientReader<armonik::api::grpc::v1::worker::ProcessRequest>> request_iterator_;
  std::string session_id_;
  std::string task_id_;
  armonik::api::grpc::v1::TaskOptions task_options_;
  google::protobuf::RepeatedPtrField<std::string> expected_result_;
  std::string token_;
  armonik::api::grpc::v1::Configuration config_;


public:
  /**
   * @brief Construct a new Task Handler object
   * 
   * @param client 
   * @param request_iterator 
   */
  TaskHandler(std::unique_ptr<armonik::api::grpc::v1::agent::Agent::Stub> client,
              std::unique_ptr<grpc::ClientReader<armonik::api::grpc::v1::worker::ProcessRequest>> request_iterator);

  /**
   * @brief 
   * 
   */
  void init();

  /**
   * @brief 
   * 
   * @param task_request 
   * @param is_last 
   * @param chunk_max_size 
   * @return std::future<std::vector<armonik::api::grpc::v1::agent::CreateTaskRequest>> 
   */
  static std::future<std::vector<armonik::api::grpc::v1::agent::CreateTaskRequest>>
  task_chunk_stream(armonik::api::grpc::v1::TaskRequest task_request, bool is_last, size_t chunk_max_size);

  /**
   * @brief 
   * 
   * @param task_requests 
   * @param task_options 
   * @param chunk_max_size 
   * @return std::vector<std::future<std::vector<armonik::api::grpc::v1::agent::CreateTaskRequest>>> 
   */
  static auto to_request_stream(const std::vector<armonik::api::grpc::v1::TaskRequest> &task_requests,
                                armonik::api::grpc::v1::TaskOptions task_options, size_t chunk_max_size)
      -> std::vector<std::future<std::vector<armonik::api::grpc::v1::agent::CreateTaskRequest>>>;

  /**
   * @brief Create a tasks async object
   * @param channel
   * @param session_id
   * @param task_options
   * @param task_requests
   * @return std::future<armonik::api::grpc::v1::agent::CreateTaskReply>
   */
  std::future<armonik::api::grpc::v1::agent::CreateTaskReply>
  create_tasks_async(std::string &session_id,
                     armonik::api::grpc::v1::TaskOptions task_options,
                     const std::vector<armonik::api::grpc::v1::TaskRequest> &task_requests);

  /**
   * @brief 
   * 
   * @param channel 
   * @param data 
   * @return std::future<std::vector<armonik::api::grpc::v1::agent::ResultReply>> 
   */
  std::future<std::vector<armonik::api::grpc::v1::agent::ResultReply>>
  send_result(std::vector<std::byte> &data);

  std::vector<std::string> get_result_ids(std::vector<std::string> results);
};
