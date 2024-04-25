package armonik.client.event.impl;

import armonik.api.grpc.v1.events.EventsCommon.EventSubscriptionRequest;
import armonik.api.grpc.v1.events.EventsCommon.EventSubscriptionResponse;
import armonik.api.grpc.v1.events.EventsGrpc;
import armonik.api.grpc.v1.events.EventsGrpc.EventsBlockingStub;
import armonik.client.event.impl.util.factory.EventClientRequestFactory;
import armonik.client.event.impl.util.records.EventUpdateResponse;
import armonik.client.event.spec.IEventClientSync;
import com.google.common.collect.Lists;
import io.grpc.ManagedChannel;

import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;

public class EventClientSync implements IEventClientSync {

  private final EventsBlockingStub blockingStub;


  public EventClientSync(ManagedChannel managedChannel) {
    this.blockingStub = EventsGrpc.newBlockingStub(managedChannel);
  }


  @Override
  public Map<String, EventUpdateResponse> getEventsUpdateBySessionId(String sessionId, List<String> resultIds) {
    EventSubscriptionRequest request = EventClientRequestFactory.CreateEventSubscriptionRequest(sessionId, resultIds);

    List<EventSubscriptionResponse> subscriptionResponses = Lists.newArrayList(blockingStub.getEvents(request));

    return subscriptionResponses.stream()
      .collect(Collectors.toMap(
        EventSubscriptionResponse::getSessionId,
        esr -> new EventUpdateResponse(
          esr.getTaskStatusUpdate(),
          esr.getResultStatusUpdate(),
          esr.getResultOwnerUpdate(),
          esr.getNewTask(),
          esr.getNewResult())
      ));
  }
}
