package armonik.client.session.spec;

import armonik.api.grpc.v1.Objects;
import armonik.api.grpc.v1.sessions.SessionsCommon.ListSessionsRequest.Sort;
import armonik.api.grpc.v1.sessions.SessionsCommon.SessionRaw;
import armonik.api.grpc.v1.sessions.SessionsFilters.Filters;

import java.util.List;

public interface ISessionClientSync {
  default String createSession(Objects.TaskOptions taskOptions, List<String> partitionIds) {
    throw new UnsupportedOperationException("Method not implemented");
  }

  default SessionRaw getSession(String sessionId) throws Exception {
    throw new UnsupportedOperationException("Method not implemented");
  }

  default List<SessionRaw> listSessions(int page, int pageSize, Filters filter, Sort sort) throws Exception {
    throw new UnsupportedOperationException("Method not implemented");
  }

  default SessionRaw cancelSession(String sessionId) throws Exception {
    throw new UnsupportedOperationException("Method not implemented");
  }

  default SessionRaw pauseSession(String sessionId) throws Exception {
    throw new UnsupportedOperationException("Method not implemented");
  }

  default SessionRaw resumeSession(String sessionId) throws Exception {
    throw new UnsupportedOperationException("Method not implemented");
  }

  default SessionRaw closeSession(String sessionId) throws Exception {
    throw new UnsupportedOperationException("Method not implemented");
  }

  default SessionRaw purgeSession(String sessionId) throws Exception {
    throw new UnsupportedOperationException("Method not implemented");
  }

  default SessionRaw deleteSession(String sessionId) throws Exception {
    throw new UnsupportedOperationException("Method not implemented");
  }

  default SessionRaw stopSubmissionSession(String sessionId, boolean client, boolean worker) throws Exception {
    throw new UnsupportedOperationException("Method not implemented");
  }


}
