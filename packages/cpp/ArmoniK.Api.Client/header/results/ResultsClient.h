#pragma once

#include "results_service.grpc.pb.h"

namespace armonik {
namespace api {
namespace client {

namespace {

template <typename T>
struct is_result_create : std::conditional<std::is_same<T, std::pair<const std::string, std::string>>::value ||
                                               std::is_same<T, std::pair<std::string, std::string>>::value ||
                                               std::is_same<T, std::pair<std::string, absl::string_view>>::value,
                                           std::true_type, std::false_type>::type {};

} // namespace

class ResultsClient {
public:
  struct Configuration {
    int32_t data_chunk_max_size;
  };

public:
  explicit ResultsClient(std::unique_ptr<armonik::api::grpc::v1::results::Results::StubInterface> stub)
      : stub(std::move(stub)) {}

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
  list_results(armonik::api::grpc::v1::results::Filters filters, int32_t &total, int32_t page = -1,
               int32_t page_size = 500, armonik::api::grpc::v1::results::ListResultsRequest::Sort sort = default_sort);

  /**
   * Get a result by id
   * @param result_id Result id
   * @return Result information
   */
  armonik::api::grpc::v1::results::ResultRaw get_result(std::string result_id);

  /**
   * Get the ids of the tasks that should produce the results
   * @param session_id Session Id
   * @param result_ids List of result ids
   * @return Mapping between a result_id the and corresponding owner task
   */
  std::map<std::string, std::string> get_owner_task_id(std::string session_id, std::vector<std::string> result_ids);

  /**
   * Create the metadata of multiple results at once
   * Data have to be uploaded separately
   * @param session_id Session id
   * @param names Names of the results to be created
   * @return Map matching the names to their result_id
   */
  std::map<std::string, std::string> create_results_metadata(std::string session_id,
                                                             const std::vector<std::string> &names);
  [[deprecated("Use the create_results_metadata method instead")]] std::map<std::string, std::string>
  create_results(absl::string_view session_id, const std::vector<std::string> &names);

  /**
   * Create results with data included in the request
   *
   * @param session_id Session id
   * @param results_to_create Vector of pairs made with : result name, result data
   * @return Map matching the names to their result_id
   */
  std::map<std::string, std::string>
  create_results(std::string session_id, const std::vector<std::pair<std::string, std::string>> &results_to_create);

  /**
   * Create results with data included in the request
   * @param session_id Session id
   * @param results_to_create Map associating the result's name to their data
   * @return Map matching the names to their result_id
   */
  std::map<std::string, std::string> create_results(std::string session_id,
                                                    const std::map<std::string, std::string> &results_to_create);

  /**
   * Create results with data included in the request
   * @param session_id Session id
   * @param results_to_create Map associating the result's name to their data
   * @return Map matching the names to their result_id
   */
  std::map<std::string, std::string>
  create_results(std::string session_id, const std::unordered_map<std::string, std::string> &results_to_create);

  /**
   * Create results with data included in the request
   * @tparam pair_iterator Iterator of string pairs each made with : result name, result data
   * @tparam pair_value_type String pair type made of : result name, result data
   * @param session_id Session id
   * @param begin Beginning of the iterator
   * @param end End of the iterator
   * @return Map matching the names to their result_id
   */
  template <class pair_iterator, typename pair_value_type = typename std::enable_if<
                                     is_result_create<typename std::iterator_traits<pair_iterator>::value_type>::value,
                                     typename std::iterator_traits<pair_iterator>::value_type>::type>
  std::map<std::string, std::string> create_results(std::string session_id, const pair_iterator &begin,
                                                    const pair_iterator &end) {
    armonik::api::grpc::v1::results::CreateResultsRequest request;

    request.set_session_id(std::move(session_id));
    for (auto t = begin; t != end; t++) {
      auto result_create = request.mutable_results()->Add();
      *result_create->mutable_name() = static_cast<pair_value_type>(*t).first;
      result_create->mutable_data()->assign(static_cast<pair_value_type>(*t).second.data(),
                                            static_cast<pair_value_type>(*t).second.length());
    }

    return send_create_results(request);
  }

  /**
   * Upload data for result
   * @param session_id Session id
   * @param result_id Result Id
   * @param payload
   */
  void upload_result_data(std::string session_id, std::string result_id, absl::string_view payload);

  /**
   * Retrieve data from a result
   * @param session_id Session id
   * @param result_id Result Id
   * @return Result data
   */
  std::string download_result_data(std::string session_id, std::string result_id);

  /**
   * Deletes the results data
   * @param session_id Session id
   * @param result_ids Result ids
   */
  void delete_results_data(std::string session_id, const std::vector<std::string> &result_ids);
  [[deprecated("Use the delete_results_data method instead")]] void
  delete_results(const std::string &session_id, const std::vector<std::string> &result_ids);

  /**
   * Get the service configuration
   * @return Result service configuration
   */
  Configuration get_service_configuration();

private:
  std::unique_ptr<armonik::api::grpc::v1::results::Results::StubInterface> stub;
  static const armonik::api::grpc::v1::results::ListResultsRequest::Sort default_sort;

  std::map<std::string, std::string>
  send_create_results(const armonik::api::grpc::v1::results::CreateResultsRequest &request);
};
} // namespace client
} // namespace api
} // namespace armonik
