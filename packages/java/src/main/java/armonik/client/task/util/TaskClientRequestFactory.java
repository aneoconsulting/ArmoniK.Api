package armonik.client.task.util;

import armonik.api.grpc.v1.Objects.TaskOptions;
import armonik.api.grpc.v1.tasks.TasksCommon;
import armonik.api.grpc.v1.tasks.TasksCommon.*;
import armonik.api.grpc.v1.tasks.TasksCommon.ListTasksRequest.Sort;
import armonik.api.grpc.v1.tasks.TasksCommon.SubmitTasksRequest.TaskCreation;
import armonik.api.grpc.v1.tasks.TasksFilters.Filters;

import java.util.List;

/**
 * TaskClientRequestFactory provides static methods for creating gRPC request objects related to task management.
 * It encapsulates the logic for constructing various types of task-related requests, such as listing tasks,
 * retrieving task details, canceling tasks, retrieving result IDs, counting tasks by status, and submitting tasks.
 */
public abstract class TaskClientRequestFactory {
  public static ListTasksRequest createListTasksSummaryRequest(int page, int pageSize, Filters filters, Sort sort) {
    return ListTasksRequest.newBuilder()
      .setPage(page)
      .setPageSize(pageSize)
      .setFilters(filters)
      .setSort(sort)
      .build();
  }

  public static ListTasksRequest createListTasksDetailedRequest(int page, int pageSize, Filters filters, Sort sort) {
    return TasksCommon.ListTasksRequest.newBuilder()
      .setSort(sort)
      .setPageSize(page)
      .setPageSize(pageSize)
      .setFilters(filters)
      .build();
  }

  public static GetTaskRequest createGetTaskRequest(String taskId) {
    return GetTaskRequest.newBuilder()
      .setTaskId(taskId)
      .build();
  }

  public static CancelTasksRequest createCancelTasksRequest(List<String> tasksIds) {
    return CancelTasksRequest.newBuilder()
      .addAllTaskIds(tasksIds)
      .build();
  }

  public static GetResultIdsRequest createGetResultIdsRequest(List<String> tasksIds) {
    return GetResultIdsRequest.newBuilder()
      .addAllTaskId(tasksIds)
      .build();

  }

  public static CountTasksByStatusRequest createCountTasksByStatusRequest(Filters filters) {
    return CountTasksByStatusRequest.newBuilder()
      .setFilters(filters)
      .build();
  }

  public static SubmitTasksRequest createSubmitTasksRequest(String sessionId, List<TaskCreation> taskCreations, TaskOptions taskOptions) {
   return SubmitTasksRequest.newBuilder()
      .setSessionId(sessionId)
      .setTaskOptions(taskOptions)
      .addAllTaskCreations(taskCreations
        .stream()
        .map(taskCreation -> {
          boolean taskOptionExist = taskCreation.getTaskOptions().getMaxRetries() != Integer.MIN_VALUE ;
          if(!taskOptionExist){
            return taskCreation.toBuilder().setTaskOptions(taskOptions).build();
          }
          return taskCreation;
        })
        .toList()
      )
      .build();
  }
}
