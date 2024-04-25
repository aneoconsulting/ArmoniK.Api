package armonik.client.event.impl;

import armonik.api.grpc.v1.events.EventsCommon.EventSubscriptionRequest;
import armonik.api.grpc.v1.events.EventsCommon.EventSubscriptionResponse;
import armonik.api.grpc.v1.events.EventsGrpc;
import armonik.client.event.impl.util.factory.EventClientRequestFactory;
import armonik.client.event.impl.util.records.EventUpdateResponse;
import armonik.client.event.spec.IEventClientAsync;
import io.grpc.ManagedChannel;
import io.grpc.stub.StreamObserver;

import java.util.List;
import java.util.Map;
import java.util.Map.Entry;

public class EventClientAsync implements IEventClientAsync {
  private final EventsGrpc.EventsStub eventsStub;

  public EventClientAsync(ManagedChannel managedChannel) {
    eventsStub = EventsGrpc.newStub(managedChannel);
  }

  /**
   * @param sessionId the id of the session
   * @param responseObserver A map containing responses keyed by session ID. The values are instances of {@link EventUpdateResponse}
   */
  @Override
  public void waitForEventsUpdateBySessionId(String sessionId, List<String> resultIds, StreamObserver<Entry<String, EventUpdateResponse>> responseObserver) {
    StreamObserver<EventSubscriptionResponse> observer = new StreamObserver<>() {
      @Override
      public void onNext(EventSubscriptionResponse esr) {
        responseObserver
          .onNext(
            Map.entry(
              sessionId,
              new EventUpdateResponse(
                esr.getTaskStatusUpdate(),
                esr.getResultStatusUpdate(),
                esr.getResultOwnerUpdate(),
                esr.getNewTask(),
                esr.getNewResult()
              )
            )
          );
      }

      @Override
      public void onError(Throwable throwable) {
        responseObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        responseObserver.onCompleted();
      }
    };
    EventSubscriptionRequest request = EventClientRequestFactory.CreateEventSubscriptionRequest(sessionId, resultIds);
    eventsStub.getEvents(request, observer);
  }
}
