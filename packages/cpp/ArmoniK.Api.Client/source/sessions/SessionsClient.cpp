#include "sessions/SessionsClient.h"
#include "exceptions/ArmoniKApiException.h"

armonik::api::grpc::v1::sessions::ListSessionsRequest::Sort get_default_sort() {
  armonik::api::grpc::v1::sessions::ListSessionsRequest::Sort sort;
  sort.mutable_field()->mutable_session_raw_field()->set_field(
      armonik::api::grpc::v1::sessions::SESSION_RAW_ENUM_FIELD_CREATED_AT);
  sort.set_direction(armonik::api::grpc::v1::sort_direction::SORT_DIRECTION_ASC);
  return sort;
}
const armonik::api::grpc::v1::sessions::ListSessionsRequest::Sort armonik::api::client::SessionsClient::default_sort =
    get_default_sort();

armonik::api::client::SessionsClient::SessionsClient(
    std::unique_ptr<armonik::api::grpc::v1::sessions::Sessions::StubInterface> stub)
    : stub(std::move(stub)) {}

std::string
armonik::api::client::SessionsClient::create_session(const armonik::api::grpc::v1::TaskOptions &default_task_options,
                                                     const std::vector<std::string> &partitions) {
  ::grpc::ClientContext context;
  armonik::api::grpc::v1::sessions::CreateSessionRequest request;
  armonik::api::grpc::v1::sessions::CreateSessionReply response;

  *request.mutable_default_task_option() = default_task_options;
  request.mutable_partition_ids()->Add(partitions.begin(), partitions.end());

  auto status = stub->CreateSession(&context, request, &response);
  if (!status.ok()) {
    throw armonik::api::common::exceptions::ArmoniKApiException("Could not create session : " + status.error_message());
  }
  return response.session_id();
}

armonik::api::grpc::v1::sessions::SessionRaw
armonik::api::client::SessionsClient::get_session(const std::string &session_id) {
  ::grpc::ClientContext context;
  armonik::api::grpc::v1::sessions::GetSessionRequest request;
  armonik::api::grpc::v1::sessions::GetSessionResponse response;

  request.set_session_id(session_id);

  auto status = stub->GetSession(&context, request, &response);
  if (!status.ok()) {
    throw armonik::api::common::exceptions::ArmoniKApiException("Could not get session : " + status.error_message());
  }
  return response.session();
}

armonik::api::grpc::v1::sessions::SessionRaw
armonik::api::client::SessionsClient::cancel_session(const std::string &session_id) {
  ::grpc::ClientContext context;
  armonik::api::grpc::v1::sessions::CancelSessionRequest request;
  armonik::api::grpc::v1::sessions::CancelSessionResponse response;

  request.set_session_id(session_id);
  auto status = stub->CancelSession(&context, request, &response);
  if (!status.ok()) {
    throw armonik::api::common::exceptions::ArmoniKApiException("Could not cancel session : " + status.error_message());
  }
  return response.session();
}

std::vector<armonik::api::grpc::v1::sessions::SessionRaw> armonik::api::client::SessionsClient::list_sessions(
    const armonik::api::grpc::v1::sessions::Filters &filters, int32_t &total, int32_t page, int32_t page_size,
    const armonik::api::grpc::v1::sessions::ListSessionsRequest::Sort &sort) {
  armonik::api::grpc::v1::sessions::ListSessionsRequest request;
  armonik::api::grpc::v1::sessions::ListSessionsResponse response;

  *request.mutable_filters() = filters;
  *request.mutable_sort() = sort;
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
    std::vector<armonik::api::grpc::v1::sessions::SessionRaw> rawSessions;
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
    return rawSessions;
  }
}
