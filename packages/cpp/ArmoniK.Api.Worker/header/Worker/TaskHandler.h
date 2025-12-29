#pragma once

#include <future>
#include <string>
#include <mutex>
#include <map>
#include <cstddef>
#include <vector>

#include "absl/strings/string_view.h"

#include "agent_common.pb.h"
#include "agent_service.grpc.pb.h"
#include "worker_common.pb.h"
#include "worker_service.grpc.pb.h"




namespace armonik {
namespace api {
namespace worker {


/**
 * @brief Provides helper methods to create tasks, access input payload/dependencies,
 * and send results back to the ArmoniK agent.
 */
class TaskHandler {

public:
  struct FileMapping {
    void *addr   = nullptr;  ///< Start address of the mapping (nullptr if none)
    size_t length = 0;       ///< Length in bytes of the mapping
    int fd       = -1;       ///< File descriptor (-1 if none)
  };

private:

  armonik::api::grpc::v1::agent::Agent::Stub &stub_;
  const armonik::api::grpc::v1::worker::ProcessRequest &request_;

  std::string session_id_;
  std::string task_id_;
  armonik::api::grpc::v1::TaskOptions task_options_;
  std::vector<std::string> expected_result_;
  std::string token_;
  armonik::api::grpc::v1::Configuration config_;
  std::string data_folder_;

  // Payload metadata and lazy-loaded mapping/views/caches.
  std::string payload_id_;                     ///< Payload file identifier.
  mutable FileMapping payload_mapping_;        ///< Payload mapping (lifetime: TaskHandler).
  mutable bool payload_mapped_ = false;        ///< True once mapping is established.

  mutable absl::string_view payload_view_;     ///< Zero-copy view into payload_mapping_.
  mutable bool payload_view_built_ = false;

  mutable std::string payload_cache_;          ///< Owning copy for std::string API.
  mutable bool payload_cache_built_ = false;

  // Data dependencies metadata + mmap + caches
  std::vector<std::string> data_dependencies_ids_;   ///< Dependency file identifiers.

  mutable std::map<std::string, FileMapping> dependency_mappings_;  ///< Mapping per dependency.
  mutable bool dependency_mappings_built_ = false;

  mutable std::map<std::string, absl::string_view> dependency_views_;   ///< Zero-copy view per dependency.
  mutable bool dependency_views_built_ = false;

  mutable std::map<std::string, std::string> dependency_cache_;   ///< Owning copy per dependency.
  mutable bool dependency_cache_built_ = false;

  // Mappings used to write results (kept alive until destruction).
  std::vector<FileMapping> write_mappings_;
  mutable std::mutex write_mappings_mutex_;


public:
  /**
   * @brief Creates a TaskHandler bound to the given agent stub and process request.
   *
   * @param client Agent gRPC stub.
   * @param request Worker process request.
   */
  TaskHandler(armonik::api::grpc::v1::agent::Agent::Stub &client,
              const armonik::api::grpc::v1::worker::ProcessRequest &request);


  /**
   * @brief Destructor: releases all mmap'ed regions and closes their file descriptors.
   */
  ~TaskHandler();


  /**
   * @brief Splits a TaskRequest into a stream of CreateTaskRequest chunks.
   *
   * @param task_request Task request to chunk.
   * @param is_last Whether this is the last chunk in the stream.
   * @param token Authentication token.
   * @param chunk_max_size Maximum chunk size in bytes.
   * @return Future resolving to the list of CreateTaskRequest chunks.
   */
  static std::future<std::vector<armonik::api::grpc::v1::agent::CreateTaskRequest>>
  task_chunk_stream(armonik::api::grpc::v1::TaskRequest task_request, bool is_last, const std::string &token,
                    size_t chunk_max_size);


  /**
   * @brief Converts a list of TaskRequest into a stream of CreateTaskRequest chunks.
   *
   * @param task_requests Task requests to chunk.
   * @param task_options Task options for this batch.
   * @param token Authentication token.
   * @param chunk_max_size Maximum chunk size in bytes.
   * @return A list of futures, one per task, each resolving to its chunk list.
   */
  static std::vector<std::future<std::vector<armonik::api::grpc::v1::agent::CreateTaskRequest>>>
  to_request_stream(const std::vector<armonik::api::grpc::v1::TaskRequest> &task_requests,
                    armonik::api::grpc::v1::TaskOptions task_options, const std::string &token, size_t chunk_max_size);


  /**
   * @brief Converts a list of TaskRequest into a stream of CreateTaskRequest chunks.
   *
   * @param task_requests Task requests to chunk.
   * @param task_options Task options for this batch.
   * @param token Authentication token.
   * @param chunk_max_size Maximum chunk size in bytes.
   * @return A list of futures, one per task, each resolving to its chunk list.
   */
  std::future<armonik::api::grpc::v1::agent::CreateTaskReply>
  create_tasks_async(armonik::api::grpc::v1::TaskOptions task_options,
                     const std::vector<armonik::api::grpc::v1::TaskRequest> &task_requests);


  /**
   * @brief Sends a result for the current task.
   *
   * The mapping used to write the result is kept alive until the destructor runs.
   *
   * @param key Result key.
   * @param data Result payload.
   * @return Future that completes once the gRPC notification is done.
   */
  std::future<void> send_result(std::string key, absl::string_view data);


  /**
   * @brief Extracts result identifiers from a CreateResultsMetaData response payload.
   *
   * @param results Result metadata entries.
   * @return List of result ids.
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
   * @brief Returns data dependencies as owning strings.
   *
   * @return Map from dependency id to content.
   */
  absl::string_view getPayloadView() const;


  /**
   * @brief Get the Data Dependencies object
   *
   * @return std::vector<std::byte>
   */
  const std::map<std::string, std::string> &getDataDependencies() const;


  /**
   * @brief Returns zero-copy views of data dependencies.
   *
   * The returned views point into memory-mapped regions. Their lifetime is tied
   * to this TaskHandler instance.
   *
   * @return Map from dependency id to string_view.
   */
  const std::map<std::string, absl::string_view> &getDataDependenciesView() const;


  
  /**
   * @return Task options for the current task.
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
