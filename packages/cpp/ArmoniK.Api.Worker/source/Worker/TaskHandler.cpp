#include "Worker/TaskHandler.h"
#include "exceptions/ArmoniKApiException.h"
#include "utils/string_utils.h"
#include <fstream>
#include <future>
#include <sstream>
#include <string>

#include "agent_common.pb.h"
#include "agent_service.grpc.pb.h"

#include "worker_common.pb.h"
#include "worker_service.grpc.pb.h"

// add 1 start
#include <cerrno>
#include <cstring>
#include <sys/mman.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <unistd.h>
// add 1 end

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


// add 2 start : helpers to read/write file with mmap
namespace {
  
// Shorthand alias for the nested type
using FileMapping = armonik::api::worker::TaskHandler::FileMapping;


/**
 * @brief mmap a file for reading and return a FileMapping.
 *
 * - If the file does not exist: returns an empty mapping (addr=nullptr, len=0, fd=-1).
 * - On other system errors: throws ArmoniKApiException.
 * - The returned mapping keeps the file descriptor open; it must be cleaned
 *   later with cleanup_mapping().
 */
FileMapping mmap_file_read(const std::string &path) {
  FileMapping mapping{};

  int fd = ::open(path.c_str(), O_RDONLY);
  if (fd == -1) {
    if (errno == ENOENT) {
      // File does not exist: behave like "empty content"
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
    // Empty file: no mapping, but keep fd so destructor can close it.
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
 * @brief mmap a file for writing and return a FileMapping.
 *
 * - Creates or truncates the file.
 * - Resizes it to @p len bytes.
 * - Maps it shared writable.
 * - The mapping and fd are kept valid until cleaned with cleanup_mapping().
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
    // Zero-length file: nothing to map; destructor will just close fd.
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
 * @brief Release a FileMapping: munmap (if any) and close fd (if any).
 *
 * Safe to call multiple times; after the first call, mapping becomes empty.
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



/*
std::string mmap_read_file(const std::string &path) {
  // Open the file in read-only mode (POSIX open).
  // fd >= 0 on success, -1 on error.
  int fd = ::open(path.c_str(), O_RDONLY);

  // If the file cannot be opened
  if (fd == -1) {
    // If the file does not exist, return an empty string instead of throwing.
    if (errno == ENOENT) {
      return {};
    }

    // For any other error, build a descriptive message and throw.
    std::ostringstream oss;
    oss << "Failed to open file '" << path << "' for reading: " << std::strerror(errno);
    throw armonik::api::common::exceptions::ArmoniKApiException(oss.str());
  }

  // Retrieve file metadata (size, etc.).
  struct stat sb {};
  if (::fstat(fd, &sb) == -1) {
    // Preserve errno before closing fd.
    int err = errno;
    ::close(fd);

    std::ostringstream oss;
    oss << "fstat failed on '" << path << "': " << std::strerror(err);
    throw armonik::api::common::exceptions::ArmoniKApiException(oss.str());
  }

  // If the file is empty, there is nothing to map; return an empty string.
  if (sb.st_size == 0) {
    ::close(fd);
    return {};
  }

  // Convert file size to a size_t for convenience.
  size_t len = static_cast<size_t>(sb.st_size);

  // Map the file into memory:
  //  - nullptr: let the kernel choose the mapping address
  //  - len: size of the mapping
  //  - PROT_READ: read-only mapping
  //  - MAP_PRIVATE: changes are not written back to the file
  //  - fd: file descriptor
  //  - 0: start at offset 0
  void *addr = ::mmap(nullptr, len, PROT_READ, MAP_PRIVATE, fd, 0);
  int mmap_err = errno;

  // We no longer need the file descriptor once the file is mapped.
  ::close(fd);

  // Check for mmap failure.
  if (addr == MAP_FAILED) {
    std::ostringstream oss;
    oss << "mmap failed on '" << path << "': " << std::strerror(mmap_err);
    throw armonik::api::common::exceptions::ArmoniKApiException(oss.str());
  }

  // Create a std::string by copying the mapped bytes.
  std::string result(static_cast<const char *>(addr), len);

  // Unmap the memory region now that we have our copy.
  ::munmap(addr, len);

  return result;
}


void mmap_write_file(const std::string &path, absl::string_view data) {
  // Open the file for read/write, create it if it does not exist,
  // and truncate it to zero length if it already exists.
  // Mode 0644: rw-r--r-- (owner can read/write, others read-only).
  int fd = ::open(path.c_str(), O_RDWR | O_CREAT | O_TRUNC, 0644);
  if (fd == -1) {
    std::ostringstream oss;
    oss << "Failed to open file '" << path << "' for writing: " << std::strerror(errno);
    throw armonik::api::common::exceptions::ArmoniKApiException(oss.str());
  }

  const size_t len = data.size();

  // Resize the file to exactly 'len' bytes so that the mapping
  // will have enough space to hold the buffer.
  if (::ftruncate(fd, static_cast<off_t>(len)) == -1) {
    int err = errno;
    ::close(fd);

    std::ostringstream oss;
    oss << "ftruncate failed on '" << path << "': " << std::strerror(err);
    throw armonik::api::common::exceptions::ArmoniKApiException(oss.str());
  }

  // If there is nothing to write, just close and return.
  if (len == 0) {
    ::close(fd);
    return;
  }

  // Map the file into memory for writing:
  //  - nullptr: let the kernel choose the mapping address
  //  - len: mapping size
  //  - PROT_WRITE: writable mapping
  //  - MAP_SHARED: changes are propagated to the underlying file
  //  - fd: file descriptor
  //  - 0: start at offset 0 in the file
  void *addr = ::mmap(nullptr, len, PROT_WRITE, MAP_SHARED, fd, 0);
  int mmap_err = errno;
  if (addr == MAP_FAILED) {
    ::close(fd);
    std::ostringstream oss;
    oss << "mmap failed on '" << path << "': " << std::strerror(mmap_err);
    throw armonik::api::common::exceptions::ArmoniKApiException(oss.str());
  }

  // Copy the buffer into the mapped region.
  std::memcpy(addr, data.data(), len);

  // Optionally force the changes to be flushed to disk.
  ::msync(addr, len, MS_SYNC);

  // Unmap the memory region and close the file descriptor.
  ::munmap(addr, len);
  ::close(fd);
  }*/

} // namespace
// add 2 end 




/**
 * @brief Construct a new Task Handler object
 *
 * @param client the agent client
 * @param request_iterator The request iterator
 */
armonik::api::worker::TaskHandler::TaskHandler(Agent::Stub &client, 
                                               const ProcessRequest &request)
  
/*
    : stub_(client), request_(request) {
      token_ = request_.communication_token();
      session_id_ = request_.session_id();
      task_id_ = request_.task_id();
      task_options_ = request_.task_options();
      const std::string payload_id = request_.payload_id();
      data_folder_ = request_.data_folder();
      std::ostringstream string_stream(std::ios::binary);
      string_stream << std::ifstream(armonik::api::common::utils::pathJoin(data_folder_, payload_id), std::fstream::binary).rdbuf();
      payload_ = string_stream.str();
      string_stream.clear();
      config_ = request_.configuration();
      expected_result_.assign(request_.expected_output_keys().begin(), request_.expected_output_keys().end());

  for (auto &&dd : request_.data_dependencies()) {
    // TODO Replace with lazy loading via a custom std::map (to not break compatibility)
    string_stream
        << std::ifstream(armonik::api::common::utils::pathJoin(data_folder_, dd), std::fstream::binary).rdbuf();
    data_dependencies_[dd] = string_stream.str();
    string_stream.clear();
  }
}
*/

// add 3 start add payload_id_, payload_loaded_, data_dependencies_ids_, data_dependencies_loaded_
// Also add liste d’initialisation for all variable
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


armonik::api::worker::TaskHandler::~TaskHandler() {
  // Release payload mapping
  cleanup_mapping(payload_mapping_);

  // Release all dependency mappings
  for (auto &kv : dependency_mappings_) {
    cleanup_mapping(kv.second);
  }

  // Release mappings created by send_result
  {
    std::lock_guard<std::mutex> lock(write_mappings_mutex_);
    for (auto &m : write_mappings_) {
      cleanup_mapping(m);
    }
    write_mappings_.clear();
  }
}


/**
 * @brief Create a task_chunk_stream.
 *
 * @param task_request a task request
 * @param is_last A boolean indicating if this is the last request.
 * @param chunk_max_size Maximum chunk size.
 * @return std::future<std::vector<armonik::api::grpc::v1::agent::CreateTaskRequest>>
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
 * @brief Convert task_requests to request_stream.
 *
 * @param task_requests List of task requests
 * @param task_options The Task Options used for this batch of tasks
 * @param chunk_max_size Maximum chunk size.
 * @return std::vector<std::future<std::vector<armonik::api::grpc::v1::agent::CreateTaskRequest>>>
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
 * @brief Create a tasks async object
 * @param task_options The Task Options used for this batch of tasks
 * @param task_requests List of task requests
 * @return Successfully sent task
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
 * @brief Send task result
 *
 * @param key the key of result
 * @param data The result data
 * @return A future containing a vector of ResultReply
 */


std::future<void> armonik::api::worker::TaskHandler::send_result(std::string key, 
                                                                 absl::string_view data) {
  return std::async(std::launch::async,
                   [this, key = std::move(key), data]() mutable {
                     ::grpc::ClientContext context;
/*
    std::ofstream output(armonik::api::common::utils::pathJoin(data_folder_, key),
                         std::fstream::binary | std::fstream::trunc);
    output << data;
    output.close();
*/
                    const std::string path =
                          armonik::api::common::utils::pathJoin(data_folder_, key);

                      // Create write mapping and copy data into it.
                      FileMapping mapping = mmap_file_write(path, data.size());
                      if (mapping.length > 0 && mapping.addr != nullptr &&
                          !data.empty()) {
                        std::memcpy(mapping.addr, data.data(), mapping.length);
                        // Ensure data is flushed to the file; mapping will be
                        // released later in the destructor.
                        ::msync(mapping.addr, mapping.length, MS_SYNC);
                      }

                      // Keep the mapping alive until ~TaskHandler()
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
 * @brief Get the result ids object
 *
 * @param results The results data
 * @return std::vector<std::string> list of result ids
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
 * @brief Get the Payload object
 *
 * @return std::vector<std::byte>
 */

 /*
const std::string &armonik::api::worker::TaskHandler::getPayload() const { return payload_; }
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




const std::string &
armonik::api::worker::TaskHandler::getPayload() const {
  if (!payload_cache_built_) {
    absl::string_view v = getPayloadView();
    payload_cache_.assign(v.data(), v.size());
    payload_cache_built_ = true;
  }
  return payload_cache_;
}


// ---------- Data dependencies (views + cache) ----------

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
 * @brief Get the Task Options object
 *
 * @return armonik::api::grpc::v1::TaskOptions
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
