#include <iostream>
#include <memory>
#include <string>

#include <grpc++/grpc++.h>

#include "submitter/SessionContext.h"
#include "submitter/SubmitterClientExt.h"
#include "submitter_service.grpc.pb.h"

#include "utils/GuuId.h"
#include "utils/StringsUtils.h"
#include "serilog/serilog.h"

using grpc::Channel;
using grpc::ClientContext;
using grpc::Status;
using armonik::api::grpc::v1::submitter::CreateSessionRequest;
using armonik::api::grpc::v1::submitter::CreateSessionReply;
using armonik::api::grpc::v1::submitter::Submitter;
using armonik::api::grpc::v1::TaskOptions;


using namespace armonik::api::common::serilog;

// Function to create a session
std::shared_ptr<SessionContext> createSession(const std::string& server_address)
{
  // Create a gRPC channel to communicate with the server
  std::shared_ptr<Channel> channel = grpc::CreateChannel(server_address, grpc::InsecureChannelCredentials());

  // Create a stub for the Submitter service
  std::unique_ptr<Submitter::Stub> stub = Submitter::NewStub(channel);

  // Create a request object and set its fields, including the default_task_option field of type TaskOptions
  CreateSessionRequest request;
  TaskOptions task_options;

  task_options.mutable_options()->insert({"key1", "value1"});
  task_options.mutable_options()->insert({"key2", "value2"});
  task_options.mutable_max_duration()->set_seconds(3600);
  task_options.mutable_max_duration()->set_nanos(0);
  task_options.set_max_retries(3);
  task_options.set_priority(1);
  task_options.set_partition_id("default");
  task_options.set_application_name("my-app");
  task_options.set_application_version("1.0");
  task_options.set_application_namespace("my-namespace");
  task_options.set_application_service("my-service");
  task_options.set_engine_type("Unified");

  *request.mutable_default_task_option() = task_options;

  // Send the CreateSession request to the server and get the response
  auto session_context = std::make_shared<SessionContext>(channel, task_options);

  CreateSessionReply reply;
  grpc::ClientContext context;
  Status status = stub->CreateSession(&context, request, &reply);

  // Check if the request was successful and print the response
  if (status.ok())
  {
    std::cout << "CreateSession response: " << reply.DebugString() << std::endl;
  }
  else
  {
    std::cout << "CreateSession request failed with error code " << status.error_code() << ": " << status.
      error_message() << std::endl;
  }
  session_context->set_session_id(reply.session_id());
  return session_context;
}

int main(int argc, char** argv)
{
  serilog log;
  serilog::init(logging_level::debug, logging_level::verbose);
  
  log.enrich([&](serilog_context& ctx) {
    ctx.add("EnrichedThreadId", std::this_thread::get_id());
    });
  log.enrich([&](serilog_context& ctx) {
    ctx.add("EnrichedFieldValue", 1);

    });
  log.add_property("AddedProperty", time(nullptr));

  // Call the createSession function with the server address
  ::putenv("GRPC_DNS_RESOLVER=native");

  std::cout << "Starting client..." << std::endl;
  std::string server_address = "ddu-srv-wsl:5001";
  auto session_context = createSession(server_address);
  std::tuple payload = std::make_tuple<std::string, std::vector<char>, std::vector<std::string>>(
    armonik::api::common::utils::GuuId::generate_uuid(), {'a', 'r', 'm', 'o', 'n', 'i', 'k'}, {});

  try
  {
    std::vector<std::tuple<std::string, std::vector<char>, std::vector<std::string>>> payloads;

    for (int i = 0; i < 1000; i++)
    {
      payloads.push_back(std::make_tuple<std::string, std::vector<char>, std::vector<std::string>>(
        armonik::api::common::utils::GuuId::generate_uuid(), { 'a', 'r', 'm', 'o', 'n', 'i', 'k' }, {}));
    }
    const auto task_ids = SubmitterClientExt::submit_tasks_with_dependencies(*session_context, payloads, 5);
    for (const auto& task_id : task_ids)
    {
      std::cout << "Generate task_ids : " << task_id << std::endl;
    }
  }
  catch (std::exception& e)
  {
    std::cerr << e.what() << std::endl;
  }

 

  std::cout << "Stooping client..." << std::endl;
  return 0;
}
