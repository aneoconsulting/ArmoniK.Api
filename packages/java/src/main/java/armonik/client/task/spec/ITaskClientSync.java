package armonik.client.task.spec;

import armonik.api.grpc.v1.Objects;
import armonik.api.grpc.v1.tasks.TasksCommon;
import armonik.api.grpc.v1.tasks.TasksFilters;

import java.util.List;

public interface ITaskClientSync {
  List<TasksCommon.TaskSummary> listTasks(int page, int pageSize, TasksFilters.Filters filters, TasksCommon.ListTasksRequest.Sort sort);

  List<TasksCommon.TaskDetailed> listTasksDetailed(int page, int pageSize, TasksFilters.Filters filters, TasksCommon.ListTasksRequest.Sort sort);

  TasksCommon.TaskDetailed getTask(String taskId);

  List<TasksCommon.TaskSummary> cancelTasks(List<String> tasksIds);

  List<TasksCommon.GetResultIdsResponse.MapTaskResult> getResultIds(List<String> tasksIds);

  List<Objects.StatusCount> countTasksByStatus(TasksFilters.Filters filters);

  List<TasksCommon.SubmitTasksResponse.TaskInfo> submitTasks(String sessionId, List<TasksCommon.SubmitTasksRequest.TaskCreation> taskCreations, Objects.TaskOptions taskOptions);
}
