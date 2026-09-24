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
  /** Per thread, never static state shared across threads (R12). */
  static final ThreadLocal<Binding> PARSE_BINDING = ThreadLocal.withInitial(Binding::new);
  /** Cell D's reused read buffer, so it pays what cells B and C pay. */
  static final ThreadLocal<byte[]> DBUF = ThreadLocal.withInitial(() -> new byte[1 << 16]);

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

  /** The dilution caveat, lifted. Client and server in ONE process means the CPU counter
   *  carries both halves, so every ratio here is a floor: the difference between arms is
   *  entirely client-side but is divided by a total containing a common server. With
   *  `-Dak.rpc.serve=<path>` this process is ONLY the server, and a second process with
   *  `-Dak.rpc.connect=<path>` measures a CPU counter that contains only the client. */
  static Server serveOnly(String path, boolean pinned, byte[] body) throws Exception {
    MethodDescriptor<byte[], byte[]> md = MethodDescriptor.<byte[], byte[]>newBuilder()
        .setType(MethodDescriptor.MethodType.UNARY)
        .setFullMethodName(PATH.substring(1))
        .setRequestMarshaller(BYTES).setResponseMarshaller(BYTES).build();
    ServerServiceDefinition svc = ServerServiceDefinition.builder("ak.Bench")
        .addMethod(md, ServerCalls.asyncUnaryCall((req, obs) -> {
          obs.onNext(body);
          obs.onCompleted();
        })).build();
    java.io.File f = new java.io.File(path);
    f.delete();
    NettyServerBuilder sb = NettyServerBuilder.forAddress(
            new io.grpc.netty.shaded.io.netty.channel.unix.DomainSocketAddress(f))
        .channelType(io.grpc.netty.shaded.io.netty.channel.epoll.EpollServerDomainSocketChannel.class)
        .bossEventLoopGroup(new io.grpc.netty.shaded.io.netty.channel.epoll.EpollEventLoopGroup(1))
        .workerEventLoopGroup(new io.grpc.netty.shaded.io.netty.channel.epoll.EpollEventLoopGroup())
        .addService(svc).maxInboundMessageSize(ARMONIK_MAX_MESSAGE);
    if (pinned) sb.flowControlWindow(ARMONIK_WINDOW);
    return sb.build().start();
  }

  public static void main(String[] args) throws Exception {
    String serve = System.getProperty("ak.rpc.serve");
    if (serve != null) {
      Server sv = serveOnly(serve, !"0".equals(System.getProperty("ak.rpc.pinned", "1")),
          ((Message) PbArms.build(ID, Values.ASCII)).toByteArray());
      System.out.println("SERVING " + serve);
      System.out.flush();
      sv.awaitTermination();
      return;
    }
    // The grid. B minus A is the TRANSPORT difference and C minus B the CODEC difference;
    // neither is recoverable from A against C, which moves both at once. Cell B is not a
    // contrivance: it is README section 13's outcome 2, the fallback the original Java
    // report recommended, priced directly instead of inferred from two halves.
    //
    //   cell   codec           transport
    //   A      protobuf-java   grpc-java
    //   B      protobuf-java   core          <- outcome 2
    //   C      core            core
    //   D      core            grpc-java     <- D-A and C-B are both the codec difference
    String cell = System.getProperty("ak.rpc.cell", "C").toUpperCase();
    boolean pinned = !"0".equals(System.getProperty("ak.rpc.pinned", "1"));
    boolean uds = !"0".equals(System.getProperty("ak.rpc.uds", "1"));
    String delivery = System.getProperty("ak.rpc.delivery", "queue");
    boolean coreTransport = cell.equals("B") || cell.equals("C");
    Parse parse = (cell.equals("C") || cell.equals("D"))
        ? (b, off, len) -> FfiArms.parse(PARSE_BINDING.get(), ID, b, off, len)
        : PARSE_PBJ;

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

    // One server, reachable by BOTH transports over the same socket: grpc-java through
    // netty's epoll domain-socket channel, and the core through tonic's `unix:` target,
    // which `Endpoint::from_shared` strips and turns into a UnixStream connector
    // (tonic 0.14.6, transport/channel/endpoint.rs:175 and new_uds at :111).
    // With `-Dak.rpc.connect=<path>` the server is somebody else's process, so the CPU
    // counter this arm reads contains ONLY the client. That is the row that turns every
    // ratio here from a floor into a ratio.
    String connect = System.getProperty("ak.rpc.connect");
    java.io.File sock = connect != null ? new java.io.File(connect)
        : new java.io.File(System.getProperty("java.io.tmpdir"),
            // Not ProcessHandle (Java 9): this file is in the Java 8 floor's compilation too,
            // and ProcessHandle broke that build from 5241ced until the R-D5 re-gate.
            "ak-rpc-" + java.lang.management.ManagementFactory.getRuntimeMXBean().getName()
                .replaceAll("[^0-9A-Za-z]", "_") + ".sock");
    if (connect == null) sock.deleteOnExit();
    io.grpc.netty.shaded.io.netty.channel.epoll.EpollEventLoopGroup boss = null, work = null;
    Server server = null;
    if (connect != null) {
      work = new io.grpc.netty.shaded.io.netty.channel.epoll.EpollEventLoopGroup();
    } else {
    NettyServerBuilder sb;
    if (uds) {
      boss = new io.grpc.netty.shaded.io.netty.channel.epoll.EpollEventLoopGroup(1);
      work = new io.grpc.netty.shaded.io.netty.channel.epoll.EpollEventLoopGroup();
      sb = NettyServerBuilder.forAddress(
              new io.grpc.netty.shaded.io.netty.channel.unix.DomainSocketAddress(sock))
          .channelType(io.grpc.netty.shaded.io.netty.channel.epoll.EpollServerDomainSocketChannel.class)
          .bossEventLoopGroup(boss).workerEventLoopGroup(work);
    } else {
      sb = NettyServerBuilder.forAddress(new InetSocketAddress("127.0.0.1", 0));
    }
    sb.addService(svc).maxInboundMessageSize(ARMONIK_MAX_MESSAGE);
    if (pinned) sb.flowControlWindow(ARMONIK_WINDOW);
    server = sb.build().start();
    }
    final int port = (uds || connect != null) ? -1 : server.getPort();

    String coreTarget = uds ? "unix:" + sock.getAbsolutePath() : "http://127.0.0.1:" + port;

    Runner runner;
    if (coreTransport) {
      runner = delivery.equals("blocking")
          ? new CoreRunner(coreTarget, parse, pinned)
          : new QueueRunner(coreTarget, parse, delivery.equals("queue-vt"), pinned);
    } else {
      runner = uds
          ? new GrpcJavaRunner(
                new io.grpc.netty.shaded.io.netty.channel.unix.DomainSocketAddress(sock),
                io.grpc.netty.shaded.io.netty.channel.epoll.EpollDomainSocketChannel.class,
                work, pinned, parse)
          : new GrpcJavaRunner(new InetSocketAddress("127.0.0.1", port), null, null, pinned, parse);
    }

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
    if (server != null) server.shutdownNow();
    if (boss != null) boss.shutdownGracefully(0, 0, java.util.concurrent.TimeUnit.SECONDS);
    if (work != null) work.shutdownGracefully(0, 0, java.util.concurrent.TimeUnit.SECONDS);

    // ---- everything below here is after the last measurement (R9) ----
    StringBuilder sb2 = new StringBuilder();
    sb2.append("cell=").append(cell)
       .append("  codec=").append(parse == PARSE_PBJ ? "protobuf-java" : "core")
       .append("  transport=").append(coreTransport ? "core (tonic)" : "grpc-java")
       .append(coreTransport ? "/" + delivery : "")
       .append("  socket=").append(uds ? "unix domain" : "loopback TCP")
       .append(connect != null ? "  server=SEPARATE PROCESS (CPU is client only)"
                               : "  server=same process (CPU carries both halves)")
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

  /** The codec half of the grid. The transport moves opaque bytes and never sees a
   *  message type, so the parser is free to vary against it -- which is what makes
   *  B minus A the transport difference and C minus B the codec difference. */
  interface Parse { Object parse(byte[] b, int off, int len); }

  static final Parse PARSE_PBJ = (b, off, len) -> {
    try { return PbArms.parse(ID, b, off, len); }
    catch (Exception e) { throw new IllegalStateException(e); }
  };

  interface Runner extends AutoCloseable {
    /** One call, including parsing the response into a live object graph. */
    void call() throws Exception;
    @Override void close();
  }

  /** The incumbent, end to end. */
  static final class GrpcJavaRunner implements Runner {
    final ManagedChannel ch;
    final MethodDescriptor<byte[], Object> md;

    GrpcJavaRunner(java.net.SocketAddress addr, Class<? extends io.grpc.netty.shaded.io.netty.channel.Channel> ct,
                   io.grpc.netty.shaded.io.netty.channel.EventLoopGroup elg,
                   boolean pinned, Parse parse) throws Exception {
      NettyChannelBuilder cb = NettyChannelBuilder.forAddress(addr)
          .usePlaintext().maxInboundMessageSize(ARMONIK_MAX_MESSAGE);
      if (ct != null) cb.channelType(ct).eventLoopGroup(elg);
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
            @Override public Object parse(InputStream s) {
              // Cell A reads it with protobuf-java; cell D reads the same bytes with the
              // core, so D minus A is the codec difference under grpc-java's transport.
              if (parse == PARSE_PBJ) return pm.parse(s);
              // Into a REUSED buffer, not readAll's fresh array. The first version
              // allocated 540 KB per call and copied into it, where cell A's marshaller
              // never materialises one and cells B and C both copy into a buffer they
              // keep -- so cell D was paying an allocation and a copy that neither its
              // own comparator nor the other codec delta pays. That handicap was worth
              // about 400 ns per element and it was sitting in the arm that decides
              // whether the two halves of outcome 2 are additive.
              try {
                int n = s.available();
                byte[] b = DBUF.get();
                if (b.length < n) { b = new byte[Integer.highestOneBit(n - 1) * 2]; DBUF.set(b); }
                int at = 0;
                for (;;) {
                  int k = s.read(b, at, b.length - at);
                  if (k < 0) break;
                  at += k;
                  if (at == b.length) break;
                }
                return parse.parse(b, 0, at);
              } catch (IOException e) {
                throw new IllegalStateException(e);
              }
            }
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

    final Parse parse;

    CoreRunner(String target, Parse parse) { this(target, parse, true); }

    CoreRunner(String target, Parse parse, boolean pinned) {
      this.parse = parse;
      NativeRpc.ensureBound();
      // Explicit worker count: ABI v1 section 3 refuses Runtime::new() because Rust reads
      // the cgroup quota and a requests-only pod would take every CPU on the node.
      rt = NativeRpc.runtimeNew(Integer.getInteger("ak.rpc.workers", 2));
      if (rt == 0) throw new IllegalStateException("ak_runtime_new failed");
      byte[] uri = target.getBytes(StandardCharsets.UTF_8);
      // ArmoniK's INTENDED configuration on the core side too. Nagle: -1, leave the
      // default, because tonic already defaults tcp_nodelay=true and
      // packages/rust/armonik-transport ships tcp_nagle_algorithm=false, so both are
      // already Nagle-off and pinning it would change nothing.
      client = pinned
          ? NativeRpc.clientNewOpts(rt, uri, uri.length, ARMONIK_WINDOW, ARMONIK_WINDOW,
                                    -1, ARMONIK_MAX_MESSAGE, ARMONIK_MAX_MESSAGE, -1)
          : NativeRpc.clientNew(rt, uri, uri.length);
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
      SINK.addAndGet(System.identityHashCode(parse.parse(b, 0, n)));
      // Crossing two.
      NativeRpc.bytesFree(o[0], o[1], o[2]);
    }

    @Override public void close() {
      NativeRpc.clientDestroy(client);
      NativeRpc.runtimeDestroy(rt);
    }
  }

  /**
   * ABI v1 section 9's completion queue: submit, then block in `ak_queue_next`.
   *
   * <p>Three forward crossings per call and ZERO reverse, against the blocking mode's two
   * and zero. The point is not the crossing arithmetic, which costs the queue about 12 ns
   * a call; it is that the host thread waits in a downcall it entered rather than being
   * blocked in a native frame for the duration of an RPC. On a virtual thread that is the
   * difference {@code logs/java/pinning.log} measured as 2,420 ms against 305.
   *
   * <p>One drainer, as section 9 describes. The drainer keeps N calls in flight: it takes a
   * completion, parses it, and submits a replacement.
   */
  static final class QueueRunner implements Runner {
    final CoreRunner base;
    final long q;
    final boolean virtual;
    final ThreadLocal<long[]> comp = ThreadLocal.withInitial(() -> new long[5]);
    final java.util.concurrent.atomic.AtomicLong tags = new java.util.concurrent.atomic.AtomicLong();

    QueueRunner(String target, Parse parse, boolean virtual, boolean pinned) {
      this.base = new CoreRunner(target, parse, pinned);
      this.virtual = virtual;
      this.q = NativeRpc.queueNew();
      if (q == 0) throw new IllegalStateException("ak_queue_new failed");
    }

    /** Submit one. Crossing one. */
    long submit() {
      long h = NativeRpc.callUnaryQ(base.client, base.pathPtr, base.pathLen,
          REQUEST, 0, REQUEST.length, q, tags.incrementAndGet());
      if (h == 0) throw new IllegalStateException("ak_call_unary_q failed");
      return h;
    }

    /** Take one, parse it, release it. Crossings two and three. */
    void take() {
      long[] o = comp.get();
      int rc = NativeRpc.queueNext(q, Long.MAX_VALUE, o);
      if (rc != NativeRpc.QUEUE_OK) throw new IllegalStateException("queueNext rc=" + rc);
      if (o[0] != 0) throw new IllegalStateException("call status " + o[0]);
      int n = (int) o[3];
      byte[] b = base.buf.get();
      if (b.length < n) { b = new byte[Integer.highestOneBit(n - 1) * 2]; base.buf.set(b); }
      Mem.copyToBytes(o[2], b, 0, n);
      SINK.addAndGet(System.identityHashCode(base.parse.parse(b, 0, n)));
      NativeRpc.bytesFree(o[2], o[3], o[4]);
    }

    @Override public void call() { NativeRpc.callDestroy(submit()); take(); }

    @Override public void close() {
      NativeRpc.queueShutdown(q);
      NativeRpc.queueDestroy(q);
      base.close();
    }
  }

  /** JDK 21's virtual thread, reached reflectively so this file still compiles at 17. */
  static Thread virtualThread(Runnable r) {
    try {
      Object b = Thread.class.getMethod("ofVirtual").invoke(null);
      Class<?> bc = Class.forName("java.lang.Thread$Builder");
      return (Thread) bc.getMethod("unstarted", Runnable.class).invoke(b, r);
    } catch (ReflectiveOperationException e) {
      throw new IllegalStateException("virtual threads need JDK 21", e);
    }
  }

  // ---- driving ----------------------------------------------------------------------

  static long drive(Runner r, int inflight, long forNs) throws Exception {
    if (r instanceof QueueRunner) return driveQueue((QueueRunner) r, inflight, forNs);
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

  /** One drainer keeping `inflight` calls outstanding, which is the shape section 9's
   *  queue is for: the host thread waits in a downcall rather than in a native frame. */
  static long driveQueue(QueueRunner r, int inflight, long forNs) throws Exception {
    final long[] made = new long[1];
    Runnable body = () -> {
      long deadline = System.nanoTime() + forNs;
      long[] hs = new long[inflight];
      for (int i = 0; i < inflight; i++) hs[i] = r.submit();
      while (System.nanoTime() < deadline) {
        r.take();
        made[0]++;
        NativeRpc.callDestroy(hs[(int) (made[0] % inflight)]);
        hs[(int) (made[0] % inflight)] = r.submit();
      }
      for (int i = 0; i < inflight; i++) { r.take(); made[0]++; }
      for (long h : hs) NativeRpc.callDestroy(h);
    };
    Thread t = r.virtual ? virtualThread(body) : new Thread(body);
    t.start();
    t.join();
    return Math.max(made[0], 1);
  }

  static long cpuNanos() {
    java.lang.management.OperatingSystemMXBean os =
        java.lang.management.ManagementFactory.getOperatingSystemMXBean();
    if (os instanceof com.sun.management.OperatingSystemMXBean)
      return ((com.sun.management.OperatingSystemMXBean) os).getProcessCpuTime();
    throw new IllegalStateException("no process CPU counter");
  }
}
