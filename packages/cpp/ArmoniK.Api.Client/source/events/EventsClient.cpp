#include "events/EventsClient.h"
#include "events_common.pb.h"
#include "events_service.grpc.pb.h"
#include "exceptions/ArmoniKApiException.h"
#include "objects.pb.h"
#include "results_service.grpc.pb.h"

namespace armonik {
namespace api {
namespace client {

void EventsClient::wait_for_result_availability(std::string session_id, std::vector<std::string> result_ids) {

  ::grpc::ClientContext context;
  armonik::api::grpc::v1::events::EventSubscriptionRequest request;

  armonik::api::grpc::v1::events::EventSubscriptionResponse response;

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
  request.add_returned_events(static_cast<armonik::api::grpc::v1::events::EventsEnum>(
      armonik::api::grpc::v1::events::EventSubscriptionResponse::UpdateCase::kResultStatusUpdate));
  request.add_returned_events(static_cast<armonik::api::grpc::v1::events::EventsEnum>(
      armonik::api::grpc::v1::events::EventSubscriptionResponse::UpdateCase::kNewResult));

  auto stream = stub->GetEvents(&context, request);
  if (!stream) {
    throw armonik::api::common::exceptions::ArmoniKApiException("Result has been aborted");
  }

  while (stream->Read(&response)) {
    if (response.update_case() ==
        armonik::api::grpc::v1::events::EventSubscriptionResponse::UpdateCase::kResultStatusUpdate) {
      if (response.mutable_result_status_update()->status() ==
          armonik::api::grpc::v1::result_status::ResultStatus::RESULT_STATUS_COMPLETED) {
        result_ids.erase(
            std::remove(result_ids.begin(), result_ids.end(), response.mutable_result_status_update()->result_id()),
            result_ids.end());
        if (result_ids.empty()) {
          break;
        }
      }

      if (response.mutable_result_status_update()->status() ==
          armonik::api::grpc::v1::result_status::ResultStatus::RESULT_STATUS_ABORTED) {
        throw armonik::api::common::exceptions::ArmoniKApiException("Result has been aborted");
      }
    }

    if (response.update_case() == armonik::api::grpc::v1::events::EventSubscriptionResponse::UpdateCase::kNewResult) {
      if (response.mutable_new_result()->status() ==
          armonik::api::grpc::v1::result_status::ResultStatus::RESULT_STATUS_COMPLETED) {
        result_ids.erase(std::remove(result_ids.begin(), result_ids.end(), response.mutable_new_result()->result_id()),
                         result_ids.end());
        if (result_ids.empty()) {
          break;
        }
      }

      if (response.mutable_new_result()->status() ==
          armonik::api::grpc::v1::result_status::ResultStatus::RESULT_STATUS_ABORTED) {
        throw armonik::api::common::exceptions::ArmoniKApiException("Result has been aborted");
      }
    }
  }
}

} // namespace client
} // namespace api
} // namespace armonik
