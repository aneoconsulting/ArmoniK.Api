package armonik.client.task.impl.util;

import armonik.api.grpc.v1.Objects.TaskOptions;
import armonik.api.grpc.v1.tasks.TasksCommon;
import armonik.api.grpc.v1.tasks.TasksCommon.*;
import armonik.api.grpc.v1.tasks.TasksCommon.ListTasksRequest.Sort;
import armonik.api.grpc.v1.tasks.TasksCommon.SubmitTasksRequest.TaskCreation;
import armonik.api.grpc.v1.tasks.TasksFilters.Filters;

import java.util.List;

import static armonik.api.grpc.v1.Objects.TaskOptions.getDefaultInstance;

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
    SubmitTasksRequest.Builder requestBuilder = SubmitTasksRequest.newBuilder();

    requestBuilder.setSessionId(sessionId);

    if (!taskOptions.equals(getDefaultInstance())) {
      requestBuilder.setTaskOptions(taskOptions);
    }
    taskCreations.forEach(taskCreation -> {
      requestBuilder
        .addTaskCreationsBuilder()
        .setPayloadId(taskCreation.getPayloadId())
        .addAllDataDependencies(taskCreation.getDataDependenciesList())
        .addAllExpectedOutputKeys(taskCreation.getExpectedOutputKeysList())
        .setTaskOptions(!taskCreation.getTaskOptions().equals(getDefaultInstance()) ? taskCreation.getTaskOptions() : getDefaultInstance());
    });

    return requestBuilder.build();
  }
}
