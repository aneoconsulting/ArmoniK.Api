package armonik.client.event.impl;

import armonik.api.grpc.v1.events.EventsCommon.EventSubscriptionRequest;
import armonik.api.grpc.v1.events.EventsCommon.EventSubscriptionResponse;
import armonik.api.grpc.v1.events.EventsGrpc;
import armonik.api.grpc.v1.events.EventsGrpc.EventsBlockingStub;
import armonik.client.event.impl.util.factory.EventClientRequestFactory;
import armonik.client.event.impl.util.records.EventSubscriptionResponseRecord;
import armonik.client.event.spec.IEventClientSync;
import com.google.common.collect.Lists;
import io.grpc.ManagedChannel;

import java.util.List;

/**
 * EventClientSync is a synchronous implementation of the {@link IEventClientSync} interface.
 * It communicates with the event service using a blocking stub, making synchronous calls to retrieve event updates.
 */
public class EventClientSync implements IEventClientSync {

  private final EventsBlockingStub eventsBlockingStub;


  /**
   * Constructs an EventClientSync object with the provided managed channel.
   *
   * @param managedChannel the managed channel used for communication with the event service
   */
  public EventClientSync(ManagedChannel managedChannel) {
    this.eventsBlockingStub = EventsGrpc.newBlockingStub(managedChannel);
  }


  /**
   * Retrieves event updates for the specified session ID and result IDs.
   *
   * @param sessionId the session ID for which event updates are requested
   * @param resultIds the list of result IDs for which event updates are requested
   * @return a list containing event update responses
   */
  @Override
  public List<EventSubscriptionResponseRecord> getEventsUpdateBySessionId(String sessionId, List<String> resultIds) {
    EventSubscriptionRequest request = EventClientRequestFactory.CreateEventSubscriptionRequest(sessionId, resultIds);

    List<EventSubscriptionResponse> subscriptionResponses = Lists.newArrayList(eventsBlockingStub.getEvents(request));

    return subscriptionResponses.stream()
      .map(
        esr -> new EventSubscriptionResponseRecord(
          sessionId,
          esr.getTaskStatusUpdate(),
          esr.getResultStatusUpdate(),
          esr.getResultOwnerUpdate(),
          esr.getNewTask(),
          esr.getNewResult())
      ).toList();
  }
}
