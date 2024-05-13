package armonik.client.task;

import armonik.api.grpc.v1.Objects.StatusCount;
import armonik.api.grpc.v1.Objects.TaskOptions;
import armonik.api.grpc.v1.tasks.TasksCommon.*;
import armonik.api.grpc.v1.tasks.TasksCommon.ListTasksRequest.Sort;
import armonik.api.grpc.v1.tasks.TasksCommon.SubmitTasksRequest.TaskCreation;
import armonik.api.grpc.v1.tasks.TasksCommon.SubmitTasksResponse.TaskInfo;
import armonik.api.grpc.v1.tasks.TasksFilters.Filters;
import armonik.api.grpc.v1.tasks.TasksGrpc;
import armonik.api.grpc.v1.tasks.TasksGrpc.TasksBlockingStub;
import armonik.client.task.util.ListTasksRequestParams;
import armonik.client.task.util.SubmitTasksRequestParams;
import armonik.client.task.util.TaskClientRequestFactory;
import io.grpc.ManagedChannel;

import java.util.List;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.Future;

import static armonik.api.grpc.v1.tasks.TasksCommon.GetResultIdsResponse.MapTaskResult;

/**
 * TaskClient provides methods for interacting with task-related functionalities.
 * It communicates with a gRPC server using a blocking stub to perform various operations on tasks.
 */
public class TaskClient {
  /** The blocking stub for communicating with the gRPC server. */
  private final TasksBlockingStub taskStub;

  /**
   * Constructs a new TaskClient with the specified managed channel.
   *
   * @param managedChannel the managed channel used for communication with the server
   */
  public TaskClient(ManagedChannel managedChannel) {
    this.taskStub = TasksGrpc.newBlockingStub(managedChannel);
  }

  /**
   * Retrieves a list of task summaries based on the specified request parameters.
   *
   * @param requestParams the parameters for listing task summaries
   * @return a list of TaskSummary objects representing the retrieved task summaries
   */
  public List<TaskSummary> listTasks(ListTasksRequestParams requestParams) {
    ListTasksRequest request = TaskClientRequestFactory.createListTasksSummaryRequest(
      requestParams.page(),
      requestParams.pageSize(),
      requestParams.filters(),
      requestParams.sort());
    return taskStub.listTasks(request).getTasksList();
  }

  /**
   * Retrieves a list of detailed tasks based on the specified request parameters.
   *
   * @param requestParams the parameters for listing detailed tasks
   * @return the retrieved detailed tasks
   */
  public List<TaskDetailed> listTasksDetailed(ListTasksRequestParams requestParams) {
    ListTasksRequest request = TaskClientRequestFactory.createListTasksDetailedRequest(
      requestParams.page(),
      requestParams.pageSize(),
      requestParams.filters(),
      requestParams.sort()
    );

    return taskStub.listTasksDetailed(request).getTasksList();
  }

  /**
   * Retrieves the detailed information for the task with the specified task ID.
   *
   * @param taskId the ID of the task to retrieve
   * @return the TaskDetailed object representing the retrieved task details
   */
  public TaskDetailed getTask(String taskId) {
    GetTaskRequest request = TaskClientRequestFactory.createGetTaskRequest(taskId);
    return taskStub.getTask(request).getTask();
  }

  /**
   * Cancels tasks with the specified task IDs.
   *
   * @param tasksIds the IDs of the tasks to cancel
   * @return a list of TaskSummary objects representing the canceled tasks
   */
  public List<TaskSummary> cancelTasks(List<String> tasksIds) {
    CancelTasksRequest request = TaskClientRequestFactory.createCancelTasksRequest(tasksIds);
    return taskStub.cancelTasks(request).getTasksList();
  }

  /**
   * Retrieves result IDs associated with the specified task IDs.
   *
   * @param tasksIds the IDs of the tasks for which result IDs are requested
   * @return a list of MapTaskResult objects representing the retrieved result IDs
   */
  public List<MapTaskResult> getResultIds(List<String> tasksIds) {
    GetResultIdsRequest request = TaskClientRequestFactory.createGetResultIdsRequest(tasksIds);
    return taskStub.getResultIds(request).getTaskResultsList();
  }

  /**
   * Counts tasks based on their status and the specified filters.
   *
   * @param filters the filters to apply when counting tasks
   * @return a list of StatusCount objects representing the count of tasks by status
   */
  public List<StatusCount> countTasksByStatus(Filters filters) {
    CountTasksByStatusRequest request = TaskClientRequestFactory.createCountTasksByStatusRequest(filters);
    return taskStub.countTasksByStatus(request).getStatusList();
  }

  /**
   * Submits tasks to the server based on the specified request parameters.
   *
   * @param requestParams the parameters for submitting tasks
   * @return a list of TaskInfo objects representing information about the submitted tasks
   */
  public List<TaskInfo> submitTasks(SubmitTasksRequestParams requestParams) {
    SubmitTasksRequest request = TaskClientRequestFactory.createSubmitTasksRequest(
      requestParams.sessionId(),
      requestParams.taskCreations(),
      requestParams.taskOptions()
    );
    return taskStub.submitTasks(request).getTaskInfosList();
  }
}
