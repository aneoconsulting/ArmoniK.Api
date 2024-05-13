/**
 * ArmonikSession provides a client interface to interact with Armonik session management functionality.
 * This class allows creating, retrieving, listing, canceling, pausing, resuming, closing, purging, deleting,
 * and stopping submissions for sessions in the Armonik system.
 */

package armonik.client.session;

import armonik.api.grpc.v1.Objects.TaskOptions;
import armonik.api.grpc.v1.sessions.SessionsCommon.*;
import armonik.api.grpc.v1.sessions.SessionsCommon.ListSessionsRequest.Sort;
import armonik.api.grpc.v1.sessions.SessionsFilters.Filters;
import armonik.api.grpc.v1.sessions.SessionsGrpc;
import armonik.api.grpc.v1.sessions.SessionsGrpc.SessionsBlockingStub;
import armonik.client.session.util.ListSessionRequestParams;
import armonik.client.session.util.SessionClientRequestFactory;
import io.grpc.ManagedChannel;

import java.util.List;

/**
 * SessionClient provides methods for interacting with session-related functionalities.
 * It communicates with a gRPC server using a blocking stub to perform various operations on sessions.
 */
public class SessionClient {
  /** The blocking stub for communicating with the gRPC server. */
  private final SessionsBlockingStub sessionsStub;

  /**
   * Constructs a new SessionClient with the specified managed channel.
   *
   * @param managedChannel the managed channel used for communication with the server
   */
  public SessionClient(ManagedChannel managedChannel) {
    sessionsStub = SessionsGrpc.newBlockingStub(managedChannel);
  }

  /**
   * Creates a new session with the given task options and partition IDs.
   *
   * @param taskOptions the task options for the session
   * @param partitionIds the partition IDs for the session
   * @return the ID of the created session
   */
  public String createSession(TaskOptions taskOptions, List<String> partitionIds) {
    CreateSessionRequest request = SessionClientRequestFactory.createSessionRequest(taskOptions, partitionIds);
    return sessionsStub.createSession(request).getSessionId();
  }

  /**
   * Retrieves the session with the specified session ID.
   *
   * @param sessionId the ID of the session to retrieve
   * @return the SessionRaw object representing the retrieved session
   */
  public SessionRaw getSession(String sessionId) {
    GetSessionRequest request = SessionClientRequestFactory.createGetSessionRequest(sessionId);
    return sessionsStub.getSession(request).getSession();
  }

  /**
   * Lists sessions based on the specified request parameters.
   *
   * @param requestParams the parameters for listing sessions
   * @return a list of SessionRaw objects representing the retrieved sessions
   */
  public List<SessionRaw> listSessions(ListSessionRequestParams requestParams) {
    ListSessionsRequest request = SessionClientRequestFactory.createListSessionsRequest(
      requestParams.page(),
      requestParams.pageSize(),
      requestParams.filter(),
      requestParams.sort()
    );
    return sessionsStub.listSessions(request).getSessionsList();
  }

  /**
   * Cancels the session with the specified session ID.
   *
   * @param sessionId the ID of the session to cancel
   * @return the SessionRaw object representing the canceled session
   */
  public SessionRaw cancelSession(String sessionId) {
    CancelSessionRequest request = SessionClientRequestFactory.createCancelSessionRequest(sessionId);
    return sessionsStub.cancelSession(request).getSession();
  }

  /**
   * Pauses the session with the specified session ID.
   *
   * @param sessionId the ID of the session to pause
   * @return the SessionRaw object representing the paused session
   */
  public SessionRaw pauseSession(String sessionId) {
    PauseSessionRequest request = SessionClientRequestFactory.createPauseSessionRequest(sessionId);
    return sessionsStub.pauseSession(request).getSession();
  }

  /**
   * Resumes the session with the specified session ID.
   *
   * @param sessionId the ID of the session to resume
   * @return the SessionRaw object representing the resumed session
   */
  public SessionRaw resumeSession(String sessionId) {
    ResumeSessionRequest request = SessionClientRequestFactory.createResumeSessionRequest(sessionId);
    return sessionsStub.resumeSession(request).getSession();
  }


  /**
   * Closes the session with the specified session ID.
   *
   * @param sessionId the ID of the session to close
   * @return the SessionRaw object representing the closed session
   */
  public SessionRaw closeSession(String sessionId) {
    CloseSessionRequest closeSessionRequest = SessionClientRequestFactory.createCloseSessionRequest(sessionId);
    return sessionsStub.closeSession(closeSessionRequest).getSession();
  }
  /**
   * Purges the session with the specified session ID.
   *
   * @param sessionId the ID of the session to purge
   * @return the SessionRaw object representing the purged session
   */
  public SessionRaw purgeSession(String sessionId) {
    PurgeSessionRequest purgeSessionRequest = SessionClientRequestFactory.createPurgeSessionRequest(sessionId);
    return sessionsStub.purgeSession(purgeSessionRequest).getSession();
  }

  /**
   * Deletes the session with the specified session ID.
   *
   * @param sessionId the ID of the session to delete
   * @return the SessionRaw object representing the deleted session
   */
  public SessionRaw deleteSession(String sessionId) {
    DeleteSessionRequest deleteSessionRequest = SessionClientRequestFactory.createDeleteSessionRequest(sessionId);
    return sessionsStub.deleteSession(deleteSessionRequest).getSession();
  }

  /**
   * Stops submission for the session with the specified session ID.
   *
   * @param sessionId the ID of the session to stop submission for
   * @param client a boolean indicating whether to stop client submission
   * @param worker a boolean indicating whether to stop worker submission
   * @return the SessionRaw object representing the session with stopped submission
   */
  public SessionRaw stopSubmissionSession(String sessionId, boolean client, boolean worker) {
    StopSubmissionRequest request = SessionClientRequestFactory.createStopSubmissionSessionRequest(sessionId, client, worker);
    return sessionsStub.stopSubmission(request).getSession();
  }


}
