/**
 * @file SubmitterClientExt.h
 * @brief Header file for the SubmitterClientExt class.
 */
#include "submitter/SubmitterClient.h"

#include <future>
#include <sstream>
#include <string>

#include "submitter_common.pb.h"
#include "submitter_service.grpc.pb.h"
#include "objects.grpc.pb.h"


using grpc::Channel;
using grpc::ClientContext;
using grpc::Status;
using armonik::api::grpc::v1::submitter::CreateSessionRequest;
using armonik::api::grpc::v1::submitter::CreateSessionReply;
using armonik::api::grpc::v1::submitter::Submitter;
using armonik::api::grpc::v1::TaskOptions;
using armonik::api::grpc::v1::TaskRequest;
using armonik::api::grpc::v1::ResultRequest;
using namespace armonik::api::grpc::v1::submitter;

/*
*/
/*SubmitterClient::SubmitterClient(std::shared_ptr<armonik::api::grpc::v1::submitter::Submitter::Stub> stub)
{
  stub_ = stub;
}*/

SubmitterClient::SubmitterClient(armonik::api::grpc::v1::submitter::Submitter::StubInterface* stub)
{
  stub_ = stub;
}

void SubmitterClient::init(std::shared_ptr<Channel>& channel, TaskOptions& task_options)
{
  channel = grpc::CreateChannel("172.30.39.223:5001", grpc::InsecureChannelCredentials());

  task_options.mutable_options()->insert({ "key1", "value1" });
  task_options.mutable_options()->insert({ "key2", "value2" });
  task_options.mutable_max_duration()->set_seconds(3600);
  task_options.mutable_max_duration()->set_nanos(0);
  task_options.set_max_retries(3);
  task_options.set_priority(1);
  task_options.set_partition_id("cpp");
  task_options.set_application_name("my-app");
  task_options.set_application_version("1.0");
  task_options.set_application_namespace("my-namespace");
  task_options.set_application_service("my-service");
  task_options.set_engine_type("Unified");
}

/**
 * @brief Create a new session.
 * @param partition_ids The partitions ids.
 * @param task_options The task options.
 */
std::string SubmitterClient::create_session(TaskOptions task_options,
                                               const std::vector<std::string>& partition_ids = {"default"})
{
  CreateSessionRequest request;
  *request.mutable_default_task_option() = task_options;
  for (const auto& partition_id : partition_ids)
  {
    request.add_partition_ids(partition_id);
  }
  CreateSessionReply reply;
  
  Status status = stub_->CreateSession(&context_, request, &reply);
  std::cout << "create_session response: " << reply.session_id() << std::endl;
  if (status.ok())
  {
    return reply.session_id();
  }
  std::cout << "CreateSession rpc failed: " << status.error_message()
    << std::endl;
  return "";
}

/**
 * @brief Cancel a session.
 *
 * @param session_id The session id.
 */
void SubmitterClient::cancel_session(const std::string& session_id)
{
  armonik::api::grpc::v1::Session request;

  request.set_id(session_id);

  armonik::api::grpc::v1::Empty reply;

  stub_->CancelSession(&context_, request, &reply);
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
std::vector<std::future<std::vector<
  CreateLargeTaskRequest>>> SubmitterClient::to_request_stream(const std::vector<TaskRequest>& task_requests,
                                                                  std::string& session_id,
                                                                  const TaskOptions& task_options,
                                                                  const size_t chunk_max_size)
{
  std::vector<std::future<std::vector<CreateLargeTaskRequest>>> async_chunk_payload_tasks;
  async_chunk_payload_tasks.push_back(std::async([session_id, task_options]()
  {
    CreateLargeTaskRequest_InitRequest create_large_task_request_init;
    create_large_task_request_init.set_session_id(session_id);
    *create_large_task_request_init.mutable_task_options() = task_options;

    CreateLargeTaskRequest create_large_task_request;
    *create_large_task_request.mutable_init_request() = create_large_task_request_init;

    return std::vector{create_large_task_request};
  }));


  for (auto task_request = task_requests.begin(); task_request != task_requests.end(); ++task_request)
  {
    const bool is_last = task_request == task_requests.end() - 1 ? true : false;

    async_chunk_payload_tasks.push_back(task_chunk_stream(*task_request, is_last, chunk_max_size));
  }
  std::vector<std::future<CreateLargeTaskRequest>> requests;

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
std::future<std::vector<CreateLargeTaskRequest>> SubmitterClient::task_chunk_stream(
  const TaskRequest& task_request, bool is_last,
  size_t chunk_max_size)
{
  return std::async(std::launch::async, [&task_request, chunk_max_size, is_last]()
  {
    std::vector<CreateLargeTaskRequest> requests;
    armonik::api::grpc::v1::InitTaskRequest header_task_request;
    armonik::api::grpc::v1::TaskRequestHeader header;

    header.mutable_data_dependencies()->Add(task_request.data_dependencies().begin(),
                                            task_request.data_dependencies().end());
    header.mutable_expected_output_keys()->Add(task_request.expected_output_keys().begin(),
                                               task_request.expected_output_keys().end());
    *header_task_request.mutable_header() = header;

    CreateLargeTaskRequest create_init_task_request;
    *create_init_task_request.mutable_init_task() = header_task_request;

    //Add init task request
    requests.push_back(create_init_task_request);


    //TODO : Need to stop when future is canceled with std::atomic<bool>


    if (task_request.payload().empty())
    {
      CreateLargeTaskRequest empty_task_request;

      armonik::api::grpc::v1::DataChunk task_payload;
      *task_payload.mutable_data() = {};
      *empty_task_request.mutable_task_payload() = task_payload;
      requests.push_back(empty_task_request);
    }

    size_t start = 0;

    while (start < task_request.payload().size())
    {
      //TODO : Need to stop when future is canceled with std::atomic<bool>

      size_t chunk_size = std::min(chunk_max_size, task_request.payload().size() - start);

      CreateLargeTaskRequest chunk_task_request;

      armonik::api::grpc::v1::DataChunk task_payload;

      *task_payload.mutable_data() = task_request.payload().substr(start, chunk_size);
      *chunk_task_request.mutable_task_payload() = task_payload;

      requests.push_back(chunk_task_request);

      start += chunk_size;
    }

    CreateLargeTaskRequest complete_task_request;
    armonik::api::grpc::v1::DataChunk end_payload;

    end_payload.set_data_complete(true);
    *complete_task_request.mutable_task_payload() = end_payload;
    requests.push_back(complete_task_request);

    if (is_last)
    {
      CreateLargeTaskRequest last_task_request;
      armonik::api::grpc::v1::InitTaskRequest init_task_request;

      init_task_request.set_last_task(true);
      *last_task_request.mutable_init_task() = init_task_request;

      requests.push_back(last_task_request);
    }

    return requests;
  });
}

/**
 * @brief Asynchronously create tasks.
 *
 * @param channel The shared_ptr to the gRPC channel.
 * @param session_id The session ID.
 * @param task_options The TaskOptions object.
 * @param task_requests A vector of TaskRequest objects.
 * @return A future containing a CreateTaskReply object.
 */
std::future<CreateTaskReply> SubmitterClient::create_tasks_async(const std::shared_ptr<grpc::Channel>& channel,
                                                                    std::string& session_id,
                                                                    const TaskOptions& task_options,
                                                                    const std::vector<TaskRequest>&
                                                                    task_requests)
{
  return std::async(std::launch::async, [channel, &task_requests, &session_id, &task_options]()
  {
    armonik::api::grpc::v1::Configuration config_response;
    grpc::ClientContext context_configuration;
    auto client = Submitter::NewStub(channel);

    const auto config_status = client->GetServiceConfiguration(&context_configuration, armonik::api::grpc::v1::Empty(),
                                                               &config_response);
    size_t chunk = 0;
    if (config_status.ok())
    {
      chunk = config_response.data_chunk_max_size();
    }
    else
    {
      throw std::runtime_error("Fail to get service configuration");
    }

    auto reply = new CreateTaskReply();

    reply->set_allocated_creation_status_list(
      new armonik::api::grpc::v1::submitter::CreateTaskReply_CreationStatusList());
    grpc::ClientContext context_client_writer;
    std::unique_ptr stream(
      client->CreateLargeTasks(&context_client_writer, reply));

    //task_chunk_stream(task_requests, )
    std::vector<std::future<CreateLargeTaskRequest>> async_task_requests;
    std::vector<std::future<CreateLargeTaskRequest>> create_large_task_requests;
    auto create_task_request_async = to_request_stream(
      task_requests, session_id, task_options, chunk);

    for (auto& f : create_task_request_async)
    {
      for (auto& create_large_task_request : f.get())
      {
        stream->Write(create_large_task_request);
      }
    }

    stream->WritesDone();
    grpc::Status status = stream->Finish();
    if (!status.ok())
    {
      std::stringstream message;
      message << "Error: " << status.error_code() << ": "
        << status.error_message() << ". details : " << status.error_details() << std::endl;
      throw std::runtime_error(message.str().c_str());
    }

    auto response = CreateTaskReply(*reply);
    delete reply;
    return response;
  });
}

/**
 * @brief Submit tasks with dependencies.
 *
 * @param session_context The SessionContext object.
 * @param payloads_with_dependencies A vector of tuples containing task payload and its dependencies.
 * @param max_retries Maximum number of retries.
 * @return A vector of task IDs.
 */
std::vector<std::string> SubmitterClient::submit_tasks_with_dependencies(SessionContext& session_context,
                                                                            std::vector<std::tuple<
                                                                              std::string, std::vector<char>,
                                                                              std::vector<std::string>>>
                                                                            payloads_with_dependencies,
                                                                            int max_retries = 5)
{
  std::vector<std::string> task_ids;
  std::vector<TaskRequest> requests;

  for (auto& payload : payloads_with_dependencies)
  {
    TaskRequest request;
    auto& bytes = std::get<1>(payload);

    //TODO : Avoid copy of payload here. Play with std::vector<char> and an allocated char
    request.add_expected_output_keys(std::get<0>(payload));

    *request.mutable_payload() = std::string(bytes.begin(), bytes.end());

    *request.mutable_data_dependencies() = {std::get<2>(payload).begin(), std::get<2>(payload).end()};

    requests.push_back(request);
  }

  auto tasks_async = create_tasks_async(session_context.get_channel(), session_context.get_session_id(),
                                        session_context.get_task_options(), requests);

  const CreateTaskReply createTaskReply = tasks_async.get();


  switch (createTaskReply.Response_case())
  {
  case CreateTaskReply::RESPONSE_NOT_SET:
    throw std::runtime_error("Issue with Server !");
  case CreateTaskReply::kCreationStatusList:
    {
      auto task_reply_creation_statuses = createTaskReply.creation_status_list().creation_statuses();

      std::transform(task_reply_creation_statuses.begin(), task_reply_creation_statuses.end(),
                     std::back_inserter(task_ids),
                     [](const CreateTaskReply_CreationStatus& x) { return x.task_info().task_id(); });
      break;
    }

  case CreateTaskReply::kError:
    std::stringstream message;
    message << "Error while creating tasks ! : Error Message : " << createTaskReply.error() << std::endl;
    throw std::runtime_error(message.str().c_str());
  }
  return task_ids;
}


/**
 * @brief Asynchronously create tasks.
 *
 * @param channel The shared_ptr to the gRPC channel.
 * @param session_id The session ID.
 * @param task_options The TaskOptions object.
 * @param result_requests A vector of ResultRequest objects.
 * @return A future containing data result.
 */
std::future<std::vector<int8_t>> SubmitterClient::get_result_async(const std::shared_ptr<grpc::Channel>& channel,
                                                                      std::string& session_id,
                                                                      const TaskOptions& task_options,
                                                                      const ResultRequest&
                                                                      result_requests)
{
  return std::async(std::launch::async, [channel, &result_requests, &session_id, &task_options]()
  {
    armonik::api::grpc::v1::submitter::ResultReply result_writer;
    grpc::ClientContext* context_configuration;
    auto client = Submitter::NewStub(channel);

    auto streamingCall = client->TryGetResultStream(context_configuration, result_requests);


    size_t size;

    if (streamingCall)
    {
      size = result_requests.ByteSizeLong();
    }
    else
    {
      throw std::runtime_error("Fail to get result");
    }

    std::vector<int8_t> result_data;
    for (size_t count = 0; count < size; count++)
    {
      streamingCall->WaitForInitialMetadata();
      streamingCall->Read(&result_writer);
      std::string dataString;
      switch (result_writer.type_case())
      {
      case result_writer.kResult:
        dataString = result_writer.result().data();
        for (size_t i = 0; i < dataString.length(); i++)
        {
          result_data.push_back(dataString[i]);
        }
        break;
      case result_writer.kError:
        throw std::runtime_error("Error in task ");
        break;
      case result_writer.kNotCompletedTask:
        throw std::runtime_error("Task not completed");
        break;
      case result_writer.TYPE_NOT_SET:
        throw std::runtime_error("Issue with the Server");
        break;
      default:
        throw std::runtime_error("Unknown return type !");
        break;
      }
    }

    return result_data;
  });
}

/*
std::future<CreateTaskReply> SubmitterClient::list_tasks(const std::shared_ptr<grpc::Channel>& channel,
                                                              std::string& session_id,
                                                              const TaskOptions& task_options,
                                                              const std::vector<TaskRequest>&
                                                              task_requests)
{

}


std::future<CreateTaskReply> SubmitterClient::task_status(const std::shared_ptr<grpc::Channel>& channel,
                                                              std::string& session_id,
                                                              const TaskOptions& task_options,
                                                              const std::vector<TaskRequest>&
                                                              task_requests)
{

}                                                             
*/
