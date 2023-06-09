#include <iostream>
#include <memory>
#include <string>

#include <grpc++/grpc++.h>

#include <gmock/gmock.h> 
#include <gtest/gtest.h>

#include "SubmitterClientTest.h"

#include "submitter/SessionContext.h"
#include "submitter/SubmitterClient.h"
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

using ::testing::AtLeast;
using ::testing::_;

using namespace armonik::api::common::serilog;


TEST(testMock, createSession)
{
  MockStubInterface stub;

  ClientContext context;
  CreateSessionReply reply;
  CreateSessionRequest request;

  const std::vector<std::string>& partition_ids = { "default" };


  TaskOptions task_options;

  std::shared_ptr<Channel> channel = grpc::CreateChannel("172.30.39.223:5001", grpc::InsecureChannelCredentials());

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


  ASSERT_EQ(task_options.partition_id(), "cpp");


  EXPECT_CALL(stub, CreateSession(_,_,_)).Times(AtLeast(1));

  

 
  SubmitterClient submitter(&stub);
  std::string session_id = submitter.create_session(task_options, partition_ids);

  std::cout << "create_session response: " << session_id << std::endl;


  ASSERT_TRUE(session_id.empty());
}



TEST(testMock, cancleSession)
{
  MockStubInterface stub;

  ClientContext context;
  CreateSessionReply reply;
  CreateSessionRequest request;

  const std::vector<std::string>& partition_ids = { "default" };


  TaskOptions task_options;

  std::shared_ptr<Channel> channel = grpc::CreateChannel("172.30.39.223:5001", grpc::InsecureChannelCredentials());

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
  

  EXPECT_CALL(stub, CancelSession(_, _, _)).Times(AtLeast(1));




  SubmitterClient submitter(&stub);
  std::string session_id = submitter.create_session(task_options, partition_ids);

  std::cout << "create_session response: " << session_id << std::endl;


  ASSERT_TRUE(session_id.empty());

  ASSERT_TRUE(submitter.cancel_session(session_id));

}


TEST(testMock, submitTask)
{
  
  serilog log(logging_format::CONSOLE);

  std::cout << "Serilog closed" << std::endl;

  log.enrich([&](serilog_context& ctx) {
    ctx.add("threadid", std::this_thread::get_id());
    });
  log.enrich([&](serilog_context& ctx) {
    ctx.add("fieldTestValue", 1);

    });
  log.add_property("time", time(nullptr));

  // Call the createSession function with the server address
  ::putenv("GRPC_DNS_RESOLVER=native");


  std::cout << "Starting client..." << std::endl;
  std::string server_address = "172.30.39.223:5001";

  CreateSessionRequest request;
  TaskOptions task_options;

  std::shared_ptr<Channel> channel = grpc::CreateChannel(server_address, grpc::InsecureChannelCredentials());

  MockStubInterface stub;
  std::unique_ptr<Submitter::Stub> ptr_test = Submitter::NewStub(channel);

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


  *request.mutable_default_task_option() = task_options;
  request.add_partition_ids(task_options.partition_id());

  auto session_context = std::make_shared<SessionContext>(channel, task_options);

  // stub = reinterpret_cast<MockStubInterface*>(ptr_test.release());

  EXPECT_CALL(stub, CreateSession(_, _, _)).Times(AtLeast(1));
  EXPECT_CALL(stub, GetServiceConfiguration(_, _, _)).Times(AtLeast(1));
  EXPECT_CALL(stub, CreateLargeTasksRaw(_, _)).Times(AtLeast(1));

  CreateSessionReply reply;
  grpc::ClientContext context;
  //Status status = ptr_test->CreateSession(&context, request, &reply);




  // stub = reinterpret_cast<MockStubInterface*> (Submitter::NewStub(channel).release());


  SubmitterClient submitter(ptr_test.release());
  const std::vector<std::string>& partition_ids = { "default" };
  std::string session_id = submitter.create_session(task_options, partition_ids);

  session_context->set_session_id(reply.session_id());

  ASSERT_TRUE(reply.session_id().empty());

  try
  {
    std::vector<std::tuple<std::string, std::vector<char>, std::vector<std::string>>> payloads;

    for (int i = 0; i < 10; i++)
    {
      payloads.push_back(std::make_tuple<std::string, std::vector<char>, std::vector<std::string>>(
        armonik::api::common::utils::GuuId::generate_uuid(), { 'a', 'r', 'm', 'o', 'n', 'i', 'k' }, {}));
    }
    const auto task_ids = submitter.submit_tasks_with_dependencies(*session_context, payloads, 5);
    for (const auto& task_id : task_ids)
    {
      std::stringstream out;
      out << "Generate task_ids : " << task_id;
      log.info(out.str());
    }
  }
  catch (std::exception& e)
  {
    log.error(e.what());
    throw;
  }
  log.info("Stopping client...OK");

}


TEST(testMock, getResult)
{
  MockStubInterface stub;

  CreateSessionReply reply;
  CreateSessionRequest request;

  const std::vector<std::string>& partition_ids = { "default" };


  TaskOptions task_options;
  armonik::api::grpc::v1::ResultRequest result_request;

  std::shared_ptr<Channel> channel = grpc::CreateChannel("172.30.39.223:5001", grpc::InsecureChannelCredentials());


  EXPECT_CALL(stub, GetServiceConfiguration(_, _, _)).Times(AtLeast(1));
  EXPECT_CALL(stub, TryGetResultStreamRaw(_, _)).Times(AtLeast(1));


  SubmitterClient submitter(&stub);

  auto result = submitter.get_result_async(channel, result_request);

  ASSERT_FALSE(result.get().empty());
}
