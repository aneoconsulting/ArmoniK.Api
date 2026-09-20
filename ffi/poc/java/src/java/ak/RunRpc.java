package ak;

import ak.shapes.Arms;
import ak.shapes.Binding;
import ak.shapes.FfiArms;
import ak.shapes.PbArms;
import com.google.protobuf.Message;
import io.grpc.CallOptions;
import io.grpc.ManagedChannel;
import io.grpc.MethodDescriptor;
import io.grpc.Server;
import io.grpc.ServerServiceDefinition;
import io.grpc.netty.shaded.io.grpc.netty.NettyChannelBuilder;
import io.grpc.netty.shaded.io.grpc.netty.NettyServerBuilder;
import io.grpc.stub.ClientCalls;
import io.grpc.stub.ServerCalls;

import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.concurrent.atomic.AtomicLong;

/**
 * ABI v1 section 9: the RPC layer in the CORE, against the RPC layer in the host.
 *
 * <p><b>What this measures, and what the first version of this file did not.</b> Outcome 2
 * puts the RPC layer on the C ABI: the host calls {@code ak_call_unary} and tonic does the
 * HTTP/2. An arm that keeps grpc-java and swaps only the marshaller measures the codec
 * inside somebody else's transport, which {@code RunR14} already did without a server
 * attached. So the variable here is the whole client stack, not the codec.
 *
 * <pre>
 *   grpc-java : grpc-java client  + protobuf-java parse
 *   core-rpc  : ak_call_unary     + the pull family's parse     (tonic, tokio, hyper)
 * </pre>
 *
 * <p><b>One server, so the server is not a variable.</b> Both arms call the same grpc-java
 * server, which answers with a fixed, pre-encoded P2.2 body through a pass-through byte
 * marshaller. It does no work and knows no schema, so what differs between the arms is the
 * client and the loopback.
 *
 * <p><b>The request is small and the response is P2.2.</b> That is what
 * {@code ListTasksDetailed} is. The first version of this file sent P2.2 in both
 * directions, which made the server pay a 540 KB parse production never pays.
 *
 * <p><b>TCP loopback, and not by choice.</b> {@code ak_client_new} takes a URI and tonic
 * connects over TCP; the core's RPC half has no Unix-domain connector, so the UDS primary
 * the branch asked for is not reachable through it. Both arms therefore run over loopback
 * TCP, which keeps them comparable, and the limitation is a finding rather than a setting.
 *
 * <p><b>CPU per RPC is the headline.</b> Client and server are in one process, so the
 * process counter covers both halves; that is stated in the output rather than implied.
 */
public final class RunRpc {

  /** Parameterised so the FIXED per-call cost can be separated from the per-byte
   *  one: the core's blocking form copies the request, builds a Grpc and polls
   *  readiness on every call, and those do not scale with the payload. */
  static final String ID = System.getProperty("ak.rpc.id", "P2.2");
  static final String PATH = "/ak.Bench/ListTasksDetailed";
  static final int[] INFLIGHT = {1, 8, 16};
  static final long WARM_NS = Long.getLong("ak.rpc.warmns", 4_000_000_000L);
  static final long MEAS_NS = Long.getLong("ak.rpc.measns", 6_000_000_000L);
  static final int ARMONIK_WINDOW = 4 * 1024 * 1024;
  static final int ARMONIK_MAX_MESSAGE = 2 * 1024 * 1024;

  static final AtomicLong SINK = new AtomicLong();
  static byte[] REQUEST = new byte[16];

  /** Pass-through, for the server and for the request side of both clients. */
  static final MethodDescriptor.Marshaller<byte[]> BYTES =
      new MethodDescriptor.Marshaller<byte[]>() {
        @Override public InputStream stream(byte[] v) { return new ByteArrayInputStream(v); }
        @Override public byte[] parse(InputStream s) { return readAll(s); }
      };

  static byte[] readAll(InputStream s) {
    try {
      byte[] out = new byte[Math.max(s.available(), 64)];
      int at = 0;
      for (;;) {
        if (at == out.length) out = Arrays.copyOf(out, out.length * 2);
        int k = s.read(out, at, out.length - at);
        if (k < 0) break;
        at += k;
      }
      return at == out.length ? out : Arrays.copyOf(out, at);
    } catch (IOException e) {
      throw new IllegalStateException(e);
    }
  }

  public static void main(String[] args) throws Exception {
    String arm = System.getProperty("ak.rpc.arm", "core-rpc");
    boolean pinned = !"0".equals(System.getProperty("ak.rpc.pinned", "1"));

    // The wire the server answers with: P2.2, encoded once, by the incumbent, so neither
    // arm's codec is in the server's cost and both parse identical bytes.
    final byte[] body = ((Message) PbArms.build(ID, Values.ASCII)).toByteArray();

    MethodDescriptor<byte[], byte[]> serverMd = MethodDescriptor.<byte[], byte[]>newBuilder()
        .setType(MethodDescriptor.MethodType.UNARY)
        .setFullMethodName(PATH.substring(1))
        .setRequestMarshaller(BYTES).setResponseMarshaller(BYTES).build();

    ServerServiceDefinition svc = ServerServiceDefinition.builder("ak.Bench")
        .addMethod(serverMd, ServerCalls.asyncUnaryCall((req, obs) -> {
          obs.onNext(body);
          obs.onCompleted();
        }))
        .build();

    NettyServerBuilder sb = NettyServerBuilder.forAddress(new InetSocketAddress("127.0.0.1", 0))
        .addService(svc).maxInboundMessageSize(ARMONIK_MAX_MESSAGE);
    if (pinned) sb.flowControlWindow(ARMONIK_WINDOW);
    Server server = sb.build().start();
    final int port = server.getPort();

    Runner runner = arm.equals("grpc-java")
        ? new GrpcJavaRunner(port, pinned) : new CoreRunner(port);

    long[][] rows = new long[INFLIGHT.length][3];
    for (int k = 0; k < INFLIGHT.length; k++) {
      long calls0 = drive(runner, INFLIGHT[k], WARM_NS);
      long cpu0 = cpuNanos(), t0 = System.nanoTime();
      long calls = drive(runner, INFLIGHT[k], MEAS_NS);
      long wall = System.nanoTime() - t0, cpu = cpuNanos() - cpu0;
      rows[k][0] = INFLIGHT[k];
      rows[k][1] = cpu / calls;
      rows[k][2] = wall / calls;
      SINK.addAndGet(calls0);
    }

    runner.close();
    server.shutdownNow();

    // ---- everything below here is after the last measurement (R9) ----
    StringBuilder sb2 = new StringBuilder();
    sb2.append("arm=").append(arm)
       .append("  transport=loopback TCP (the core's RPC half has no UDS connector)")
       .append("  config=").append(pinned
           ? "ArmoniK: flowControlWindow=" + ARMONIK_WINDOW + " (BDP OFF)"
           : "grpc-java default: 1048576, BDP ON")
       .append('\n');
    sb2.append("server=grpc-java answering a fixed pre-encoded body, identical for both arms")
       .append("\nresponse=").append(body.length).append(" B (").append(ID)
       .append(")  request=").append(REQUEST.length).append(" B")
       .append("  warm=").append(WARM_NS / 1_000_000).append(" ms")
       .append("  measure=").append(MEAS_NS / 1_000_000).append(" ms")
       .append("  sink=").append(SINK.get()).append('\n');
    sb2.append("CPU is PROCESS cpu: client and server are both in this process.\n\n");
    sb2.append(String.format("%-10s %14s %14s%n", "in flight", "CPU us/RPC", "wall us/RPC"));
    for (long[] r : rows)
      sb2.append(String.format("%-10d %14.1f %14.1f%n", r[0], r[1] / 1000.0, r[2] / 1000.0));
    System.out.print(sb2);
    System.out.flush();
  }

  // ---- the two client stacks --------------------------------------------------------

  interface Runner extends AutoCloseable {
    /** One call, including parsing the response into a live object graph. */
    void call() throws Exception;
    @Override void close();
  }

  /** The incumbent, end to end. */
  static final class GrpcJavaRunner implements Runner {
    final ManagedChannel ch;
    final MethodDescriptor<byte[], Object> md;

    GrpcJavaRunner(int port, boolean pinned) throws Exception {
      NettyChannelBuilder cb = NettyChannelBuilder.forAddress("127.0.0.1", port)
          .usePlaintext().maxInboundMessageSize(ARMONIK_MAX_MESSAGE);
      if (pinned) cb.flowControlWindow(ARMONIK_WINDOW);
      ch = cb.build();
      final MethodDescriptor.Marshaller<Message> pm =
          io.grpc.protobuf.lite.ProtoLiteUtils.marshaller(
              (Message) PbArms.build(ID, Values.ASCII));
      md = MethodDescriptor.<byte[], Object>newBuilder()
          .setType(MethodDescriptor.MethodType.UNARY)
          .setFullMethodName(PATH.substring(1))
          .setRequestMarshaller(BYTES)
          .setResponseMarshaller(new MethodDescriptor.Marshaller<Object>() {
            @Override public InputStream stream(Object v) { return pm.stream((Message) v); }
            @Override public Object parse(InputStream s) { return pm.parse(s); }
          })
          .build();
    }

    @Override public void call() {
      SINK.addAndGet(System.identityHashCode(
          ClientCalls.blockingUnaryCall(ch, md, CallOptions.DEFAULT, REQUEST)));
    }

    @Override public void close() { ch.shutdownNow(); }
  }

  /** The core: tonic behind the C ABI, and the pull family for the response. */
  static final class CoreRunner implements Runner {
    final long rt;
    final long client;
    final long pathPtr;
    final int pathLen;
    final ThreadLocal<long[]> out = ThreadLocal.withInitial(() -> new long[3]);
    final ThreadLocal<Binding> dec = ThreadLocal.withInitial(Binding::new);
    final ThreadLocal<byte[]> buf = ThreadLocal.withInitial(() -> new byte[1 << 16]);

    CoreRunner(int port) {
      NativeRpc.ensureBound();
      // Explicit worker count: ABI v1 section 3 refuses Runtime::new() because Rust reads
      // the cgroup quota and a requests-only pod would take every CPU on the node.
      rt = NativeRpc.runtimeNew(Integer.getInteger("ak.rpc.workers", 2));
      if (rt == 0) throw new IllegalStateException("ak_runtime_new failed");
      byte[] uri = ("http://127.0.0.1:" + port).getBytes(StandardCharsets.UTF_8);
      client = NativeRpc.clientNew(rt, uri, uri.length);
      if (client == 0) throw new IllegalStateException("ak_client_new failed");
      byte[] p = PATH.getBytes(StandardCharsets.UTF_8);
      pathLen = p.length;
      pathPtr = Mem.alloc(pathLen);
      Mem.copyFromBytes(p, 0, pathPtr, pathLen);
    }

    @Override public void call() {
      long[] o = out.get();
      // Crossing one.
      int rc = NativeRpc.callUnary(client, pathPtr, pathLen, REQUEST, 0, REQUEST.length, o);
      if (rc != 0) throw new IllegalStateException("ak_call_unary returned " + rc);
      int n = (int) o[1];
      byte[] b = buf.get();
      if (b.length < n) { b = new byte[Integer.highestOneBit(n - 1) * 2]; buf.set(b); }
      // Unsafe, not a crossing: the copy is the host's and folding it into the call would
      // have made this arm one crossing against its own specification's two.
      Mem.copyToBytes(o[0], b, 0, n);
      SINK.addAndGet(System.identityHashCode(FfiArms.parse(dec.get(), ID, b, 0, n)));
      // Crossing two.
      NativeRpc.bytesFree(o[0], o[1], o[2]);
    }

    @Override public void close() {
      NativeRpc.clientDestroy(client);
      NativeRpc.runtimeDestroy(rt);
    }
  }

  // ---- driving ----------------------------------------------------------------------

  static long drive(Runner r, int inflight, long forNs) throws Exception {
    final java.util.concurrent.atomic.AtomicLong made = new java.util.concurrent.atomic.AtomicLong();
    final long deadline = System.nanoTime() + forNs;
    if (inflight == 1) {
      while (System.nanoTime() < deadline) { r.call(); made.incrementAndGet(); }
      return Math.max(made.get(), 1);
    }
    Thread[] ts = new Thread[inflight];
    for (int i = 0; i < inflight; i++) {
      ts[i] = new Thread(() -> {
        try {
          while (System.nanoTime() < deadline) { r.call(); made.incrementAndGet(); }
        } catch (Exception e) { throw new IllegalStateException(e); }
      });
      ts[i].start();
    }
    for (Thread t : ts) t.join();
    return Math.max(made.get(), 1);
  }

  static long cpuNanos() {
    java.lang.management.OperatingSystemMXBean os =
        java.lang.management.ManagementFactory.getOperatingSystemMXBean();
    if (os instanceof com.sun.management.OperatingSystemMXBean)
      return ((com.sun.management.OperatingSystemMXBean) os).getProcessCpuTime();
    throw new IllegalStateException("no process CPU counter");
  }
}
