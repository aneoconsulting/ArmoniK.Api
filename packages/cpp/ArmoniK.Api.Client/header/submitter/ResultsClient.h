#ifndef ARMONIK_API_RESULTSCLIENT_H
#define ARMONIK_API_RESULTSCLIENT_H

#include <results_service.grpc.pb.h>

namespace armonik {
namespace api {
namespace client {
class ResultsClient {
public:
  explicit ResultsClient(std::unique_ptr<armonik::api::grpc::v1::results::Results::Stub> stub)
      : stub(std::move(stub)) {}

  std::map<std::string, std::string> create_results(absl::string_view session_id,
                                                    const std::vector<std::string> &names);
  void upload_result_data(const std::string &session_id, const std::string &result_id, absl::string_view payload);

  /**
   * Deletes the results data
   * @param session_id Session id
   * @param result_ids Result ids
   */
  void delete_results(const std::string &session_id, const std::vector<std::string> &result_ids);

  /**
   * List the results
   * @param filters Filter to be used
   * @param total Output for the total of results available from this request (used for pagination)
   * @param page Page to request, use -1 to get all pages.
   * @param page_size Size of the requested page, ignored if page is -1
   * @param sort How the results are sorted, ascending creation date by default
   * @return List of results
   *
   * @note If the results corresponding to the filters change while this call is going for page==-1,
   * or between calls, then the returned values may not be consistent depending on the sorting used.
   * For example, a sort by ascending creation date (the default) will be stable if results are being created in between
   * requests.
   */
  std::vector<armonik::api::grpc::v1::results::ResultRaw>
  list_results(const armonik::api::grpc::v1::results::Filters &filters, int32_t &total, int32_t page = -1,
               int32_t page_size = 500,
               const armonik::api::grpc::v1::results::ListResultsRequest::Sort &sort = get_default_sort());

private:
  std::unique_ptr<armonik::api::grpc::v1::results::Results::Stub> stub;
  static armonik::api::grpc::v1::results::ListResultsRequest::Sort get_default_sort();
};
} // namespace client
} // namespace api
} // namespace armonik

#endif // ARMONIK_API_RESULTSCLIENT_H
