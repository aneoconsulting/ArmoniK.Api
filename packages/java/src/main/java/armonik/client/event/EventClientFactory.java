package armonik.client.event;

import armonik.client.event.impl.EventClientAsync;
import armonik.client.event.impl.EventClientSync;
import armonik.client.event.spec.IEventClientAsync;
import armonik.client.event.spec.IEventClientSync;
import io.grpc.ManagedChannel;

/**
 * EventClientFactory provides static factory methods to create instances of event client implementations.
 */
public abstract class EventClientFactory {
  /**
   * Creates an asynchronous event client instance with the provided managed channel.
   *
   * @param managedChannel the managed channel used for communication with the event service
   * @return an instance of {@link IEventClientAsync} representing an asynchronous event client
   */
  public static IEventClientAsync createEventClientAsync(ManagedChannel managedChannel) {
    return new EventClientAsync(managedChannel);
  }

  /**
   * Creates a synchronous event client instance with the provided managed channel.
   *
   * @param managedChannel the managed channel used for communication with the event service
   * @return an instance of {@link IEventClientSync} representing a synchronous event client
   */
  public static IEventClientSync createEventClientSync(ManagedChannel managedChannel) {
    return new EventClientSync(managedChannel);
  }
}
