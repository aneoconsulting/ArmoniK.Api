#include "Worker/TaskHandler.h"

#include "exceptions/ArmoniKApiException.h"
#include "utils/string_utils.h"

#include <fstream>
#include <future>
#include <sstream>
#include <string>
#include <cerrno>
#include <cstring>

#include "agent_common.pb.h"
#include "agent_service.grpc.pb.h"
#include "worker_common.pb.h"
#include "worker_service.grpc.pb.h"

#include <sys/mman.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <unistd.h>

using armonik::api::grpc::v1::ResultRequest;
using armonik::api::grpc::v1::TaskOptions;
using armonik::api::grpc::v1::TaskRequest;
using armonik::api::grpc::v1::agent::Agent;
using armonik::api::grpc::v1::agent::CreateTaskReply;
using armonik::api::grpc::v1::agent::CreateTaskRequest;
using armonik::api::grpc::v1::worker::ProcessRequest;
using ::grpc::Channel;
using ::grpc::ChannelInterface;
using ::grpc::ClientContext;
using ::grpc::Status;


namespace {
  
// Shorthand alias for the nested type
using FileMapping = armonik::api::worker::TaskHandler::FileMapping;


/**
 * @brief Maps a file into memory for reading.
 *
 * If the file does not exist, returns an empty mapping (addr=nullptr, length=0, fd=-1).
 * On other errors, throws ArmoniKApiException.
 *
 * @param path Path to the file to map.
 * @return A FileMapping describing the mapping and its file descriptor.
 */
FileMapping mmap_file_read(const std::string &path) {
  FileMapping mapping{};

  int fd = ::open(path.c_str(), O_RDONLY);
  if (fd == -1) {
    if (errno == ENOENT) {
      return mapping;
    }

    std::ostringstream oss;
    oss << "Failed to open file '" << path << "' for reading: " << std::strerror(errno);
    throw armonik::api::common::exceptions::ArmoniKApiException(oss.str());
  }

  struct stat sb {};
  if (::fstat(fd, &sb) == -1) {
    int err = errno;
    ::close(fd);

    std::ostringstream oss;
    oss << "fstat failed on '" << path << "': " << std::strerror(err);
    throw armonik::api::common::exceptions::ArmoniKApiException(oss.str());
  }

  mapping.fd     = fd;
  mapping.length = static_cast<size_t>(sb.st_size);

  if (mapping.length == 0) {
    mapping.addr = nullptr;
    return mapping;
  }

  void *addr = ::mmap(nullptr, mapping.length, PROT_READ, MAP_PRIVATE, fd, 0);
  int mmap_err = errno;

  if (addr == MAP_FAILED) {
    ::close(fd);
    std::ostringstream oss;
    oss << "mmap failed on '" << path << "': " << std::strerror(mmap_err);
    throw armonik::api::common::exceptions::ArmoniKApiException(oss.str());
  }

  mapping.addr = addr;
  return mapping;
}


/**
 * @brief Maps a file into memory for writing.
 *
 * Creates or truncates the file, resizes it to @p length bytes and maps it
 * with a shared writable mapping.
 *
 * @param path Path to the file to create/map.
 * @param length Size of the file and mapping in bytes.
 * @return A FileMapping describing the mapping and its file descriptor.
 */
FileMapping mmap_file_write(const std::string &path, size_t len) {
  FileMapping mapping{};

  int fd = ::open(path.c_str(), O_RDWR | O_CREAT | O_TRUNC, 0644);
  if (fd == -1) {
    std::ostringstream oss;
    oss << "Failed to open file '" << path << "' for writing: " << std::strerror(errno);
    throw armonik::api::common::exceptions::ArmoniKApiException(oss.str());
  }

  if (::ftruncate(fd, static_cast<off_t>(len)) == -1) {
    int err = errno;
    ::close(fd);
    std::ostringstream oss;
    oss << "ftruncate failed on '" << path << "': " << std::strerror(err);
    throw armonik::api::common::exceptions::ArmoniKApiException(oss.str());
  }

  mapping.fd     = fd;
  mapping.length = len;

  if (len == 0) {
    mapping.addr = nullptr;
    return mapping;
  }

  void *addr = ::mmap(nullptr, len, PROT_WRITE, MAP_SHARED, fd, 0);
  int mmap_err = errno;

  if (addr == MAP_FAILED) {
    ::close(fd);
    std::ostringstream oss;
    oss << "mmap (write) failed on '" << path << "': " << std::strerror(mmap_err);
    throw armonik::api::common::exceptions::ArmoniKApiException(oss.str());
  }

  mapping.addr = addr;
  return mapping;
}


/**
 * @brief Releases a FileMapping by unmapping memory and closing its file descriptor.
 *
 * This function is idempotent: calling it multiple times is safe.
 *
 * @param mapping Mapping to release.
 * @return void
 */
void cleanup_mapping(FileMapping &mapping) {
  if (mapping.addr != nullptr && mapping.length > 0) {
    ::munmap(mapping.addr, mapping.length);
  }
  if (mapping.fd != -1) {
    ::close(mapping.fd);
  }
  mapping.addr   = nullptr;
  mapping.length = 0;
  mapping.fd     = -1;
}

} 

/**
 * @brief Creates a TaskHandler bound to the given agent stub and process request.
 *
 * @param client Agent gRPC stub.
 * @param request Worker process request.
 */
armonik::api::worker::TaskHandler::TaskHandler(Agent::Stub &client, 
                                               const ProcessRequest &request)
    : stub_(client),
      request_(request),
      session_id_(request_.session_id()),
      task_id_(request_.task_id()),
      task_options_(request_.task_options()),
      expected_result_(request_.expected_output_keys().begin(),
                       request_.expected_output_keys().end()),
      token_(request_.communication_token()),
      config_(request_.configuration()),
      data_folder_(request_.data_folder()),
      payload_id_(request_.payload_id()),
      payload_mapping_(),
      payload_mapped_(false),
      payload_view_(),
      payload_view_built_(false),
      payload_cache_(),
      payload_cache_built_(false),
      data_dependencies_ids_(),
      dependency_mappings_(),
      dependency_mappings_built_(false),
      dependency_views_(),
      dependency_views_built_(false),
      dependency_cache_(),
      dependency_cache_built_(false),
      write_mappings_(),
      write_mappings_mutex_(){

  // On garde simplement la liste des IDs de dépendances, sans les lire.
  for (const auto &dd : request_.data_dependencies()) {
    data_dependencies_ids_.push_back(dd);
  }
}


/**
 * @brief Releases all memory-mapped regions and closes their file descriptors.
 */
armonik::api::worker::TaskHandler::~TaskHandler() {
  cleanup_mapping(payload_mapping_);

  for (auto &kv : dependency_mappings_) {
    cleanup_mapping(kv.second);
  }

  {
    std::lock_guard<std::mutex> lock(write_mappings_mutex_);
    for (auto &m : write_mappings_) {
      cleanup_mapping(m);
    }
    write_mappings_.clear();
  }
}


/**
 * @brief Splits a TaskRequest into a stream of CreateTaskRequest chunks.
 *
 * @param task_request Task request to chunk.
 * @param is_last Whether this is the last chunk in the stream.
 * @param token Authentication token.
 * @param chunk_max_size Maximum chunk size in bytes.
 * @return Future resolving to the list of CreateTaskRequest chunks.
 */
std::future<std::vector<CreateTaskRequest>>
armonik::api::worker::TaskHandler::task_chunk_stream(TaskRequest task_request, bool is_last, const std::string &token,
                                                     size_t chunk_max_size) {
  return std::async(std::launch::async, [task_request = std::move(task_request), chunk_max_size, is_last, token]() {
    std::vector<CreateTaskRequest> requests;
    armonik::api::grpc::v1::InitTaskRequest header_task_request;
    armonik::api::grpc::v1::TaskRequestHeader header;

    header.mutable_data_dependencies()->Add(task_request.data_dependencies().begin(),
                                            task_request.data_dependencies().end());
    header.mutable_expected_output_keys()->Add(task_request.expected_output_keys().begin(),
                                               task_request.expected_output_keys().end());
    *header_task_request.mutable_header() = std::move(header);

    CreateTaskRequest create_init_task_request;
    *create_init_task_request.mutable_init_task() = std::move(header_task_request);
    create_init_task_request.set_communication_token(token);

    requests.push_back(std::move(create_init_task_request));

    if (task_request.payload().empty()) {
      CreateTaskRequest empty_task_request;

      armonik::api::grpc::v1::DataChunk task_payload;
      *task_payload.mutable_data() = {};
      *empty_task_request.mutable_task_payload() = std::move(task_payload);
      empty_task_request.set_communication_token(token);
      requests.push_back(std::move(empty_task_request));
    }

    size_t start = 0;

    while (start < task_request.payload().size()) {

      size_t chunk_size = std::min(chunk_max_size, task_request.payload().size() - start);

      CreateTaskRequest chunk_task_request;

      armonik::api::grpc::v1::DataChunk task_payload;

      *task_payload.mutable_data() = task_request.payload().substr(start, chunk_size);
      *chunk_task_request.mutable_task_payload() = std::move(task_payload);
      chunk_task_request.set_communication_token(token);

      requests.push_back(std::move(chunk_task_request));

      start += chunk_size;
    }

    CreateTaskRequest complete_task_request;
    armonik::api::grpc::v1::DataChunk end_payload;

    end_payload.set_data_complete(true);
    *complete_task_request.mutable_task_payload() = std::move(end_payload);
    complete_task_request.set_communication_token(token);
    requests.push_back(std::move(complete_task_request));

    if (is_last) {
      CreateTaskRequest last_task_request;
      armonik::api::grpc::v1::InitTaskRequest init_task_request;

      init_task_request.set_last_task(true);
      *last_task_request.mutable_init_task() = std::move(init_task_request);
      last_task_request.set_communication_token(token);

      requests.push_back(std::move(last_task_request));
    }

    return requests;
  });
}


/**
 * @brief Converts a list of TaskRequest into a stream of CreateTaskRequest chunks.
 *
 * @param task_requests Task requests to chunk.
 * @param task_options Task options for this batch.
 * @param token Authentication token.
 * @param chunk_max_size Maximum chunk size in bytes.
 * @return A list of futures, one per task, each resolving to its chunk list.
 */
std::vector<std::future<std::vector<CreateTaskRequest>>>
armonik::api::worker::TaskHandler::to_request_stream(const std::vector<TaskRequest> &task_requests,
                                                     TaskOptions task_options, const std::string &token,
                                                     const size_t chunk_max_size) {
  std::vector<std::future<std::vector<CreateTaskRequest>>> async_chunk_payload_tasks;

  async_chunk_payload_tasks.push_back(std::async([task_options = std::move(task_options), token]() mutable {
    grpc::v1::agent::CreateTaskRequest_InitRequest create_task_request_init;
    *create_task_request_init.mutable_task_options() = std::move(task_options);

    CreateTaskRequest create_task_request;
    *create_task_request.mutable_init_request() = std::move(create_task_request_init);
    create_task_request.set_communication_token(token);

    return std::vector<CreateTaskRequest>{std::move(create_task_request)};
  }));

  for (auto task_request = task_requests.begin(); task_request != task_requests.end(); ++task_request) {
    const bool is_last = task_request == task_requests.end() - 1;

    async_chunk_payload_tasks.push_back(task_chunk_stream(*task_request, is_last, token, chunk_max_size));
  }

  return async_chunk_payload_tasks;
}

/**
 * @brief Sends a batch of tasks asynchronously.
 *
 * @param task_options Task options for this batch.
 * @param task_requests Task requests to create.
 * @return Future resolving to the CreateTaskReply.
 */
std::future<CreateTaskReply>
armonik::api::worker::TaskHandler::create_tasks_async(TaskOptions task_options,
                                                      const std::vector<TaskRequest> &task_requests) {
  return std::async(std::launch::async, [this, &task_requests, &task_options]() mutable {
    size_t chunk = config_.data_chunk_max_size();

    CreateTaskReply reply{};

    reply.set_allocated_creation_status_list(new armonik::api::grpc::v1::agent::CreateTaskReply_CreationStatusList());
    ::grpc::ClientContext context_client_writer;
    auto stream(stub_.CreateTask(&context_client_writer, &reply));

    auto create_task_request_async = to_request_stream(task_requests, std::move(task_options), token_, chunk);

    for (auto &f : create_task_request_async) {
      for (auto &create_task_request : f.get()) {
        stream->Write(create_task_request);
      }
    }

    stream->WritesDone();
    ::grpc::Status status = stream->Finish();
    if (!status.ok()) {
      std::stringstream message;
      message << "Error: " << status.error_code() << ": " << status.error_message()
              << ". details : " << status.error_details() << std::endl;
      throw std::runtime_error(message.str().c_str());
    }

    return reply;
  });
}


/**
 * @brief Sends a result for the current task.
 *
 * The memory-mapped region used to write the file is kept alive until the TaskHandler
 * instance is destroyed.
 *
 * @param key Result key.
 * @param data Result payload.
 * @return Future that completes once the gRPC notification is done.
 */
std::future<void> armonik::api::worker::TaskHandler::send_result(std::string key, 
                                                                 absl::string_view data) {
  return std::async(std::launch::async,
                   [this, key = std::move(key), data]() mutable {
                     ::grpc::ClientContext context;

                    const std::string path =
                          armonik::api::common::utils::pathJoin(data_folder_, key);

                      FileMapping mapping = mmap_file_write(path, data.size());
                      if (mapping.length > 0 && mapping.addr != nullptr &&
                          !data.empty()) {
                        std::memcpy(mapping.addr, data.data(), mapping.length);
                        ::msync(mapping.addr, mapping.length, MS_SYNC);
                      }

                      {
                        std::lock_guard<std::mutex> lock(write_mappings_mutex_);
                        write_mappings_.push_back(mapping);
                      }

                    armonik::api::grpc::v1::agent::NotifyResultDataResponse reply;
                    armonik::api::grpc::v1::agent::NotifyResultDataRequest request;
                    request.set_communication_token(token_);
                    armonik::api::grpc::v1::agent::NotifyResultDataRequest::ResultIdentifier result_id;
                    result_id.set_session_id(session_id_);
                    result_id.set_result_id(key);
                    *(request.mutable_ids()->Add()) = result_id;

                    auto status = stub_.NotifyResultData(&context, request, &reply);

                    if (!status.ok()) {
                      std::stringstream message;
                      message << "Error: " << status.error_code() 
                              << ": " << status.error_message()
                              << ". details: " << status.error_details() 
                              << std::endl;
                      throw armonik::api::common::exceptions::ArmoniKApiException(message.str());
                    }

                    if (reply.result_ids_size() != 1) {
                      throw armonik::api::common::exceptions::ArmoniKApiException(
                        "Received erroneous reply for send data");
                    }
                  });
}

/**
 * @brief Extracts result identifiers from a CreateResultsMetaData response payload.
 *
 * @param results Result metadata entries.
 * @return List of result ids.
 */
std::vector<std::string> armonik::api::worker::TaskHandler::get_result_ids(
    std::vector<armonik::api::grpc::v1::agent::CreateResultsMetaDataRequest_ResultCreate> results) {
  std::vector<std::string> result_ids;

  ::grpc::ClientContext context_client_writer;
  armonik::api::grpc::v1::agent::CreateResultsMetaDataRequest request;
  armonik::api::grpc::v1::agent::CreateResultsMetaDataResponse reply;

  *request.mutable_results() = {results.begin(), results.end()};
  request.set_session_id(session_id_);

  Status status = stub_.CreateResultsMetaData(&context_client_writer, request, &reply);
  if (!status.ok()) {
    throw armonik::api::common::exceptions::ArmoniKApiException(status.error_message());
  }

  auto results_reply = reply.results();

  for (auto &result_reply : results_reply) {
    result_ids.push_back(result_reply.result_id());
  }

  return result_ids;
}


/**
 * @brief Get the Session Id object
 *
 * @return std::string
 */
const std::string &armonik::api::worker::TaskHandler::getSessionId() const { return session_id_; }


/**
 * @brief Get the Task Id object
 *
 * @return std::string
 */
const std::string &armonik::api::worker::TaskHandler::getTaskId() const { return task_id_; }


/**
 * @brief Returns a zero-copy view of the payload.
 *
 * The returned view points into a memory-mapped region. Its lifetime is tied to this
 * TaskHandler instance.
 *
 * @return Payload view.
 */
absl::string_view
armonik::api::worker::TaskHandler::getPayloadView() const {
  if (!payload_view_built_) {
    const std::string path =
        armonik::api::common::utils::pathJoin(data_folder_, payload_id_);

    if (!payload_mapped_) {
      payload_mapping_ = mmap_file_read(path);
      payload_mapped_  = true;
    }

    if (payload_mapping_.addr != nullptr && payload_mapping_.length > 0) {
      payload_view_ = absl::string_view(
          static_cast<const char *>(payload_mapping_.addr),
          payload_mapping_.length);
    } else {
      payload_view_ = absl::string_view();
    }

    payload_view_built_ = true;
  }

  return payload_view_;
}


/**
 * @brief Returns the payload as an owning string.
 *
 * @return Payload content (may contain binary data).
 */
const std::string &
armonik::api::worker::TaskHandler::getPayload() const {
  if (!payload_cache_built_) {
    absl::string_view v = getPayloadView();
    payload_cache_.assign(v.data(), v.size());
    payload_cache_built_ = true;
  }
  return payload_cache_;
}


/**
 * @brief Returns zero-copy views of data dependencies.
 *
 * The returned views point into memory-mapped regions. Their lifetime is tied to this
 * TaskHandler instance.
 *
 * @return Map from dependency id to string_view.
 */
const std::map<std::string, absl::string_view> &
armonik::api::worker::TaskHandler::getDataDependenciesView() const {
  if (!dependency_views_built_) {
    if (!dependency_mappings_built_) {
      for (const auto &dd : data_dependencies_ids_) {
        const std::string path =
            armonik::api::common::utils::pathJoin(data_folder_, dd);

        FileMapping mapping = mmap_file_read(path);
        dependency_mappings_.emplace(dd, mapping);
      }
      dependency_mappings_built_ = true;
    }

    dependency_views_.clear();
    for (const auto &kv : dependency_mappings_) {
      const auto &id = kv.first;
      const auto &m  = kv.second;

      if (m.addr != nullptr && m.length > 0) {
        dependency_views_.emplace(
            id,
            absl::string_view(static_cast<const char *>(m.addr), m.length));
      } else {
        dependency_views_.emplace(id, absl::string_view());
      }
    }

    dependency_views_built_ = true;
  }

  return dependency_views_;
}


/**
 * @brief Returns data dependencies as owning strings.
 *
 * @return Map from dependency id to content (may contain binary data).
 */
const std::map<std::string, std::string> &
armonik::api::worker::TaskHandler::getDataDependencies() const {
  if (!dependency_cache_built_) {
    const auto &views = getDataDependenciesView();
    dependency_cache_.clear();
    for (const auto &kv : views) {
      const auto &id = kv.first;
      const auto &v  = kv.second;
      dependency_cache_[id].assign(v.data(), v.size());
    }
    dependency_cache_built_ = true;
  }

  return dependency_cache_;
}








/**
 * @brief Gets the task options.
 *
 * @return Task options for the current task.
 */
const armonik::api::grpc::v1::TaskOptions &armonik::api::worker::TaskHandler::getTaskOptions() const {
  return task_options_;
}


/**
 * @brief Get the Expected Results object
 *
 * @return google::protobuf::RepeatedPtrField<std::string>
 */
const std::vector<std::string> &armonik::api::worker::TaskHandler::getExpectedResults() const {
  return expected_result_;
}


/**
 * @brief Get the Configuration object
 *
 * @return armonik::api::grpc::v1::Configuration
 */
const armonik::api::grpc::v1::Configuration &armonik::api::worker::TaskHandler::getConfiguration() const {
  return config_;
}
