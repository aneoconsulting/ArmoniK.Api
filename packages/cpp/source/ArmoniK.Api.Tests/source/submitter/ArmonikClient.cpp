#include <iostream>
#include <memory>
#include <string>
#include <grpcpp/grpcpp.h>
#include "submitter_service.pb.h"
#include "submitter_common.pb.h"
#include "submitter_service.grpc.pb.h"

using grpc::Channel;
using grpc::ClientContext;
using grpc::Status;
using armonik::api::grpc::v1::TaskRequest;
using armonik::api::grpc::v1::TaskOptions;
using armonik::api::grpc::v1::TaskRequestHeader;
using armonik::api::grpc::v1::TaskId;
using armonik::api::grpc::v1::DataChunk;
using namespace armonik::api::grpc::v1::submitter;

class SubmitterClient {
private:
  grpc::ClientContext context_;

 public:
  SubmitterClient(const std::shared_ptr<Channel>& channel)
      : stub_(Submitter::NewStub(channel)) {}

  

  std::string CreateSession(TaskOptions task_options,
                            const std::vector<std::string>& partition_ids);


  void CreateLargeTask(const std::string& session_id,
                       const TaskOptions& options,
                       const std::vector<std::string>& expected_output_keys,
                       const std::vector<std::string>& data_dependencies,
                       const std::vector<uint8_t>& payload) {
    CreateTaskReply response;
    armonik::api::grpc::v1::Configuration config_response;
    stub_->GetServiceConfiguration(&context_, armonik::api::grpc::v1::Empty(), &config_response);

    std::unique_ptr<grpc::ClientWriter<CreateLargeTaskRequest>> writer(
        stub_->CreateLargeTasks(&context_, &response));

    // Send the initialization request
    CreateLargeTaskRequest_InitRequest init_request;
   
    init_request.set_session_id(session_id);
    *init_request.mutable_task_options() = options;
    CreateLargeTaskRequest request;
    *request.mutable_init_request() = init_request;
    writer->Write(request);
    
    // Send the task request
    TaskRequestHeader header;
    *header.mutable_expected_output_keys() = {expected_output_keys.begin(),
                                               expected_output_keys.end()};
    *header.mutable_data_dependencies() = {data_dependencies.begin(),
                                            data_dependencies.end()};

    size_t chunk_size = 80 * 1024;
    size_t offset = 0;
    bool last_chunk = false;
    while (!last_chunk) {
      DataChunk chunk;
      if (offset + chunk_size < payload.size()) {
        *chunk.mutable_data() = std::string(payload.begin() + offset,
                                             payload.begin() + offset + chunk_size);
        offset += chunk_size;
      } else {
        *chunk.mutable_data() = std::string(payload.begin() + offset,
                                             payload.end());
        last_chunk = true;
      }
      armonik::api::grpc::v1::InitTaskRequest init_task_request;
      init_task_request.mutable_header()->CopyFrom(header);
      if (last_chunk) {
        init_task_request.set_last_task(true);
      }
      CreateLargeTaskRequest request;
      
      request.set_allocated_init_task(&init_task_request);
      request.set_allocated_task_payload(&chunk);
      writer->Write(request);
    }

    writer->WritesDone();
    grpc::Status status = writer->Finish();
    if (!status.ok()) {
      std::cerr << "Error: " << status.error_code() << ": "
                << status.error_message() << std::endl;
    }
  }

 private:
  std::unique_ptr<Submitter::Stub> stub_;
};

std::string SubmitterClient::CreateSession(TaskOptions task_options, const std::vector<std::string>& partition_ids = {"default"})
{
  CreateSessionRequest request;
  request.set_allocated_default_task_option(&task_options);
  for (const auto& partition_id : partition_ids) {
    request.add_partition_ids(partition_id);
  }
  CreateSessionReply reply;
  Status status = stub_->CreateSession(&context_, request, &reply);
  if (status.ok()) {
    return reply.session_id();
  } else {
    std::cout << "CreateSession rpc failed: " << status.error_message()
      << std::endl;
    return "";
  }
}
