package armonik.client.task;

import armonik.client.task.impl.TaskClientAsync;
import armonik.client.task.impl.TaskClientSync;
import armonik.client.task.spec.ITaskClientAsync;
import armonik.client.task.spec.ITaskClientSync;
import io.grpc.ManagedChannel;

public abstract class TaskClientFactory {
  public ITaskClientSync createTaskClientSync(ManagedChannel managedChannel){
    return new TaskClientSync(managedChannel);
  }

  public ITaskClientAsync createTaskClientAsync(ManagedChannel managedChannel){
    return new TaskClientAsync(managedChannel);
  }
}
