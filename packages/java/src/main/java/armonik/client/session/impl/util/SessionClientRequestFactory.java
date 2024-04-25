package armonik.client.session.impl.util;

import armonik.api.grpc.v1.Objects;
import armonik.api.grpc.v1.sessions.SessionsCommon.*;
import armonik.api.grpc.v1.sessions.SessionsCommon.ListSessionsRequest.Sort;
import armonik.api.grpc.v1.sessions.SessionsFilters;

import java.util.List;

public abstract class SessionClientRequestFactory {


  public static CreateSessionRequest createSessionRequest(Objects.TaskOptions taskOptions, List<String> partitionIds) {
    List<String> partIds = partitionIds == null || partitionIds.isEmpty() ? null : partitionIds;
    return CreateSessionRequest.newBuilder()
      .setDefaultTaskOption(taskOptions)
      .addAllPartitionIds(partIds)
      .build();
  }

  public static GetSessionRequest createGetSessionRequest(String sessionId) {
    return GetSessionRequest
      .newBuilder()
      .setSessionId(sessionId)
      .build();
  }

  public static ListSessionsRequest createListSessionsRequest(int page, int pageSize, SessionsFilters.Filters filter, Sort sort) {
    return ListSessionsRequest.newBuilder()
      .setPage(page)
      .setPageSize(pageSize)
      .setFilters(filter)
      .setSort(sort)
      .build();
  }

  public static CancelSessionRequest createCancelSessionRequest(String sessionId) {
    return CancelSessionRequest.newBuilder().setSessionId(sessionId).build();
  }

  public static PauseSessionRequest createPauseSessionRequest(String sessionId) {
    return PauseSessionRequest.newBuilder().setSessionId(sessionId).build();
  }

  public static ResumeSessionRequest createResumeSessionRequest(String sessionId) {
    return ResumeSessionRequest.newBuilder().setSessionId(sessionId).build();
  }

  public static CloseSessionRequest createCloseSessionRequest(String sessionId) {
    return CloseSessionRequest.newBuilder().setSessionId(sessionId).build();
  }

  public static PurgeSessionRequest createPurgeSessionRequest(String sessionId) {
    return PurgeSessionRequest.newBuilder().setSessionId(sessionId).build();

  }

  public static DeleteSessionRequest createDeleteSessionRequest(String sessionId) {
    return DeleteSessionRequest.newBuilder().setSessionId(sessionId).build();
  }

  public static StopSubmissionRequest createStopSubmissionSessionRequest(String sessionId, boolean client, boolean worker) {
    return StopSubmissionRequest.newBuilder()
      .setSessionId(sessionId)
      .setClient(client)
      .setWorker(worker)
      .build();
  }
}
