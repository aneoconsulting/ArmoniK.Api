#include "submitter/ResultsClient.h"
#include "exceptions/ArmoniKApiException.h"
#include <sstream>

namespace armonik {
namespace api {
namespace client {

std::map<std::string, std::string> ResultsClient::create_results(absl::string_view session_id,
                                                                 const std::vector<std::string> &names) {
  std::map<std::string, std::string> mapping;
  ::grpc::ClientContext context;
  armonik::api::grpc::v1::results::CreateResultsMetaDataRequest results_request;
  armonik::api::grpc::v1::results::CreateResultsMetaDataResponse results_response;

  // Creates the result creation requests
  std::vector<armonik::api::grpc::v1::results::CreateResultsMetaDataRequest_ResultCreate> results_create;
  results_create.reserve(names.size());
  for (auto &&name : names) {
    armonik::api::grpc::v1::results::CreateResultsMetaDataRequest_ResultCreate result_create;
    result_create.set_name(name);
    results_create.push_back(result_create);
  }

  results_request.mutable_results()->Add(results_create.begin(), results_create.end());
  results_request.mutable_session_id()->assign(session_id.data(), session_id.size());

  // Creates the results
  auto status = stub->CreateResultsMetaData(&context, results_request, &results_response);

  if (!status.ok()) {
    std::stringstream message;
    message << "Error: " << status.error_code() << ": " << status.error_message()
            << ". details : " << status.error_details() << std::endl;
    auto str = message.str();
    std::cerr << "Could not create results for submit: " << str << std::endl;
    throw armonik::api::common::exceptions::ArmoniKApiException(str);
  }

  for (auto &&res : results_response.results()) {
    mapping.insert({res.name(), res.result_id()});
  }
  return mapping;
}
void ResultsClient::upload_result_data(const std::string &session_id, const std::string &result_id,
                                       absl::string_view payload) {
  ::grpc::ClientContext context;
  armonik::api::grpc::v1::results::ResultsServiceConfigurationResponse configuration;
  auto status = stub->GetServiceConfiguration(&context, armonik::api::grpc::v1::Empty(), &configuration);
  if (!status.ok()) {
    throw armonik::api::common::exceptions::ArmoniKApiException("Unable to get result configuration : " +
                                                                status.error_message());
  }

  size_t maxChunkSize = configuration.data_chunk_max_size();

  armonik::api::grpc::v1::results::UploadResultDataResponse response;
  // response.set_allocated_result(new armonik::api::grpc::v1::results::ResultRaw());
  ::grpc::ClientContext streamContext;
  auto stream = stub->UploadResultData(&streamContext, &response);
  armonik::api::grpc::v1::results::UploadResultDataRequest request;
  request.mutable_id()->set_session_id(session_id);
  request.mutable_id()->set_result_id(result_id);
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
  status = stream->Finish();
  if (!status.ok()) {
    throw armonik::api::common::exceptions::ArmoniKApiException("Unable to finish upload result " +
                                                                status.error_message());
  }
}

void ResultsClient::delete_results(const std::string &session_id, const std::vector<std::string> &result_ids) {
  if (result_ids.empty()) {
    return;
  }

  ::grpc::ClientContext context;
  armonik::api::grpc::v1::results::DeleteResultsDataRequest request;
  armonik::api::grpc::v1::results::DeleteResultsDataResponse response;
  *request.mutable_session_id() = session_id;
  request.mutable_result_id()->Add(result_ids.begin(), result_ids.end());

  auto status = stub->DeleteResultsData(&context, request, &response);
  if (!status.ok()) {
    throw armonik::api::common::exceptions::ArmoniKApiException("Unable to delete results " + status.error_message());
  }
}

std::vector<armonik::api::grpc::v1::results::ResultRaw>
ResultsClient::list_results(const grpc::v1::results::Filters &filters, int32_t &total, int32_t page, int32_t page_size,
                            const grpc::v1::results::ListResultsRequest::Sort &sort) {
  armonik::api::grpc::v1::results::ListResultsRequest request;
  armonik::api::grpc::v1::results::ListResultsResponse response;

  *request.mutable_filters() = filters;
  *request.mutable_sort() = sort;
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
    return rawResults;
  }
}
armonik::api::grpc::v1::results::ListResultsRequest::Sort ResultsClient::get_default_sort() {
  static armonik::api::grpc::v1::results::ListResultsRequest::Sort sort;
  if (sort.direction() == grpc::v1::sort_direction::SORT_DIRECTION_UNSPECIFIED) {
    sort.set_direction(grpc::v1::sort_direction::SORT_DIRECTION_ASC);
    sort.mutable_field()->mutable_result_raw_field()->set_field(grpc::v1::results::RESULT_RAW_ENUM_FIELD_CREATED_AT);
  }
  return sort;
}

} // namespace client
} // namespace api
} // namespace armonik
