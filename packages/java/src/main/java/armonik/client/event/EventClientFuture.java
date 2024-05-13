package armonik.client.event;

import armonik.api.grpc.v1.events.EventsGrpc;
import armonik.client.event.util.records.EventSubscriptionResponseRecord;
import io.grpc.ManagedChannel;

import java.util.List;
import java.util.concurrent.CompletableFuture;

/**
 * EventClientFuture provides asynchronous operations for interacting with event-related functionalities.
 * It utilizes CompletableFuture to asynchronously retrieve events using the EventClient.
 */
public class EventClientFuture {
  /** The EventClient used for synchronous communication with the server. */
  private final EventClient client;


  /**
   * Constructs a new EventClientFuture with the specified managed channel.
   *
   * @param managedChannel the managed channel used for communication with the server
   */
  public EventClientFuture(ManagedChannel managedChannel) {
    this.client = new EventClient(managedChannel);
  }

  /**
   * Asynchronously retrieves a list of event subscription response records for the given session ID and result IDs.
   *
   * @param sessionId the session ID for which events are requested
   * @param resultIds the list of result IDs for which events are requested
   * @return a CompletableFuture representing the asynchronous operation to retrieve events
   */
  public CompletableFuture<List<EventSubscriptionResponseRecord>> getEvents(String sessionId, List<String> resultIds) {
    return CompletableFuture.supplyAsync(() ->  client.getEvents(sessionId, resultIds));
  }

}
