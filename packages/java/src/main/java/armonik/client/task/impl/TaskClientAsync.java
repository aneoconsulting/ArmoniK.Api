package armonik.client.task.impl;

import armonik.api.grpc.v1.Objects.TaskOptions;
import armonik.api.grpc.v1.tasks.TasksCommon.*;
import armonik.api.grpc.v1.tasks.TasksCommon.GetResultIdsResponse.MapTaskResult;
import armonik.api.grpc.v1.tasks.TasksCommon.ListTasksRequest.Sort;
import armonik.api.grpc.v1.tasks.TasksCommon.SubmitTasksRequest.TaskCreation;
import armonik.api.grpc.v1.tasks.TasksCommon.SubmitTasksResponse.TaskInfo;
import armonik.api.grpc.v1.tasks.TasksFilters.Filters;
import armonik.api.grpc.v1.tasks.TasksGrpc;
import armonik.api.grpc.v1.tasks.TasksGrpc.TasksStub;
import armonik.client.task.impl.util.TaskClientRequestFactory;
import armonik.client.task.spec.ITaskClientAsync;
import io.grpc.ManagedChannel;
import io.grpc.stub.StreamObserver;

import java.util.List;

public class TaskClientAsync implements ITaskClientAsync {
  private final TasksStub taskStub;


  public TaskClientAsync(ManagedChannel managedChannel) {
    this.taskStub = TasksGrpc.newStub(managedChannel);
  }

  @Override
  public void submitTasks(String sessionId, List<TaskCreation> taskCreations, TaskOptions taskOptions, StreamObserver<List<TaskInfo>> responseObserver) {
    SubmitTasksRequest request = TaskClientRequestFactory.createSubmitTasksRequest(sessionId, taskCreations, taskOptions);
    StreamObserver<SubmitTasksResponse> observer = new StreamObserver<>() {
      @Override
      public void onNext(SubmitTasksResponse submitTasksResponse) {
        responseObserver.onNext(submitTasksResponse.getTaskInfosList());
      }

      @Override
      public void onError(Throwable throwable) {
        responseObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        responseObserver.onCompleted();
      }
    };

    taskStub.submitTasks(request, observer);
  }

  @Override
  public void countTasksByStatus(Filters filters, StreamObserver<CountTasksByStatusResponse> responseObserver) {
    CountTasksByStatusRequest request = TaskClientRequestFactory.createCountTasksByStatusRequest(filters);

    StreamObserver<CountTasksByStatusResponse> observer = new StreamObserver<>() {
      @Override
      public void onNext(CountTasksByStatusResponse countTasksByStatusResponse) {
        responseObserver.onNext(countTasksByStatusResponse);
      }

      @Override
      public void onError(Throwable throwable) {
        responseObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        responseObserver.onCompleted();
      }
    };


    taskStub.countTasksByStatus(request, observer);
  }

  @Override
  public void getResultIds(List<String> tasksIds, StreamObserver<List<MapTaskResult>> responseObserver) {
    GetResultIdsRequest request = TaskClientRequestFactory.createGetResultIdsRequest(tasksIds);

    StreamObserver<GetResultIdsResponse> observer = new StreamObserver<GetResultIdsResponse>() {
      @Override
      public void onNext(GetResultIdsResponse getResultIdsResponse) {
        responseObserver.onNext(getResultIdsResponse.getTaskResultsList());
      }

      @Override
      public void onError(Throwable throwable) {
        responseObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        responseObserver.onCompleted();
      }
    };
    taskStub.getResultIds(request, observer);
  }

  @Override
  public void cancelTasks(List<String> tasksIds, StreamObserver<List<TaskSummary>> responseObserver) {
    CancelTasksRequest request = TaskClientRequestFactory.createCancelTasksRequest(tasksIds);

    StreamObserver<CancelTasksResponse> observer = new StreamObserver<>() {
      @Override
      public void onNext(CancelTasksResponse cancelTasksResponse) {
        responseObserver.onNext(cancelTasksResponse.getTasksList());
      }

      @Override
      public void onError(Throwable throwable) {
        responseObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        responseObserver.onCompleted();
      }
    };

    taskStub.cancelTasks(request, observer);
  }

  @Override
  public void getTask(String taskId, StreamObserver<TaskDetailed> responseObserver) throws NoSuchMethodException {
    GetTaskRequest request = TaskClientRequestFactory.createGetTaskRequest(taskId);

    StreamObserver<GetTaskResponse> observer = new StreamObserver<>() {

      @Override
      public void onNext(GetTaskResponse getTaskResponse) {
        responseObserver.onNext(getTaskResponse.getTask());
      }

      @Override
      public void onError(Throwable throwable) {
        responseObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        responseObserver.onCompleted();
      }
    };
    taskStub.getTask(request, observer);
  }

  @Override
  public void listTasksDetailed(int page, int pageSize, Filters filters, Sort sort, StreamObserver<List<TaskDetailed>> responseObserver) {

    ListTasksRequest request = TaskClientRequestFactory.createListTasksDetailedRequest(page, pageSize, filters, sort);
    StreamObserver<ListTasksDetailedResponse> observer = new StreamObserver<>() {
      @Override
      public void onNext(ListTasksDetailedResponse listTasksDetailedResponse) {
        responseObserver.onNext(listTasksDetailedResponse.getTasksList());
      }

      @Override
      public void onError(Throwable throwable) {
        responseObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        responseObserver.onCompleted();
      }
    };

    taskStub.listTasksDetailed(request, observer);
  }

  @Override
  public void listTasks(int page, int pageSize, Filters filters, Sort sort, StreamObserver<List<TaskSummary>> responseObserver) {
    ListTasksRequest request = TaskClientRequestFactory.createListTasksSummaryRequest(page, pageSize, filters, sort);

    StreamObserver<ListTasksResponse> observer = new StreamObserver<>() {
      @Override
      public void onNext(ListTasksResponse listTasksResponse) {
        responseObserver.onNext(listTasksResponse.getTasksList());
      }

      @Override
      public void onError(Throwable throwable) {
        responseObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        responseObserver.onCompleted();
      }
    };


    taskStub.listTasks(request, observer);
  }
}
