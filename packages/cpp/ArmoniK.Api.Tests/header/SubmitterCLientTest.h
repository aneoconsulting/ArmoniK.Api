#pragma once
#include <future>
#include <string>

#include "gmock/gmock.h"

#include "submitter_common.pb.h"
#include "submitter_service.grpc.pb.h"

#include "submitter/SubmitterClient.h"

/**
 * @brief Aims to mock the gRPC client stub
 *
 */
class MockStubInterface : public armonik::api::grpc::v1::submitter::Submitter::StubInterface {
public:
  MOCK_METHOD(::grpc::Status, GetServiceConfiguration,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::Empty &request,
               ::armonik::api::grpc::v1::Configuration *response));
  MOCK_METHOD(::grpc::Status, CreateSession,
              (::grpc::ClientContext * context,
               const ::armonik::api::grpc::v1::submitter::CreateSessionRequest &request,
               ::armonik::api::grpc::v1::submitter::CreateSessionReply *response));
  MOCK_METHOD(::grpc::Status, CancelSession,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::Session &request,
               ::armonik::api::grpc::v1::Empty *response));
  MOCK_METHOD(::grpc::Status, ListTasks,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::submitter::TaskFilter &request,
               ::armonik::api::grpc::v1::TaskIdList *response));
  MOCK_METHOD(::grpc::Status, CreateSmallTasks,
              (::grpc::ClientContext * context,
               const ::armonik::api::grpc::v1::submitter::CreateSmallTaskRequest &request,
               ::armonik::api::grpc ::v1::submitter::CreateTaskReply *response));
  MOCK_METHOD(::grpc::Status, ListSessions,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::submitter::SessionFilter &request,
               ::armonik::api::grpc::v1::submitter::SessionIdList *response));
  MOCK_METHOD(::grpc::Status, CountTasks,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::submitter::TaskFilter &request,
               ::armonik::api::grpc::v1::Count *response));
  MOCK_METHOD(::grpc::Status, TryGetTaskOutput,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::TaskOutputRequest &request,
               ::armonik::api::grpc::v1::Output *response));
  MOCK_METHOD(::grpc::Status, WaitForAvailability,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::ResultRequest &request,
               ::armonik::api::grpc::v1::submitter::AvailabilityReply *response));
  MOCK_METHOD(::grpc::Status, WaitForCompletion,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::submitter::WaitRequest &request,
               ::armonik::api::grpc::v1::Count *response));
  MOCK_METHOD(::grpc::Status, CancelTasks,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::submitter::TaskFilter &request,
               ::armonik::api::grpc::v1::Empty *response));
  MOCK_METHOD(::grpc::Status, GetTaskStatus,
              (::grpc::ClientContext * context,
               const ::armonik::api::grpc::v1::submitter::GetTaskStatusRequest &request,
               ::armonik::api::grpc::v1::submitter::GetTaskStatusReply *response));
  MOCK_METHOD(::grpc::Status, GetResultStatus,
              (::grpc::ClientContext * context,
               const ::armonik::api::grpc::v1::submitter::GetResultStatusRequest &request,
               ::armonik::api::grpc ::v1::submitter::GetResultStatusReply *response));

  MOCK_METHOD(void, GetServiceConfiguration,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::Empty *request,
               ::armonik::api::grpc::v1::Configuration *response, std::function<void(::grpc::Status)>));
  MOCK_METHOD(void, GetServiceConfiguration,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::Empty *request,
               ::armonik::api::grpc::v1::Configuration *response, ::grpc::ClientUnaryReactor *reactor));
  MOCK_METHOD(void, CreateSession,
              (::grpc::ClientContext * context,
               const ::armonik::api::grpc::v1::submitter::CreateSessionRequest *request,
               ::armonik::api::grpc::v1::submitter::CreateSessionReply *response, std::function<void(::grpc::Status)>));
  MOCK_METHOD(void, CreateSession,
              (::grpc::ClientContext * context,
               const ::armonik::api::grpc::v1::submitter::CreateSessionRequest *request,
               ::armonik::api::grpc::v1::submitter::CreateSessionReply *response, ::grpc::ClientUnaryReactor *reactor));
  MOCK_METHOD(void, CancelSession,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::Session *request,
               ::armonik::api::grpc::v1::Empty *response, std ::function<void(::grpc::Status)>));
  MOCK_METHOD(void, CancelSession,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::Session *request,
               ::armonik::api::grpc::v1::Empty *response, ::grpc::ClientUnaryReactor *reactor));
  MOCK_METHOD(void, CreateSmallTasks,
              (::grpc::ClientContext * context,
               const ::armonik::api::grpc::v1::submitter::CreateSmallTaskRequest *request,
               ::armonik::api::grpc ::v1::submitter::CreateTaskReply *response, std::function<void(::grpc::Status)>));
  MOCK_METHOD(void, CreateSmallTasks,
              (::grpc::ClientContext * context,
               const ::armonik::api::grpc::v1::submitter::CreateSmallTaskRequest *request,
               ::armonik::api::grpc ::v1::submitter::CreateTaskReply *response, ::grpc::ClientUnaryReactor *reactor));
  MOCK_METHOD(void, CreateLargeTasks,
              (::grpc::ClientContext * context, ::armonik::api::grpc::v1::submitter::CreateTaskReply *response,
               ::grpc::ClientWriteReactor<::armonik::api::grpc::v1::submitter::CreateLargeTaskRequest> *reactor));
  MOCK_METHOD(void, ListTasks,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::submitter::TaskFilter *request,
               ::armonik::api::grpc::v1::TaskIdList *response, std::function<void(::grpc::Status)>));
  MOCK_METHOD(void, ListTasks,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::submitter::TaskFilter *request,
               ::armonik::api::grpc::v1::TaskIdList *response, ::grpc::ClientUnaryReactor *reactor));
  MOCK_METHOD(void, ListSessions,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::submitter::SessionFilter *request,
               ::armonik::api::grpc::v1::submitter::SessionIdList *response, std::function<void(::grpc::Status)>));
  MOCK_METHOD(void, ListSessions,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::submitter::SessionFilter *request,
               ::armonik::api::grpc::v1::submitter::SessionIdList *response, ::grpc::ClientUnaryReactor *reactor));
  MOCK_METHOD(void, CountTasks,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::submitter::TaskFilter *request,
               ::armonik::api::grpc::v1::Count *response, std::function<void(::grpc::Status)>));
  MOCK_METHOD(void, CountTasks,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::submitter::TaskFilter *request,
               ::armonik::api::grpc::v1::Count *response, ::grpc::ClientUnaryReactor *reactor));
  MOCK_METHOD(void, TryGetResultStream,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::ResultRequest *request,
               ::grpc::ClientReadReactor<::armonik::api::grpc::v1::submitter::ResultReply> *reactor));
  MOCK_METHOD(void, TryGetTaskOutput,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::TaskOutputRequest *request,
               ::armonik::api::grpc::v1::Output *response, std::function<void(::grpc::Status)>));
  MOCK_METHOD(void, TryGetTaskOutput,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::TaskOutputRequest *request,
               ::armonik::api::grpc::v1::Output *response, ::grpc::ClientUnaryReactor *reactor));
  MOCK_METHOD(void, WaitForAvailability,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::ResultRequest *request,
               ::armonik::api::grpc::v1::submitter::AvailabilityReply *response, std::function<void(::grpc::Status)>));
  MOCK_METHOD(void, WaitForAvailability,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::ResultRequest *request,
               ::armonik::api::grpc::v1::submitter::AvailabilityReply *response, ::grpc::ClientUnaryReactor *reactor));
  MOCK_METHOD(void, WaitForCompletion,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::submitter::WaitRequest *request,
               ::armonik::api::grpc::v1::Count *response, std::function<void(::grpc::Status)>));
  MOCK_METHOD(void, WaitForCompletion,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::submitter::WaitRequest *request,
               ::armonik::api::grpc::v1::Count *response, ::grpc::ClientUnaryReactor *reactor));
  MOCK_METHOD(void, CancelTasks,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::submitter::TaskFilter *request,
               ::armonik::api::grpc::v1::Empty *response, std::function<void(::grpc::Status)>));
  MOCK_METHOD(void, CancelTasks,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::submitter::TaskFilter *request,
               ::armonik::api::grpc::v1::Empty *response, ::grpc::ClientUnaryReactor *reactor));
  MOCK_METHOD(void, GetTaskStatus,
              (::grpc::ClientContext * context,
               const ::armonik::api::grpc::v1::submitter::GetTaskStatusRequest *request,
               ::armonik::api::grpc::v1::submitter::GetTaskStatusReply *response, std::function<void(::grpc::Status)>));
  MOCK_METHOD(void, GetTaskStatus,
              (::grpc::ClientContext * context,
               const ::armonik::api::grpc::v1::submitter::GetTaskStatusRequest *request,
               ::armonik::api::grpc::v1::submitter::GetTaskStatusReply *response, ::grpc::ClientUnaryReactor *reactor));
  MOCK_METHOD(void, GetResultStatus,
              (::grpc::ClientContext * context,
               const ::armonik::api::grpc::v1::submitter::GetResultStatusRequest *request,
               ::armonik::api::grpc ::v1::submitter::GetResultStatusReply *response,
               std::function<void(::grpc::Status)>));
  MOCK_METHOD(void, GetResultStatus,
              (::grpc::ClientContext * context,
               const ::armonik::api::grpc::v1::submitter::GetResultStatusRequest *request,
               ::armonik::api::grpc ::v1::submitter::GetResultStatusReply *response,
               ::grpc::ClientUnaryReactor *reactor));

  MOCK_METHOD(::grpc::ClientAsyncResponseReaderInterface<::armonik::api::grpc::v1::Configuration> *,
              AsyncGetServiceConfigurationRaw,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::Empty &request,
               ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientAsyncResponseReaderInterface<::armonik::api::grpc::v1::Configuration> *,
              PrepareAsyncGetServiceConfigurationRaw,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::Empty &request,
               ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientAsyncResponseReaderInterface<::armonik::api::grpc::v1::submitter::CreateSessionReply> *,
              AsyncCreateSessionRaw,
              (::grpc::ClientContext * context,
               const ::armonik::api::grpc::v1::submitter ::CreateSessionRequest &request, ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientAsyncResponseReaderInterface<::armonik::api::grpc::v1::submitter::CreateSessionReply> *,
              PrepareAsyncCreateSessionRaw,
              (::grpc::ClientContext * context,
               const ::armonik::api::grpc::v1::submitter ::CreateSessionRequest &request, ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientAsyncResponseReaderInterface<::armonik::api::grpc::v1::Empty> *, AsyncCancelSessionRaw,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::Session &request,
               ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientAsyncResponseReaderInterface<::armonik::api::grpc::v1::Empty> *,
              PrepareAsyncCancelSessionRaw,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::Session &request,
               ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientAsyncResponseReaderInterface<::armonik::api::grpc::v1::submitter::CreateTaskReply> *,
              AsyncCreateSmallTasksRaw,
              (::grpc::ClientContext * context,
               const ::armonik::api::grpc::v1::submitter ::CreateSmallTaskRequest &request,
               ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientAsyncResponseReaderInterface<::armonik::api::grpc::v1::submitter::CreateTaskReply> *,
              PrepareAsyncCreateSmallTasksRaw,
              (::grpc::ClientContext * context,
               const ::armonik::api::grpc::v1::submitter ::CreateSmallTaskRequest &request,
               ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientWriterInterface<::armonik::api::grpc::v1::submitter::CreateLargeTaskRequest> *,
              CreateLargeTasksRaw,
              (::grpc::ClientContext * context, ::armonik::api::grpc::v1::submitter::CreateTaskReply *response));
  MOCK_METHOD(::grpc::ClientAsyncWriterInterface<::armonik::api::grpc::v1::submitter::CreateLargeTaskRequest> *,
              AsyncCreateLargeTasksRaw,
              (::grpc::ClientContext * context, ::armonik::api::grpc::v1::submitter::CreateTaskReply *response,
               ::grpc::CompletionQueue *cq, void *tag));
  MOCK_METHOD(::grpc::ClientAsyncWriterInterface<::armonik::api::grpc::v1::submitter::CreateLargeTaskRequest> *,
              PrepareAsyncCreateLargeTasksRaw,
              (::grpc::ClientContext * context, ::armonik::api::grpc::v1::submitter::CreateTaskReply *response,
               ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientAsyncResponseReaderInterface<::armonik::api::grpc::v1::TaskIdList> *, AsyncListTasksRaw,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::submitter ::TaskFilter &request,
               ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientAsyncResponseReaderInterface<::armonik::api::grpc::v1::TaskIdList> *,
              PrepareAsyncListTasksRaw,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::submitter ::TaskFilter &request,
               ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientAsyncResponseReaderInterface<::armonik::api::grpc::v1::submitter::SessionIdList> *,
              AsyncListSessionsRaw,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::submitter ::SessionFilter &request,
               ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientAsyncResponseReaderInterface<::armonik::api::grpc::v1::submitter::SessionIdList> *,
              PrepareAsyncListSessionsRaw,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::submitter ::SessionFilter &request,
               ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientAsyncResponseReaderInterface<::armonik::api::grpc::v1::Count> *, AsyncCountTasksRaw,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::submitter ::TaskFilter &request,
               ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientAsyncResponseReaderInterface<::armonik::api::grpc::v1::Count> *, PrepareAsyncCountTasksRaw,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::submitter ::TaskFilter &request,
               ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientReaderInterface<::armonik::api::grpc::v1::submitter::ResultReply> *, TryGetResultStreamRaw,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::ResultRequest &request));
  MOCK_METHOD(::grpc::ClientAsyncReaderInterface<::armonik::api::grpc::v1::submitter::ResultReply> *,
              AsyncTryGetResultStreamRaw,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::ResultRequest &request,
               ::grpc::CompletionQueue *cq, void *tag));
  MOCK_METHOD(::grpc::ClientAsyncReaderInterface<::armonik::api::grpc::v1::submitter::ResultReply> *,
              PrepareAsyncTryGetResultStreamRaw,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::ResultRequest &request,
               ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientAsyncResponseReaderInterface<::armonik::api::grpc::v1::Output> *, AsyncTryGetTaskOutputRaw,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::TaskOutputRequest &request,
               ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientAsyncResponseReaderInterface<::armonik::api::grpc::v1::Output> *,
              PrepareAsyncTryGetTaskOutputRaw,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::TaskOutputRequest &request,
               ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientAsyncResponseReaderInterface<::armonik::api::grpc::v1::submitter::AvailabilityReply> *,
              AsyncWaitForAvailabilityRaw,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::ResultRequest &request,
               ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientAsyncResponseReaderInterface<::armonik::api::grpc::v1::submitter::AvailabilityReply> *,
              PrepareAsyncWaitForAvailabilityRaw,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::ResultRequest &request,
               ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientAsyncResponseReaderInterface<::armonik::api::grpc::v1::Count> *, AsyncWaitForCompletionRaw,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::submitter ::WaitRequest &request,
               ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientAsyncResponseReaderInterface<::armonik::api::grpc::v1::Count> *,
              PrepareAsyncWaitForCompletionRaw,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::submitter ::WaitRequest &request,
               ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientAsyncResponseReaderInterface<::armonik::api::grpc::v1::Empty> *, AsyncCancelTasksRaw,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::submitter ::TaskFilter &request,
               ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientAsyncResponseReaderInterface<::armonik::api::grpc::v1::Empty> *, PrepareAsyncCancelTasksRaw,
              (::grpc::ClientContext * context, const ::armonik::api::grpc::v1::submitter ::TaskFilter &request,
               ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientAsyncResponseReaderInterface<::armonik::api::grpc::v1::submitter::GetTaskStatusReply> *,
              AsyncGetTaskStatusRaw,
              (::grpc::ClientContext * context,
               const ::armonik::api::grpc::v1::submitter ::GetTaskStatusRequest &request, ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientAsyncResponseReaderInterface<::armonik::api::grpc::v1::submitter::GetTaskStatusReply> *,
              PrepareAsyncGetTaskStatusRaw,
              (::grpc::ClientContext * context,
               const ::armonik::api::grpc::v1::submitter ::GetTaskStatusRequest &request, ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientAsyncResponseReaderInterface<::armonik::api::grpc::v1::submitter::GetResultStatusReply> *,
              AsyncGetResultStatusRaw,
              (::grpc::ClientContext * context,
               const ::armonik::api::grpc::v1::submitter ::GetResultStatusRequest &request,
               ::grpc::CompletionQueue *cq));
  MOCK_METHOD(::grpc::ClientAsyncResponseReaderInterface<::armonik::api::grpc::v1::submitter::GetResultStatusReply> *,
              PrepareAsyncGetResultStatusRaw,
              (::grpc::ClientContext * context,
               const ::armonik::api::grpc::v1::submitter ::GetResultStatusRequest &request,
               ::grpc::CompletionQueue *cq));
  MOCK_METHOD((::grpc::ClientReaderWriterInterface<::armonik::api::grpc::v1::submitter ::WatchResultRequest,
                                                   ::armonik::api::grpc::v1::submitter::WatchResultStream> *),
              WatchResultsRaw, (::grpc::ClientContext * context));
  MOCK_METHOD((::grpc::ClientAsyncReaderWriterInterface<::armonik::api::grpc::v1::submitter::WatchResultRequest,
                                                        ::armonik::api::grpc::v1::submitter::WatchResultStream> *),
              AsyncWatchResultsRaw, (::grpc::ClientContext * context, ::grpc::CompletionQueue *cq, void *tag));
  MOCK_METHOD((::grpc::ClientAsyncReaderWriterInterface<::armonik::api::grpc::v1::submitter::WatchResultRequest,
                                                        ::armonik::api::grpc::v1::submitter::WatchResultStream> *),
              PrepareAsyncWatchResultsRaw, (::grpc::ClientContext * context, ::grpc::CompletionQueue *cq));
};

/**
 * @brief Initializes task options creates channel with server address
 *
 * @param channel The gRPC channel to communicate with the server.
 * @param default_task_options The default task options.
 */
void init(std::shared_ptr<grpc::Channel> &channel, armonik::api::grpc::v1::TaskOptions &task_options);
