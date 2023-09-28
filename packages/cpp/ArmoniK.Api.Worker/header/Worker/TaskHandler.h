#pragma once
#include <future>
#include <string>

#include "agent_common.pb.h"
#include "agent_service.grpc.pb.h"

#include "worker_common.pb.h"
#include "worker_service.grpc.pb.h"

namespace armonik {
namespace api {
namespace worker {

// #include "SessionContext.h"

/**
 * @brief The TaskHandler class provides methods to create and handle tasks
 *
 */
class TaskHandler {

private:
  armonik::api::grpc::v1::agent::Agent::Stub &stub_;
  const armonik::api::grpc::v1::worker::ProcessRequest &request_;
  std::string session_id_;
  std::string task_id_;
  armonik::api::grpc::v1::TaskOptions task_options_;
  std::vector<std::string> expected_result_;
  std::string payload_;
  std::map<std::string, std::string> data_dependencies_;
  std::string token_;
  armonik::api::grpc::v1::Configuration config_;
  std::string data_folder_;

public:
  /**
   * @brief Construct a new Task Handler object
   *
   * @param client the agent client
   * @param request The process request
   */
  TaskHandler(armonik::api::grpc::v1::agent::Agent::Stub &client,
              const armonik::api::grpc::v1::worker::ProcessRequest &request);

  /**
   * @brief Create a task_chunk_stream.
   *
   * @param task_request a task request
   * @param is_last A boolean indicating if this is the last request.
   * @param chunk_max_size Maximum chunk size.
   * @return std::future<std::vector<armonik::api::grpc::v1::agent::CreateTaskRequest>>
   */
  static std::future<std::vector<armonik::api::grpc::v1::agent::CreateTaskRequest>>
  task_chunk_stream(armonik::api::grpc::v1::TaskRequest task_request, bool is_last, const std::string &token,
                    size_t chunk_max_size);

  /**
   * @brief Convert task_requests to request_stream.
   *
   * @param task_requests List of task requests
   * @param task_options The Task Options used for this batch of tasks
   * @param chunk_max_size Maximum chunk size.
   * @return std::vector<std::future<std::vector<armonik::api::grpc::v1::agent::CreateTaskRequest>>>
   */
  static std::vector<std::future<std::vector<armonik::api::grpc::v1::agent::CreateTaskRequest>>>
  to_request_stream(const std::vector<armonik::api::grpc::v1::TaskRequest> &task_requests,
                    armonik::api::grpc::v1::TaskOptions task_options, const std::string &token, size_t chunk_max_size);

  /**
   * @brief Create a tasks async object
   * @param task_options The Task Options used for this batch of tasks
   * @param task_requests List of task requests
   * @return Successfully sent task
   */
  std::future<armonik::api::grpc::v1::agent::CreateTaskReply>
  create_tasks_async(armonik::api::grpc::v1::TaskOptions task_options,
                     const std::vector<armonik::api::grpc::v1::TaskRequest> &task_requests);

  /**
   * @brief Send task result
   *
   * @param key the key of result
   * @param data The result data
   * @return A future containing a vector of ResultReply
   */
  std::future<void> send_result(std::string key, absl::string_view data);

  /**
   * @brief Get the result ids object
   *
   * @param results The results data
   * @return std::vector<std::string> list of result ids
   */
  std::vector<std::string>
  get_result_ids(std::vector<armonik::api::grpc::v1::agent::CreateResultsMetaDataRequest_ResultCreate> results);

  /**
   * @brief Get the Session Id object
   *
   * @return std::string
   */
  const std::string &getSessionId() const;

  /**
   * @brief Get the Task Id object
   *
   * @return std::string
   */
  const std::string &getTaskId() const;
  /**
   * @brief Get the Payload object
   *
   * @return std::vector<std::byte>
   */
  const std::string &getPayload() const;
  /**
   * @brief Get the Data Dependencies object
   *
   * @return std::vector<std::byte>
   */
  const std::map<std::string, std::string> &getDataDependencies() const;

  /**
   * @brief Get the Task Options object
   *
   * @return armonik::api::grpc::v1::TaskOptions
   */
  const armonik::api::grpc::v1::TaskOptions &getTaskOptions() const;

  /**
   * @brief Get the Expected Results object
   *
   * @return google::protobuf::RepeatedPtrField<std::string>
   */
  const std::vector<std::string> &getExpectedResults() const;

  /**
   * @brief Get the Configuration object
   *
   * @return armonik::api::grpc::v1::Configuration
   */
  const armonik::api::grpc::v1::Configuration &getConfiguration() const;
};

} // namespace worker
} // namespace api
} // namespace armonik
