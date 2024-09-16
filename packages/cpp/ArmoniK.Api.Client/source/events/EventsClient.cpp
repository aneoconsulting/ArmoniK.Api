#include <utility>

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
  // context.set_deadline(std::chrono::system_clock::now() + std::chrono::seconds(30));
  EventSubscriptionRequest request;

  EventSubscriptionResponse response;

  armonik::api::grpc::v1::results::Filters filters;

  for (auto &&result_id : result_ids) {
    armonik::api::grpc::v1::results::FilterField filter_field;
    filter_field.mutable_field()->mutable_result_raw_field()->set_field(
        armonik::api::grpc::v1::results::RESULT_RAW_ENUM_FIELD_RESULT_ID);
    filter_field.mutable_filter_string()->set_operator_(grpc::v1::FILTER_STRING_OPERATOR_EQUAL);
    filter_field.mutable_filter_string()->set_value(result_id);
    *filters.mutable_or_()->Add()->mutable_and_()->Add() = filter_field;
  }

  *request.mutable_session_id() = std::move(session_id);
  *request.mutable_results_filters() = filters;
  request.mutable_returned_events()->Add(EventsEnum::EVENTS_ENUM_RESULT_STATUS_UPDATE);
  request.mutable_returned_events()->Add(EventsEnum::EVENTS_ENUM_NEW_RESULT);
  // request.add_returned_events(static_cast<EventsEnum>(EventSubscriptionResponse::UpdateCase::kResultStatusUpdate));
  // request.add_returned_events(static_cast<EventsEnum>(EventSubscriptionResponse::UpdateCase::kNewResult));

  for (const auto &event : request.returned_events()) {
    std::cout << "Captured event: " << static_cast<int>(event) << std::endl;
  }

  auto stream = stub->GetEvents(&context, request);
  if (!stream) {
    throw armonik::api::common::exceptions::ArmoniKApiException("Could not get events ");
  }

  while (stream->Read(&response)) {
    try {
      std::cout << response.DebugString() << std::endl;
      std::string update_or_new;
      if (response.update_case() == EventSubscriptionResponse::UpdateCase::kResultStatusUpdate) {
        std::cout << "Result update case completed " << std::endl;
        if (response.mutable_result_status_update()->status() == ResultStatus::RESULT_STATUS_COMPLETED) {
          update_or_new = response.mutable_result_status_update()->result_id();
          std::cout << "Result update case completed " << std::endl;
          printf("f Result update case completed  ");
          // if (!update_or_new.empty()) {
          result_ids.erase(
              std::remove(result_ids.begin(), result_ids.end(), response.mutable_result_status_update()->result_id()),
              result_ids.end());
          if (result_ids.empty()) {
            break;
          }
          //}
        }
        if (response.mutable_result_status_update()->status() == ResultStatus::RESULT_STATUS_ABORTED) {
          throw armonik::api::common::exceptions::ArmoniKApiException(
              "Result " + response.mutable_result_status_update()->result_id() + " has been aborted");
        }
      }
      if (response.update_case() == EventSubscriptionResponse::UpdateCase::kNewResult) {
        if (response.mutable_new_result()->status() == ResultStatus::RESULT_STATUS_COMPLETED) {
          update_or_new = response.mutable_new_result()->result_id();
          std::cout << "New result case completed " << std::endl;
          // if (!update_or_new.empty()) {
          result_ids.erase(
              std::remove(result_ids.begin(), result_ids.end(), response.mutable_new_result()->result_id()),
              result_ids.end());
          if (result_ids.empty()) {
            break;
          }
          // }
          if (response.mutable_new_result()->status() == ResultStatus::RESULT_STATUS_ABORTED) {
            throw armonik::api::common::exceptions::ArmoniKApiException(
                "Result " + response.mutable_new_result()->result_id() + " has been aborted");
          }
        }
      }
    } catch (const std::exception &e) {
      std::cerr << "Error while reading event response: " << e.what() << std::endl;
    }
  }
}

} // namespace client
} // namespace api
} // namespace armonik
