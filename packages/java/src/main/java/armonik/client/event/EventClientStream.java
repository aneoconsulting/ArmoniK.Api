package armonik.client.event;

import armonik.api.grpc.v1.events.EventsCommon.EventSubscriptionRequest;
import armonik.api.grpc.v1.events.EventsCommon.EventSubscriptionResponse;
import armonik.api.grpc.v1.events.EventsGrpc;
import armonik.api.grpc.v1.events.EventsGrpc.EventsStub;
import armonik.client.event.util.factory.EventClientRequestFactory;
import io.grpc.ManagedChannel;
import io.grpc.stub.StreamObserver;

import java.util.List;


@Deprecated(forRemoval = true)
public class EventClientStream {
  private final EventsStub eventsStub;

  public EventClientStream(ManagedChannel managedChannel) {
    eventsStub = EventsGrpc.newStub(managedChannel);
  }

  public void getEvents(String sessionId, List<String> resultIds, StreamObserver<EventSubscriptionResponse> observer) {
    EventSubscriptionRequest request = EventClientRequestFactory.CreateEventSubscriptionRequest(sessionId, resultIds);
    eventsStub.getEvents(request, observer);
  }
}
