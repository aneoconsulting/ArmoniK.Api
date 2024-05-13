package armonik.client.session;

import armonik.api.grpc.v1.Objects;
import armonik.api.grpc.v1.sessions.SessionsCommon;
import armonik.api.grpc.v1.sessions.SessionsCommon.SessionRaw;
import armonik.client.session.util.ListSessionRequestParams;
import io.grpc.ManagedChannel;
import org.checkerframework.checker.units.qual.C;

import java.util.List;
import java.util.concurrent.CompletableFuture;

/**
 * SessionClientFuture provides asynchronous operations for interacting with session-related functionalities.
 * It utilizes CompletableFuture to asynchronously perform operations using the SessionClient.
 */
public class SessionClientFuture {
  /** The SessionClient used for synchronous communication with the server. */
  private final SessionClient sessionClient;

  /**
   * Constructs a new SessionClientFuture with the specified managed channel.
   *
   * @param managedChannel the managed channel used for communication with the server
   */
  public SessionClientFuture(ManagedChannel managedChannel) {
    this.sessionClient = new SessionClient(managedChannel);
  }

  /**
   * Asynchronously creates a new session with the given task options and partition IDs.
   *
   * @param taskOptions the task options for the session
   * @param partitionIds the partition IDs for the session
   * @return a CompletableFuture representing the asynchronous operation to create a session
   */
  public CompletableFuture<String> createSession(Objects.TaskOptions taskOptions, List<String> partitionIds) {
    return CompletableFuture.supplyAsync(() -> sessionClient.createSession(taskOptions, partitionIds));
  }

  /**
   * Asynchronously retrieves the session with the specified session ID.
   *
   * @param sessionId the ID of the session to retrieve
   * @return a CompletableFuture representing the asynchronous operation to retrieve a session
   */
  public CompletableFuture<SessionRaw> getSession(String sessionId) {
    return CompletableFuture.supplyAsync(() -> sessionClient.getSession(sessionId));
  }

  /**
   * Asynchronously lists sessions based on the specified request parameters.
   *
   * @param requestParams the parameters for listing sessions
   * @return a CompletableFuture representing the asynchronous operation to list sessions
   */
  public CompletableFuture<List<SessionRaw>> listSessions(ListSessionRequestParams requestParams) {
    return CompletableFuture.supplyAsync(() -> sessionClient.listSessions(requestParams));
  }

  /**
   * Asynchronously cancels the session with the specified session ID.
   *
   * @param sessionId the ID of the session to cancel
   * @return a CompletableFuture representing the asynchronous operation to cancel a session
   */
  public CompletableFuture<SessionRaw> cancelSession(String sessionId) {
    return CompletableFuture.supplyAsync(() -> sessionClient.cancelSession(sessionId));
  }

  /**
   * Asynchronously pauses the session with the specified session ID.
   *
   * @param sessionId the ID of the session to pause
   * @return a CompletableFuture representing the asynchronous operation to pause a session
   */
  public CompletableFuture<SessionRaw> pauseSession(String sessionId) {
    return CompletableFuture.supplyAsync(() -> sessionClient.pauseSession(sessionId));
  }

  /**
   * Asynchronously resumes the session with the specified session ID.
   *
   * @param sessionId the ID of the session to resume
   * @return a CompletableFuture representing the asynchronous operation to resume a session
   */
  public CompletableFuture<SessionRaw> resumeSession(String sessionId) {
    return CompletableFuture.supplyAsync(() -> sessionClient.resumeSession(sessionId));
  }

  /**
   * Asynchronously closes the session with the specified session ID.
   *
   * @param sessionId the ID of the session to close
   * @return a CompletableFuture representing the asynchronous operation to close a session
   */
  public CompletableFuture<SessionRaw> closeSession(String sessionId) {
    return CompletableFuture.supplyAsync(() -> sessionClient.closeSession(sessionId));
  }

  /**
   * Asynchronously purges the session with the specified session ID.
   *
   * @param sessionId the ID of the session to purge
   * @return a CompletableFuture representing the asynchronous operation to purge a session
   */
  public CompletableFuture<SessionRaw> purgeSession(String sessionId) {
    return CompletableFuture.supplyAsync(() -> sessionClient.purgeSession(sessionId));
  }

  /**
   * Asynchronously deletes the session with the specified session ID.
   *
   * @param sessionId the ID of the session to delete
   * @return a CompletableFuture representing the asynchronous operation to delete a session
   */
  public CompletableFuture<SessionRaw> deleteSession(String sessionId) {
    return CompletableFuture.supplyAsync(() -> sessionClient.deleteSession(sessionId));
  }

  /**
   * Asynchronously stops submission for the session with the specified session ID.
   *
   * @param sessionId the ID of the session to stop submission for
   * @param client a boolean indicating whether to stop client submission
   * @param worker a boolean indicating whether to stop worker submission
   * @return a CompletableFuture representing the asynchronous operation to stop submission for a session
   */
  public CompletableFuture<SessionRaw> stopSubmissionSession(String sessionId, boolean client, boolean worker) {
    return CompletableFuture.supplyAsync(() -> sessionClient.stopSubmissionSession(sessionId, client, worker));
  }
}
