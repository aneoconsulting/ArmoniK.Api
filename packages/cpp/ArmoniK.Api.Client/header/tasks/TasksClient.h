#pragma once

#include "tasks_common.pb.h"
#include "tasks_service.grpc.pb.h"

class TasksClient final{
  explicit TasksClient(std::unique_ptr<armonik::api::grpc::v1::tasks::Tasks::StubInterface> stub);

  /**
   * Get informations about the given task
   * @param session_id Task id
   * @return TaskDetailed object containing information about the task
   */
  armonik::api::grpc::v1::tasks::TaskDetailed get_task(const std::string &task_id);

  /**
   * Cancel a list of tasks
   * @param task_ids List of task ids
   * @return Vector of TaskSummary objects containing information about the canceled tasks
   */
  std::vector<armonik::api::grpc::v1::tasks::TaskSummary> cancel_tasks(const std::vector<std::string> &task_ids);

  /**
   * List the Sessions
   * @param filters Filter to be used
   * @param total Output for the total of session available for this request (used for pagination)
   * @param page Page to request, use -1 to get all pages.
   * @param page_size Size of the requested page, ignored if page is -1
   * @param sort How the sessions are sorted, ascending creation date by default
   * @return List of sessions
   *
   * @note If the sessions corresponding to the filters change while this call is going for page==-1,
   * or between calls, then the returned values may not be consistent depending on the sorting used.
   * For example, a sort by ascending creation date (the default) will be stable if sessions are being created in
   * between requests.
   */
  std::vector<armonik::api::grpc::v1::sessions::SessionRaw>
  list_sessions(const armonik::api::grpc::v1::sessions::Filters &filters, int32_t &total, int32_t page = -1,
                int32_t page_size = 500,
                const armonik::api::grpc::v1::sessions::ListSessionsRequest::Sort &sort = default_sort);

private:
  std::unique_ptr<armonik::api::grpc::v1::sessions::Sessions::StubInterface> stub;
  static const armonik::api::grpc::v1::sessions::ListSessionsRequest::Sort default_sort;
};
