package armonik.client.event;

import armonik.api.grpc.v1.events.EventsCommon.EventSubscriptionRequest;
import armonik.api.grpc.v1.events.EventsCommon.EventSubscriptionResponse;
import armonik.api.grpc.v1.events.EventsGrpc;
import armonik.api.grpc.v1.events.EventsGrpc.EventsBlockingStub;
import armonik.client.event.util.factory.EventClientRequestFactory;
import armonik.client.event.util.records.EventSubscriptionResponseRecord;
import io.grpc.ManagedChannel;

import java.util.ArrayList;
import java.util.Iterator;
import java.util.List;
import java.util.concurrent.CompletableFuture;


/**
 * EventClient is a client for interacting with event-related functionalities.
 * It communicates with a gRPC server using a blocking stub to retrieve events.
 */
public class EventClient {
  /** The blocking stub for communicating with the gRPC server. */
  private final EventsBlockingStub eventsBlockingStub;

  /**
   * Constructs a new EventClient with the specified managed channel.
   *
   * @param managedChannel the managed channel used for communication with the server
   */
  public EventClient(ManagedChannel managedChannel) {
    eventsBlockingStub = EventsGrpc.newBlockingStub(managedChannel);
  }

  /**
   * Retrieves a list of event subscription response records for the given session ID and result IDs.
   *
   * @param sessionId the session ID for which events are requested
   * @param resultIds the list of result IDs for which events are requested
   * @return a list of EventSubscriptionResponseRecord objects representing the events
   */
  public List<EventSubscriptionResponseRecord> getEvents(String sessionId, List<String> resultIds) {
    EventSubscriptionRequest request = EventClientRequestFactory.CreateEventSubscriptionRequest(sessionId, resultIds);
    return mapToRecord(sessionId, request);
  }

  /**
   * Maps the received event subscription response to EventSubscriptionResponseRecord objects.
   *
   * @param sessionId the session ID for which events are being mapped
   * @param request the event subscription request
   * @return a list of EventSubscriptionResponseRecord objects representing the events
   */
  private List<EventSubscriptionResponseRecord> mapToRecord(String sessionId, EventSubscriptionRequest request) {
    List<EventSubscriptionResponseRecord> responseRecords = new ArrayList<>();
    Iterator<EventSubscriptionResponse> events = eventsBlockingStub.getEvents(request);
    while (events.hasNext()) {
      var esr = events.next();
      responseRecords
        .add(new EventSubscriptionResponseRecord(sessionId,
          esr.getTaskStatusUpdate(),
          esr.getResultStatusUpdate(),
          esr.getResultOwnerUpdate(),
          esr.getNewTask(),
          esr.getNewResult()));

    }
    return responseRecords;
  }
}
