package armonik.client.task;

import armonik.api.grpc.v1.Objects;
import armonik.api.grpc.v1.Objects.StatusCount;
import armonik.api.grpc.v1.tasks.TasksCommon;
import armonik.api.grpc.v1.tasks.TasksCommon.GetResultIdsResponse.MapTaskResult;
import armonik.api.grpc.v1.tasks.TasksCommon.SubmitTasksResponse.TaskInfo;
import armonik.api.grpc.v1.tasks.TasksCommon.TaskDetailed;
import armonik.api.grpc.v1.tasks.TasksCommon.TaskSummary;
import armonik.api.grpc.v1.tasks.TasksFilters;
import armonik.api.grpc.v1.tasks.TasksFilters.Filters;
import armonik.api.grpc.v1.tasks.TasksGrpc;
import armonik.client.task.util.ListTasksRequestParams;
import armonik.client.task.util.SubmitTasksRequestParams;
import com.google.common.util.concurrent.Futures;
import com.google.common.util.concurrent.ListenableFuture;
import io.grpc.ManagedChannel;

import java.util.List;
import java.util.concurrent.CompletableFuture;

/**
 * TaskClientFuture provides asynchronous operations for interacting with task-related functionalities.
 * It utilizes CompletableFuture to asynchronously perform operations using the TaskClient.
 */
public class TaskClientFuture {
  /** The TaskClient used for synchronous communication with the server. */
  private final TaskClient client;

  /**
   * Constructs a new TaskClientFuture with the specified managed channel.
   *
   * @param managedChannel the managed channel used for communication with the server
   */
  public TaskClientFuture(ManagedChannel managedChannel) {
    this.client = new TaskClient(managedChannel);
  }

  /**
   * Asynchronously submits tasks to the server based on the specified request parameters.
   *
   * @param requestParams the parameters for submitting tasks
   * @return a CompletableFuture representing the asynchronous operation to submit tasks
   */
  public CompletableFuture<List<TaskInfo>> submitTasks(SubmitTasksRequestParams requestParams) {
    return CompletableFuture.supplyAsync(() -> client.submitTasks(requestParams));
  }

  /**
   * Asynchronously counts tasks based on their status and the specified filters.
   *
   * @param filters the filters to apply when counting tasks
   * @return a CompletableFuture representing the asynchronous operation to count tasks by status
   */
  public CompletableFuture<List<StatusCount>> countTasksByStatus(Filters filters) {
    return CompletableFuture.supplyAsync(() -> client.countTasksByStatus(filters));
  }

  /**
   * Asynchronously retrieves result IDs associated with the specified task IDs.
   *
   * @param tasksIds the IDs of the tasks for which result IDs are requested
   * @return a CompletableFuture representing the asynchronous operation to retrieve result IDs
   */
  public CompletableFuture<List<MapTaskResult>> getResultIds(List<String> tasksIds) {
    return CompletableFuture.supplyAsync(() -> client.getResultIds(tasksIds));
  }

  /**
   * Asynchronously cancels tasks with the specified task IDs.
   *
   * @param tasksIds the IDs of the tasks to cancel
   * @return a CompletableFuture representing the asynchronous operation to cancel tasks
   */
  public CompletableFuture<List<TaskSummary>> cancelTasks(List<String> tasksIds) {
    return CompletableFuture.supplyAsync(() -> client.cancelTasks(tasksIds));
  }

  /**
   * Asynchronously retrieves detailed information for the task with the specified task ID.
   *
   * @param taskId the ID of the task to retrieve
   * @return a CompletableFuture representing the asynchronous operation to retrieve task details
   */
  public CompletableFuture<TaskDetailed> getTask(String taskId) {
    return CompletableFuture.supplyAsync(() -> client.getTask(taskId));
  }

  /**
   * Asynchronously retrieves a list of detailed tasks based on the specified request parameters.
   *
   * @param requestParams the parameters for listing detailed tasks
   * @return a CompletableFuture representing the asynchronous operation to retrieve detailed tasks
   */
  public CompletableFuture<List<TaskDetailed>> listTasksDetailed(ListTasksRequestParams requestParams) {
    return CompletableFuture.supplyAsync(() -> client.listTasksDetailed(requestParams)) ;
  }

  /**
   * Asynchronously retrieves a list of task summaries based on the specified request parameters.
   *
   * @param requestParams the parameters for listing task summaries
   * @return a CompletableFuture representing the asynchronous operation to retrieve task summaries
   */
  public CompletableFuture<List<TaskSummary>> listTasks(ListTasksRequestParams requestParams) {
    return CompletableFuture.supplyAsync(() -> client.listTasks(requestParams));
  }
}
