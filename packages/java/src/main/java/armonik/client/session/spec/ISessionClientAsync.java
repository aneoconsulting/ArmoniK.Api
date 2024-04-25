package armonik.client.session.spec;

import armonik.api.grpc.v1.Objects;
import armonik.api.grpc.v1.sessions.SessionsCommon;
import armonik.api.grpc.v1.sessions.SessionsFilters;
import io.grpc.stub.StreamObserver;

import java.util.List;

public interface ISessionClientAsync {
  void createSession(Objects.TaskOptions taskOptions, List<String> partitionIds, StreamObserver<String> streamObserver);

  void getSession(String sessionId, StreamObserver<SessionsCommon.SessionRaw> rawStreamObserver);

  void listSessions(int page, int pageSize, SessionsFilters.Filters filter, SessionsCommon.ListSessionsRequest.Sort sort, StreamObserver<List<SessionsCommon.SessionRaw>> listStreamObserver);

  void cancelSession(String sessionId, StreamObserver<SessionsCommon.SessionRaw> sessionRawStreamObserver) throws Exception;

  void pauseSession(String sessionId, StreamObserver<SessionsCommon.SessionRaw> sessionRawStreamObserver);

  void resumeSession(String sessionId, StreamObserver<SessionsCommon.SessionRaw> sessionRawStreamObserver);

  void closeSession(String sessionId, StreamObserver<SessionsCommon.SessionRaw> sessionRawStreamObserver);

  void purgeSession(String sessionId, StreamObserver<SessionsCommon.SessionRaw> sessionRawStreamObserver);

  void deleteSession(String sessionId, StreamObserver<SessionsCommon.SessionRaw> sessionRawStreamObserver);

  void stopSubmissionSession(String sessionId, boolean client, boolean worker, StreamObserver<SessionsCommon.SessionRaw> sessionRawStreamObserver);
}
