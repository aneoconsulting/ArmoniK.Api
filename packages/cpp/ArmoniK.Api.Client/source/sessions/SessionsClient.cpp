#include "sessions/SessionsClient.h"
#include "exceptions/ArmoniKApiException.h"
#include "sessions_common.pb.h"
#include "sessions_service.grpc.pb.h"
#include <utility>

using namespace armonik::api::grpc::v1::sessions;

static ListSessionsRequest::Sort get_default_sort() {
  ListSessionsRequest::Sort sort;
  sort.mutable_field()->mutable_session_raw_field()->set_field(
      armonik::api::grpc::v1::sessions::SESSION_RAW_ENUM_FIELD_CREATED_AT);
  sort.set_direction(armonik::api::grpc::v1::sort_direction::SORT_DIRECTION_ASC);
  return sort;
}
const ListSessionsRequest::Sort armonik::api::client::SessionsClient::default_sort = get_default_sort();

std::string
armonik::api::client::SessionsClient::create_session(armonik::api::grpc::v1::TaskOptions default_task_options,
                                                     const std::vector<std::string> &partitions) {
  ::grpc::ClientContext context;
  CreateSessionRequest request;
  CreateSessionReply response;

  *request.mutable_default_task_option() = std::move(default_task_options);
  request.mutable_partition_ids()->Add(partitions.begin(), partitions.end());

  auto status = stub->CreateSession(&context, request, &response);
  if (!status.ok()) {
    throw armonik::api::common::exceptions::ArmoniKApiException("Could not create session : " + status.error_message());
  }
  return std::move(*response.mutable_session_id());
}

SessionRaw armonik::api::client::SessionsClient::get_session(std::string session_id) {
  ::grpc::ClientContext context;
  GetSessionRequest request;
  GetSessionResponse response;

  request.set_session_id(std::move(session_id));

  auto status = stub->GetSession(&context, request, &response);
  if (!status.ok()) {
    throw armonik::api::common::exceptions::ArmoniKApiException("Could not get session : " + status.error_message());
  }
  return std::move(*response.mutable_session());
}

SessionRaw armonik::api::client::SessionsClient::cancel_session(std::string session_id) {
  ::grpc::ClientContext context;
  CancelSessionRequest request;
  CancelSessionResponse response;

  request.set_session_id(std::move(session_id));
  auto status = stub->CancelSession(&context, request, &response);
  if (!status.ok()) {
    throw armonik::api::common::exceptions::ArmoniKApiException("Could not cancel session : " + status.error_message());
  }
  return std::move(*response.mutable_session());
}

std::vector<armonik::api::grpc::v1::sessions::SessionRaw>
armonik::api::client::SessionsClient::list_sessions(armonik::api::grpc::v1::sessions::Filters filters, int32_t &total,
                                                    int32_t page, int32_t page_size,
                                                    armonik::api::grpc::v1::sessions::ListSessionsRequest::Sort sort) {
  ListSessionsRequest request;
  ListSessionsResponse response;

  *request.mutable_filters() = std::move(filters);
  *request.mutable_sort() = std::move(sort);
  request.set_page_size(page_size);

  if (page >= 0) {
    request.set_page(page);
    ::grpc::ClientContext context;
    auto status = stub->ListSessions(&context, request, &response);
    if (!status.ok()) {
      throw armonik::api::common::exceptions::ArmoniKApiException("Unable to list sessions : " +
                                                                  status.error_message());
    }
    total = response.total();
    return {response.sessions().begin(), response.sessions().end()};
  } else {
    std::vector<SessionRaw> rawSessions;
    int current_page = 0;
    do {
      request.set_page(current_page);
      ::grpc::ClientContext context;
      auto status = stub->ListSessions(&context, request, &response);
      if (!status.ok()) {
        throw armonik::api::common::exceptions::ArmoniKApiException("Unable to list sessions : " +
                                                                    status.error_message());
      }
      // Append only the additional sessions
      // If the current_page is a re-request, this will add only the new information
      rawSessions.insert(rawSessions.end(),
                         response.sessions().begin() + ((int32_t)rawSessions.size() - current_page * page_size),
                         response.sessions().end());
      if (response.sessions_size() >= page_size) {
        ++current_page;
      }

      response.clear_sessions();
    } while ((int32_t)rawSessions.size() < response.total());
    total = response.total();
    // NRVO
    return rawSessions;
  }
}

SessionRaw armonik::api::client::SessionsClient::pause_session(std::string session_id) {
  ::grpc::ClientContext context;
  PauseSessionRequest request;
  PauseSessionResponse response;

  request.set_session_id(std::move(session_id));
  auto status = stub->PauseSession(&context, request, &response);
  if (!status.ok()) {
    throw armonik::api::common::exceptions::ArmoniKApiException("Could not pause session : " + status.error_message());
  }

  return std::move(*response.mutable_session());
}

armonik::api::grpc::v1::sessions::SessionRaw
armonik::api::client::SessionsClient::resume_session(std::string session_id) {
  ::grpc::ClientContext context;
  ResumeSessionRequest request;
  ResumeSessionResponse response;

  request.set_session_id(std::move(session_id));
  auto status = stub->ResumeSession(&context, request, &response);
  if (!status.ok()) {
    throw armonik::api::common::exceptions::ArmoniKApiException("Could not resume session : " + status.error_message());
  }

  return std::move(*response.mutable_session());
}

SessionRaw armonik::api::client::SessionsClient::purge_session(std::string session_id) {
  ::grpc::ClientContext context;
  PurgeSessionRequest request;
  PurgeSessionResponse response;

  request.set_session_id(std::move(session_id));
  auto status = stub->PurgeSession(&context, request, &response);
  if (!status.ok()) {
    throw armonik::api::common::exceptions::ArmoniKApiException("Could not purge session : " + status.error_message());
  }

  return std::move(*response.mutable_session());
}

SessionRaw armonik::api::client::SessionsClient::delete_session(std::string session_id) {
  ::grpc::ClientContext context;
  DeleteSessionRequest request;
  DeleteSessionResponse response;

  request.set_session_id(std::move(session_id));
  auto status = stub->DeleteSession(&context, request, &response);
  if (!status.ok()) {
    throw armonik::api::common::exceptions::ArmoniKApiException("Could not delete session : " + status.error_message());
  }

  return std::move(*response.mutable_session());
}

SessionRaw armonik::api::client::SessionsClient::stop_submission_session(std::string session_id, bool client,
                                                                         bool worker) {
  ::grpc::ClientContext context;
  StopSubmissionRequest request;
  StopSubmissionResponse response;

  request.set_session_id(std::move(session_id));
  request.set_client(client);
  request.set_worker(worker);
  auto status = stub->StopSubmission(&context, request, &response);
  if (!status.ok()) {
    throw armonik::api::common::exceptions::ArmoniKApiException("Could not stop submission session : " +
                                                                status.error_message());
  }

  return std::move(*response.mutable_session());
}

SessionRaw armonik::api::client::SessionsClient::close_session(std::string session_id) {
  ::grpc::ClientContext context;
  CloseSessionRequest request;
  CloseSessionResponse response;

  request.set_session_id(std::move(session_id));
  auto status = stub->CloseSession(&context, request, &response);
  if (!status.ok()) {
    throw armonik::api::common::exceptions::ArmoniKApiException("Could not close session : " + status.error_message());
  }

  return std::move(*response.mutable_session());
}
