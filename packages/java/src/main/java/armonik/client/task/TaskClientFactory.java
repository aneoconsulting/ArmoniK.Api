package armonik.client.task;

import armonik.client.task.impl.TaskClientAsync;
import armonik.client.task.impl.TaskClientSync;
import armonik.client.task.spec.ITaskClientAsync;
import armonik.client.task.spec.ITaskClientSync;
import io.grpc.ManagedChannel;

/**
 * TaskClientFactory provides factory methods to create instances of task client implementations.
 */
public abstract class TaskClientFactory {

  /**
   * Creates a synchronous task client instance with the provided managed channel.
   *
   * @param managedChannel the managed channel used for communication with the task service
   * @return an instance of {@link ITaskClientSync} representing a synchronous task client
   */
  public static ITaskClientSync createTaskClientSync(ManagedChannel managedChannel){
    return new TaskClientSync(managedChannel);
  }

  /**
   * Creates an asynchronous task client instance with the provided managed channel.
   *
   * @param managedChannel the managed channel used for communication with the task service
   * @return an instance of {@link ITaskClientAsync} representing an asynchronous task client
   */
  public static ITaskClientAsync createTaskClientAsync(ManagedChannel managedChannel){
    return new TaskClientAsync(managedChannel);
  }
}
