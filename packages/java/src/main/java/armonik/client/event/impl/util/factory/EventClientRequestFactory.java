package armonik.client.event.impl.util.factory;

import armonik.api.grpc.v1.FiltersCommon;
import armonik.api.grpc.v1.events.EventsCommon.EventSubscriptionRequest;
import armonik.api.grpc.v1.results.ResultsFields;
import armonik.api.grpc.v1.results.ResultsFilters;

import java.util.List;

import static armonik.api.grpc.v1.events.EventsCommon.EventsEnum.EVENTS_ENUM_NEW_RESULT;
import static armonik.api.grpc.v1.events.EventsCommon.EventsEnum.EVENTS_ENUM_RESULT_STATUS_UPDATE;
import static armonik.api.grpc.v1.results.ResultsFields.ResultRawEnumField.RESULT_RAW_ENUM_FIELD_RESULT_ID;

/**
 * EventClientRequestFactory provides static methods to create event subscription requests.
 * It constructs EventSubscriptionRequest objects based on provided session ID and result IDs.
 */
public class EventClientRequestFactory {

  /**
   * Creates an event subscription request with the specified session ID and result IDs.
   *
   * @param sessionId  the session ID for which event subscription is requested
   * @param resultIds  the list of result IDs to filter events
   * @return an EventSubscriptionRequest object configured with the provided session ID and result IDs
   */
  public static EventSubscriptionRequest CreateEventSubscriptionRequest(String sessionId, List<String> resultIds){
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
