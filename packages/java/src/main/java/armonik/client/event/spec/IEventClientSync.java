package armonik.client.event.spec;

import armonik.client.event.impl.util.records.EventUpdateResponse;

import java.util.List;
import java.util.Map;

public interface IEventClientSync {
  Map<String, EventUpdateResponse> getEventsUpdateBySessionId(String sessionId, List<String> resultIds);

}
