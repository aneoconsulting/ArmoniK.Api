package armonik.client.event.spec;

import armonik.client.event.impl.util.records.EventSubscriptionResponseRecord;

import java.util.List;

public interface IEventClientSync {
  List<EventSubscriptionResponseRecord> getEventsUpdateBySessionId(String sessionId, List<String> resultIds);

}
