package armonik.client.session;

import armonik.client.session.impl.SessionClientAsync;
import armonik.client.session.impl.SesssionClientSync;
import armonik.client.session.spec.ISessionClientAsync;
import armonik.client.session.spec.ISessionClientSync;
import io.grpc.ManagedChannel;

/**
 * SessionClientFactory provides static factory methods to create instances of session client implementations.
 */
public abstract class SessionClientFactory {

  /**
   * Creates an asynchronous session client instance with the provided managed channel.
   *
   * @param managedChannel the managed channel used for communication with the session service
   * @return an instance of {@link ISessionClientAsync} representing an asynchronous session client
   */
  public static ISessionClientAsync createSessionClientAsync(ManagedChannel managedChannel) {
    return new SessionClientAsync(managedChannel);
  }

  /**
   * Creates a synchronous session client instance with the provided managed channel.
   *
   * @param managedChannel the managed channel used for communication with the session service
   * @return an instance of {@link ISessionClientSync} representing a synchronous session client
   */
  public static ISessionClientSync createSesssionClientSync(ManagedChannel managedChannel) {
    return new SesssionClientSync(managedChannel);
  }
}
