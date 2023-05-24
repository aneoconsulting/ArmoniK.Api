#include <future>
#include <sstream>
#include <string>


#include "Worker/TaskHandler.h"


using grpc::Channel;
using grpc::ClientContext;
using grpc::Status;
using armonik::api::grpc::v1::agent::CreateTaskRequest;
using armonik::api::grpc::v1::agent::CreateTaskReply;
using armonik::api::grpc::v1::agent::Agent;
using armonik::api::grpc::v1::TaskOptions;
using armonik::api::grpc::v1::TaskRequest;
using armonik::api::grpc::v1::ResultRequest;
using namespace armonik::api::grpc::v1::agent;


std::future<std::vector<
  CreateTaskRequest>> TaskHandler::task_chunk_stream(const TaskRequest& task_request, bool is_last,
                                                     size_t chunk_max_size)
{
  return std::async(std::launch::async, [&task_request, chunk_max_size, is_last]()
  {
    std::vector<CreateTaskRequest> requests;
    armonik::api::grpc::v1::InitTaskRequest header_task_request;
    armonik::api::grpc::v1::TaskRequestHeader header;

    header.mutable_data_dependencies()->Add(task_request.data_dependencies().begin(),
                                            task_request.data_dependencies().end());
    header.mutable_expected_output_keys()->Add(task_request.expected_output_keys().begin(),
                                               task_request.expected_output_keys().end());
    *header_task_request.mutable_header() = header;

    CreateTaskRequest create_init_task_request;
    *create_init_task_request.mutable_init_task() = header_task_request;

    requests.push_back(create_init_task_request);

    if (task_request.payload().empty())
    {
      CreateTaskRequest empty_task_request;

      armonik::api::grpc::v1::DataChunk task_payload;
      *task_payload.mutable_data() = {};
      *empty_task_request.mutable_task_payload() = task_payload;
      requests.push_back(empty_task_request);
    }

    size_t start = 0;

    while (start < task_request.payload().size())
    {

      size_t chunk_size = std::min(chunk_max_size, task_request.payload().size() - start);

      CreateTaskRequest chunk_task_request;

      armonik::api::grpc::v1::DataChunk task_payload;

      *task_payload.mutable_data() = task_request.payload().substr(start, chunk_size);
      *chunk_task_request.mutable_task_payload() = task_payload;

      requests.push_back(chunk_task_request);

      start += chunk_size;
    }

    CreateTaskRequest complete_task_request;
    armonik::api::grpc::v1::DataChunk end_payload;

    end_payload.set_data_complete(true);
    *complete_task_request.mutable_task_payload() = end_payload;
    requests.push_back(complete_task_request);

    if (is_last)
    {
      CreateTaskRequest last_task_request;
      armonik::api::grpc::v1::InitTaskRequest init_task_request;

      init_task_request.set_last_task(true);
      *last_task_request.mutable_init_task() = init_task_request;

      requests.push_back(last_task_request);
    }

    return requests;
  });
}


std::vector<std::future<std::vector<
  CreateTaskRequest>>> TaskHandler::to_request_stream(const std::vector<TaskRequest>& task_requests,
                                                      const TaskOptions& task_options,
                                                      const size_t chunk_max_size)
{
  std::vector<std::future<std::vector<CreateTaskRequest>>> async_chunk_payload_tasks;

  async_chunk_payload_tasks.push_back(std::async([task_options]()
  {
    CreateTaskRequest_InitRequest create_task_request_init;
    *create_task_request_init.mutable_task_options() = task_options;

    CreateTaskRequest create_task_request;
    *create_task_request.mutable_init_request() = create_task_request_init;

    return std::vector{create_task_request};
  }));

  for (auto task_request = task_requests.begin(); task_request != task_requests.end(); ++task_request)
  {
    const bool is_last = task_request == task_requests.end() - 1 ? true : false;

    async_chunk_payload_tasks.push_back(task_chunk_stream(*task_request, is_last, chunk_max_size));
  }

  return async_chunk_payload_tasks;
}


std::future<CreateTaskReply> TaskHandler::create_tasks_async(const std::shared_ptr<grpc::ChannelInterface>& channel,
                                                                    std::string& session_id, 
                                                                    const TaskOptions& task_options,
                                                                    const std::vector<TaskRequest>& task_requests)
{
    return std::async(std::launch::async, [channel, &task_requests, &session_id, &task_options]()
  {
    armonik::api::grpc::v1::Configuration config_response;
    grpc::ClientContext context_configuration;
    auto client = Agent::NewStub(channel);

   // const auto config_status = client->GetServiceConfiguration(&context_configuration, armonik::api::grpc::v1::Empty(),
   //                                                            &config_response);
    size_t chunk = 0;
    /*if (config_status.ok())
    {
      chunk = config_response.data_chunk_max_size();
    }
    else
    {
      throw std::runtime_error("Fail to get service configuration");
    }*/

    auto reply = new CreateTaskReply();

    reply->set_allocated_creation_status_list(
      new armonik::api::grpc::v1::agent::CreateTaskReply_CreationStatusList());
    grpc::ClientContext context_client_writer;
    std::unique_ptr stream(
      client->CreateTask(&context_client_writer, reply));

    auto create_task_request_async = to_request_stream(
      task_requests, task_options, chunk);

    for (auto& f : create_task_request_async)
    {
      for (auto& create_task_request : f.get())
      {
        stream->Write(create_task_request);
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