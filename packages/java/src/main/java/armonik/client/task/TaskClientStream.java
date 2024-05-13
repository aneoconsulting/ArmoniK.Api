package armonik.client.task;

import armonik.api.grpc.v1.Objects.TaskOptions;
import armonik.api.grpc.v1.tasks.TasksCommon.*;
import armonik.api.grpc.v1.tasks.TasksCommon.ListTasksRequest.Sort;
import armonik.api.grpc.v1.tasks.TasksCommon.SubmitTasksRequest.TaskCreation;
import armonik.api.grpc.v1.tasks.TasksFilters.Filters;
import armonik.api.grpc.v1.tasks.TasksGrpc;
import armonik.api.grpc.v1.tasks.TasksGrpc.TasksStub;
import armonik.client.task.util.ListTasksRequestParams;
import armonik.client.task.util.SubmitTasksRequestParams;
import armonik.client.task.util.TaskClientRequestFactory;
import io.grpc.ManagedChannel;
import io.grpc.stub.StreamObserver;

import java.util.List;

@Deprecated(forRemoval = true)
public class TaskClientStream {
  private final TasksStub taskStub;

  public TaskClientStream(ManagedChannel managedChannel) {
    this.taskStub = TasksGrpc.newStub(managedChannel);
  }

  public void submitTasks(SubmitTasksRequestParams requestParams, StreamObserver<SubmitTasksResponse> observer) {
    SubmitTasksRequest request = TaskClientRequestFactory.createSubmitTasksRequest(
      requestParams.sessionId(),
      requestParams.taskCreations(),
      requestParams.taskOptions());



    taskStub.submitTasks(request, observer);
  }

  public void countTasksByStatus(Filters filters, StreamObserver<CountTasksByStatusResponse> observer) {
    CountTasksByStatusRequest request = TaskClientRequestFactory.createCountTasksByStatusRequest(filters);
    taskStub.countTasksByStatus(request, observer);
  }

  public void getResultIds(List<String> tasksIds, StreamObserver<GetResultIdsResponse> observer) {
    GetResultIdsRequest request = TaskClientRequestFactory.createGetResultIdsRequest(tasksIds);
    taskStub.getResultIds(request, observer);
  }

  public void cancelTasks(List<String> tasksIds, StreamObserver<CancelTasksResponse> observer) {
    CancelTasksRequest request = TaskClientRequestFactory.createCancelTasksRequest(tasksIds);
    taskStub.cancelTasks(request, observer);
  }

  public void getTask(String taskId, StreamObserver<GetTaskResponse> observer) throws NoSuchMethodException {
    GetTaskRequest request = TaskClientRequestFactory.createGetTaskRequest(taskId);
    taskStub.getTask(request, observer);
  }

  public void listTasksDetailed(ListTasksRequestParams requestParams, StreamObserver<ListTasksDetailedResponse> observer) {
    ListTasksRequest request = TaskClientRequestFactory.createListTasksDetailedRequest(
      requestParams.page(),
      requestParams.pageSize(),
      requestParams.filters(),
      requestParams.sort());
    taskStub.listTasksDetailed(request, observer);
  }

  public void listTasks(ListTasksRequestParams requestParams, StreamObserver<ListTasksResponse> observer) {
    ListTasksRequest request = TaskClientRequestFactory.createListTasksSummaryRequest(
      requestParams.page(),
      requestParams.pageSize(),
      requestParams.filters(),
      requestParams.sort());
    taskStub.listTasks(request, observer);
  }
}
