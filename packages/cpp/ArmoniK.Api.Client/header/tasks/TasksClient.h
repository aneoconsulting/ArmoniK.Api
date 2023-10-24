#pragma once

#include "tasks_common.pb.h"
#include "tasks_service.grpc.pb.h"

#include "Task.h"

namespace armonik {
namespace api {
namespace client {

class TasksClient {

  explicit TasksClient(std::unique_ptr<armonik::api::grpc::v1::tasks::Tasks::StubInterface> stub)
      : stub(std::move(stub)){};

  /**
   * List the Tasks
   * @note This function returns a summary view of each task
   * @param filters Filter to be used
   * @param total Output for the total of session available for this request (used for pagination)
   * @param page Page to request, use -1 to get all pages.
   * @param page_size Size of the requested page, ignored if page is -1
   * @param sort How the sessions are sorted, ascending creation date by default
   * @return List of tasks summary
   *
   * @note If the tasks corresponding to the filters change while this call is going for page==-1,
   * or between calls, then the returned values may not be consistent depending on the sorting used.
   * For example, a sort by ascending creation date (the default) will be stable if tasks are being created in
   * between requests.
   */
  std::vector<armonik::api::grpc::v1::tasks::TaskSummary>
  list_tasks(armonik::api::grpc::v1::tasks::Filters filters, int32_t &total, int32_t page = -1, int32_t page_size = 500,
             armonik::api::grpc::v1::tasks::ListTasksRequest::Sort sort = default_sort());

  /**
   * List the Tasks
   * @note This function returns a detailed view of each task
   * @param filters Filter to be used
   * @param total Output for the total of session available for this request (used for pagination)
   * @param page Page to request, use -1 to get all pages.
   * @param page_size Size of the requested page, ignored if page is -1
   * @param sort How the sessions are sorted, ascending creation date by default
   * @return List of tasks summary
   *
   * @note If the tasks corresponding to the filters change while this call is going for page==-1,
   * or between calls, then the returned values may not be consistent depending on the sorting used.
   * For example, a sort by ascending creation date (the default) will be stable if tasks are being created in
   * between requests.
   */
  std::vector<armonik::api::grpc::v1::tasks::TaskDetailed>
  list_tasks_detailed(armonik::api::grpc::v1::tasks::Filters filters, int32_t &total, int32_t page = -1,
                      int32_t page_size = 500,
                      armonik::api::grpc::v1::tasks::ListTasksRequest::Sort sort = default_sort());

  /**
   * Get informations about the given task
   * @param session_id Task id
   * @return TaskDetailed object containing information about the task
   */
  armonik::api::grpc::v1::tasks::TaskDetailed get_task(std::string task_id);

  /**
   * Cancel a list of tasks
   * @param task_ids List of task ids
   * @return Vector of TaskSummary objects containing information about the canceled tasks
   */
  std::vector<armonik::api::grpc::v1::tasks::TaskSummary> cancel_tasks(const std::vector<std::string> &task_ids);

  /**
   * Get the result ids of each task
   * @param task_ids List of tasks
   * @return Map associating the task id to its result ids
   */
  std::map<std::string, std::vector<std::string>> get_result_ids(const std::vector<std::string> &task_ids);

  /**
   * Count tasks by status
   * @param filters Task filter, optional
   * @return Map of each task status and its count
   */
  std::map<armonik::api::grpc::v1::task_status::TaskStatus, int32_t>
  count_tasks_by_status(armonik::api::grpc::v1::tasks::Filters filters);

  /**
   * Create tasks metadata and submit task for processing
   * @param session_id Session id
   * @param task_creations List of task creations
   * @param task_options Task options common for this submission. Will be merged with the session task options
   * @return List of submitted task info
   */
  std::vector<TaskInfo> submit_tasks(std::string session_id, const std::vector<TaskCreation> &task_creations,
                                     const armonik::api::grpc::v1::TaskOptions &task_options = no_task_options);

private:
  std::unique_ptr<armonik::api::grpc::v1::tasks::Tasks::StubInterface> stub;
  static armonik::api::grpc::v1::tasks::ListTasksRequest::Sort default_sort();
  static const armonik::api::grpc::v1::TaskOptions no_task_options;
};

} // namespace client
} // namespace api
} // namespace armonik
