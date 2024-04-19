package armonik.client.result;

import armonik.client.result.impl.ResultClientAsync;
import armonik.client.result.impl.ResultClientSync;
import armonik.client.result.spec.IResultClientAsync;
import armonik.client.result.spec.IResultClientSync;
import io.grpc.ManagedChannel;

/**
 * ResultClientFactory provides static factory methods to create instances of result client implementations.
 */
public abstract class ResultClientFactory {
  /**
   * Creates a synchronous result client instance with the provided managed channel.
   *
   * @param managedChannel the managed channel used for communication with the result service
   * @return an instance of {@link IResultClientSync} representing a synchronous result client
   */
  public static IResultClientSync createResultClientSync(ManagedChannel managedChannel) {
    return new ResultClientSync(managedChannel);
  }

  /**
   * Creates an asynchronous result client instance with the provided managed channel.
   *
   * @param managedChannel the managed channel used for communication with the result service
   * @return an instance of {@link IResultClientAsync} representing an asynchronous result client
   */
  public static IResultClientAsync createResultClientAsync(ManagedChannel managedChannel) {
    return new ResultClientAsync(managedChannel);
  }
}
