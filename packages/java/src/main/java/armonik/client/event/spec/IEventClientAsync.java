package armonik.client.event.spec;

import armonik.client.event.impl.util.records.EventUpdateResponse;
import io.grpc.stub.StreamObserver;

import java.util.List;
import java.util.Map;

public interface IEventClientAsync {

  void waitForEventsUpdateBySessionId(String sessionId, List<String> resultIds, StreamObserver<Map.Entry<String, EventUpdateResponse>> responseObserver);
}
