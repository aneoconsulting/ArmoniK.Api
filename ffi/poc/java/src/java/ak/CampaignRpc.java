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
import io.grpc.Status;
import io.grpc.netty.shaded.io.grpc.netty.NettyChannelBuilder;
import io.grpc.netty.shaded.io.grpc.netty.NettyServerBuilder;
import io.grpc.netty.shaded.io.netty.channel.epoll.EpollDomainSocketChannel;
import io.grpc.netty.shaded.io.netty.channel.epoll.EpollEventLoopGroup;
import io.grpc.netty.shaded.io.netty.channel.epoll.EpollServerDomainSocketChannel;
import io.grpc.netty.shaded.io.netty.channel.unix.DomainSocketAddress;
import io.grpc.stub.ClientCalls;
import io.grpc.stub.ServerCalls;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.CyclicBarrier;

/**
 * design/CAMPAIGN.md section 4.2, the RPC grid, for the JVM.
 *
 * <p><b>Two processes.</b> {@code --serve <socket>} is the server, its own JVM, pinned to
 * {@code AK_CPU_SERVER} by the runner; everything else is the client, pinned to
 * {@code AK_CPU_CLIENT}, and runs ALL SIX CELLS in one process, interleaved per round in a
 * rotated order (req 22). The client's CPU is {@code CLOCK_PROCESS_CPUTIME_ID} (req 21),
 * which contains no server work.
 *
 * <pre>
 *   cell  codec                              transport
 *   A     protobuf-java via grpc-java's      grpc-java (netty, epoll, unix socket)
 *         marshaller (ProtoLiteUtils)
 *   B     protobuf-java (toByteArray /       the core: ak_call_unary, BLOCKING delivery
 *         parseFrom(byte[]))
 *   C     core-ffi (push decode; encode      the core, blocking
 *         + ak_enc_take)
 *   D     core-ffi, as grpc-java             grpc-java
 *         Marshaller
 * </pre>
 *
 * <p><b>Unknown-field mode</b> (req 12 as amended 85cb00f, req 10): C and D run once per
 * mode, {@code C-retain}, {@code C-drop}, {@code D-retain}, {@code D-drop}. Retain is a
 * Binding with {@code retain = true}: every decode arms the root's options (every position
 * with the shim's grow) and disarms after, and encode goes through the u-groups; drop is
 * the default Binding (NULL options). Each mode has its own per-thread Binding. Before
 * timing, each C and D cell also decodes the P2.2 body in its mode and re-encodes it, and
 * the bytes must equal the body. After the run the shim's leak counter
 * ({@code Native.unkLive}) and every retain Binding's reclaim counters must read 0 (a
 * non-zero reading aborts, req 18). A and B run the incumbent in its default mode.
 *
 * <p><b>The no-unknown build</b> (WP5 step 10): this class compiled against the tree
 * generated from the plan relowered with unknown="drop" and run on that build's core
 * runs A, B, {@code C-nounk} and {@code D-nounk}; A and B are the in-process controls that
 * let a ratio be formed against the full build's process. Every sample carries
 * {@code unknown_mode} (default, retain, drop, no-unknown) and {@code build}.
 *
 * <p><b>The server does identical work in every cell</b> (req 13): direction (a) answers
 * every request with the SAME pre-serialised P2.2 bytes; direction (b) parses the request
 * with protobuf-java and answers with an empty body. Both through a pass-through byte
 * marshaller, so no cell's codec runs on the server.
 *
 * <p><b>Directions</b> (req 14): (a) empty request, P2.2 response, which the client decodes;
 * (b) a P2.2 request, encoded per call from a FRESH object graph (req 11: a pool rebuilt
 * between chunks, outside the timed phases), empty response.
 *
 * <p><b>Every call is checked</b> (req 18): an exception or a non-OK status aborts, and the
 * response length must equal the expected one (the P2.2 body in (a), 0 in (b)); the first
 * failure exits non-zero before any sample is written.
 *
 * <p><b>Timing</b>: a sample is {@code ak.camp.calls} calls spread over {@code inflight}
 * blocking threads (1, 8, 16: req 15), run in chunks of {@code ak.camp.chunk} calls per
 * thread; only the chunks' run phases are timed (CPU and wall), the pool refills between
 * them are not.
 *
 * <p><b>Transport</b> (req 17), one per process, {@code ak.camp.transport}:
 * {@code shipped}: grpc-java as packages/java's GrpcChannelBuilder configures it (defaults,
 * max inbound message 8 MiB, max inbound metadata 1 MiB, plaintext), the core with its
 * defaults (no options: tonic's), the server with grpc-java's defaults and 8 MiB inbound.
 * {@code pinned}: 4 MiB stream and connection windows on all three (grpc-java: setting the
 * window turns BDP auto-tuning off; the core: stream_window = connection_window = 4 MiB,
 * adaptive_window = 0, tcp_nagle = 0), max messages 8 MiB.
 */
public final class CampaignRpc {
  static final String PAYLOAD = "P2.2";
  static final String GET = "ak.Bench/Get";   // direction (a)
  static final String PUT = "ak.Bench/Put";   // direction (b)
  static final int WIN = 4 * 1024 * 1024;
  static final int MAXMSG = 8 * 1024 * 1024;
  static final int[] INFLIGHT = {1, 8, 16};
  static long sinkv;

  static final MethodDescriptor.Marshaller<byte[]> BYTES = new MethodDescriptor.Marshaller<byte[]>() {
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

  static MethodDescriptor<byte[], byte[]> bytesMd(String name) {
    return MethodDescriptor.<byte[], byte[]>newBuilder().setType(MethodDescriptor.MethodType.UNARY)
        .setFullMethodName(name).setRequestMarshaller(BYTES).setResponseMarshaller(BYTES).build();
  }

  // ---- the server ------------------------------------------------------------------

  static void serve(String path, boolean pinned) throws Exception {
    final byte[] body = PbArms.build(PAYLOAD, Values.ASCII).toByteArray();
    final byte[] empty = new byte[0];
    ServerServiceDefinition svc = ServerServiceDefinition.builder("ak.Bench")
        .addMethod(bytesMd(GET), ServerCalls.asyncUnaryCall((req, obs) -> {
          obs.onNext(body);
          obs.onCompleted();
        }))
        .addMethod(bytesMd(PUT), ServerCalls.asyncUnaryCall((req, obs) -> {
          try {
            Message m = PbArms.parseArray(PAYLOAD, req);
            if (m.getSerializedSize() <= 0) throw new IllegalStateException("empty P2.2");
          } catch (Exception e) {
            obs.onError(Status.INVALID_ARGUMENT.withDescription(e.toString()).asRuntimeException());
            return;
          }
          obs.onNext(empty);
          obs.onCompleted();
        }))
        .build();
    java.io.File f = new java.io.File(path);
    f.delete();
    NettyServerBuilder sb = NettyServerBuilder.forAddress(new DomainSocketAddress(f))
        .channelType(EpollServerDomainSocketChannel.class)
        .bossEventLoopGroup(new EpollEventLoopGroup(1)).workerEventLoopGroup(new EpollEventLoopGroup())
        .addService(svc).maxInboundMessageSize(MAXMSG);
    if (pinned) sb.flowControlWindow(WIN);
    Server s = sb.build().start();
    System.out.println("SERVING " + path + " transport=" + (pinned ? "pinned" : "shipped")
        + " response=" + body.length + "B");
    System.out.flush();
    s.awaitTermination();
  }

  // ---- the client cells ----------------------------------------------------------------

  static byte[] EXPECT_A;                                  // the P2.2 body, direction (a)
  /** This thread's Bindings. A sample's threads end with the sample, so each releases its
   *  own ({@link #releaseThread}): folds the leak counters in and frees the native state.
   *  (Before this, every sample's Bindings stayed allocated to the end of the process.) */
  static final ThreadLocal<ArrayList<Binding>> MINE = new ThreadLocal<ArrayList<Binding>>() {
    @Override protected ArrayList<Binding> initialValue() { return new ArrayList<Binding>(); }
  };
  static final java.util.concurrent.atomic.AtomicLong UNK_RECLAIMED = new java.util.concurrent.atomic.AtomicLong(),
      UNK_LEFT = new java.util.concurrent.atomic.AtomicLong(),
      BINDINGS_RETAIN = new java.util.concurrent.atomic.AtomicLong(),
      BINDINGS_DROP = new java.util.concurrent.atomic.AtomicLong();
  static final ThreadLocal<Binding> BIND = new ThreadLocal<Binding>() {
    @Override protected Binding initialValue() {
      Binding b = new Binding();
      MINE.get().add(b);
      BINDINGS_DROP.incrementAndGet();
      return b;
    }
  };
  static final ThreadLocal<Binding> BIND_U = new ThreadLocal<Binding>() {
    @Override protected Binding initialValue() {
      Binding b = new Binding();
      FfiArms.setRetain(b, true);
      MINE.get().add(b);
      BINDINGS_RETAIN.incrementAndGet();
      return b;
    }
  };
  static void releaseThread() {
    for (Binding b : MINE.get()) {
      long[] u = FfiArms.unkCounters(b);
      UNK_RECLAIMED.addAndGet(u[0]);
      UNK_LEFT.addAndGet(u[1]);
      b.close();
    }
    MINE.remove();
    BIND.remove();
    BIND_U.remove();
  }
  static final ThreadLocal<byte[]> BUF = new ThreadLocal<byte[]>() {
    @Override protected byte[] initialValue() { return new byte[1 << 20]; }
  };

  static void fail(String why) { Campaign.abort("req 18: " + why); }

  /** protobuf-java's parser for the payload's root, for cell B's in-place parse. */
  static final com.google.protobuf.Parser<? extends Message> PB_PARSER =
      PbArms.build(PAYLOAD, Values.ASCII).getParserForType();

  abstract static class Cell {
    final String name;
    final boolean incumbentCodec;
    final boolean retain;                        // C and D: the unknown-field mode (req 12)
    Cell(String name, boolean incumbentCodec, boolean retain) {
      this.name = name; this.incumbentCodec = incumbentCodec; this.retain = retain;
    }
    /** This thread's Binding in this cell's mode. */
    Binding bind() { return retain ? BIND_U.get() : BIND.get(); }
    abstract void callA();                       // (a)
    abstract void callB(Object request);         // (b); request from the pool
    void close() {}
    /** The request object for (b): protobuf-java for A and B, the facade for C and D. */
    Object fresh() { return incumbentCodec ? PbArms.build(PAYLOAD, Values.ASCII) : Arms.build(PAYLOAD, Values.ASCII); }
  }

  /** grpc-java, with either codec as the response / request marshaller. */
  static final class GrpcCell extends Cell {
    final ManagedChannel ch;
    final MethodDescriptor<byte[], Object> mdA;
    final MethodDescriptor<Object, byte[]> mdB;

    GrpcCell(String name, boolean incumbent, boolean retain, String sock, EpollEventLoopGroup elg, boolean pinned) {
      super(name, incumbent, retain);
      NettyChannelBuilder cb = NettyChannelBuilder.forAddress(new DomainSocketAddress(sock))
          .channelType(EpollDomainSocketChannel.class).eventLoopGroup(elg)
          .usePlaintext().maxInboundMessageSize(MAXMSG).maxInboundMetadataSize(1024 * 1024);
      if (pinned) cb.flowControlWindow(WIN);
      ch = cb.build();
      final MethodDescriptor.Marshaller<Message> pm = io.grpc.protobuf.lite.ProtoLiteUtils.marshaller(
          PbArms.build(PAYLOAD, Values.ASCII).getDefaultInstanceForType());
      MethodDescriptor.Marshaller<Object> resp = new MethodDescriptor.Marshaller<Object>() {
        @Override public InputStream stream(Object v) { throw new UnsupportedOperationException(); }
        @Override public Object parse(InputStream s) {
          try {
            if (incumbentCodec) {
              int n = s.available();
              if (n != EXPECT_A.length) fail(name + "/a: response " + n + " B, want " + EXPECT_A.length);
              return pm.parse(s);                // grpc-java's production path
            }
            byte[] b = BUF.get();
            int at = 0;
            for (;;) {
              if (at == b.length) { b = Arrays.copyOf(b, b.length * 2); BUF.set(b); }
              int k = s.read(b, at, b.length - at);
              if (k < 0) break;
              at += k;
            }
            if (at != EXPECT_A.length) fail(name + "/a: response " + at + " B, want " + EXPECT_A.length);
            return FfiArms.decode(bind(), PAYLOAD, b, 0, at);
          } catch (IOException e) {
            throw new IllegalStateException(e);
          }
        }
      };
      MethodDescriptor.Marshaller<Object> req = new MethodDescriptor.Marshaller<Object>() {
        @Override public InputStream stream(Object v) {
          if (incumbentCodec) return pm.stream((Message) v);   // grpc-java's production path
          Binding b = bind();
          FfiArms.encode(b, PAYLOAD, v);
          return new ByteArrayInputStream(b.take());
        }
        @Override public Object parse(InputStream s) { throw new UnsupportedOperationException(); }
      };
      mdA = MethodDescriptor.<byte[], Object>newBuilder().setType(MethodDescriptor.MethodType.UNARY)
          .setFullMethodName(GET).setRequestMarshaller(BYTES).setResponseMarshaller(resp).build();
      mdB = MethodDescriptor.<Object, byte[]>newBuilder().setType(MethodDescriptor.MethodType.UNARY)
          .setFullMethodName(PUT).setRequestMarshaller(req).setResponseMarshaller(BYTES).build();
    }

    static final byte[] EMPTY = new byte[0];

    void callA() {
      sinkv += System.identityHashCode(ClientCalls.blockingUnaryCall(ch, mdA, CallOptions.DEFAULT, EMPTY));
    }

    void callB(Object request) {
      byte[] r = ClientCalls.blockingUnaryCall(ch, mdB, CallOptions.DEFAULT, request);
      if (r.length != 0) fail(name + "/b: response " + r.length + " B, want 0");
    }

    @Override void close() { ch.shutdownNow(); }
  }

  /** The core's transport, blocking delivery (req 16), with either codec. */
  static final class CoreCell extends Cell {
    final long rt, client, pathA, pathB;
    final int lenA, lenB;
    final ThreadLocal<long[]> out = new ThreadLocal<long[]>() {
      @Override protected long[] initialValue() { return new long[3]; }
    };

    CoreCell(String name, boolean incumbent, boolean retain, String sock, boolean pinned) {
      super(name, incumbent, retain);
      rt = NativeRpc.runtimeNew(Integer.getInteger("ak.rpc.workers", 2));
      if (rt == 0) fail("ak_runtime_new");
      byte[] uri = ("unix:" + sock).getBytes(StandardCharsets.UTF_8);
      client = pinned
          ? NativeRpc.clientNewOpts(rt, uri, uri.length, WIN, WIN, 0, MAXMSG, MAXMSG, 0)
          : NativeRpc.clientNew(rt, uri, uri.length);
      if (client == 0) fail("ak_client_new");
      byte[] a = ("/" + GET).getBytes(StandardCharsets.UTF_8), b = ("/" + PUT).getBytes(StandardCharsets.UTF_8);
      lenA = a.length;
      lenB = b.length;
      pathA = Mem.alloc(lenA);
      pathB = Mem.alloc(lenB);
      Mem.copyFromBytes(a, 0, pathA, lenA);
      Mem.copyFromBytes(b, 0, pathB, lenB);
    }

    static final byte[] EMPTY = new byte[0];

    void callA() {
      long[] o = out.get();
      int rc = NativeRpc.callUnary(client, pathA, lenA, EMPTY, 0, 0, o);
      if (rc != 0) fail(name + "/a: ak_call_unary returned " + rc);
      int n = (int) o[1];
      if (n != EXPECT_A.length) { NativeRpc.bytesFree(o[0], o[1], o[2]); fail(name + "/a: response " + n + " B"); }
      Object x;
      try {
        if (incumbentCodec) {
          // R-H17: cell B parses the core's response IN PLACE, as the C++ and C# hosts do: a
          // direct ByteBuffer over the core's bytes (one JNI call, no copy), which
          // protobuf-java reads through its Unsafe direct-buffer decoder. The parsed message
          // owns its strings and bytes (aliasing is off), so the buffer is freed after.
          try {
            x = PB_PARSER.parseFrom(NativeRpc.directBuffer(o[0], n));
          } finally {
            NativeRpc.bytesFree(o[0], o[1], o[2]);
          }
        } else {
          // Cell C: the binding's decode entry takes a Java array (it resolves spans against
          // it, ABI v1 7.4), so the core's bytes are copied once into one, and the binding
          // copies them once more into its native scratch for the core to parse.
          byte[] b = BUF.get();
          if (b.length < n) { b = new byte[Integer.highestOneBit(n - 1) * 2]; BUF.set(b); }
          Mem.copyToBytes(o[0], b, 0, n);
          NativeRpc.bytesFree(o[0], o[1], o[2]);
          x = FfiArms.decode(bind(), PAYLOAD, b, 0, n);
        }
      } catch (Exception e) {
        throw new IllegalStateException(e);
      }
      sinkv += System.identityHashCode(x);
    }

    void callB(Object request) {
      byte[] w;
      if (incumbentCodec) {
        w = ((Message) request).toByteArray();
      } else {
        Binding bd = bind();
        FfiArms.encode(bd, PAYLOAD, request);
        w = bd.take();
      }
      long[] o = out.get();
      int rc = NativeRpc.callUnary(client, pathB, lenB, w, 0, w.length, o);
      if (rc != 0) fail(name + "/b: ak_call_unary returned " + rc);
      int n = (int) o[1];
      NativeRpc.bytesFree(o[0], o[1], o[2]);
      if (n != 0) fail(name + "/b: response " + n + " B, want 0");
    }

    @Override void close() {
      NativeRpc.clientDestroy(client);
      NativeRpc.runtimeDestroy(rt);
    }
  }

  // ---- one sample ------------------------------------------------------------------

  /** `calls` calls over `inflight` blocking threads, chunked; returns {cpu_ns, wall_ns}. */
  static long[] sample(final Cell cell, final String dir, final int inflight, int calls, int chunk)
      throws Exception {
    final int perThread = Math.max(1, calls / inflight);
    final int ch = Math.max(1, Math.min(chunk, perThread));
    final int chunks = (perThread + ch - 1) / ch;
    final CyclicBarrier bar = new CyclicBarrier(inflight + 1);
    final Throwable[] err = new Throwable[1];
    Thread[] ts = new Thread[inflight];
    for (int t = 0; t < inflight; t++) {
      ts[t] = new Thread(() -> {
        Object[] pool = new Object[ch];
        try {
          int left = perThread;
          for (int c = 0; c < chunks; c++) {
            int k = Math.min(ch, left);
            if (dir.equals("b")) for (int i = 0; i < k; i++) pool[i] = cell.fresh();   // untimed
            bar.await();          // everyone prepared
            bar.await();          // the coordinator has read the clocks: go
            for (int i = 0; i < k; i++) {
              if (dir.equals("a")) cell.callA(); else cell.callB(pool[i]);
            }
            left -= k;
            bar.await();          // chunk done
          }
        } catch (Throwable e) {
          err[0] = e;
          Campaign.abort("req 18: " + cell.name + "/" + dir + ": " + e);
        } finally {
          releaseThread();
        }
      }, "rpc-" + cell.name + "-" + t);
      ts[t].start();
    }
    long cpu = 0, wall = 0;
    for (int c = 0; c < chunks; c++) {
      bar.await();
      long c0 = NativeRpc.processCpuNs(), t0 = System.nanoTime();
      bar.await();
      bar.await();
      wall += System.nanoTime() - t0;
      cpu += NativeRpc.processCpuNs() - c0;
    }
    for (Thread t : ts) t.join();
    if (err[0] != null) Campaign.abort("req 18: " + err[0]);
    return new long[] {cpu, wall, (long) perThread * inflight};
  }

  public static void main(String[] args) throws Exception {
    if (args.length >= 2 && args[0].equals("--serve")) {
      serve(args[1], "pinned".equals(System.getProperty("ak.camp.transport", "pinned")));
      return;
    }
    String sock = System.getProperty("ak.camp.socket");
    if (sock == null) throw new IllegalStateException("-Dak.camp.socket (the server's socket) is unset");
    String transport = System.getProperty("ak.camp.transport", "pinned");
    boolean pinned = transport.equals("pinned");
    int calls = Integer.getInteger("ak.camp.calls", 2000);
    int chunk = Integer.getInteger("ak.camp.chunk", 64);
    int warm = Integer.getInteger("ak.camp.warm", 2);
    String out = System.getProperty("ak.camp.out");
    NativeRpc.ensureBound();
    EXPECT_A = PbArms.build(PAYLOAD, Values.ASCII).toByteArray();

    EpollEventLoopGroup elg = new EpollEventLoopGroup();
    List<Cell> cells = new ArrayList<Cell>();
    cells.add(new GrpcCell("A", true, false, sock, elg, pinned));
    cells.add(new CoreCell("B", true, false, sock, pinned));
    if (ak.Variant.UNKNOWN_FIELDS) {
      cells.add(new CoreCell("C-retain", false, true, sock, pinned));
      cells.add(new CoreCell("C-drop", false, false, sock, pinned));
      cells.add(new GrpcCell("D-retain", false, true, sock, elg, pinned));
      cells.add(new GrpcCell("D-drop", false, false, sock, elg, pinned));
    } else {
      // WP5 step 10: the no-unknown build; A and B are the in-process controls.
      cells.add(new CoreCell("C-nounk", false, false, sock, pinned));
      cells.add(new GrpcCell("D-nounk", false, false, sock, elg, pinned));
    }

    String[] dirs = {"a", "b"};
    // Correctness first, per cell and direction: one checked call each (req 18, 26).
    for (Cell c : cells) {
      c.callA();
      c.callB(c.fresh());
      if (!c.incumbentCodec) {                 // the mode's own decode and encode, byte identity
        Binding b = c.bind();
        FfiArms.encode(b, PAYLOAD, FfiArms.decode(b, PAYLOAD, EXPECT_A, 0, EXPECT_A.length));
        byte[] re = b.take();
        if (!Arrays.equals(re, EXPECT_A)) fail(c.name + ": P2.2 decoded and re-encoded in its mode is "
            + re.length + " B and differs from the body");
      }
    }
    for (int k = 0; k < warm; k++)
      for (String d : dirs)
        for (int inf : INFLIGHT)
          for (Cell c : cells)
            sample(c, d, inf, calls, chunk);
    Campaign.meta("{\"suite\":\"rpc\",\"warmup_samples_per_cell\":" + warm + ",\"jit_ms_after_warmup\":"
        + Campaign.jitMs() + ",\"transport\":\"" + transport + "\",\"calls\":" + calls
        + ",\"chunk\":" + chunk + ",\"core_workers\":" + Integer.getInteger("ak.rpc.workers", 2)
        + ",\"socket\":\"unix\"}");
    for (int r = 1; r <= Campaign.ROUNDS; r++) {
      for (String d : dirs)
        for (int inf : INFLIGHT)
          for (Cell c : Campaign.rotate(cells, r)) {
            long[] v = sample(c, d, inf, calls, chunk);
            Campaign.add(new Campaign.Sample().s("suite", "rpc").s("cell", c.name).s("payload", PAYLOAD)
                .s("unknown_mode", c.incumbentCodec ? "default" : c.retain ? "retain"
                    : ak.Variant.UNKNOWN_FIELDS ? "drop" : "no-unknown").s("build", ak.Variant.NAME)
                .s("dir", d).s("transport", transport).n("inflight", inf).s("delivery",
                    c instanceof CoreCell ? "blocking" : null)
                .n("launch", Campaign.LAUNCH).n("round", r).n("cpu_ns", v[0]).n("wall_ns", v[1])
                .n("iters", v[2]));
          }
      Campaign.meta("{\"round\":" + r + ",\"jit_ms\":" + Campaign.jitMs() + "}");
    }
    for (Cell c : cells) c.close();
    // The leak counters (decision 11 rule 3), after every call of the run.
    releaseThread();                           // the correctness phase's Bindings
    long live = ak.Variant.UNKNOWN_FIELDS ? Native.unkLive() : 0L, reclaimed = UNK_RECLAIMED.get(), left = UNK_LEFT.get();
    Campaign.meta("{\"unk_leak\":{\"buffers_alive\":" + live + ",\"reclaimed_after_failed_decodes\":"
        + reclaimed + ",\"left_after_successful_decodes\":" + left + ",\"retain_bindings\":"
        + BINDINGS_RETAIN.get() + ",\"drop_bindings\":" + BINDINGS_DROP.get() + "}}");
    System.out.println("unknown-field leak counters after the run: buffers alive " + live
        + ", reclaimed after failed decodes " + reclaimed + ", left after successful decodes " + left
        + " (" + BINDINGS_RETAIN.get() + " retain / " + BINDINGS_DROP.get() + " drop bindings)");
    if (live != 0 || reclaimed != 0 || left != 0) fail("unknown-field leak counters are not 0");
    elg.shutdownGracefully(0, 0, java.util.concurrent.TimeUnit.SECONDS);
    Campaign.meta("{\"sink\":" + sinkv + "}");
    Campaign.flush(out);
    System.exit(0);
  }
}
