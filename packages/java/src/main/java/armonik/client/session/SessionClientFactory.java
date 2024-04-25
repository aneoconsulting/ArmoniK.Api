package armonik.client.session;

import armonik.client.session.impl.SessionClientAsync;
import armonik.client.session.impl.SesssionClientSync;
import armonik.client.session.spec.ISessionClientAsync;
import armonik.client.session.spec.ISessionClientSync;
import io.grpc.ManagedChannel;

public abstract class SessionClientFactory {
  public static ISessionClientAsync createSessionClientAsync(ManagedChannel managedChannel) {
    return new SessionClientAsync(managedChannel);
  }

  public static ISessionClientSync createSesssionClientSync(ManagedChannel managedChannel) {
    return new SesssionClientSync(managedChannel);
  }
}
