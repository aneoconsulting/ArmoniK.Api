package armonik.client.task.impl;

import armonik.api.grpc.v1.Objects.StatusCount;
import armonik.api.grpc.v1.Objects.TaskOptions;
import armonik.api.grpc.v1.tasks.TasksCommon.*;
import armonik.api.grpc.v1.tasks.TasksCommon.ListTasksRequest.Sort;
import armonik.api.grpc.v1.tasks.TasksCommon.SubmitTasksRequest.TaskCreation;
import armonik.api.grpc.v1.tasks.TasksCommon.SubmitTasksResponse.TaskInfo;
import armonik.api.grpc.v1.tasks.TasksFilters.Filters;
import armonik.api.grpc.v1.tasks.TasksGrpc;
import armonik.api.grpc.v1.tasks.TasksGrpc.TasksBlockingStub;
import armonik.client.task.impl.util.TaskClientRequestFactory;
import armonik.client.task.spec.ITaskClientSync;
import io.grpc.ManagedChannel;

import java.util.List;

import static armonik.api.grpc.v1.tasks.TasksCommon.GetResultIdsResponse.MapTaskResult;

public class TaskClientSync implements ITaskClientSync {
  private final TasksBlockingStub taskStub;


  public TaskClientSync(ManagedChannel managedChannel) {
    this.taskStub = TasksGrpc.newBlockingStub(managedChannel);
  }

  @Override
  public List<TaskSummary> listTasks(int page, int pageSize, Filters filters, Sort sort) {
    ListTasksRequest request = TaskClientRequestFactory.createListTasksSummaryRequest(page, pageSize, filters, sort);
    return taskStub.listTasks(request).getTasksList();
  }

  @Override
  public List<TaskDetailed> listTasksDetailed(int page, int pageSize, Filters filters, Sort sort) {
    ListTasksRequest request = TaskClientRequestFactory.createListTasksDetailedRequest(page, pageSize, filters, sort);
    return taskStub.listTasksDetailed(request).getTasksList();
  }

  @Override
  public TaskDetailed getTask(String taskId) {
    GetTaskRequest request = TaskClientRequestFactory.createGetTaskRequest(taskId);
    return taskStub.getTask(request).getTask();
  }

  @Override
  public List<TaskSummary> cancelTasks(List<String> tasksIds) {
    CancelTasksRequest request = TaskClientRequestFactory.createCancelTasksRequest(tasksIds);
    return taskStub.cancelTasks(request).getTasksList();
  }


  @Override
  public List<MapTaskResult> getResultIds(List<String> tasksIds) {
    GetResultIdsRequest request = TaskClientRequestFactory.createGetResultIdsRequest(tasksIds);
    return taskStub.getResultIds(request).getTaskResultsList();
  }


  @Override
  public List<StatusCount> countTasksByStatus(Filters filters) {
    CountTasksByStatusRequest request = TaskClientRequestFactory.createCountTasksByStatusRequest(filters);
    return taskStub.countTasksByStatus(request).getStatusList();
  }

  @Override
  public List<TaskInfo> submitTasks(String sessionId, List<TaskCreation> taskCreations, TaskOptions taskOptions) {
    SubmitTasksRequest request = TaskClientRequestFactory.createSubmitTasksRequest(sessionId, taskCreations, taskOptions);
    return taskStub.submitTasks(request).getTaskInfosList();
  }
}
