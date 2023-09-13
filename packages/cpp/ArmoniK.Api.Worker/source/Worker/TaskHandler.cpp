#include "Worker/TaskHandler.h"
#include "exceptions/ArmoniKApiException.h"
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
armonik::api::worker::TaskHandler::TaskHandler(Agent::Stub &client,
                                               ::grpc::ServerReader<ProcessRequest> &request_iterator)
    : stub_(client), request_iterator_(request_iterator) {}

/**
 * @brief Initialise the task handler
 *
 */
void armonik::api::worker::TaskHandler::init() {
  ProcessRequest Request;
  if (!request_iterator_.Read(&Request)) {
    throw std::runtime_error("Request stream ended unexpectedly.");
  }

  if (Request.compute().type_case() != armonik::api::grpc::v1::worker::ProcessRequest_ComputeRequest::kInitRequest) {
    throw std::runtime_error("Expected a Compute request type with InitRequest to start the stream.");
  }
  auto *init_request = Request.mutable_compute()->mutable_init_request();
  session_id_ = init_request->session_id();
  task_id_ = init_request->task_id();
  task_options_ = init_request->task_options();
  expected_result_.assign(std::make_move_iterator(init_request->mutable_expected_output_keys()->begin()),
                          std::make_move_iterator(init_request->mutable_expected_output_keys()->end()));
  token_ = Request.communication_token();
  config_ = std::move(*init_request->mutable_configuration());

  auto *datachunk = &init_request->payload();
  payload_.resize(datachunk->data().size());
  std::memcpy(payload_.data(), datachunk->data().data(), datachunk->data().size());

  while (!datachunk->data_complete()) {
    if (!request_iterator_.Read(&Request)) {
      throw std::runtime_error("Request stream ended unexpectedly.");
    }
    if (Request.compute().type_case() != armonik::api::grpc::v1::worker::ProcessRequest_ComputeRequest::kPayload) {
      throw std::runtime_error("Expected a Compute request type with Payload to continue the stream.");
    }

    datachunk = &Request.compute().payload();
    if (datachunk->type_case() == armonik::api::grpc::v1::DataChunk::kData) {
      size_t prev_size = payload_.size();
      payload_.resize(payload_.size() + datachunk->data().size());
      std::memcpy(payload_.data() + prev_size, datachunk->data().data(), datachunk->data().size());
    } else if (datachunk->type_case() == armonik::api::grpc::v1::DataChunk::TYPE_NOT_SET) {
      throw std::runtime_error("Expected a Compute request type with a DataChunk Payload to continue the stream.");
    } else if (datachunk->type_case() == armonik::api::grpc::v1::DataChunk::kDataComplete) {
      break;
    }
  }

  armonik::api::grpc::v1::worker::ProcessRequest_ComputeRequest::InitData *init_data;

  do {
    if (!request_iterator_.Read(&Request)) {
      throw std::runtime_error("Request stream ended unexpectedly.");
    }
    if (Request.compute().type_case() != armonik::api::grpc::v1::worker::ProcessRequest_ComputeRequest::kInitData) {
      throw std::runtime_error("Expected a Compute request type with InitData to continue the stream.");
    }

    init_data = Request.mutable_compute()->mutable_init_data();
    if (init_data->type_case() == armonik::api::grpc::v1::worker::ProcessRequest_ComputeRequest_InitData::kKey) {
      const std::string &key = init_data->key();
      std::string data_dep;
      while (true) {
        ProcessRequest dep_request;
        if (!request_iterator_.Read(&dep_request)) {
          throw std::runtime_error("Request stream ended unexpectedly.");
        }
        if (dep_request.compute().type_case() != armonik::api::grpc::v1::worker::ProcessRequest_ComputeRequest::kData) {
          throw std::runtime_error("Expected a Compute request type with Data to continue the stream.");
        }

        auto chunk = dep_request.compute().data();
        if (chunk.type_case() == armonik::api::grpc::v1::DataChunk::kData) {
          size_t prevSize = data_dep.size();
          data_dep.resize(prevSize + chunk.data().size());
          std::memcpy(data_dep.data() + prevSize, chunk.data().data(), chunk.data().size());
        } else if (datachunk->type_case() == armonik::api::grpc::v1::DataChunk::TYPE_NOT_SET) {
          throw std::runtime_error("Expected a Compute request type with a DataChunk Payload to continue the stream.");
        } else if (datachunk->type_case() == armonik::api::grpc::v1::DataChunk::kDataComplete) {
          break;
        }
      }
      data_dependencies_[key] = data_dep;
    }
  } while (init_data->type_case() == armonik::api::grpc::v1::worker::ProcessRequest_ComputeRequest_InitData::kKey);
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

    return std::vector{std::move(create_task_request)};
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
std::future<armonik::api::grpc::v1::agent::ResultReply>
armonik::api::worker::TaskHandler::send_result(std::string key, std::string_view data) {
  return std::async(std::launch::async, [this, key = std::move(key), data]() mutable {
    ::grpc::ClientContext context_client_writer;

    armonik::api::grpc::v1::agent::ResultReply reply;

    size_t max_chunk = config_.data_chunk_max_size();
    const size_t data_size = data.size();
    size_t start = 0;

    auto stream = stub_.SendResult(&context_client_writer, &reply);

    armonik::api::grpc::v1::agent::Result init_msg;
    init_msg.mutable_init()->set_key(std::move(key));
    init_msg.set_communication_token(token_);

    stream->Write(init_msg);

    while (start < data_size) {
      size_t chunkSize = std::min(max_chunk, data_size - start);

      armonik::api::grpc::v1::agent::Result msg;
      msg.set_communication_token(token_);
      auto chunk = msg.mutable_data();
      chunk->mutable_data()->resize(chunkSize);
      std::memcpy(chunk->mutable_data()->data(), data.data() + start, chunkSize);

      stream->Write(msg);

      start += chunkSize;
    }

    armonik::api::grpc::v1::agent::Result end_msg;
    end_msg.set_communication_token(token_);
    end_msg.mutable_data()->set_data_complete(true);
    stream->Write(end_msg);

    stream->WritesDone();
    ::grpc::Status status = stream->Finish();

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
