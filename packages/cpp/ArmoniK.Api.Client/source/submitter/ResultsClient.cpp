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
  size_t offset = 0;

  while (offset < payload.size()) {
    size_t chunkSize = std::min(maxChunkSize, payload.size() - offset);
    auto chunk = payload.substr(offset, chunkSize);
    request.mutable_data_chunk()->assign(chunk.data(), chunk.size());
    if (!stream->Write(request)) {
      throw armonik::api::common::exceptions::ArmoniKApiException("Unable to continue upload result");
    }
    offset += chunkSize;
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
} // namespace client
} // namespace api
} // namespace armonik