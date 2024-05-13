package armonik.client.event;

import static armonik.api.grpc.v1.events.EventsCommon.EventsEnum.EVENTS_ENUM_NEW_RESULT;
import static armonik.api.grpc.v1.events.EventsCommon.EventsEnum.EVENTS_ENUM_RESULT_STATUS_UPDATE;
import static armonik.api.grpc.v1.results.ResultsFields.ResultRawEnumField.RESULT_RAW_ENUM_FIELD_RESULT_ID;

import java.util.ArrayList;
import java.util.HashSet;
import java.util.Iterator;
import java.util.List;
import java.util.Set;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;

import armonik.api.grpc.v1.FiltersCommon;
import armonik.api.grpc.v1.events.EventsCommon.EventSubscriptionRequest;
import armonik.api.grpc.v1.events.EventsCommon.EventSubscriptionResponse;
import armonik.api.grpc.v1.events.EventsGrpc;
import armonik.api.grpc.v1.events.EventsGrpc.EventsBlockingStub;
import armonik.api.grpc.v1.results.ResultsFields;
import armonik.api.grpc.v1.results.ResultsFilters;
import armonik.client.event.util.records.EventSubscriptionResponseRecord;
import io.grpc.ManagedChannel;
import io.grpc.stub.StreamObserver;

/**
 * EventClient is a client for interacting with event-related functionalities.
 * It communicates with a gRPC server using a blocking stub to retrieve events.
 */
public class EventClient {
  /** The blocking and nonblocking stub for communicating with the gRPC server. */
  private final EventsBlockingStub eventsBlockingStub;
  private final EventsGrpc.EventsStub eventsStub;

  /**
   * Constructs a new EventClient with the specified managed channel.
   *
   * @param managedChannel the managed channel used for communication with the
   *                       server
   */
  public EventClient(ManagedChannel managedChannel) {
    eventsBlockingStub = EventsGrpc.newBlockingStub(managedChannel);
    eventsStub = EventsGrpc.newStub(managedChannel);
  }

  /**
   * Retrieves a list of event subscription response records for the given session
   * ID and result IDs.
   *
   * @param sessionId the session ID for which events are requested
   * @param resultIds the list of result IDs for which events are requested
   * @return a list of EventSubscriptionResponseRecord objects representing the
   *         events
   */
  public List<EventSubscriptionResponseRecord> getEvents(String sessionId, List<String> resultIds) {
    EventSubscriptionRequest request = CreateEventSubscriptionRequest(sessionId, resultIds);
    return mapToRecord(sessionId, request, resultIds);
  }

  /**
   * Maps the received event subscription response to
   * EventSubscriptionResponseRecord objects.
   *
   * @param sessionId the session ID for which events are being mapped
   * @param request   the event subscription request
   * @return a list of EventSubscriptionResponseRecord objects representing the
   *         events
   */
  private List<EventSubscriptionResponseRecord> mapToRecord(String sessionId, EventSubscriptionRequest request,
      List<String> resultIds) {
    List<EventSubscriptionResponseRecord> responseRecords = new ArrayList<>();
    Iterator<EventSubscriptionResponse> events = eventsBlockingStub.getEvents(request);
    Set<String> resultsExpected = new HashSet<>(resultIds);

    while (events.hasNext()) {
      var esr = events.next();
      resultsExpected.remove(esr.getNewResult().getResultId());
      responseRecords
          .add(new EventSubscriptionResponseRecord(sessionId,
              esr.getTaskStatusUpdate(),
              esr.getResultStatusUpdate(),
              esr.getResultOwnerUpdate(),
              esr.getNewTask(),
              esr.getNewResult()));
      if (resultsExpected.isEmpty()) {
        try {
          Thread.sleep(10000);
        } catch (InterruptedException e) {
          System.out.println("Thread was interrupted while sleeping");
        }
        break;
      }
    }
    return responseRecords;
  }

  /**
   * Retrieves a list of event subscription response records for the given session
   * asynchrone
   * ID and result IDs.
   *
   * @param sessionId the session ID for which events are requested
   * @param resultIds the list of result IDs for which events are requested
   * @return a list of EventSubscriptionResponseRecord objects representing the
   *         events
   * @throws InterruptedException
   */
  public List<EventSubscriptionResponseRecord> getEventResponseRecords(String sessionId, List<String> resultIds)
      throws InterruptedException {

    EventSubscriptionRequest request = CreateEventSubscriptionRequest(sessionId, resultIds);
    List<EventSubscriptionResponseRecord> responseRecords = new ArrayList<>();
    CountDownLatch finishLatch = new CountDownLatch(1);

    StreamObserver<EventSubscriptionResponse> responseObserver = new StreamObserver<EventSubscriptionResponse>() {

      @Override
      public void onNext(EventSubscriptionResponse esr) {
        responseRecords.add(new EventSubscriptionResponseRecord(
            sessionId,
            esr.getTaskStatusUpdate(),
            esr.getResultStatusUpdate(),
            esr.getResultOwnerUpdate(),
            esr.getNewTask(),
            esr.getNewResult()));
      }

      @Override
      public void onError(Throwable t) {
        t.printStackTrace();
        finishLatch.countDown();
      }

      @Override
      public void onCompleted() {
        System.out.println("Stream completed");
        finishLatch.countDown();
      }
    };

    eventsStub.getEvents(request, responseObserver);

    // Wait for the response observer to finish
    if (!finishLatch.await(1, TimeUnit.MINUTES)) {
      System.out.println("Request not completed within the timeout.");
    }

    return responseRecords;
  }

  /**
   * Creates an event subscription request with the specified session ID and
   * result IDs.
   *
   * @param sessionId the session ID for which event subscription is requested
   * @param resultIds the list of result IDs to filter events
   * @return an EventSubscriptionRequest object configured with the provided
   *         session ID and result IDs
   */
  public static EventSubscriptionRequest CreateEventSubscriptionRequest(String sessionId, List<String> resultIds) {
    FiltersCommon.FilterString filterString = FiltersCommon.FilterString.newBuilder()
        .setOperator(FiltersCommon.FilterStringOperator.FILTER_STRING_OPERATOR_EQUAL)
        .build();

    ResultsFields.ResultField.Builder resultField = ResultsFields.ResultField.newBuilder()
        .setResultRawField(ResultsFields.ResultRawField.newBuilder().setField(RESULT_RAW_ENUM_FIELD_RESULT_ID));

    ResultsFilters.FilterField.Builder filterFieldBuilder = ResultsFilters.FilterField.newBuilder()
        .setField(resultField)
        .setFilterString(filterString);

    ResultsFilters.Filters.Builder resultFiltersBuilder = ResultsFilters.Filters.newBuilder();
    for (String resultId : resultIds) {
      filterFieldBuilder.setFilterString(FiltersCommon.FilterString.newBuilder().setValue(resultId).build());
      resultFiltersBuilder.addOr(ResultsFilters.FiltersAnd.newBuilder().addAnd(filterFieldBuilder).build());
    }

    return EventSubscriptionRequest.newBuilder()
        .setResultsFilters(resultFiltersBuilder.build())
        .addReturnedEvents(EVENTS_ENUM_RESULT_STATUS_UPDATE)
        .addReturnedEvents(EVENTS_ENUM_NEW_RESULT)
        .setSessionId(sessionId)
        .build();
  }
}
