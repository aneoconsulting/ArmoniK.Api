#include "tasks/TasksClient.h"
#include "exceptions/ArmoniKApiException.h"
#include <sstream>

using namespace armonik::api::client;

static inline ::grpc::Status call_stub_list(armonik::api::grpc::v1::tasks::Tasks::StubInterface *stub,
                                            const armonik::api::grpc::v1::tasks::ListTasksRequest &request,
                                            armonik::api::grpc::v1::tasks::ListTasksDetailedResponse *response) {
  ::grpc::ClientContext context;
  return stub->ListTasksDetailed(&context, request, response);
}

static inline ::grpc::Status call_stub_list(armonik::api::grpc::v1::tasks::Tasks::StubInterface *stub,
                                            const armonik::api::grpc::v1::tasks::ListTasksRequest &request,
                                            armonik::api::grpc::v1::tasks::ListTasksResponse *response) {
  ::grpc::ClientContext context;
  return stub->ListTasks(&context, request, response);
}

/**
 * Common function called to list tasks
 * @tparam T Result value type (TaskSummary or TaskDetailed
 * @tparam U Response type
 * @param stub Task stub
 * @param filters Filter to be used
 * @param total Output for the total of session available for this request (used for pagination)
 * @param page Page to request, use -1 to get all pages.
 * @param page_size Size of the requested page, ignored if page is -1
 * @param sort How the tasks are sorted, ascending creation date by default
 * @return Vector of information about the tasks
 */
template <typename T, typename U, class = decltype(std::declval<U>().tasks()),
          class = decltype(std::declval<U>().total())>
static std::vector<T> list_tasks_common(armonik::api::grpc::v1::tasks::Tasks::StubInterface *stub,
                                        armonik::api::grpc::v1::tasks::Filters filters, int32_t &total, int32_t page,
                                        int32_t page_size, armonik::api::grpc::v1::tasks::ListTasksRequest::Sort sort) {
  armonik::api::grpc::v1::tasks::ListTasksRequest request;
  U response;
  *request.mutable_filters() = std::move(filters);
  *request.mutable_sort() = std::move(sort);
  request.set_page_size(page_size);

  if (page >= 0) {
    request.set_page(page);
    auto status = call_stub_list(stub, request, &response);

    if (!status.ok()) {
      throw armonik::api::common::exceptions::ArmoniKApiException("Unable to list results " + status.error_message());
    }
    total = response.total();
    return {response.tasks().begin(), response.tasks().end()};
  } else {
    std::vector<T> rawResults;
    int current_page = 0;
    do {
      request.set_page(current_page);
      ::grpc::ClientContext context;
      auto status = call_stub_list(stub, request, &response);
      if (!status.ok()) {
        throw armonik::api::common::exceptions::ArmoniKApiException("Unable to list results " + status.error_message());
      }
      // Append only the additional tasks
      // If the current_page is a re-request, this will add only the new information
      rawResults.insert(rawResults.end(),
                        response.tasks().begin() + ((int32_t)rawResults.size() - current_page * page_size),
                        response.tasks().end());
      if (response.total() >= page_size) {
        ++current_page;
      }

      response.clear_tasks();
    } while ((int32_t)rawResults.size() < response.total());
    total = response.total();
    return rawResults;
  }
}

std::vector<armonik::api::grpc::v1::tasks::TaskSummary>
TasksClient::list_tasks(armonik::api::grpc::v1::tasks::Filters filters, int32_t &total, int32_t page, int32_t page_size,
                        armonik::api::grpc::v1::tasks::ListTasksRequest::Sort sort) {
  return list_tasks_common<armonik::api::grpc::v1::tasks::TaskSummary,
                           armonik::api::grpc::v1::tasks::ListTasksResponse>(stub.get(), std::move(filters), total,
                                                                             page, page_size, std::move(sort));
}

std::vector<armonik::api::grpc::v1::tasks::TaskDetailed>
TasksClient::list_tasks_detailed(armonik::api::grpc::v1::tasks::Filters filters, int32_t &total, int32_t page,
                                 int32_t page_size, armonik::api::grpc::v1::tasks::ListTasksRequest::Sort sort) {
  return list_tasks_common<armonik::api::grpc::v1::tasks::TaskDetailed,
                           armonik::api::grpc::v1::tasks::ListTasksDetailedResponse>(
      stub.get(), std::move(filters), total, page, page_size, std::move(sort));
}

armonik::api::grpc::v1::tasks::TaskDetailed TasksClient::get_task(std::string task_id) {
  ::grpc::ClientContext context;
  armonik::api::grpc::v1::tasks::GetTaskRequest request;
  armonik::api::grpc::v1::tasks::GetTaskResponse response;
  *request.mutable_task_id() = std::move(task_id);
  auto status = stub->GetTask(&context, request, &response);
  if (!status.ok()) {
    std::stringstream message;
    message << "Error getting task: " << status.error_code() << ": " << status.error_message()
            << ". details : " << status.error_details() << std::endl;
    auto str = message.str();
    throw armonik::api::common::exceptions::ArmoniKApiException(str);
  }

  return response.task();
}

std::vector<armonik::api::grpc::v1::tasks::TaskSummary>
TasksClient::cancel_tasks(const std::vector<std::string> &task_ids) {
  ::grpc::ClientContext context;
  armonik::api::grpc::v1::tasks::CancelTasksRequest request;
  armonik::api::grpc::v1::tasks::CancelTasksResponse response;
  request.mutable_task_ids()->Add(task_ids.begin(), task_ids.end());
  auto status = stub->CancelTasks(&context, request, &response);
  if (!status.ok()) {
    std::stringstream message;
    message << "Error canceling tasks: " << status.error_code() << ": " << status.error_message()
            << ". details : " << status.error_details() << std::endl;
    auto str = message.str();
    throw armonik::api::common::exceptions::ArmoniKApiException(str);
  }

  return {std::make_move_iterator(response.mutable_tasks()->begin()),
          std::make_move_iterator(response.mutable_tasks()->end())};
}

std::map<std::string, std::vector<std::string>> TasksClient::get_result_ids(const std::vector<std::string> &task_ids) {
  ::grpc::ClientContext context;
  armonik::api::grpc::v1::tasks::GetResultIdsRequest request;
  armonik::api::grpc::v1::tasks::GetResultIdsResponse response;
  request.mutable_task_id()->Add(task_ids.begin(), task_ids.end());
  auto status = stub->GetResultIds(&context, request, &response);
  if (!status.ok()) {
    std::stringstream message;
    message << "Error getting result ids from tasks: " << status.error_code() << ": " << status.error_message()
            << ". details : " << status.error_details() << std::endl;
    auto str = message.str();
    throw armonik::api::common::exceptions::ArmoniKApiException(str);
  }

  std::map<std::string, std::vector<std::string>> map_results;

  for (auto &&tid_rid : *response.mutable_task_results()) {
    map_results[std::move(*tid_rid.mutable_task_id())] = {
        std::make_move_iterator(tid_rid.mutable_result_ids()->begin()),
        std::make_move_iterator(tid_rid.mutable_result_ids()->end())};
  }
  return map_results;
}

std::map<armonik::api::grpc::v1::task_status::TaskStatus, int32_t>
TasksClient::count_tasks_by_status(armonik::api::grpc::v1::tasks::Filters filters) {
  ::grpc::ClientContext context;
  armonik::api::grpc::v1::tasks::CountTasksByStatusRequest request;
  armonik::api::grpc::v1::tasks::CountTasksByStatusResponse response;
  *request.mutable_filters() = std::move(filters);

  auto status = stub->CountTasksByStatus(&context, request, &response);
  if (!status.ok()) {
    std::stringstream message;
    message << "Error getting result ids from tasks: " << status.error_code() << ": " << status.error_message()
            << ". details : " << status.error_details() << std::endl;
    auto str = message.str();
    throw armonik::api::common::exceptions::ArmoniKApiException(str);
  }

  std::map<armonik::api::grpc::v1::task_status::TaskStatus, int32_t> map_status;
  for (auto &&status_count : *response.mutable_status()) {
    map_status[status_count.status()] = status_count.count();
  }
  return map_status;
}

std::vector<armonik::api::common::TaskInfo>
TasksClient::submit_tasks(std::string session_id, const std::vector<armonik::api::common::TaskCreation> &task_creations,
                          const armonik::api::grpc::v1::TaskOptions &task_options) {
  ::grpc::ClientContext context;
  armonik::api::grpc::v1::tasks::SubmitTasksRequest request;
  armonik::api::grpc::v1::tasks::SubmitTasksResponse response;

  *request.mutable_session_id() = std::move(session_id);
  if (task_options.max_retries() != INT32_MIN) {
    // not default task_options
    *request.mutable_task_options() = task_options;
  }

  for (auto &&t : task_creations) {
    auto new_t = request.mutable_task_creations()->Add();
    *new_t->mutable_payload_id() = t.payload_id;
    new_t->mutable_data_dependencies()->Add(t.data_dependencies.begin(), t.data_dependencies.end());
    new_t->mutable_expected_output_keys()->Add(t.expected_output_keys.begin(), t.expected_output_keys.end());
    if (t.taskOptions.max_retries() != INT32_MIN) {
      // not default task_options
      *new_t->mutable_task_options() = t.taskOptions;
    }
  }

  auto status = stub->SubmitTasks(&context, request, &response);
  if (!status.ok()) {
    std::stringstream message;
    message << "Error submitting tasks " << status.error_code() << ": " << status.error_message()
            << ". details : " << status.error_details() << std::endl;
    auto str = message.str();
    throw armonik::api::common::exceptions::ArmoniKApiException(str);
  }
  std::vector<armonik::api::common::TaskInfo> infos;
  infos.reserve(response.task_infos_size());
  for (auto &&info : *response.mutable_task_infos()) {
    infos.push_back({std::move(*info.mutable_task_id()),
                     std::vector<std::string>{std::make_move_iterator(info.mutable_expected_output_ids()->begin()),
                                              std::make_move_iterator(info.mutable_expected_output_ids()->end())},
                     std::vector<std::string>{std::make_move_iterator(info.mutable_data_dependencies()->begin()),
                                              std::make_move_iterator(info.mutable_data_dependencies()->end())},
                     std::move(*info.mutable_payload_id())});
  }
  return infos;
}

armonik::api::grpc::v1::tasks::ListTasksRequest::Sort TasksClient::default_sort() {
  armonik::api::grpc::v1::tasks::ListTasksRequest::Sort sort;
  sort.set_direction(grpc::v1::sort_direction::SORT_DIRECTION_ASC);
  sort.mutable_field()->mutable_task_summary_field()->set_field(grpc::v1::tasks::TASK_SUMMARY_ENUM_FIELD_CREATED_AT);
  return sort;
}

const armonik::api::grpc::v1::TaskOptions TasksClient::no_task_options =
    armonik::api::common::TaskCreation::get_no_task_options();
