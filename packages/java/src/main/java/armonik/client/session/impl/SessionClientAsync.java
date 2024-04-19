package armonik.client.session.impl;

import armonik.api.grpc.v1.Objects;
import armonik.api.grpc.v1.sessions.SessionsCommon;
import armonik.api.grpc.v1.sessions.SessionsCommon.*;
import armonik.api.grpc.v1.sessions.SessionsCommon.ListSessionsRequest.Sort;
import armonik.api.grpc.v1.sessions.SessionsFilters.Filters;
import armonik.api.grpc.v1.sessions.SessionsGrpc;
import armonik.api.grpc.v1.sessions.SessionsGrpc.SessionsStub;
import armonik.client.session.impl.util.SessionClientRequestFactory;
import armonik.client.session.spec.ISessionClientAsync;
import io.grpc.ManagedChannel;
import io.grpc.stub.StreamObserver;

import java.util.List;

public class SessionClientAsync implements ISessionClientAsync {
  private final SessionsStub sessionsStub;

  /**
   * Constructs a new ArmonikSession object using the provided gRPC ManagedChannel.
   *
   * @param managedChannel The gRPC ManagedChannel to communicate with the Armonik service.
   */

  public SessionClientAsync(ManagedChannel managedChannel) {
    sessionsStub = SessionsGrpc.newStub(managedChannel);
  }

  @Override
  public void createSession(Objects.TaskOptions taskOptions, List<String> partitionIds, StreamObserver<String> streamObserver) {
    StreamObserver<CreateSessionReply> observer = new StreamObserver<>() {
      @Override
      public void onNext(CreateSessionReply createSessionReply) {
        streamObserver.onNext(createSessionReply.getSessionId());
      }

      @Override
      public void onError(Throwable throwable) {
        streamObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        streamObserver.onCompleted();
      }
    };

    List<String> partIds = partitionIds == null || partitionIds.isEmpty() ? null : partitionIds;
    SessionsCommon.CreateSessionRequest request = SessionsCommon.CreateSessionRequest.newBuilder()
      .setDefaultTaskOption(taskOptions)
      .addAllPartitionIds(partIds)
      .build();
    sessionsStub.createSession(request, observer);
  }


  @Override
  public void getSession(String sessionId, StreamObserver<SessionRaw> rawStreamObserver) {
    StreamObserver<SessionsCommon.GetSessionResponse> observer = new StreamObserver<>() {
      @Override
      public void onNext(GetSessionResponse getSessionResponse) {
        rawStreamObserver.onNext(getSessionResponse.getSession());
      }

      @Override
      public void onError(Throwable throwable) {
        rawStreamObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        rawStreamObserver.onCompleted();
      }
    };

    GetSessionRequest request = SessionClientRequestFactory.createGetSessionRequest(sessionId);


    sessionsStub.getSession(request, observer);
  }

  @Override
  public void listSessions(int page, int pageSize, Filters filter, Sort sort, StreamObserver<List<SessionRaw>> listStreamObserver) {
    ListSessionsRequest request = SessionClientRequestFactory.createListSessionsRequest(page, pageSize, filter, sort);
    StreamObserver<ListSessionsResponse> observer = new StreamObserver<ListSessionsResponse>() {
      @Override
      public void onNext(ListSessionsResponse listSessionsResponse) {
        listStreamObserver.onNext(listSessionsResponse.getSessionsList());
      }

      @Override
      public void onError(Throwable throwable) {
        listStreamObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
          listStreamObserver.onCompleted();
      }
    };
    sessionsStub.listSessions(request, observer);
  }

  @Override
  public void cancelSession(String sessionId, StreamObserver<SessionRaw> sessionRawStreamObserver) throws Exception {
    CancelSessionRequest request = SessionClientRequestFactory.createCancelSessionRequest(sessionId);

    StreamObserver<CancelSessionResponse> observer = new StreamObserver<>() {
      @Override
      public void onNext(CancelSessionResponse cancelSessionResponse) {
        sessionRawStreamObserver.onNext(cancelSessionResponse.getSession());
      }

      @Override
      public void onError(Throwable throwable) {
        sessionRawStreamObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        sessionRawStreamObserver.onCompleted();
      }
    };
    sessionsStub.cancelSession(request,observer);
  }

  @Override
  public void pauseSession(String sessionId, StreamObserver<SessionRaw> sessionRawStreamObserver) {
    PauseSessionRequest request = SessionClientRequestFactory.createPauseSessionRequest(sessionId);

    StreamObserver<PauseSessionResponse> observer = new StreamObserver<>() {
      @Override
      public void onNext(PauseSessionResponse pauseSessionResponse) {
        sessionRawStreamObserver.onNext(pauseSessionResponse.getSession());
      }

      @Override
      public void onError(Throwable throwable) {
        sessionRawStreamObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        sessionRawStreamObserver.onCompleted();
      }
    };
    sessionsStub.pauseSession(request,observer);
  }

  @Override
  public void resumeSession(String sessionId, StreamObserver<SessionRaw> sessionRawStreamObserver) {
    ResumeSessionRequest request = SessionClientRequestFactory.createResumeSessionRequest(sessionId);

    StreamObserver<ResumeSessionResponse> observer = new StreamObserver<>() {
      @Override
      public void onNext(ResumeSessionResponse resumeSessionResponse) {
        sessionRawStreamObserver.onNext(resumeSessionResponse.getSession());
      }

      @Override
      public void onError(Throwable throwable) {
        sessionRawStreamObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        sessionRawStreamObserver.onCompleted();
      }
    };
    sessionsStub.resumeSession(request,observer);
  }

  @Override
  public void closeSession(String sessionId, StreamObserver<SessionRaw> sessionRawStreamObserver){
    CloseSessionRequest closeSessionRequest = SessionClientRequestFactory.createCloseSessionRequest(sessionId);
    StreamObserver<CloseSessionResponse> observer = new StreamObserver<>() {
      @Override
      public void onNext(CloseSessionResponse closeSessionResponse) {
        sessionRawStreamObserver.onNext(closeSessionResponse.getSession());
      }

      @Override
      public void onError(Throwable throwable) {
        sessionRawStreamObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        sessionRawStreamObserver.onCompleted();
      }
    };
    sessionsStub.closeSession(closeSessionRequest,observer);
  }

  @Override
  public void purgeSession(String sessionId, StreamObserver<SessionRaw> sessionRawStreamObserver){
    PurgeSessionRequest purgeSessionRequest = SessionClientRequestFactory.createPurgeSessionRequest(sessionId);

    StreamObserver<PurgeSessionResponse> observer = new StreamObserver<>() {
      @Override
      public void onNext(PurgeSessionResponse purgeSessionResponse) {
        sessionRawStreamObserver.onNext(purgeSessionResponse.getSession());
      }

      @Override
      public void onError(Throwable throwable) {
        sessionRawStreamObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        sessionRawStreamObserver.onCompleted();
      }
    };
    sessionsStub.purgeSession(purgeSessionRequest,observer);
  }

  @Override
  public void deleteSession(String sessionId, StreamObserver<SessionRaw> sessionRawStreamObserver){
    DeleteSessionRequest deleteSessionRequest = SessionClientRequestFactory.createDeleteSessionRequest(sessionId);

    StreamObserver<DeleteSessionResponse> observer = new StreamObserver<>() {
      @Override
      public void onNext(DeleteSessionResponse deleteSessionResponse) {
        sessionRawStreamObserver.onNext(deleteSessionResponse.getSession());
      }

      @Override
      public void onError(Throwable throwable) {
        sessionRawStreamObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        sessionRawStreamObserver.onCompleted();
      }
    };

    sessionsStub.deleteSession(deleteSessionRequest,observer);
  }

  @Override
  public void stopSubmissionSession(String sessionId, boolean client, boolean worker, StreamObserver<SessionRaw> sessionRawStreamObserver){
    StopSubmissionRequest request = SessionClientRequestFactory.createStopSubmissionSessionRequest(sessionId, client, worker);


    StreamObserver<StopSubmissionResponse> observer = new StreamObserver<>() {
      @Override
      public void onNext(StopSubmissionResponse stopSubmissionResponse) {
        sessionRawStreamObserver.onNext(stopSubmissionResponse.getSession());
      }

      @Override
      public void onError(Throwable throwable) {
        sessionRawStreamObserver.onError(throwable);
      }

      @Override
      public void onCompleted() {
        sessionRawStreamObserver.onCompleted();
      }
    };
    sessionsStub.stopSubmission(request,observer);
  }



}
