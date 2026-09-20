package ak;

/**
 * ABI v1 section 9's RPC half: the core owns the transport and the host makes a call.
 *
 * <p>This is what outcome 2 actually proposes for the RPC layer. It is a different question
 * from the codec: {@code RunR14} measured the codec inside gRPC's marshaller, and an arm
 * that keeps grpc-java and swaps the marshaller measures the same thing again with a server
 * attached. Here the core's tonic client makes the call and grpc-java is the thing being
 * replaced rather than the thing being borrowed.
 *
 * <p><b>Two crossings per call and none per field.</b> {@link #callUnary} returns bytes the
 * core still owns, and {@link #bytesFree} gives them back. The copy in between is
 * {@code Unsafe.copyMemory} on the Java side, which is an intrinsic and not a crossing --
 * folding it into the call would have made this one crossing and flattered the arm against
 * its own specification.
 *
 * <p>Loaded from a SEPARATE library: the {@code rpc} feature links tonic and tokio, and the
 * codec arms must not carry them. That is why the feature exists and why this is not in
 * {@link Native}.
 */
public final class NativeRpc {
  private NativeRpc() {}

  private static boolean bound;

  public static synchronized void ensureBound() {
    if (bound) return;
    String lib = System.getProperty("ak.rpclib");
    if (lib == null) throw new IllegalStateException("-Dak.rpclib is unset");
    System.load(lib);
    bound = true;
  }

  public static native long runtimeNew(int workerThreads);
  public static native void runtimeDestroy(long rt);
  public static native long clientNew(long rt, byte[] uri, int len);
  public static native void clientDestroy(long client);

  /** Crossing one. {@code out} receives {ptr, len, owner}; the core still owns the bytes. */
  public static native int callUnary(long client, long pathPtr, int pathLen,
                                     byte[] req, int reqOff, int reqLen, long[] out);

  /** Crossing two, and the only other one. */
  public static native void bytesFree(long ptr, long len, long owner);
}
