package armonik.client.event;

import armonik.client.event.impl.EventClientAsync;
import armonik.client.event.impl.EventClientSync;
import armonik.client.event.spec.IEventClientAsync;
import armonik.client.event.spec.IEventClientSync;
import io.grpc.ManagedChannel;

public abstract class EventClientFactory {
  public static IEventClientAsync createEventClientAsync(ManagedChannel managedChannel) {
    return new EventClientAsync(managedChannel);
  }

  public static IEventClientSync createEventClientSync(ManagedChannel managedChannel) {
    return new EventClientSync(managedChannel);
  }
}
