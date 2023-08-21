/**
 * @file SubmitterClientExt.h
 * @brief Header file for the SubmitterClientExt class.
 */
#include "submitter/SubmitterClient.h"

#include <future>
#include <sstream>
#include <string>

#include "exceptions/ArmoniKApiException.h"
#include "exceptions/ArmoniKTaskError.h"
#include "exceptions/ArmoniKTaskNotCompletedException.h"
#include "objects.pb.h"
#include "submitter_common.pb.h"
#include "submitter_service.grpc.pb.h"

using armonik::api::grpc::v1::ResultRequest;
using armonik::api::grpc::v1::TaskOptions;
using armonik::api::grpc::v1::TaskRequest;
using armonik::api::grpc::v1::submitter::CreateSessionReply;
using armonik::api::grpc::v1::submitter::CreateSessionRequest;
using armonik::api::grpc::v1::submitter::Submitter;
using grpc::Channel;
using grpc::ClientContext;
using grpc::Status;
using namespace armonik::api::grpc::v1::submitter;

/**
 * @brief Construct a new Submitter Client:: Submitter Client object
 *
 * @param stub the gRPC client stub
 */
API_CLIENT_NAMESPACE::SubmitterClient::SubmitterClient(std::unique_ptr<Submitter::StubInterface> stub) {
  stub_ = std::move(stub);
}

/**
 * @brief Create a new session.
 * @param partition_ids The partitions ids.
 * @param default_task_options The default task options.
 */
std::string API_CLIENT_NAMESPACE::SubmitterClient::create_session(TaskOptions default_task_options,
                                                                  const std::vector<std::string> &partition_ids = {}) {
  CreateSessionRequest request;
  *request.mutable_default_task_option() = std::move(default_task_options);
  for (const auto &partition_id : partition_ids) {
    request.add_partition_ids(partition_id);
  }
  CreateSessionReply reply;

  Status status = stub_->CreateSession(&context_, request, &reply);
  if (!status.ok()) {
    std::stringstream message;
    message << "Error: " << status.error_code() << ": " << status.error_message()
            << ". details : " << status.error_details() << std::endl;
    std::cout << "CreateSession rpc failed: " << std::endl;
    throw std::runtime_error(message.str().c_str());
  }
  return reply.session_id();
}

/**
 * @brief Convert task_requests to request_stream.
 *
 * @param task_requests A vector of TaskRequest objects.
 * @param session_id The session ID.
 * @param task_options The TaskOptions object.
 * @param chunk_max_size Maximum chunk size.
 * @return A vector of futures containing CreateLargeTaskRequest objects.
 */
std::vector<std::future<std::vector<CreateLargeTaskRequest>>>
API_CLIENT_NAMESPACE::SubmitterClient::to_request_stream(const std::vector<TaskRequest> &task_requests,
                                                         std::string session_id, TaskOptions task_options,
                                                         const size_t chunk_max_size) {
  std::vector<std::future<std::vector<CreateLargeTaskRequest>>> async_chunk_payload_tasks;
  async_chunk_payload_tasks.push_back(
      std::async([session_id = std::move(session_id), task_options = std::move(task_options)]() mutable {
        CreateLargeTaskRequest_InitRequest create_large_task_request_init;
        create_large_task_request_init.set_session_id(std::move(session_id));
        *create_large_task_request_init.mutable_task_options() = std::move(task_options);

        CreateLargeTaskRequest create_large_task_request;
        *create_large_task_request.mutable_init_request() = std::move(create_large_task_request_init);

        return std::vector{std::move(create_large_task_request)};
      }));

  for (auto task_request = task_requests.begin(); task_request != task_requests.end(); ++task_request) {
    const bool is_last = task_request == task_requests.end() - 1;

    async_chunk_payload_tasks.push_back(task_chunk_stream(*task_request, is_last, chunk_max_size));
  }

  return async_chunk_payload_tasks;
}

/**
 * @brief Create a task_chunk_stream.
 *
 * @param task_request The TaskRequest object.
 * @param is_last A boolean indicating if this is the last request.
 * @param chunk_max_size Maximum chunk size.
 * @return A future containing a vector of CreateLargeTaskRequest objects.
 */
std::future<std::vector<CreateLargeTaskRequest>>
API_CLIENT_NAMESPACE::SubmitterClient::task_chunk_stream(const TaskRequest &task_request, bool is_last,
                                                         size_t chunk_max_size) {
  return std::async(std::launch::async, [&task_request, chunk_max_size, is_last]() {
    std::vector<CreateLargeTaskRequest> requests;
    armonik::api::grpc::v1::InitTaskRequest header_task_request;
    armonik::api::grpc::v1::TaskRequestHeader header;

    header.mutable_data_dependencies()->Add(task_request.data_dependencies().begin(),
                                            task_request.data_dependencies().end());
    header.mutable_expected_output_keys()->Add(task_request.expected_output_keys().begin(),
                                               task_request.expected_output_keys().end());
    *header_task_request.mutable_header() = std::move(header);

    CreateLargeTaskRequest create_init_task_request;
    *create_init_task_request.mutable_init_task() = std::move(header_task_request);

    // Add init task request
    requests.push_back(std::move(create_init_task_request));

    if (task_request.payload().empty()) {
      CreateLargeTaskRequest empty_task_request;

      armonik::api::grpc::v1::DataChunk task_payload;
      *task_payload.mutable_data() = {};
      *empty_task_request.mutable_task_payload() = std::move(task_payload);
      requests.push_back(std::move(empty_task_request));
    }

    size_t start = 0;

    while (start < task_request.payload().size()) {

      size_t chunk_size = std::min(chunk_max_size, task_request.payload().size() - start);

      CreateLargeTaskRequest chunk_task_request;

      armonik::api::grpc::v1::DataChunk task_payload;

      *task_payload.mutable_data() = task_request.payload().substr(start, chunk_size);
      *chunk_task_request.mutable_task_payload() = std::move(task_payload);

      requests.push_back(std::move(chunk_task_request));

      start += chunk_size;
    }

    CreateLargeTaskRequest complete_task_request;
    armonik::api::grpc::v1::DataChunk end_payload;

    end_payload.set_data_complete(true);
    *complete_task_request.mutable_task_payload() = std::move(end_payload);
    requests.push_back(std::move(complete_task_request));

    if (is_last) {
      CreateLargeTaskRequest last_task_request;
      armonik::api::grpc::v1::InitTaskRequest init_task_request;

      init_task_request.set_last_task(true);
      *last_task_request.mutable_init_task() = std::move(init_task_request);

      requests.push_back(std::move(last_task_request));
    }

    return requests;
  });
}

/**
 * @brief Asynchronously create tasks.
 *
 * @param session_id The session ID.
 * @param task_options The TaskOptions object.
 * @param task_requests A vector of TaskRequest objects.
 * @return A future containing a CreateTaskReply object.
 */
std::future<CreateTaskReply>
API_CLIENT_NAMESPACE::SubmitterClient::create_tasks_async(std::string session_id, TaskOptions task_options,
                                                          const std::vector<TaskRequest> &task_requests) {
  return std::async(std::launch::async, [this, task_requests, session_id = std::move(session_id),
                                         task_options = std::move(task_options)]() mutable {
    armonik::api::grpc::v1::Configuration config_response;
    ClientContext context_configuration;

    const auto config_status =
        stub_->GetServiceConfiguration(&context_configuration, armonik::api::grpc::v1::Empty(), &config_response);
    size_t chunk = 0;
    if (config_status.ok()) {
      chunk = config_response.data_chunk_max_size();
    } else {
      throw std::runtime_error("Fail to get service configuration");
    }

    CreateTaskReply reply{};

    reply.set_allocated_creation_status_list(new CreateTaskReply_CreationStatusList());
    ClientContext context_client_writer;
    std::unique_ptr stream(stub_->CreateLargeTasks(&context_client_writer, &reply));

    // task_chunk_stream(task_requests, )
    std::vector<std::future<CreateLargeTaskRequest>> async_task_requests;
    std::vector<std::future<CreateLargeTaskRequest>> create_large_task_requests;
    auto create_task_request_async =
        to_request_stream(task_requests, std::move(session_id), std::move(task_options), chunk);

    for (auto &f : create_task_request_async) {
      for (auto &create_large_task_request : f.get()) {
        stream->Write(create_large_task_request);
      }
    }

    stream->WritesDone();
    Status status = stream->Finish();
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
 * @brief Submit tasks with dependencies.
 *
 * @param session_id The session ID.
 * @param task_options The task options
 * @param payloads_with_dependencies A vector of tuples containing task payload
 * and its dependencies.
 * @param max_retries Maximum number of retries.
 * @return A vector of task IDs.
 */
std::tuple<std::vector<std::string>, std::vector<std::string>>
API_CLIENT_NAMESPACE::SubmitterClient::submit_tasks_with_dependencies(
    std::string session_id, TaskOptions task_options, const std::vector<payload_data> &payloads_with_dependencies,
    int max_retries = 5) {
  std::vector<std::string> task_ids;
  std::vector<std::string> failed_task_ids;
  std::vector<TaskRequest> requests;
  for (auto &payload : payloads_with_dependencies) {
    TaskRequest request;

    request.add_expected_output_keys(payload.keys);

    *request.mutable_payload() = payload.payload;

    *request.mutable_data_dependencies() = {payload.dependencies.begin(), payload.dependencies.end()};

    requests.push_back(request);
  }

  auto tasks_async = create_tasks_async(std::move(session_id), std::move(task_options), requests);

  const CreateTaskReply createTaskReply = tasks_async.get();

  switch (createTaskReply.Response_case()) {
  case CreateTaskReply::RESPONSE_NOT_SET:
    throw std::runtime_error("Issue with Server !");
  case CreateTaskReply::kCreationStatusList: {
    auto task_reply_creation_statuses = createTaskReply.creation_status_list().creation_statuses();

    for (auto &task_created : task_reply_creation_statuses) {
      if (task_created.Status_case() == CreateTaskReply_CreationStatus::kTaskInfo) {
        task_ids.push_back(task_created.task_info().task_id());
      } else {
        failed_task_ids.push_back(task_created.task_info().task_id());
      }
    }
    break;
  }

  case CreateTaskReply::kError:
    std::stringstream message;
    message << "Error while creating tasks ! : Error Message : " << createTaskReply.error() << std::endl;
    throw std::runtime_error(message.str().c_str());
  }
  return std::make_tuple(std::move(task_ids), std::move(failed_task_ids));
}

/**
 * @brief Asynchronously gets tasks.
 *
 * @param result_request A vector of ResultRequest objects.
 * @return A future containing data result.
 */
std::future<std::string> API_CLIENT_NAMESPACE::SubmitterClient::get_result_async(const ResultRequest &result_request) {
  return std::async(std::launch::async, [this, &result_request]() {
    ResultReply reply;
    ClientContext context_result;

    auto streamingCall = stub_->TryGetResultStream(&context_result, result_request);

    std::string result_data;
    bool dataComplete = false;
    while (streamingCall->Read(&reply)) {
      size_t offset = result_data.size();
      switch (reply.type_case()) {
      case ResultReply::kResult:
        switch (reply.result().type_case()) {
        case armonik::api::grpc::v1::DataChunk::kData:
          result_data.resize(offset + reply.result().data().size());
          std::memcpy(result_data.data() + offset, reply.result().data().data(), reply.result().data().size());
          dataComplete = false; // Setting to false in case we receive stuff out of order
          break;
        case armonik::api::grpc::v1::DataChunk::kDataComplete:
          dataComplete = true;
          break;
        case armonik::api::grpc::v1::DataChunk::TYPE_NOT_SET:
          throw ArmoniK::Api::Common::exceptions::ArmoniKApiException("Issue with server, invalid data chunk");
        }
        break;
      case ResultReply::kError:
        throw ArmoniK::Api::Common::exceptions::ArmoniKTaskError("Can't get result because it's in error",
                                                                 reply.error());
      case ResultReply::kNotCompletedTask:
        throw ArmoniK::Api::Common::exceptions::ArmoniKTaskNotCompletedException(reply.not_completed_task());
      case ResultReply::TYPE_NOT_SET:
        throw ArmoniK::Api::Common::exceptions::ArmoniKApiException("Issue with server, invalid reply");
      }
    }

    if (!dataComplete) {
      throw ArmoniK::Api::Common::exceptions::ArmoniKApiException("Retrieved data is incomplete");
    }

    return result_data;
  });
}
std::map<std::string, armonik::api::grpc::v1::result_status::ResultStatus>
ArmoniK::Api::Client::SubmitterClient::get_result_status(const std::string &session_id,
                                                         const std::vector<std::string> &result_ids) {
  grpc::ClientContext context;
  armonik::api::grpc::v1::submitter::GetResultStatusRequest request;
  armonik::api::grpc::v1::submitter::GetResultStatusReply reply;

  request.set_session_id(session_id);
  request.mutable_result_ids()->Add(result_ids.begin(), result_ids.end());

  auto status = stub_->GetResultStatus(&context, request, &reply);
  if (!status.ok()) {
    throw ArmoniK::Api::Common::exceptions::ArmoniKApiException("Couldn't get result status : " +
                                                                status.error_message());
  }

  std::map<std::string, armonik::api::grpc::v1::result_status::ResultStatus> statuses;
  for (auto &&id_status : reply.id_statuses()) {
    statuses[id_status.result_id()] = id_status.status();
  }

  return statuses;
}
