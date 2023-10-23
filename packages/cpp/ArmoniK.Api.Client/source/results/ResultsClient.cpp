#include "results/ResultsClient.h"
#include "exceptions/ArmoniKApiException.h"
#include <sstream>

namespace armonik {
namespace api {
namespace client {

std::map<std::string, std::string> ResultsClient::create_results(absl::string_view session_id,
                                                                 const std::vector<std::string> &names) {
  return create_results_metadata(std::string(session_id), names);
}

std::map<std::string, std::string> ResultsClient::create_results_metadata(std::string session_id,
                                                                          const std::vector<std::string> &names) {
  std::map<std::string, std::string> mapping;
  ::grpc::ClientContext context;
  armonik::api::grpc::v1::results::CreateResultsMetaDataRequest request;
  armonik::api::grpc::v1::results::CreateResultsMetaDataResponse response;

  request.set_session_id(std::move(session_id));
  request.mutable_results()->Reserve((int)names.size());
  for (auto &&name : names) {
    *request.mutable_results()->Add()->mutable_name() = name;
  }

  // Creates the results
  auto status = stub->CreateResultsMetaData(&context, request, &response);

  if (!status.ok()) {
    std::stringstream message;
    message << "Error creating results metadata: " << status.error_code() << ": " << status.error_message()
            << ". details : " << status.error_details() << std::endl;
    auto str = message.str();
    throw armonik::api::common::exceptions::ArmoniKApiException(str);
  }

  for (auto &&res : response.results()) {
    mapping.insert({res.name(), res.result_id()});
  }
  return mapping;
}

void ResultsClient::upload_result_data(std::string session_id, std::string result_id, absl::string_view payload) {

  size_t maxChunkSize = get_service_configuration().data_chunk_max_size;

  armonik::api::grpc::v1::results::UploadResultDataResponse response;
  ::grpc::ClientContext context;
  auto stream = stub->UploadResultData(&context, &response);
  armonik::api::grpc::v1::results::UploadResultDataRequest request;
  request.mutable_id()->set_session_id(std::move(session_id));
  request.mutable_id()->set_result_id(std::move(result_id));
  stream->Write(request);

  while (!payload.empty()) {
    auto chunk = payload.substr(0, maxChunkSize);
    request.mutable_data_chunk()->assign(chunk.data(), chunk.size());
    if (!stream->Write(request)) {
      throw armonik::api::common::exceptions::ArmoniKApiException("Unable to continue upload result");
    }
    payload = payload.substr(chunk.size());
  }

  if (!stream->WritesDone()) {
    throw armonik::api::common::exceptions::ArmoniKApiException("Unable to upload result");
  }
  auto status = stream->Finish();
  if (!status.ok()) {
    throw armonik::api::common::exceptions::ArmoniKApiException("Unable to finish upload result " +
                                                                status.error_message());
  }
}

void ResultsClient::delete_results(const std::string &session_id, const std::vector<std::string> &result_ids) {
  delete_results_data(session_id, result_ids);
}

void ResultsClient::delete_results_data(std::string session_id, const std::vector<std::string> &result_ids) {
  if (result_ids.empty()) {
    return;
  }

  ::grpc::ClientContext context;
  armonik::api::grpc::v1::results::DeleteResultsDataRequest request;
  armonik::api::grpc::v1::results::DeleteResultsDataResponse response;
  *request.mutable_session_id() = std::move(session_id);
  request.mutable_result_id()->Add(result_ids.begin(), result_ids.end());

  auto status = stub->DeleteResultsData(&context, request, &response);
  if (!status.ok()) {
    throw armonik::api::common::exceptions::ArmoniKApiException("Unable to delete results " + status.error_message());
  }
}

std::vector<armonik::api::grpc::v1::results::ResultRaw>
ResultsClient::list_results(grpc::v1::results::Filters filters, int32_t &total, int32_t page, int32_t page_size,
                            grpc::v1::results::ListResultsRequest::Sort sort) {
  armonik::api::grpc::v1::results::ListResultsRequest request;
  armonik::api::grpc::v1::results::ListResultsResponse response;

  *request.mutable_filters() = std::move(filters);
  *request.mutable_sort() = std::move(sort);
  request.set_page_size(page_size);

  if (page >= 0) {
    request.set_page(page);
    ::grpc::ClientContext context;
    auto status = stub->ListResults(&context, request, &response);
    if (!status.ok()) {
      throw armonik::api::common::exceptions::ArmoniKApiException("Unable to list results " + status.error_message());
    }
    total = response.total();
    return {response.results().begin(), response.results().end()};
  } else {
    std::vector<armonik::api::grpc::v1::results::ResultRaw> rawResults;
    int current_page = 0;
    do {
      request.set_page(current_page);
      ::grpc::ClientContext context;
      auto status = stub->ListResults(&context, request, &response);
      if (!status.ok()) {
        throw armonik::api::common::exceptions::ArmoniKApiException("Unable to list results " + status.error_message());
      }
      // Append only the additional results
      // If the current_page is a re-request, this will add only the new information
      rawResults.insert(rawResults.end(),
                        response.results().begin() + ((int32_t)rawResults.size() - current_page * page_size),
                        response.results().end());
      if (response.results_size() >= page_size) {
        ++current_page;
      }

      response.clear_results();
    } while ((int32_t)rawResults.size() < response.total());
    total = response.total();
    return rawResults;
  }
}

std::map<std::string, std::string>
ResultsClient::send_create_results(const grpc::v1::results::CreateResultsRequest &request) {
  ::grpc::ClientContext context;
  armonik::api::grpc::v1::results::CreateResultsResponse response;

  auto status = stub->CreateResults(&context, request, &response);

  if (!status.ok()) {
    std::stringstream message;
    message << "Error creating results with data: " << status.error_code() << ": " << status.error_message()
            << ". details : " << status.error_details() << std::endl;
    auto str = message.str();
    throw armonik::api::common::exceptions::ArmoniKApiException(str);
  }

  std::map<std::string, std::string> mapping;
  for (auto &&res : response.results()) {
    mapping.insert({res.name(), res.result_id()});
  }
  return mapping;
}
armonik::api::grpc::v1::results::ResultRaw ResultsClient::get_result(std::string result_id) {
  ::grpc::ClientContext context;
  armonik::api::grpc::v1::results::GetResultRequest request;
  armonik::api::grpc::v1::results::GetResultResponse response;
  *request.mutable_result_id() = std::move(result_id);

  auto status = stub->GetResult(&context, request, &response);

  if (!status.ok()) {
    std::stringstream message;
    message << "Error getting result: " << status.error_code() << ": " << status.error_message()
            << ". details : " << status.error_details() << std::endl;
    auto str = message.str();
    throw armonik::api::common::exceptions::ArmoniKApiException(str);
  }

  return response.result();
}

std::map<std::string, std::string> ResultsClient::get_owner_task_id(std::string session_id,
                                                                    std::vector<std::string> result_ids) {
  ::grpc::ClientContext context;
  armonik::api::grpc::v1::results::GetOwnerTaskIdRequest request;
  armonik::api::grpc::v1::results::GetOwnerTaskIdResponse response;

  *request.mutable_session_id() = std::move(session_id);
  request.mutable_result_id()->Add(result_ids.begin(), result_ids.end());

  auto status = stub->GetOwnerTaskId(&context, request, &response);

  if (!status.ok()) {
    std::stringstream message;
    message << "Error getting owner task id: " << status.error_code() << ": " << status.error_message()
            << ". details : " << status.error_details() << std::endl;
    auto str = message.str();
    throw armonik::api::common::exceptions::ArmoniKApiException(str);
  }

  std::map<std::string, std::string> mapping;
  for (auto &&rid_tid : *response.mutable_result_task()) {
    mapping[std::move(*rid_tid.mutable_task_id())] = std::move(*rid_tid.mutable_result_id());
  }
  return mapping;
}

std::string ResultsClient::download_result_data(std::string session_id, std::string result_id) {
  ::grpc::ClientContext context;
  armonik::api::grpc::v1::results::DownloadResultDataRequest request;
  armonik::api::grpc::v1::results::DownloadResultDataResponse response;

  *request.mutable_session_id() = std::move(session_id);
  *request.mutable_result_id() = std::move(result_id);

  std::string received_data;

  auto stream = stub->DownloadResultData(&context, request);
  while (stream->Read(&response)) {
    received_data.append(response.data_chunk());
  }
  auto status = stream->Finish();
  if (!status.ok()) {
    std::stringstream message;
    message << "Error downloading result data: " << status.error_code() << ": " << status.error_message()
            << ". details : " << status.error_details() << std::endl;
    auto str = message.str();
    throw armonik::api::common::exceptions::ArmoniKApiException(str);
  }

  return received_data;
}

ResultsClient::Configuration ResultsClient::get_service_configuration() {
  ::grpc::ClientContext context;
  armonik::api::grpc::v1::Empty request;
  armonik::api::grpc::v1::results::ResultsServiceConfigurationResponse response;

  auto status = stub->GetServiceConfiguration(&context, request, &response);
  if (!status.ok()) {
    std::stringstream message;
    message << "Error getting result service configuration: " << status.error_code() << ": " << status.error_message()
            << ". details : " << status.error_details() << std::endl;
    auto str = message.str();
    throw armonik::api::common::exceptions::ArmoniKApiException(str);
  }

  return {response.data_chunk_max_size()};
}

armonik::api::grpc::v1::results::ListResultsRequest::Sort ResultsClient::default_sort() {
  armonik::api::grpc::v1::results::ListResultsRequest::Sort sort;
  sort.set_direction(grpc::v1::sort_direction::SORT_DIRECTION_ASC);
  sort.mutable_field()->mutable_result_raw_field()->set_field(grpc::v1::results::RESULT_RAW_ENUM_FIELD_CREATED_AT);
  return sort;
}

} // namespace client
} // namespace api
} // namespace armonik
