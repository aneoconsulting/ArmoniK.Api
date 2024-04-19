package armonik.client.task.spec;

import armonik.api.grpc.v1.Objects;
import armonik.api.grpc.v1.tasks.TasksCommon;
import armonik.api.grpc.v1.tasks.TasksFilters;
import io.grpc.stub.StreamObserver;

import java.util.List;

public interface ITaskClientAsync {
  void submitTasks(String sessionId, List<TasksCommon.SubmitTasksRequest.TaskCreation> taskCreations, Objects.TaskOptions taskOptions, StreamObserver<List<TasksCommon.SubmitTasksResponse.TaskInfo>> responseObserver);

  void countTasksByStatus(TasksFilters.Filters filters, StreamObserver<TasksCommon.CountTasksByStatusResponse> responseObserver);

  void getResultIds(List<String> tasksIds, StreamObserver<List<TasksCommon.GetResultIdsResponse.MapTaskResult>> responseObserver);

  void cancelTasks(List<String> tasksIds, StreamObserver<List<TasksCommon.TaskSummary>> responseObserver);

  void getTask(String taskId, StreamObserver<TasksCommon.TaskDetailed> responseObserver) throws NoSuchMethodException;

  void listTasksDetailed(int page, int pageSize, TasksFilters.Filters filters, TasksCommon.ListTasksRequest.Sort sort, StreamObserver<List<TasksCommon.TaskDetailed>> responseObserver) throws NoSuchMethodException;

  void listTasks(int page, int pageSize, TasksFilters.Filters filters, TasksCommon.ListTasksRequest.Sort sort, StreamObserver<List<TasksCommon.TaskSummary>> responseObserver) throws NoSuchMethodException;
}
