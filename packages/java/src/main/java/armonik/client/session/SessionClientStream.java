package armonik.client.session;

import armonik.api.grpc.v1.Objects;
import armonik.api.grpc.v1.Objects.TaskOptions;
import armonik.api.grpc.v1.sessions.SessionsCommon;
import armonik.api.grpc.v1.sessions.SessionsCommon.*;
import armonik.api.grpc.v1.sessions.SessionsCommon.ListSessionsRequest.Sort;
import armonik.api.grpc.v1.sessions.SessionsFilters.Filters;
import armonik.api.grpc.v1.sessions.SessionsGrpc;
import armonik.api.grpc.v1.sessions.SessionsGrpc.SessionsStub;
import armonik.client.session.util.ListSessionRequestParams;
import armonik.client.session.util.SessionClientRequestFactory;
import io.grpc.ManagedChannel;
import io.grpc.stub.StreamObserver;

import java.util.List;

@Deprecated(forRemoval = true)
public class SessionClientStream {
  private final SessionsStub sessionsStub;

  public SessionClientStream(ManagedChannel managedChannel) {
    sessionsStub = SessionsGrpc.newStub(managedChannel);
  }

  public void createSession(TaskOptions taskOptions, List<String> partitionIds, StreamObserver<CreateSessionReply> observer) {
    List<String> partIds = partitionIds == null || partitionIds.isEmpty() ? null : partitionIds;
    SessionsCommon.CreateSessionRequest request = SessionsCommon.CreateSessionRequest.newBuilder()
      .setDefaultTaskOption(taskOptions)
      .addAllPartitionIds(partIds)
      .build();
    sessionsStub.createSession(request, observer);
  }


  public void getSession(String sessionId, StreamObserver<SessionsCommon.GetSessionResponse> observer) {
    GetSessionRequest request = SessionClientRequestFactory.createGetSessionRequest(sessionId);
    sessionsStub.getSession(request, observer);
  }

  public void listSessions(ListSessionRequestParams requestParams, StreamObserver<ListSessionsResponse> observer) {
    ListSessionsRequest request = SessionClientRequestFactory.createListSessionsRequest(
      requestParams.page(),
      requestParams.pageSize(),
      requestParams.filter(),
      requestParams.sort());
    sessionsStub.listSessions(request, observer);
  }

  public void cancelSession(String sessionId, StreamObserver<CancelSessionResponse> observer) throws Exception {
    CancelSessionRequest request = SessionClientRequestFactory.createCancelSessionRequest(sessionId);
    sessionsStub.cancelSession(request, observer);
  }

  public void pauseSession(String sessionId, StreamObserver<PauseSessionResponse> observer) {
    PauseSessionRequest request = SessionClientRequestFactory.createPauseSessionRequest(sessionId);
    sessionsStub.pauseSession(request, observer);
  }

  public void resumeSession(String sessionId, StreamObserver<ResumeSessionResponse> observer) {
    ResumeSessionRequest request = SessionClientRequestFactory.createResumeSessionRequest(sessionId);
    sessionsStub.resumeSession(request, observer);
  }

  public void closeSession(String sessionId, StreamObserver<CloseSessionResponse> observer) {
    CloseSessionRequest closeSessionRequest = SessionClientRequestFactory.createCloseSessionRequest(sessionId);
    sessionsStub.closeSession(closeSessionRequest, observer);
  }

  public void purgeSession(String sessionId, StreamObserver<PurgeSessionResponse> observer) {
    PurgeSessionRequest purgeSessionRequest = SessionClientRequestFactory.createPurgeSessionRequest(sessionId);
    sessionsStub.purgeSession(purgeSessionRequest, observer);
  }

  public void deleteSession(String sessionId, StreamObserver<DeleteSessionResponse> observer) {
    DeleteSessionRequest deleteSessionRequest = SessionClientRequestFactory.createDeleteSessionRequest(sessionId);
    sessionsStub.deleteSession(deleteSessionRequest, observer);
  }

  public void stopSubmissionSession(String sessionId, boolean client, boolean worker, StreamObserver<StopSubmissionResponse> observer) {
    StopSubmissionRequest request = SessionClientRequestFactory.createStopSubmissionSessionRequest(sessionId, client, worker);
    sessionsStub.stopSubmission(request, observer);
  }


}
