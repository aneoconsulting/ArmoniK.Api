#include "Worker/TaskHandler.h"

#include <sstream>
#include <string>
#include <future>

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
using grpc::Channel;
using grpc::ChannelInterface;
using grpc::ClientContext;
using grpc::Status;
using namespace armonik::api::grpc::v1::agent;

/**
 * @brief Construct a new Task Handler object
 * 
 * @param client the agent client
 * @param request_iterator The request iterator
 */
TaskHandler::TaskHandler(std::unique_ptr<Agent::Stub> client,
                         std::unique_ptr<grpc::ClientReader<ProcessRequest>> request_iterator) {
  stub_ = std::move(client);
  request_iterator_ = std::move(request_iterator);
}

/**
 * @brief Initialise the task handler
 * 
 */
void TaskHandler::init() {
  ProcessRequest Request;
  std::unique_ptr<grpc::ClientReader<ProcessRequest>> request_iterator;
  request_iterator->Read(&Request);
  if (!request_iterator->Read(&Request)) {
    throw std::runtime_error("Request stream ended unexpectedly.");
  }

  if (Request.compute().type_case() != armonik::api::grpc::v1::worker::ProcessRequest_ComputeRequest::kInitRequest) {
    throw std::runtime_error("Expected a Compute request type with InitRequest to start the stream.");
  }
  auto &init_request = Request.compute().init_request();
  session_id_ = init_request.session_id();
  task_id_ = init_request.task_id();
  task_options_ = init_request.task_options();
  expected_result_ = init_request.expected_output_keys();
  token_ = Request.communication_token();
  config_ = init_request.configuration();

  std::vector<std::byte> payload;
  if (init_request.payload().data_complete()) {
    std::string temp = init_request.payload().data();
    payload.resize(temp.length());
    for (size_t i = 0; i < temp.length(); i++) {
      payload[i] = static_cast<std::byte>(temp[i]);
    }

  } else {
    std::vector<std::string> chuncks;
    auto datachunck = init_request.payload();

    chuncks.push_back(datachunck.data());

    while (!datachunck.data_complete()) {
      if (request_iterator->Read(&Request)) {
        throw std::runtime_error("Request stream ended unexpectedly.");
      }
      if (Request.compute().type_case() != armonik::api::grpc::v1::worker::ProcessRequest_ComputeRequest::kPayload) {
        throw std::runtime_error("Expected a Compute request type with Payload to continue the stream.");
      }

      datachunck = Request.compute().payload();

      chuncks.push_back(datachunck.data());
    }

    size_t size = chuncks.size();

    auto payload_data = new std::byte[size];
    int address = 0;
    for (auto iter = chuncks.begin(); iter != chuncks.end(); iter++){
      std::string temp_str = *iter;

      for (size_t i = 0; i < temp_str.length(); i++) {
        payload_data[i + address] = static_cast<std::byte>(temp_str[i]);
      }
      address += temp_str.length();
    }

    payload.resize(size);
    std::copy_n(payload_data, size, std::back_inserter(payload));
  }

  std::vector<std::byte *> data_dependencies;

  armonik::api::grpc::v1::worker::ProcessRequest_ComputeRequest::InitData init_data;

  do {
    if (request_iterator->Read(&Request)) {
      throw std::runtime_error("Request stream ended unexpectedly.");
    }
    if (Request.compute().type_case() != armonik::api::grpc::v1::worker::ProcessRequest_ComputeRequest::kInitData) {
      throw std::runtime_error("Expected a Compute request type with InitData to continue the stream.");
    }

    init_data = Request.compute().init_data();
    if (!init_data.key().empty()) {
      std::vector<std::string> chuncks;
      while (true) {
        if (request_iterator->Read(&Request)) {
          throw std::runtime_error("Request stream ended unexpectedly.");
        }
        if (Request.compute().type_case() != armonik::api::grpc::v1::worker::ProcessRequest_ComputeRequest::kData) {
          throw std::runtime_error("Expected a Compute request type with Data to continue the stream.");
        }

        auto datachunck = Request.compute().data();
        if (datachunck.type_case() == armonik::api::grpc::v1::DataChunk::kData) {
          chuncks.push_back(datachunck.data());
        }

        if (datachunck.type_case() == armonik::api::grpc::v1::DataChunk::TYPE_NOT_SET) {
          throw std::runtime_error("Expected a Compute request type with a DataChunk Payload to continue the stream.");
        }

        if (datachunck.type_case() == armonik::api::grpc::v1::DataChunk::kDataComplete) {
          break;
        }
      }

      size_t size = chuncks.size();

      auto data = new int8_t[size];

      size_t start = 0;

      for (size_t count = 0; count <= size; count++) {
        std::string temp_str = chuncks[count];

        for (size_t i = 0; i < temp_str.length(); i++) {
          data[i + count * size] = temp_str[i];
        }
      }

      data_dependencies.push_back(reinterpret_cast<std::byte*>(data));
    }

  } while (!init_data.key().empty());
}

/**
 * @brief Create a task_chunk_stream.
 * 
 * @param task_request a task request
 * @param is_last A boolean indicating if this is the last request.
 * @param chunk_max_size Maximum chunk size.
 * @return std::future<std::vector<armonik::api::grpc::v1::agent::CreateTaskRequest>> 
 */
std::future<std::vector<CreateTaskRequest>> TaskHandler::task_chunk_stream(TaskRequest task_request,
                                                                           bool is_last, size_t chunk_max_size) {
  return std::async(std::launch::async, [task_request = std::move(task_request), chunk_max_size, is_last]() {
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

    requests.push_back(std::move(create_init_task_request));

    if (task_request.payload().empty()) {
      CreateTaskRequest empty_task_request;

      armonik::api::grpc::v1::DataChunk task_payload;
      *task_payload.mutable_data() = {};
      *empty_task_request.mutable_task_payload() = std::move(task_payload);
      requests.push_back(std::move(empty_task_request));
    }

    size_t start = 0;

    while (start < task_request.payload().size()) {

      size_t chunk_size = std::min(chunk_max_size, task_request.payload().size() - start);

      CreateTaskRequest chunk_task_request;

      armonik::api::grpc::v1::DataChunk task_payload;

      *task_payload.mutable_data() = task_request.payload().substr(start, chunk_size);
      *chunk_task_request.mutable_task_payload() = std::move(task_payload);

      requests.push_back(std::move(chunk_task_request));

      start += chunk_size;
    }

    CreateTaskRequest complete_task_request;
    armonik::api::grpc::v1::DataChunk end_payload;

    end_payload.set_data_complete(true);
    *complete_task_request.mutable_task_payload() = std::move(end_payload);
    requests.push_back(std::move(complete_task_request));

    if (is_last) {
      CreateTaskRequest last_task_request;
      armonik::api::grpc::v1::InitTaskRequest init_task_request;

      init_task_request.set_last_task(true);
      *last_task_request.mutable_init_task() = std::move(init_task_request);

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
TaskHandler::to_request_stream(const std::vector<TaskRequest> &task_requests, TaskOptions task_options,
                               const size_t chunk_max_size) {
  std::vector<std::future<std::vector<CreateTaskRequest>>> async_chunk_payload_tasks;

  async_chunk_payload_tasks.push_back(std::async([task_options = std::move(task_options)]()mutable {
    CreateTaskRequest_InitRequest create_task_request_init;
    *create_task_request_init.mutable_task_options() = std::move(task_options);

    CreateTaskRequest create_task_request;
    *create_task_request.mutable_init_request() = std::move(create_task_request_init);

    return std::vector{std::move(create_task_request)};
  }));

  for (auto task_request = task_requests.begin(); task_request != task_requests.end(); ++task_request) {
    const bool is_last = task_request == task_requests.end() - 1 ? true : false;

    async_chunk_payload_tasks.push_back(task_chunk_stream(*task_request, is_last, chunk_max_size));
  }

  return async_chunk_payload_tasks;
}

/**
 * @brief Create a tasks async object
 * @param task_options The Task Options used for this batch of tasks
 * @param task_requests List of task requests
 * @return Successfully sent task
 */
std::future<CreateTaskReply> TaskHandler::create_tasks_async(TaskOptions task_options,
                                                             const std::vector<TaskRequest> &task_requests) {
  return std::async(std::launch::async, [this, &task_requests, &task_options]()mutable {

    size_t chunk = config_.data_chunk_max_size();


    CreateTaskReply reply{};

    reply.set_allocated_creation_status_list(new armonik::api::grpc::v1::agent::CreateTaskReply_CreationStatusList());
    grpc::ClientContext context_client_writer;
    auto stream(stub_->CreateTask(&context_client_writer, &reply));

    auto create_task_request_async = to_request_stream(task_requests, std::move(task_options), chunk);

    for (auto &f : create_task_request_async) {
      for (auto &create_task_request : f.get()) {
        stream->Write(create_task_request);
      }
    }

    stream->WritesDone();
    grpc::Status status = stream->Finish();
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
std::future<std::vector<ResultReply>> TaskHandler::send_result(std::string key, std::vector<std::byte> &data) {
  return std::async(std::launch::async, [this, key, data]() {
    std::vector<ResultReply> result;

    grpc::ClientContext context_client_writer;

    auto reply = new ResultReply;

    size_t chunck;
    size_t max_chunck = config_.data_chunk_max_size();
    const size_t data_size = data.size();
    size_t start = 0;

    auto stream = stub_->SendResult(&context_client_writer, reply);

    Result init_msg;
    init_msg.mutable_init()->set_key(key);

    while (start < data_size) {
      chunck = std::min(max_chunck, data_size - start);

      Result msg;

      armonik::api::grpc::v1::DataChunk result_msg;
      std::string data_str;
      int count = 0;
      for (int i = start; i < start + chunck; i++) {
        data_str[count++] = static_cast<char>(data[i]); 
      }

      *result_msg.mutable_data() = std::move(data_str);

      *msg.mutable_data() = std::move(result_msg);

      stream->Write(msg);

      start += chunck;
    }

    armonik::api::grpc::v1::DataChunk result_completed;
    result_completed.set_data_complete(true);

    Result end_msg;
    *end_msg.mutable_data() = std::move(result_completed);

    stream->Write(std::move(end_msg));

    stream->WritesDone();
    grpc::Status status = stream->Finish();

    if (!status.ok()) {
      std::stringstream message;
      message << "Error: " << status.error_code() << ": " << status.error_message()
              << ". details: " << status.error_details() << std::endl;
      throw std::runtime_error(message.str().c_str());
    }

    delete reply;
    return result;
  });

}

/**
 * @brief Get the result ids object
 * 
 * @param results The results data
 * @return std::vector<std::string> list of result ids
 */
std::vector<std::string> TaskHandler::get_result_ids(std::vector<CreateResultsMetaDataRequest_ResultCreate> results) {
  std::vector<std::string> result_ids;

  grpc::ClientContext context_client_writer;
  CreateResultsMetaDataRequest request;
  CreateResultsMetaDataResponse reply;

  *request.mutable_results() = {results.begin(), results.end()};
  request.set_session_id(session_id_);

  Status status = stub_->CreateResultsMetaData(&context_client_writer, request, &reply);

  auto results_reply = reply.results();

  for (auto &result_reply : results_reply) {
    result_ids.push_back(result_reply.result_id());
  }

  return result_ids;
}
