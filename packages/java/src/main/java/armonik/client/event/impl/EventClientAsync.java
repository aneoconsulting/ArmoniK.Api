package armonik.client.event.impl;

import armonik.api.grpc.v1.events.EventsCommon.EventSubscriptionRequest;
import armonik.api.grpc.v1.events.EventsCommon.EventSubscriptionResponse;
import armonik.api.grpc.v1.events.EventsGrpc;
import armonik.client.event.impl.util.factory.EventClientRequestFactory;
import armonik.client.event.impl.util.records.EventSubscriptionResponseRecord;
import armonik.client.event.spec.IEventClientAsync;
import io.grpc.ManagedChannel;
import io.grpc.stub.StreamObserver;

import java.util.List;

/**
 * EventClientAsync is an asynchronous implementation of the {@link IEventClientAsync} interface.
 * It communicates with the event service using a non-blocking stub, making asynchronous calls to wait for event updates.
 */
public class EventClientAsync implements IEventClientAsync {
  private final EventsGrpc.EventsStub eventsStub;

  /**
   * Constructs an EventClientAsync object with the provided managed channel.
   *
   * @param managedChannel the managed channel used for communication with the event service
   */
  public EventClientAsync(ManagedChannel managedChannel) {
    eventsStub = EventsGrpc.newStub(managedChannel);
  }


  /**
   * Waits for event updates for the specified session ID and result IDs, and sends responses to the provided StreamObserver.
   *
   * @param sessionId        the ID of the session for which event updates are awaited
   * @param resultIds        the list of result IDs for which event updates are awaited
   * @param responseObserver a StreamObserver to handle the responses, containing entry of EventSubscriptionResponseRecord
   */
  @Override
  public void waitForEventsUpdateBySessionId(String sessionId, List<String> resultIds, StreamObserver<EventSubscriptionResponseRecord> responseObserver) {
    StreamObserver<EventSubscriptionResponse> proxyObserver = new StreamObserver<>() {
      @Override
      public void onNext(EventSubscriptionResponse esr) {
        responseObserver
          .onNext(
            new EventSubscriptionResponseRecord(
              sessionId,
              esr.getTaskStatusUpdate(),
              esr.getResultStatusUpdate(),
              esr.getResultOwnerUpdate(),
              esr.getNewTask(),
              esr.getNewResult()
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
    eventsStub.getEvents(request, proxyObserver);
  }
}
