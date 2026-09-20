package ak;

import ak.shapes.Arms;
import ak.shapes.Binding;
import ak.shapes.FfiArms;
import ak.shapes.PbArms;
import com.google.protobuf.Message;
import io.grpc.CallOptions;
import io.grpc.Channel;
import io.grpc.ManagedChannel;
import io.grpc.MethodDescriptor;
import io.grpc.Server;
import io.grpc.ServerCallHandler;
import io.grpc.ServerServiceDefinition;
import io.grpc.stub.ClientCalls;
import io.grpc.stub.ServerCalls;
import io.grpc.stub.StreamObserver;
import io.grpc.netty.shaded.io.grpc.netty.NettyChannelBuilder;
import io.grpc.netty.shaded.io.grpc.netty.NettyServerBuilder;
import io.grpc.netty.shaded.io.netty.channel.epoll.EpollDomainSocketChannel;
import io.grpc.netty.shaded.io.netty.channel.epoll.EpollEventLoopGroup;
import io.grpc.netty.shaded.io.netty.channel.epoll.EpollServerDomainSocketChannel;
import io.grpc.netty.shaded.io.netty.channel.unix.DomainSocketAddress;
import io.grpc.protobuf.lite.ProtoLiteUtils;

import java.io.File;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.atomic.AtomicLong;

/**
 * The transport arm: the host's real gRPC stack against the core's, end to end.
 *
 * <p><b>What this is and what R14's arm was not.</b> {@code RunR14} calls the real
 * {@code ProtoLiteUtils} marshaller, which is the codec path gRPC drives, but it calls it
 * directly: no server, no channel, no HTTP/2, no event loop. This carries a real unary RPC
 * over a real transport, so the question it answers is the one SHAPES.md asks -- what does
 * the codec choice cost a call, rather than what does it cost a serialisation.
 *
 * <p><b>The two arms differ only in the marshaller.</b> Same service, same method name,
 * same server, same channel, same payload, same number in flight. The codec is the only
 * variable, which is the whole point: everything the transport does is on both sides of
 * the comparison and cancels.
 *
 * <p><b>CPU per RPC is the headline and wall clock goes beside it.</b> The cpp slice's
 * first RPC version would have published 33 ms for a path costing 1.5 ms of CPU, because
 * wall clock on a saturated event loop measures queueing. Client and server are in ONE
 * process here, so the process CPU counter covers both halves of the call: that is stated
 * rather than hidden, and it is the right total for "what does a round trip cost".
 *
 * <p><b>Unix domain socket primary.</b> A UDS removes the TCP/IP stack from both arms
 * equally, which is kernel time neither codec is responsible for. Loopback TCP is a
 * labelled second row rather than the headline.
 *
 * <p><b>The configuration is pinned to ArmoniK's, not to the stack's default</b>, and the
 * default is a labelled row of its own. They are two measurements and not one with a knob:
 * {@code flowControlWindow(int)} sets {@code autoFlowControl = false} and
 * {@code initialFlowControlWindow(int)} sets it true, both writing the same field
 * (read from the bytecode; see {@code logs/java/flow-control.log}).
 */
public final class RunRpc {

  static final String ID = "P2.2";
  static final int WARM = Integer.getInteger("ak.rpc.warm", 200);
  static final int CALLS = Integer.getInteger("ak.rpc.calls", 400);
  static final int[] INFLIGHT = {1, 8, 16};

  /** ArmoniK's transport configuration, which is what its worker server runs with. */
  static final int ARMONIK_WINDOW = 4 * 1024 * 1024;
  static final int ARMONIK_MAX_MESSAGE = 2 * 1024 * 1024;

  static final AtomicLong SINK = new AtomicLong();

  // ---- the two marshallers ----------------------------------------------------------

  /** The incumbent: exactly what a generated gRPC stub is given. */
  static MethodDescriptor.Marshaller<Object> pbMarshaller() throws Exception {
    final MethodDescriptor.Marshaller<Message> m =
        ProtoLiteUtils.marshaller((Message) PbArms.build(ID, Values.ASCII));
    return new MethodDescriptor.Marshaller<Object>() {
      @Override public InputStream stream(Object v) { return m.stream((Message) v); }
      @Override public Object parse(InputStream s) { return m.parse(s); }
    };
  }

  /**
   * The core, behind the same interface. Encode writes into the binding's own buffer and
   * the stream hands those bytes to the transport through {@link Drainable}, which is what
   * `ProtoInputStream` does and what stops this arm paying a copy the incumbent does not.
   *
   * <p>A {@code Binding} carries per-decode instance state, so with calls in flight there
   * is one per thread. A thread local is a hidden global and this slice refuses them in
   * the codec; here it is the host's own pooling decision, made explicitly and named.
   */
  static MethodDescriptor.Marshaller<Object> ffiMarshaller() {
    final ThreadLocal<Binding> enc = ThreadLocal.withInitial(Binding::new);
    final ThreadLocal<Binding> dec = ThreadLocal.withInitial(Binding::new);
    return new MethodDescriptor.Marshaller<Object>() {
      @Override public InputStream stream(Object v) {
        Binding b = enc.get();
        int n = FfiArms.encode(b, ID, v);
        return new CoreStream(b, n);
      }
      @Override public Object parse(InputStream s) {
        try {
          byte[] buf = readAll(s);
          return FfiArms.parse(dec.get(), ID, buf, 0, buf.length);
        } catch (IOException e) {
          throw new IllegalStateException(e);
        }
      }
    };
  }

  /** `KnownLength` so the transport sizes the frame without buffering; `Drainable` so it
   *  writes straight out of the core's buffer. Both are what the incumbent's stream has. */
  static final class CoreStream extends InputStream
      implements io.grpc.KnownLength, io.grpc.Drainable {
    private final Binding b;
    private final int len;
    private byte[] copy;
    private int pos;

    CoreStream(Binding b, int len) { this.b = b; this.len = len; }

    @Override public int drainTo(OutputStream target) throws IOException {
      byte[] out = new byte[len];
      int rc = Native.encTake(b.encCtx, out);
      if (rc < 0) throw new IOException("core returned " + rc);
      target.write(out, 0, rc);
      return rc;
    }

    @Override public int available() { return len - pos; }

    private byte[] materialise() {
      if (copy == null) {
        copy = new byte[len];
        Native.encTake(b.encCtx, copy);
      }
      return copy;
    }

    @Override public int read() {
      byte[] c = materialise();
      return pos < len ? c[pos++] & 0xFF : -1;
    }

    @Override public int read(byte[] dst, int off, int n) {
      byte[] c = materialise();
      if (pos >= len) return -1;
      int k = Math.min(n, len - pos);
      System.arraycopy(c, pos, dst, off, k);
      pos += k;
      return k;
    }
  }

  static byte[] readAll(InputStream s) throws IOException {
    int n = s.available();
    byte[] out = new byte[Math.max(n, 64)];
    int at = 0;
    for (;;) {
      if (at == out.length) out = Arrays.copyOf(out, out.length * 2);
      int k = s.read(out, at, out.length - at);
      if (k < 0) break;
      at += k;
    }
    return at == out.length ? out : Arrays.copyOf(out, at);
  }

  // ---- the service ------------------------------------------------------------------

  static MethodDescriptor<Object, Object> method(MethodDescriptor.Marshaller<Object> m) {
    return MethodDescriptor.<Object, Object>newBuilder()
        .setType(MethodDescriptor.MethodType.UNARY)
        .setFullMethodName("ak.Bench/ListTasksDetailed")
        .setRequestMarshaller(m)
        .setResponseMarshaller(m)
        .build();
  }

  public static void main(String[] args) throws Exception {
    boolean tcp = Boolean.getBoolean("ak.rpc.tcp");
    boolean pinned = !"0".equals(System.getProperty("ak.rpc.pinned", "1"));
    String arm = System.getProperty("ak.rpc.arm", "ffi");

    Object facade = Arms.build(ID, Values.ASCII);
    Message pbmsg = (Message) PbArms.build(ID, Values.ASCII);
    MethodDescriptor.Marshaller<Object> marsh =
        arm.equals("pbj") ? pbMarshaller() : ffiMarshaller();
    final Object response = arm.equals("pbj") ? pbmsg : facade;
    MethodDescriptor<Object, Object> md = method(marsh);

    ServerServiceDefinition svc = ServerServiceDefinition.builder("ak.Bench")
        .addMethod(md, ServerCalls.asyncUnaryCall(
            (ServerCalls.UnaryMethod<Object, Object>) (req, obs) -> {
              obs.onNext(response);
              obs.onCompleted();
            }))
        .build();

    File sock = new File(System.getProperty("java.io.tmpdir"),
        "ak-rpc-" + ProcessHandle.current().pid() + ".sock");
    sock.deleteOnExit();

    Server server;
    ManagedChannel channel;
    // Held so they can be shut down. gRPC does NOT own an event loop group the host
    // supplies, so `shutdownNow` on the server and the channel leaves these running --
    // and they are non-daemon, so the JVM never exits. That cost this arm its first run:
    // the measurements completed, `main` returned, and the buffered stdout was never
    // flushed because the exit never happened.
    EpollEventLoopGroup boss = null, work = null;
    if (tcp) {
      NettyServerBuilder sb = NettyServerBuilder.forAddress(new InetSocketAddress("127.0.0.1", 0))
          .addService(svc).maxInboundMessageSize(ARMONIK_MAX_MESSAGE);
      if (pinned) sb.flowControlWindow(ARMONIK_WINDOW);
      server = sb.build().start();
      NettyChannelBuilder cb = NettyChannelBuilder
          .forAddress("127.0.0.1", server.getPort())
          .usePlaintext().maxInboundMessageSize(ARMONIK_MAX_MESSAGE);
      if (pinned) cb.flowControlWindow(ARMONIK_WINDOW);
      channel = cb.build();
    } else {
      boss = new EpollEventLoopGroup(1);
      work = new EpollEventLoopGroup();
      NettyServerBuilder sb = NettyServerBuilder.forAddress(new DomainSocketAddress(sock))
          .channelType(EpollServerDomainSocketChannel.class)
          .bossEventLoopGroup(boss).workerEventLoopGroup(work)
          .addService(svc).maxInboundMessageSize(ARMONIK_MAX_MESSAGE);
      if (pinned) sb.flowControlWindow(ARMONIK_WINDOW);
      server = sb.build().start();
      NettyChannelBuilder cb = NettyChannelBuilder.forAddress(new DomainSocketAddress(sock))
          .channelType(EpollDomainSocketChannel.class)
          .eventLoopGroup(work)
          .usePlaintext().maxInboundMessageSize(ARMONIK_MAX_MESSAGE);
      if (pinned) cb.flowControlWindow(ARMONIK_WINDOW);
      channel = cb.build();
    }

    StringBuilder sb = new StringBuilder();
    long[][] rows = new long[INFLIGHT.length][3];
    for (int k = 0; k < INFLIGHT.length; k++) {
      drive(channel, md, facade, arm.equals("pbj") ? pbmsg : facade, WARM, INFLIGHT[k]);
      long cpu0 = cpuNanos(), t0 = System.nanoTime();
      drive(channel, md, facade, arm.equals("pbj") ? pbmsg : facade, CALLS, INFLIGHT[k]);
      long wall = System.nanoTime() - t0, cpu = cpuNanos() - cpu0;
      rows[k][0] = INFLIGHT[k];
      rows[k][1] = cpu / CALLS;
      rows[k][2] = wall / CALLS;
    }

    channel.shutdownNow();
    server.shutdownNow();
    if (boss != null) boss.shutdownGracefully(0, 0, java.util.concurrent.TimeUnit.SECONDS);
    if (work != null) work.shutdownGracefully(0, 0, java.util.concurrent.TimeUnit.SECONDS);

    // ---- everything below here is after the last measurement (R9) ----
    sb.append("arm=").append(arm)
      .append("  transport=").append(tcp ? "loopback TCP" : "unix domain socket (epoll)")
      .append("  config=").append(pinned
          ? "ArmoniK: flowControlWindow=" + ARMONIK_WINDOW + " (BDP OFF), maxInboundMessageSize="
            + ARMONIK_MAX_MESSAGE
          : "grpc-java default: 1048576, BDP auto-tuning ON")
      .append('\n');
    sb.append("payload=").append(ID).append("  calls=").append(CALLS)
      .append("  warm=").append(WARM).append("  sink=").append(SINK.get()).append('\n');
    sb.append(String.format("%-10s %14s %14s%n", "in flight", "CPU us/RPC", "wall us/RPC"));
    for (long[] r : rows)
      sb.append(String.format("%-10d %14.1f %14.1f%n", r[0], r[1] / 1000.0, r[2] / 1000.0));
    System.out.print(sb);
    System.out.flush();
  }

  static void drive(Channel ch, MethodDescriptor<Object, Object> md, Object facade,
                    Object req, int calls, int inflight) throws Exception {
    if (inflight == 1) {
      for (int i = 0; i < calls; i++)
        SINK.addAndGet(System.identityHashCode(
            ClientCalls.blockingUnaryCall(ch, md, CallOptions.DEFAULT, req)));
      return;
    }
    final CountDownLatch done = new CountDownLatch(calls);
    final java.util.concurrent.Semaphore permits = new java.util.concurrent.Semaphore(inflight);
    for (int i = 0; i < calls; i++) {
      permits.acquire();
      ClientCalls.asyncUnaryCall(ch.newCall(md, CallOptions.DEFAULT), req,
          new StreamObserver<Object>() {
            @Override public void onNext(Object v) { SINK.addAndGet(System.identityHashCode(v)); }
            @Override public void onError(Throwable t) { permits.release(); done.countDown();
              throw new IllegalStateException(t); }
            @Override public void onCompleted() { permits.release(); done.countDown(); }
          });
    }
    done.await();
  }

  /** Process CPU, both halves of the call, because both are in this process. */
  static long cpuNanos() {
    java.lang.management.OperatingSystemMXBean os =
        java.lang.management.ManagementFactory.getOperatingSystemMXBean();
    if (os instanceof com.sun.management.OperatingSystemMXBean)
      return ((com.sun.management.OperatingSystemMXBean) os).getProcessCpuTime();
    throw new IllegalStateException("no process CPU counter");
  }
}
