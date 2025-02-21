package armonik.client.event;

import static armonik.api.grpc.v1.events.EventsCommon.EventsEnum.EVENTS_ENUM_NEW_RESULT;
import static armonik.api.grpc.v1.events.EventsCommon.EventsEnum.EVENTS_ENUM_RESULT_STATUS_UPDATE;
import static armonik.api.grpc.v1.results.ResultsFields.ResultRawEnumField.RESULT_RAW_ENUM_FIELD_RESULT_ID;

import java.util.ArrayList;
import java.util.Collection;
import java.util.HashSet;
import java.util.List;
import java.util.Set;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import armonik.api.grpc.v1.FiltersCommon;
import armonik.api.grpc.v1.events.EventsCommon.EventSubscriptionRequest;
import armonik.api.grpc.v1.events.EventsCommon.EventSubscriptionResponse;
import armonik.api.grpc.v1.events.EventsGrpc;
import armonik.api.grpc.v1.result_status.ResultStatusOuterClass.ResultStatus;
import armonik.api.grpc.v1.results.ResultsFields;
import armonik.api.grpc.v1.results.ResultsFilters;
import io.grpc.ManagedChannel;
import io.grpc.stub.StreamObserver;

/**
 * EventClient is a client for interacting with event-related functionalities.
 * It communicates with a gRPC server using a blocking stub to retrieve events.
 */
public class EventClient {
  /** The blocking and nonblocking stub for communicating with the gRPC server. */
  private final EventsGrpc.EventsStub eventsStub;

  /**
   * Constructs a new EventClient with the specified managed channel.
   *
   * @param managedChannel the managed channel used for communication with the
   *                       server
   */
  public EventClient(ManagedChannel managedChannel) {
    eventsStub = EventsGrpc.newStub(managedChannel);
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

  /**
   * Waits asynchronously until the given results are completed.
   * <p>
   * This method subscribes to result status update events from the gRPC event
   * stream and waits for the specified results to reach a completed state. It
   * processes results in parallel using a thread pool with configurable bucket
   * sizes for efficiency.
   * </p>
   *
   * @param sessionId   The session ID where the results are located.
   * @param resultIds   A collection of result IDs to wait for.
   * @param bucketSize  The number of result IDs per request to the event API.
   * @param parallelism The number of parallel threads to use, where each thread
   *                    processes one bucket of results.
   * @return A {@link CompletableFuture} that completes when all specified results
   *         have been successfully processed.
   * @throws RuntimeException if any result is aborted.
   */
  public CompletableFuture<Void> waitForResultsAsync(String sessionId,
      Collection<String> resultIds,
      int bucketSize,
      int parallelism) {
    List<List<String>> chunks = chunkList(new ArrayList<>(resultIds), bucketSize);
    ExecutorService executor = Executors.newFixedThreadPool(parallelism);
    List<CompletableFuture<Void>> futures = new ArrayList<>();

    for (List<String> chunk : chunks) {
      futures.add(CompletableFuture.runAsync(() -> {
        Set<String> resultsNotFound = new HashSet<>(chunk);
        while (!resultsNotFound.isEmpty()) {
          EventSubscriptionRequest request = CreateEventSubscriptionRequest(sessionId, chunk);

          CountDownLatch latch = new CountDownLatch(1);

          eventsStub.getEvents(request, new StreamObserver<EventSubscriptionResponse>() {
            @Override
            public void onNext(EventSubscriptionResponse response) {
              if (response.getUpdateCase() == EventSubscriptionResponse.UpdateCase.RESULT_STATUS_UPDATE &&
                  resultsNotFound.contains(response.getResultStatusUpdate().getResultId())) {
                if (response.getResultStatusUpdate()
                    .getStatus() == ResultStatus.RESULT_STATUS_COMPLETED) {
                  resultsNotFound.remove(response.getResultStatusUpdate().getResultId());
                  if (resultsNotFound.isEmpty()) {
                    latch.countDown();
                  }
                } else if (response.getResultStatusUpdate()
                    .getStatus() == ResultStatus.RESULT_STATUS_ABORTED) {
                  throw new RuntimeException("Result "
                      + response.getResultStatusUpdate().getResultId() + " has been aborted");
                }
              }

              if (response.getUpdateCase() == EventSubscriptionResponse.UpdateCase.NEW_RESULT &&
                  resultsNotFound.contains(response.getNewResult().getResultId())) {
                if (response.getNewResult().getStatus() == ResultStatus.RESULT_STATUS_COMPLETED) {
                  resultsNotFound.remove(response.getNewResult().getResultId());
                  if (resultsNotFound.isEmpty()) {
                    latch.countDown();
                  }
                } else if (response.getNewResult().getStatus() == ResultStatus.RESULT_STATUS_ABORTED) {
                  throw new RuntimeException(
                      "Result " + response.getNewResult().getResultId() + " has been aborted");
                }
              }
            }

            @Override
            public void onError(Throwable t) {
              latch.countDown();
            }

            @Override
            public void onCompleted() {
              latch.countDown();
            }
          });

          try {
            latch.await();
          } catch (InterruptedException ignored) {
          }
        }
      }, executor));
    }

    return CompletableFuture.allOf(futures.toArray(new CompletableFuture[0])).thenRun(executor::shutdown);
  }

  private static List<List<String>> chunkList(List<String> list, int chunkSize) {
    List<List<String>> chunks = new ArrayList<>();
    for (int i = 0; i < list.size(); i += chunkSize) {
      chunks.add(list.subList(i, Math.min(list.size(), i + chunkSize)));
    }
    return chunks;
  }
}
