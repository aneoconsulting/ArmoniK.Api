#include "events/EventsClient.h"
#include "events_common.pb.h"
#include "events_service.grpc.pb.h"
#include "exceptions/ArmoniKApiException.h"
#include "objects.pb.h"

using armonik::api::grpc::v1::events::EventSubscriptionRequest;
using armonik::api::grpc::v1::events::EventSubscriptionResponse;
using armonik::api::grpc::v1::result_status::ResultStatus;
using namespace armonik::api::grpc::v1::events;

namespace armonik {
namespace api {
namespace client {

void EventsClient::wait_for_result_availability(std::string session_id, std::vector<std::string> result_ids) {

  ::grpc::ClientContext context;
  EventSubscriptionRequest request;

  EventSubscriptionResponse response;

  armonik::api::grpc::v1::results::Filters filters;
  armonik::api::grpc::v1::results::FilterField filter_field;
  filter_field.mutable_field()->mutable_result_raw_field()->set_field(
      armonik::api::grpc::v1::results::RESULT_RAW_ENUM_FIELD_RESULT_ID);
  filter_field.mutable_filter_string()->set_operator_(grpc::v1::FILTER_STRING_OPERATOR_EQUAL);
  for (auto &&result_id : result_ids) {
    filter_field.mutable_filter_string()->set_value(result_id);
    *filters.mutable_or_()->Add()->mutable_and_()->Add() = filter_field;
  }

  *request.mutable_session_id() = std::move(session_id);
  *request.mutable_results_filters() = filters;
  request.add_returned_events(static_cast<EventsEnum>(EventSubscriptionResponse::UpdateCase::kResultStatusUpdate));
  request.add_returned_events(static_cast<EventsEnum>(EventSubscriptionResponse::UpdateCase::kNewResult));

  auto stream = stub->GetEvents(&context, request);
  if (!stream) {
    throw armonik::api::common::exceptions::ArmoniKApiException("Result has been aborted");
  }

  while (stream->Read(&response)) {
    std::string update_or_new;
    switch (response.update_case()) {
    case EventSubscriptionResponse::UpdateCase::kResultStatusUpdate:
      switch (response.mutable_result_status_update()->status()) {
      case ResultStatus::RESULT_STATUS_COMPLETED:
        update_or_new = response.mutable_result_status_update()->result_id();
        break;
      case ResultStatus::RESULT_STATUS_ABORTED:
        throw armonik::api::common::exceptions::ArmoniKApiException("Result has been aborted");
      default:
        break;
      }
      break;
    case EventSubscriptionResponse::UpdateCase::kNewResult:
      switch (response.mutable_new_result()->status()) {
      case ResultStatus::RESULT_STATUS_COMPLETED:
        update_or_new = response.mutable_new_result()->result_id();
        break;
      case ResultStatus::RESULT_STATUS_ABORTED:
        throw armonik::api::common::exceptions::ArmoniKApiException("Result has been aborted");
      default:
        break;
      }
      break;
    default:
      break;
    }
    if (!update_or_new.empty()) {
      result_ids.erase(std::remove(result_ids.begin(), result_ids.end(), update_or_new), result_ids.end());
      if (result_ids.empty()) {
        break;
      }
    }
  }
}

} // namespace client
} // namespace api
} // namespace armonik
