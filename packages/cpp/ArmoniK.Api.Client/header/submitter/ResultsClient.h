#ifndef ARMONIK_API_RESULTSCLIENT_H
#define ARMONIK_API_RESULTSCLIENT_H

#include <results_service.grpc.pb.h>

namespace armonik::api::client {
class ResultsClient {
public:
  explicit ResultsClient(std::unique_ptr<armonik::api::grpc::v1::results::Results::Stub> stub)
      : stub(std::move(stub)) {}

  std::map<std::string, std::string> create_results(std::string_view session_id, const std::vector<std::string> &names);
  void upload_result_data(const std::string &session_id, const std::string &result_id, std::string_view payload);

private:
  std::unique_ptr<armonik::api::grpc::v1::results::Results::Stub> stub;
};
} // namespace armonik::api::client

#endif // ARMONIK_API_RESULTSCLIENT_H
