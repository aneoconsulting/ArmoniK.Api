package armonik.client.event.spec;

import armonik.client.event.impl.util.records.EventSubscriptionResponseRecord;
import io.grpc.stub.StreamObserver;

import java.util.List;

public interface IEventClientAsync {
  void waitForEventsUpdateBySessionId(String sessionId, List<String> resultIds, StreamObserver<EventSubscriptionResponseRecord> responseObserver);
}
