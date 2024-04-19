/**
 * ArmonikSession provides a client interface to interact with Armonik session management functionality.
 * This class allows creating, retrieving, listing, canceling, pausing, resuming, closing, purging, deleting,
 * and stopping submissions for sessions in the Armonik system.
 */

package armonik.client.session.impl;

import armonik.api.grpc.v1.Objects.TaskOptions;
import armonik.api.grpc.v1.sessions.SessionsCommon.*;
import armonik.api.grpc.v1.sessions.SessionsCommon.ListSessionsRequest.Sort;
import armonik.api.grpc.v1.sessions.SessionsFilters.Filters;
import armonik.api.grpc.v1.sessions.SessionsGrpc;
import armonik.client.session.impl.util.SessionClientRequestFactory;
import armonik.client.session.spec.ISessionClientSync;
import io.grpc.ManagedChannel;

import java.util.List;

public class SesssionClientSync implements ISessionClientSync {
  private final SessionsGrpc.SessionsBlockingStub sessionsStub;

  /**
   * Constructs a new ArmonikSession object using the provided gRPC ManagedChannel.
   *
   * @param managedChannel The gRPC ManagedChannel to communicate with the Armonik service.
   */

  public SesssionClientSync(ManagedChannel managedChannel){
    sessionsStub = SessionsGrpc.newBlockingStub(managedChannel);
  }

  /**
   * Creates a new session in the Armonik system with the given task options and partition IDs.
   *
   * @param taskOptions  The task options for the session.
   * @param partitionIds The list of partition IDs.
   * @return The ID of the created session.
   */
  @Override
  public String createSession(TaskOptions taskOptions, List<String> partitionIds) {
    CreateSessionRequest request = SessionClientRequestFactory.createSessionRequest(taskOptions, partitionIds);
    return sessionsStub.createSession(request).getSessionId();
  }



  /**
   * Retrieves session information for the specified session ID.
   *
   * @param sessionId The ID of the session to retrieve.
   * @return The session information.
   */
  @Override
  public SessionRaw getSession(String sessionId) {
    GetSessionRequest request = SessionClientRequestFactory.createGetSessionRequest(sessionId);
    return sessionsStub.getSession(request).getSession();
  }

  /**
   * Lists sessions based on pagination, filters, and sorting options.
   *
   * @param page     The page number.
   * @param pageSize The size of each page.
   * @param filter   The filters to apply.
   * @param sort     The sorting options.
   * @return A ListSessionsResult object containing the list of sessions and total count.
   */
  @Override
  public List<SessionRaw> listSessions(int page, int pageSize, Filters filter, Sort sort) {
    ListSessionsRequest request = SessionClientRequestFactory.createListSessionsRequest(page, pageSize, filter, sort);
    return sessionsStub.listSessions(request).getSessionsList();
  }

  /**
   * Cancels the session with the specified session ID.
   *
   * @param sessionId The ID of the session to cancel.
   * @return The updated session information after cancellation.
   */
  @Override
  public SessionRaw cancelSession(String sessionId) {
    CancelSessionRequest request = SessionClientRequestFactory.createCancelSessionRequest(sessionId);
    return sessionsStub.cancelSession(request).getSession();
  }

  /**
   * Pauses the session with the specified session ID.
   *
   * @param sessionId The ID of the session to pause.
   * @return The updated session information after pausing.
   */
  @Override
  public SessionRaw pauseSession(String sessionId) {
    PauseSessionRequest request = SessionClientRequestFactory.createPauseSessionRequest(sessionId);
    return sessionsStub.pauseSession(request).getSession();
  }

  /**
   * Resumes the session with the specified session ID.
   *
   * @param sessionId The ID of the session to resume.
   * @return The updated session information after resuming.
   */
  @Override
  public SessionRaw resumeSession(String sessionId) {
    ResumeSessionRequest request = SessionClientRequestFactory.createResumeSessionRequest(sessionId);
    return sessionsStub.resumeSession(request).getSession();
  }

  /**
   * Closes the session with the specified session ID.
   *
   * @param sessionId The ID of the session to close.
   * @return the closed session
   */
  @Override
  public SessionRaw closeSession(String sessionId){
    CloseSessionRequest closeSessionRequest = SessionClientRequestFactory.createCloseSessionRequest(sessionId);
    return sessionsStub.closeSession(closeSessionRequest).getSession();
  }

  /**
   * Purges the session with the specified session ID.
   *
   * @param sessionId The ID of the session to purge.
   * @return The updated session information after purging.
   */
  @Override
  public SessionRaw purgeSession(String sessionId){
    PurgeSessionRequest purgeSessionRequest = SessionClientRequestFactory.createPurgeSessionRequest(sessionId);
    return sessionsStub.purgeSession(purgeSessionRequest).getSession();
  }

  /**
   * Deletes the session with the specified session ID.
   *
   * @param sessionId The ID of the session to delete.
   * @return The updated session information after deletion.
   */
  @Override
  public SessionRaw deleteSession(String sessionId){
    DeleteSessionRequest deleteSessionRequest = SessionClientRequestFactory.createDeleteSessionRequest(sessionId);
    return sessionsStub.deleteSession(deleteSessionRequest).getSession();
  }

  /**
   * Stops submission for the session with the specified session ID.
   *
   * @param sessionId The ID of the session to stop submission for.
   * @param client    Boolean indicating whether to stop client submissions.
   * @param worker    Boolean indicating whether to stop worker submissions.
   * @return The updated session information after stopping submissions.
   */
  @Override
  public SessionRaw stopSubmissionSession(String sessionId, boolean client, boolean worker){
    StopSubmissionRequest request = SessionClientRequestFactory.createStopSubmissionSessionRequest(sessionId, client, worker);
    return sessionsStub.stopSubmission(request).getSession();
  }


}
