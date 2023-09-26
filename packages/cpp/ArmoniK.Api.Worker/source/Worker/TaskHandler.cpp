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

/**
 * @brief Construct a new Task Handler object
 *
 * @param client the agent client
 * @param request_iterator The request iterator
 */
armonik::api::worker::TaskHandler::TaskHandler(Agent::Stub &client, const ProcessRequest &request)
    : stub_(client), request_(request) {
  token_ = request_.communication_token();
  session_id_ = request_.session_id();
  task_id_ = request_.task_id();
  task_options_ = request_.task_options();
  const std::string payload_id = request_.payload_id();
  data_folder_ = request_.data_folder();
  std::ostringstream string_stream(std::ios::binary);
  string_stream
      << std::ifstream(armonik::api::common::utils::pathJoin(data_folder_, payload_id), std::fstream::binary).rdbuf();
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
std::future<armonik::api::grpc::v1::agent::NotifyResultDataResponse>
armonik::api::worker::TaskHandler::send_result(std::string key, absl::string_view data) {
  return std::async(std::launch::async, [this, key = std::move(key), data]() mutable {
    ::grpc::ClientContext context;

    std::ofstream output(armonik::api::common::utils::pathJoin(data_folder_, key),
                         std::fstream::binary | std::fstream::trunc);
    output << data;
    output.close();

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
      message << "Error: " << status.error_code() << ": " << status.error_message()
              << ". details: " << status.error_details() << std::endl;
      throw armonik::api::common::exceptions::ArmoniKApiException(message.str());
    }
    return reply;
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
const std::string &armonik::api::worker::TaskHandler::getPayload() const { return payload_; }

/**
 * @brief Get the Data Dependencies object
 *
 * @return std::vector<std::byte>
 */
const std::map<std::string, std::string> &armonik::api::worker::TaskHandler::getDataDependencies() const {
  return data_dependencies_;
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
