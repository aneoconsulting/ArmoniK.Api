package armonik.client.result;

import armonik.client.result.impl.ResultClientAsync;
import armonik.client.result.impl.ResultClientSync;
import armonik.client.result.spec.IResultClientAsync;
import armonik.client.result.spec.IResultClientSync;
import io.grpc.ManagedChannel;

public abstract class ResultClientFactory {
  public static IResultClientSync createResultClientSync(ManagedChannel managedChannel) {
    return new ResultClientSync(managedChannel);
  }

  public static IResultClientAsync createResultClientAsync(ManagedChannel managedChannel) {
    return new ResultClientAsync(managedChannel);
  }
}
