package ak;

import ak.shapes.Arms;
import ak.shapes.Binding;
import ak.shapes.FfiArms;
import ak.shapes.PbArms;
import ak.shapes.PbWalk;
import ak.shapes.Walk;
import com.google.protobuf.Message;
import io.grpc.CallOptions;
import io.grpc.ManagedChannel;
import io.grpc.MethodDescriptor;
import io.grpc.netty.shaded.io.grpc.netty.NettyChannelBuilder;
import io.grpc.netty.shaded.io.netty.channel.epoll.EpollDomainSocketChannel;
import io.grpc.netty.shaded.io.netty.channel.epoll.EpollEventLoopGroup;
import io.grpc.netty.shaded.io.netty.channel.unix.DomainSocketAddress;
import io.grpc.stub.ClientCalls;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

/**
 * design/CAMPAIGN.md section 4.2, the RPC grid, for the JVM.
 *
 * <p><b>Processes.</b> The server is the Rust slice's tonic {@code rpc_server}, the one RPC
 * server of every slice (CAMPAIGN req 13 as amended at 9f6d579fa, FIX-PLAN WP10; interface
 * {@code poc/rust/SERVER.md}), ONE process per launch started through
 * {@code poc/rust/serve.sh}, pinned to {@code AK_CPU_SERVER}, on two Unix domain sockets
 * (its {@code shipped} and {@code pinned} configurations). Service
 * {@code armonik.ffi.campaign.v1.Grid}: Fetch (a), Push (b), Upload (c), UploadStream (d),
 * UploadStreamCheck (the upload check). The timed client is {@code ak.RpcJmh}, one JMH
 * fork per cell, pinned to {@code AK_CPU_CLIENT}; its CPU is {@code CLOCK_PROCESS_CPUTIME_ID}
 * (req 21), which contains no server work.
 *
 * <pre>
 *   cell  codec                                 transport
 *   A     protobuf-java via grpc-java's         grpc-java (netty, epoll, unix socket),
 *         marshaller (ProtoLiteUtils)           blockingUnaryCall (a blocking stub's call)
 *   B     protobuf-java (toByteArray; parse     the core: ak_call_unary, BLOCKING delivery
 *         in place from a direct ByteBuffer)
 *   C     core-ffi (push decode; take())        the core, blocking
 *   D     core-ffi, as a grpc-java Marshaller   grpc-java, blockingUnaryCall
 *   E     host-gen (arm R)                      the core, blocking
 *   F     host-gen, as a grpc-java Marshaller   grpc-java, blockingUnaryCall
 * </pre>
 *
 * <p><b>Unknown-field mode</b> (req 12 as amended): C, D, E and F run once per mode their
 * codec has in this build: retain and drop in the full build ({@code C-retain},
 * {@code C-drop}, ...), no-unknown in the no-unknown build ({@code C-nounk}, ...). Retain
 * arms every position with the shim's grow (core-ffi) or captures into the facade's bag
 * (host-gen's CodecRetain). Before timing, each C to F cell decodes the P2.2 body in its mode
 * and re-encodes it, and the bytes must equal the body. After the run the shim's leak
 * counter and every retain Binding's reclaim counters must read 0 (req 18). A and B run the
 * incumbent in its default mode. Direction (a) is timed as {@code a} and {@code a+read}
 * (every field of the response walked, req 14).
 *
 * <p><b>Counting</b> ({@code -Dak.camp.count=1} on the counting shim, req 19): crossings per
 * call for B, C, D and E, no timing.
 *
 * <p><b>The no-unknown build</b> (WP5 step 10): this class compiled against the tree
 * generated from the plan relowered with unknown="drop" and run on that build's core
 * runs A, B and the C to F cells in {@code nounk}; A and B are the controls (each in its own
 * JMH fork) that let a ratio be formed against the full build's. Every sample carries
 * {@code unknown_mode} (default, retain, drop, no-unknown), {@code codec} and {@code build}.
 *
 * <p><b>The server does identical work in every cell</b> (req 13): direction (a) answers
 * every request with the SAME pre-serialised P2.2 bytes; (b) and (c) decode the request with
 * prost and answer empty; (d) decodes every message with prost and answers the data byte
 * count (u64 LE), plus, on UploadStreamCheck, the SHA-256 of every message as received.
 *
 * <p><b>Directions</b> (req 14): (a) empty request, P2.2 response, which the client decodes;
 * (b) a P2.2 request, encoded per call from a FRESH object graph (req 11: a pool rebuilt
 * between chunks, outside the timed phases), empty response.
 *
 * <p><b>Every call is checked</b> (req 18): an exception or a non-OK status fails, and the
 * response length must equal the expected one (the P2.2 body in (a), 0 in (b) and (c), the
 * byte count in (d)); under JMH a failure throws ({@link #THROW_ON_FAIL}), elsewhere it
 * exits non-zero. The plant ({@code -Dak.camp.plant=1}) expects one byte more in (c) and (d),
 * on the client (SERVER.md: the shared server's answers never change).
 *
 * <p><b>Transport</b> (req 17), one per process, {@code ak.camp.transport}:
 * the client's configuration against the server socket of the same name.
 * {@code shipped}: grpc-java's defaults over Netty epoll on the Unix domain socket (req 17
 * as amended, R-H28: packages/java configures no UDS channel), the core with its defaults
 * (no options: tonic's); the server's shipped socket is tonic's server defaults.
 * {@code pinned}: 4 MiB stream and connection windows (grpc-java: setting the window turns
 * BDP auto-tuning off; the core: stream_window = connection_window = 4 MiB,
 * adaptive_window = 0, tcp_nagle = 0), max messages 8 MiB; the server's pinned socket has
 * 4 MiB stream and connection windows, adaptive window off.
 */
public final class CampaignRpc {
  static final String PAYLOAD = "P2.2";
  static final String GET = "armonik.ffi.campaign.v1.Grid/Fetch";   // direction (a)
  static final String PUT = "armonik.ffi.campaign.v1.Grid/Push";    // direction (b)
  static final String UPLOAD = "armonik.ffi.campaign.v1.Grid/Upload";              // direction (c), unary
  static final String STREAM = "armonik.ffi.campaign.v1.Grid/UploadStream";        // direction (d), client stream
  static final String STREAM_CHECK = "armonik.ffi.campaign.v1.Grid/UploadStreamCheck";   // (d) with the digest
  /** Req 14 (c): the unary uploads, M5 (UploadResultDataMessage), 1 MB and 4 MB. */
  static final String[] C_PAYLOADS = {"P5.3", "P5.4"};
  /** Req 14 (d): (label, 2 MiB chunks): 4 MiB and 16 MiB, ArmoniK's UploadResultData stream. */
  static final String[] D_LABELS = {"4MiB", "16MiB"};
  static final int[] D_CHUNKS = {2, 8};
  static final int CHUNK = 2 * 1024 * 1024;
  static final int[] UP_INFLIGHT = {1, 8};
  /** -Dak.camp.plant=1 (req 18's control): every upload expects one byte more than it gets. */
  static final boolean PLANT = "1".equals(System.getProperty("ak.camp.plant"));

  /** Direction (d)'s upload: the messages as protobuf-java's and as the facade's, the data
   *  bytes and the SHA-256 of the messages' wire bytes (every encoder byte-identical, checked
   *  before any call). 2 MiB chunks of splitmix64 bytes, seeded as the rust slice's
   *  (0x5EED0000 + chunks), the ids on the first message only. */
  static final class StreamPayload {
    final Message[] p;
    final ak.shapes.UploadResultDataMessage[] f;
    final long dataBytes;
    final byte[] sha256;
    StreamPayload(int chunks) {
      long[] seed = {0x5EED0000L + chunks};
      p = new Message[chunks];
      f = new ak.shapes.UploadResultDataMessage[chunks];
      java.security.MessageDigest md;
      try { md = java.security.MessageDigest.getInstance("SHA-256"); }
      catch (java.security.NoSuchAlgorithmException e) { throw new IllegalStateException(e); }
      for (int i = 0; i < chunks; i++) {
        byte[] data = new byte[CHUNK];
        for (int k = 0; k < CHUNK; k += 8) {
          seed[0] += 0x9E3779B97F4A7C15L;
          long z = seed[0];
          z = (z ^ (z >>> 30)) * 0xBF58476D1CE4E5B9L;
          z = (z ^ (z >>> 27)) * 0x94D049BB133111EBL;
          z ^= z >>> 31;
          for (int j = 0; j < 8; j++) data[k + j] = (byte) (z >>> (8 * j));
        }
        boolean first = i == 0;
        ak.shapes.UploadResultData u = new ak.shapes.UploadResultData();
        u.session_id = first ? "session-u2" : "";
        u.result_id = first ? "result-u2" : "";
        u.data_chunk = data;
        f[i] = new ak.shapes.UploadResultDataMessage();
        f[i].upload = u;
        p[i] = ak.pb.UploadResultDataMessage.newBuilder().setUpload(ak.pb.UploadResultData.newBuilder()
            .setSessionId(u.session_id).setResultId(u.result_id)
            .setDataChunk(com.google.protobuf.ByteString.copyFrom(data))).build();
        byte[] wire = p[i].toByteArray();
        Enc e = new Enc(Arms.R_SITES);
        Arms.encodeR("P5.3", f[i], e);
        if (!Arrays.equals(e.toBytes(), wire)) throw new IllegalStateException("(d): arm R and protobuf-java encode chunk " + i + " differently");
        md.update(wire);
      }
      dataBytes = (long) chunks * CHUNK;
      sha256 = md.digest();
    }
  }
  static final StreamPayload[] STREAMS = new StreamPayload[D_CHUNKS.length];
  static final Object[] C_FACADE = new Object[C_PAYLOADS.length];
  static final Message[] C_PB = new Message[C_PAYLOADS.length];

  /** The (d) response check (req 18): the data byte count (u64 LE), then, from the check
   *  path, the SHA-256 of every message the server received. */
  static void checkStream(String who, byte[] r, StreamPayload pl, boolean check) {
    int want = check ? 40 : 8;
    if (r.length != want) fail(who + "/d: upload response " + r.length + " B, expected " + want);
    long got = 0;
    for (int i = 7; i >= 0; i--) got = (got << 8) | (r[i] & 0xFF);
    long exp = pl.dataBytes + (PLANT ? 1 : 0);
    if (got != exp) fail(who + "/d: the server received " + got + " data bytes, expected " + exp);
    if (check && !Arrays.equals(Arrays.copyOfRange(r, 8, 40), pl.sha256))
      fail(who + "/d: the server received other bytes than the uploaded ones (SHA-256 differs)");
  }
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

  static MethodDescriptor<byte[], byte[]> bytesStreamMd(String name) {
    return MethodDescriptor.<byte[], byte[]>newBuilder().setType(MethodDescriptor.MethodType.CLIENT_STREAMING)
        .setFullMethodName(name).setRequestMarshaller(BYTES).setResponseMarshaller(BYTES).build();
  }

  static MethodDescriptor<byte[], byte[]> bytesMd(String name) {
    return MethodDescriptor.<byte[], byte[]>newBuilder().setType(MethodDescriptor.MethodType.UNARY)
        .setFullMethodName(name).setRequestMarshaller(BYTES).setResponseMarshaller(BYTES).build();
  }

  /** Worker thread counts (req 4, R-H34): the client's Netty event loops (default for
   *  grpc-java: 2 x the CPUs the JVM sees; fixed here from `ak.rpc.eventLoops` so the header
   *  can state it), the core's runtime workers (`ak.rpc.workers`); grpc-java's call executor
   *  is its default shared cached pool (grows per concurrent call), stated. */
  /** D8 / D14 (owner 2026-10-03): every pool is sized to AK_WORKERS (8, the threads of the
   *  CLIENT set), passed as -Dak.workers: the core runtime (ak_runtime_new), the Netty event
   *  loop group, grpc-java's call executor (a fixed pool in place of its default cached one). */
  static final int WORKERS = Integer.getInteger("ak.workers", 8);
  static final int EVENT_LOOPS = Integer.getInteger("ak.rpc.eventLoops", WORKERS);
  static final int CORE_WORKERS = Integer.getInteger("ak.rpc.workers", WORKERS);
  static final int GRPC_EXECUTOR = Integer.getInteger("ak.rpc.executor", WORKERS);
  static java.util.concurrent.ExecutorService executor;
  static synchronized java.util.concurrent.Executor grpcExecutor() {
    if (executor == null) {
      executor = java.util.concurrent.Executors.newFixedThreadPool(GRPC_EXECUTOR, r -> {
        Thread t = new Thread(r, "grpc-exec");
        t.setDaemon(true);
        return t;
      });
    }
    return executor;
  }

  /** The server address (D10): "tcp:127.0.0.1:PORT" (every timed run) or a Unix socket path. */
  static int tcpPort(String addr) {
    return addr.startsWith("tcp:") ? Integer.parseInt(addr.substring(addr.lastIndexOf(':') + 1)) : -1;
  }

  /** Req 17 as amended (D10): TCP_NODELAY read back on the live sockets to the server's port;
   *  fails when none is found or one has Nagle on. Returns "n/n". */
  static String checkNodelay(String addr) {
    int port = tcpPort(addr);
    if (port < 0) return "uds";
    long r = Native.tcpNodelay(port);
    long n = r >>> 32, on = r & 0xffffffffL;
    if (r < 0 || n == 0) fail("TCP_NODELAY read-back: no live socket to 127.0.0.1:" + port);
    if (on != n) fail("TCP_NODELAY read-back: " + (n - on) + " of " + n + " sockets to 127.0.0.1:" + port + " have Nagle on");
    return on + "/" + n;
  }

  /** CAMPAIGN 4.0 as amended: the `armonik` transport configuration (cell A through ArmoniK's
   *  builder; the core cells keep the core's current client configuration, ak_client_new with
   *  no options: tonic's defaults, nodelay on). */
  static final boolean ARMONIK = "armonik".equals(System.getProperty("ak.camp.transport"));
  static ManagedChannel armonikChannel(String endpoint) {
    try {
      Class<?> b = Class.forName("fr.aneo.armonik.client.GrpcChannelBuilder");
      Object x = b.getMethod("forEndpoint", String.class).invoke(null, endpoint);
      x = b.getMethod("withUnsecureConnection").invoke(x);
      return (ManagedChannel) b.getMethod("build").invoke(x);
    } catch (ReflectiveOperationException e) {
      throw new IllegalStateException("packages/java GrpcChannelBuilder (build/armonik-client on the classpath)", e);
    }
  }

  static String threadsJson() {
    if (ARMONIK)   // cell A on ArmoniK's builder: grpc-java's default shared group and executor
      return "{\"ak_workers\":" + WORKERS + ",\"netty_event_loops\":\"grpc-java default shared group, "
          + System.getProperty("io.grpc.netty.shaded.io.netty.eventLoopThreads", "2 x CPUs") + " threads\",\"core_runtime_workers\":"
          + CORE_WORKERS + ",\"grpc_executor\":\"grpc-java default (shared cached pool)\""
          + ",\"cpus_seen_by_jvm\":" + Runtime.getRuntime().availableProcessors() + "}";
    return "{\"ak_workers\":" + WORKERS + ",\"netty_event_loops\":" + EVENT_LOOPS + ",\"core_runtime_workers\":"
        + CORE_WORKERS + ",\"grpc_executor\":\"fixed pool of " + GRPC_EXECUTOR + "\""
        + ",\"cpus_seen_by_jvm\":" + Runtime.getRuntime().availableProcessors() + "}";
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

  /** Set by ak.RpcJmh: a failed check throws (JMH records it and, with -foe true, stops the
   *  run) instead of exiting the forked JVM. */
  static volatile boolean THROW_ON_FAIL;

  static void fail(String why) {
    if (THROW_ON_FAIL) throw new IllegalStateException("req 18: " + why);
    Campaign.abort("req 18: " + why);
  }

  /** protobuf-java's parser for the payload's root, for cell B's in-place parse. */
  static final com.google.protobuf.Parser<? extends Message> PB_PARSER =
      PbArms.build(PAYLOAD, Values.ASCII).getParserForType();

  /** A cell's codec (req 12): the incumbent, the core through the C ABI, or host-gen. */
  static final int INC = 0, FFI = 1, HOST = 2;

  static final ThreadLocal<Enc> HENC = new ThreadLocal<Enc>() {
    @Override protected Enc initialValue() { return new Enc(Arms.R_SITES); }
  };

  abstract static class Cell {
    final String name;
    final int codec;
    final boolean incumbentCodec;
    final boolean retain;                        // C to F: the unknown-field mode (req 12)
    Cell(String name, int codec, boolean retain) {
      this.name = name; this.codec = codec; this.incumbentCodec = codec == INC; this.retain = retain;
    }
    /** This thread's Binding in this cell's mode. */
    Binding bind() { return retain ? BIND_U.get() : BIND.get(); }
    /** (a); `read` = walk every field of the decoded response (req 14, `a+read`). */
    abstract void callA(boolean read);
    abstract void callB(Object request);         // (b); request from the pool
    void close() {}
    /** The request object for (b): protobuf-java for A and B, the facade for C to F. */
    Object fresh() { return incumbentCodec ? PbArms.build(PAYLOAD, Values.ASCII) : Arms.build(PAYLOAD, Values.ASCII); }
    String mode() {
      return incumbentCodec ? "default" : retain ? "retain" : ak.Variant.UNKNOWN_FIELDS ? "drop" : "no-unknown";
    }
    /** The response bytes, decoded by this cell's codec (C, D: the binding; E, F: arm R). */
    Object decodeBytes(byte[] b, int n) throws Exception {
      if (codec == FFI) return FfiArms.decode(bind(), PAYLOAD, b, 0, n);
      return retain ? Arms.decodeRRetain(PAYLOAD, new Dec(), b, 0, n) : Arms.decodeR(PAYLOAD, new Dec(), b, 0, n);
    }
    /** A facade request encoded by this cell's codec, as the array its transport takes. */
    byte[] encodeFacade(Object v) { return encodeFacade(PAYLOAD, v); }
    byte[] encodeFacade(String id, Object v) {
      if (codec == FFI) {
        Binding b = bind();
        FfiArms.encode(b, id, v);
        return b.take();
      }
      Enc e = HENC.get();
      if (retain) Arms.encodeRRetain(id, v, e); else Arms.encodeR(id, v, e);
      return e.toBytes();
    }
    /** (c): the unary upload of C_PAYLOADS[k]. */
    abstract void callC(int k);
    /** (d): the streamed upload of STREAMS[k]; `check` = the digest path. */
    abstract void callD(int k, boolean check);
    long walk(Object x) {
      return incumbentCodec ? PbWalk.walk(Arms.root(PAYLOAD), x) : Walk.walk(Arms.root(PAYLOAD), x);
    }
  }

  /** grpc-java, with the cell's codec as the response / request marshaller. The call is
   *  `ClientCalls.blockingUnaryCall`, which is what a generated blocking stub calls: the
   *  idiomatic call of packages/java's clients (ResultsBlockingStub and the like; req 16). */
  static final class GrpcCell extends Cell {
    final ManagedChannel ch;
    final MethodDescriptor<byte[], Object> mdA;
    final MethodDescriptor<Object, byte[]> mdB, mdC, mdD, mdDCheck;

    GrpcCell(String name, int codec, boolean retain, String sock, EpollEventLoopGroup elg, boolean pinned) {
      super(name, codec, retain);
      // Req 17 as amended (D10): TCP 127.0.0.1 over Netty epoll, TCP_NODELAY set as a channel
      // option and read back on the live socket (checkNodelay); `shipped` is grpc-java's
      // defaults otherwise, `pinned` sets the window. A Unix socket path is still accepted.
      int port = tcpPort(sock);
      if (ARMONIK && port >= 0) {
        // CAMPAIGN 4.0 as amended (b58543f7b): ArmoniK's own channel, packages/java's
        // GrpcChannelBuilder called directly with its package defaults (NettyChannelBuilder
        // forAddress(host, port), maxInboundMessageSize 8 MiB, maxInboundMetadataSize 1 MiB, no
        // keepalive, idle timeout or retry, plaintext): grpc-java's default event loop group
        // and call executor, TCP_NODELAY as grpc-java's Netty sets it (read back, checkNodelay).
        ch = armonikChannel("http://127.0.0.1:" + port);
      } else {
      NettyChannelBuilder cb = port >= 0
          ? NettyChannelBuilder.forAddress(new java.net.InetSocketAddress("127.0.0.1", port))
              .channelType(io.grpc.netty.shaded.io.netty.channel.epoll.EpollSocketChannel.class)
              .withOption(io.grpc.netty.shaded.io.netty.channel.ChannelOption.TCP_NODELAY, true)
          : NettyChannelBuilder.forAddress(new DomainSocketAddress(sock))
              .channelType(EpollDomainSocketChannel.class)
              // grpc-java derives :authority from the socket path, which the Rust server's
              // HTTP/2 stack refuses (RST_STREAM PROTOCOL_ERROR): a host name, as over TCP.
              .overrideAuthority("localhost");
      cb.eventLoopGroup(elg).usePlaintext().executor(grpcExecutor());
      if (pinned) cb.flowControlWindow(WIN).maxInboundMessageSize(MAXMSG);
      ch = cb.build();
      }
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
            return decodeBytes(b, at);
          } catch (Exception e) {
            throw new IllegalStateException(e);
          }
        }
      };
      MethodDescriptor.Marshaller<Object> req = new MethodDescriptor.Marshaller<Object>() {
        @Override public InputStream stream(Object v) {
          if (incumbentCodec) return pm.stream((Message) v);   // grpc-java's production path
          return new ByteArrayInputStream(encodeFacade(v));
        }
        @Override public Object parse(InputStream s) { throw new UnsupportedOperationException(); }
      };
      // Req 14 (c), (d): M5 requests through the cell's codec (the incumbent: grpc-java's
      // marshaller for UploadResultDataMessage; D, F: the facade encoded by the core / arm R).
      final MethodDescriptor.Marshaller<Message> pm5 = io.grpc.protobuf.lite.ProtoLiteUtils.marshaller(
          ak.pb.UploadResultDataMessage.getDefaultInstance());
      MethodDescriptor.Marshaller<Object> req5 = new MethodDescriptor.Marshaller<Object>() {
        @Override public InputStream stream(Object v) {
          if (incumbentCodec) return pm5.stream((Message) v);
          return new ByteArrayInputStream(encodeFacade("P5.3", v));
        }
        @Override public Object parse(InputStream s) { throw new UnsupportedOperationException(); }
      };
      mdC = MethodDescriptor.<Object, byte[]>newBuilder().setType(MethodDescriptor.MethodType.UNARY)
          .setFullMethodName(UPLOAD).setRequestMarshaller(req5).setResponseMarshaller(BYTES).build();
      mdD = MethodDescriptor.<Object, byte[]>newBuilder().setType(MethodDescriptor.MethodType.CLIENT_STREAMING)
          .setFullMethodName(STREAM).setRequestMarshaller(req5).setResponseMarshaller(BYTES).build();
      mdDCheck = MethodDescriptor.<Object, byte[]>newBuilder().setType(MethodDescriptor.MethodType.CLIENT_STREAMING)
          .setFullMethodName(STREAM_CHECK).setRequestMarshaller(req5).setResponseMarshaller(BYTES).build();
      mdA = MethodDescriptor.<byte[], Object>newBuilder().setType(MethodDescriptor.MethodType.UNARY)
          .setFullMethodName(GET).setRequestMarshaller(BYTES).setResponseMarshaller(resp).build();
      mdB = MethodDescriptor.<Object, byte[]>newBuilder().setType(MethodDescriptor.MethodType.UNARY)
          .setFullMethodName(PUT).setRequestMarshaller(req).setResponseMarshaller(BYTES).build();
    }

    static final byte[] EMPTY = new byte[0];

    void callA(boolean read) {
      Object x = ClientCalls.blockingUnaryCall(ch, mdA, CallOptions.DEFAULT, EMPTY);
      sinkv += read ? walk(x) : System.identityHashCode(x);
    }

    void callB(Object request) {
      byte[] r = ClientCalls.blockingUnaryCall(ch, mdB, CallOptions.DEFAULT, request);
      if (r.length != 0) fail(name + "/b: response " + r.length + " B, want 0");
    }

    void callC(int k) {
      Object req = incumbentCodec ? C_PB[k] : C_FACADE[k];
      byte[] r = ClientCalls.blockingUnaryCall(ch, mdC, CallOptions.DEFAULT, req);
      if (r.length != (PLANT ? 1 : 0)) fail(name + "/c: response " + r.length + " B, want " + (PLANT ? 1 : 0));
    }

    /** (d) through grpc-java's own client streaming, in its idiomatic form: a client-
     *  streaming call has no blocking stub, so this is what the async stub does,
     *  ClientCalls.asyncClientStreamingCall with a StreamObserver, messages handed to
     *  onNext, the response awaited on a future. */
    void callD(int k, boolean check) {
      final java.util.concurrent.CompletableFuture<byte[]> done = new java.util.concurrent.CompletableFuture<byte[]>();
      io.grpc.stub.StreamObserver<Object> req = ClientCalls.asyncClientStreamingCall(
          ch.newCall(check ? mdDCheck : mdD, CallOptions.DEFAULT), new io.grpc.stub.StreamObserver<byte[]>() {
            byte[] got;
            @Override public void onNext(byte[] v) { got = v; }
            @Override public void onError(Throwable t) { done.completeExceptionally(t); }
            @Override public void onCompleted() { done.complete(got == null ? new byte[0] : got); }
          });
      StreamPayload pl = STREAMS[k];
      for (int i = 0; i < pl.p.length; i++) req.onNext(incumbentCodec ? pl.p[i] : pl.f[i]);
      req.onCompleted();
      byte[] r;
      try { r = done.get(); } catch (Exception e) { throw new IllegalStateException(name + "/d: " + e, e); }
      checkStream(name, r, pl, check);
    }

    @Override void close() { ch.shutdownNow(); }
  }

  /** The core's transport, blocking delivery (req 16), with the cell's codec. */
  static final class CoreCell extends Cell {
    final long rt, client, pathA, pathB, pathC, pathD, pathDCheck;
    final int lenA, lenB, lenC, lenD, lenDCheck;
    /** The framed twin (Bf, Cf-*, Ef-*): ak_client_set_framed(client, 1), ABI v1 section 9. */
    final boolean framed;
    /** Cc-*: cell C's request through take() into a Java array and ak_call_unary (the copy
     *  path, a labelled extra); C itself uses ak_call_unary_enc (the move path). */
    boolean copyPath;
    final ThreadLocal<long[]> out = new ThreadLocal<long[]>() {
      @Override protected long[] initialValue() { return new long[3]; }
    };

    CoreCell(String name, int codec, boolean retain, String sock, boolean pinned) {
      this(name, codec, retain, sock, pinned, false);
    }

    static long path(String p) {
      byte[] b = ("/" + p).getBytes(StandardCharsets.UTF_8);
      long x = Mem.alloc(b.length);
      Mem.copyFromBytes(b, 0, x, b.length);
      return x;
    }

    CoreCell(String name, int codec, boolean retain, String sock, boolean pinned, boolean framed) {
      super(name, codec, retain);
      this.framed = framed;
      rt = NativeRpc.runtimeNew(CORE_WORKERS);
      if (rt == 0) fail("ak_runtime_new");
      // D10: http://127.0.0.1:PORT; tonic's Endpoint sets TCP_NODELAY by default (shipped,
      // no options) and pinned passes tcp_nagle = 0; both read back (checkNodelay).
      int port = tcpPort(sock);
      byte[] uri = (port >= 0 ? "http://127.0.0.1:" + port : "unix:" + sock).getBytes(StandardCharsets.UTF_8);
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
      pathC = path(UPLOAD); lenC = UPLOAD.length() + 1;
      pathD = path(STREAM); lenD = STREAM.length() + 1;
      pathDCheck = path(STREAM_CHECK); lenDCheck = STREAM_CHECK.length() + 1;
      if (framed && NativeRpc.clientSetFramed(client, 1) != 0) fail(name + ": ak_client_set_framed");
    }

    static final byte[] EMPTY = new byte[0];

    void callA(boolean read) {
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
          // Cells C and E: the binding's decode entry and arm R both take a Java array (the
          // binding resolves spans against it, ABI v1 7.4), so the core's bytes are copied
          // once into one; the binding copies them once more into its native scratch.
          byte[] b = BUF.get();
          if (b.length < n) { b = new byte[Integer.highestOneBit(n - 1) * 2]; BUF.set(b); }
          Mem.copyToBytes(o[0], b, 0, n);
          NativeRpc.bytesFree(o[0], o[1], o[2]);
          x = decodeBytes(b, n);
        }
      } catch (Exception e) {
        throw new IllegalStateException(e);
      }
      sinkv += read ? walk(x) : System.identityHashCode(x);
    }

    /** Cell C (codec FFI, not the copy path): encode into the binding's context and hand it
     *  to ak_call_unary_enc, which moves the encoded bytes into the request body. */
    int unaryEnc(long path, int len, String id, Object v, long[] o) {
      Binding b = bind();
      FfiArms.encode(b, id, v);
      return NativeRpc.callUnaryEnc(client, path, len, b.encCtx, o);
    }

    void callB(Object request) {
      long[] o = out.get();
      int rc;
      if (codec == FFI && !copyPath) {
        rc = unaryEnc(pathB, lenB, PAYLOAD, request, o);
      } else {
        byte[] w = incumbentCodec ? ((Message) request).toByteArray() : encodeFacade(request);
        rc = NativeRpc.callUnary(client, pathB, lenB, w, 0, w.length, o);
      }
      if (rc != 0) fail(name + "/b: ak_call_unary returned " + rc);
      int n = (int) o[1];
      NativeRpc.bytesFree(o[0], o[1], o[2]);
      if (n != 0) fail(name + "/b: response " + n + " B, want 0");
    }

    void callC(int k) {
      long[] o = out.get();
      int rc;
      if (codec == FFI && !copyPath) {
        rc = unaryEnc(pathC, lenC, C_PAYLOADS[k], C_FACADE[k], o);
      } else {
        byte[] w = incumbentCodec ? C_PB[k].toByteArray() : encodeFacade(C_PAYLOADS[k], C_FACADE[k]);
        rc = NativeRpc.callUnary(client, pathC, lenC, w, 0, w.length, o);
      }
      if (rc != 0) fail(name + "/c: ak_call_unary returned " + rc);
      int n = (int) o[1];
      NativeRpc.bytesFree(o[0], o[1], o[2]);
      if (n != (PLANT ? 1 : 0)) fail(name + "/c: response " + n + " B, want " + (PLANT ? 1 : 0));
    }

    /** (d) through the core's client streaming: open, one send per message (B protobuf-java's
     *  bytes, ak_call_send; C the binding's encode, ak_call_send_enc, the context's output
     *  moved; E arm R's bytes, ak_call_send), recv, free, destroy. */
    void callD(int k, boolean check) {
      StreamPayload pl = STREAMS[k];
      long h = NativeRpc.callOpen(client, check ? pathDCheck : pathD, check ? lenDCheck : lenD);
      if (h == 0) fail(name + "/d: ak_call_open returned NULL");
      int n = pl.p.length;
      for (int i = 0; i < n; i++) {
        int last = i + 1 == n ? 1 : 0, rc;
        if (incumbentCodec) {
          byte[] w = pl.p[i].toByteArray();
          rc = NativeRpc.callSend(h, w, 0, w.length, last);
        } else if (codec == FFI) {
          Binding b = bind();
          FfiArms.encode(b, "P5.3", pl.f[i]);
          rc = NativeRpc.callSendEnc(h, b.encCtx, last);
        } else {
          Enc e = HENC.get();
          if (retain) Arms.encodeRRetain("P5.3", pl.f[i], e); else Arms.encodeR("P5.3", pl.f[i], e);
          rc = NativeRpc.callSend(h, e.buf, 0, e.len, last);
        }
        if (rc != 0) { NativeRpc.callCancel(h); NativeRpc.callDestroy(h); fail(name + "/d: ak_call_send chunk " + i + " rc " + rc); }
      }
      long[] o = new long[4];
      int rc = NativeRpc.callRecv(h, o);
      if (rc != 0) { NativeRpc.callDestroy(h); fail(name + "/d: ak_call_recv rc " + rc + ", grpc status " + o[3]); }
      byte[] r = new byte[(int) o[1]];
      if (o[1] > 0) Mem.copyToBytes(o[0], r, 0, (int) o[1]);
      NativeRpc.bytesFree(o[0], o[1], o[2]);
      NativeRpc.callDestroy(h);
      checkStream(name, r, pl, check);
    }

    @Override void close() {
      NativeRpc.clientDestroy(client);
      NativeRpc.runtimeDestroy(rt);
    }
  }

  /** The cell names of this build (req 12): A, B, C to F in every mode their codec has, the
   *  framed twins Bf, Cf-*, Ef-* and the copy-path extra Cc-*. */
  static List<String> cellNames() {
    List<String> n = new ArrayList<String>();
    String[] modes = ak.Variant.UNKNOWN_FIELDS ? new String[] {"retain", "drop"} : new String[] {"nounk"};
    n.add("A");
    n.add("B");
    for (String x : new String[] {"C", "D", "E", "F"}) for (String m : modes) n.add(x + "-" + m);
    n.add("Bf");
    for (String x : new String[] {"Cf", "Ef", "Cc"}) for (String m : modes) n.add(x + "-" + m);
    return n;
  }

  /** One cell by name; it opens its own channel (grpc-java) or client (the core). */
  static Cell cell(String name, String sock, EpollEventLoopGroup elg, boolean pinned) {
    String stem = name.contains("-") ? name.substring(0, name.indexOf('-')) : name;
    boolean retain = name.endsWith("-retain");
    switch (stem) {
      case "A": return new GrpcCell("A", INC, false, sock, elg, pinned);
      case "B": return new CoreCell("B", INC, false, sock, pinned);
      case "Bf": return new CoreCell("Bf", INC, false, sock, pinned, true);
      case "C": return new CoreCell(name, FFI, retain, sock, pinned);
      case "Cf": return new CoreCell(name, FFI, retain, sock, pinned, true);
      case "Cc": { CoreCell cc = new CoreCell(name, FFI, retain, sock, pinned); cc.copyPath = true; return cc; }
      case "D": return new GrpcCell(name, FFI, retain, sock, elg, pinned);
      case "E": return new CoreCell(name, HOST, retain, sock, pinned);
      case "Ef": return new CoreCell(name, HOST, retain, sock, pinned, true);
      case "F": return new GrpcCell(name, HOST, retain, sock, elg, pinned);
      default: throw new IllegalArgumentException("no cell " + name);
    }
  }

  static List<Cell> cells(String sock, EpollEventLoopGroup elg, boolean pinned) {
    List<Cell> cells = new ArrayList<Cell>();
    for (String n : cellNames()) cells.add(cell(n, sock, elg, pinned));
    return cells;
  }

  /** The (dir key, dir, payload, inflight) combinations every cell runs (req 14, 15): a,
   *  a+read and b at 1/8/16; c (P5.3, P5.4) and d (4 MiB, 16 MiB) at 1/8. */
  static List<String[]> combos() {
    List<String[]> out = new ArrayList<String[]>();
    for (String d : new String[] {"a", "a+read", "b"})
      for (int k : INFLIGHT) out.add(new String[] {d, d, PAYLOAD, String.valueOf(k)});
    for (int i = 0; i < C_PAYLOADS.length; i++)
      for (int k : UP_INFLIGHT) out.add(new String[] {"c:" + i, "c", C_PAYLOADS[i], String.valueOf(k)});
    for (int i = 0; i < D_LABELS.length; i++)
      for (int k : UP_INFLIGHT) out.add(new String[] {"d:" + i, "d", D_LABELS[i], String.valueOf(k)});
    return out;
  }

  /** One call of `c` in direction key `key` ("a", "a+read", "b", "c:<k>", "d:<k>"). */
  static void call(Cell c, String key, Object req) {
    if (key.equals("b")) c.callB(req);
    else if (key.startsWith("c:")) c.callC(Integer.parseInt(key.substring(2)));
    else if (key.startsWith("d:")) c.callD(Integer.parseInt(key.substring(2)), false);
    else c.callA(key.equals("a+read"));
  }

  /** Req 18 and 26 per cell, before any timing: every direction once, (d) through the digest
   *  path, and the cell's own decode and re-encode of P2.2 byte-identical. */
  static void precheck(Cell c) {
    c.callA(false);
    c.callA(true);
    c.callB(c.fresh());
    for (int k = 0; k < C_PAYLOADS.length; k++) c.callC(k);
    for (int k = 0; k < D_CHUNKS.length; k++) c.callD(k, true);
    if (!c.incumbentCodec) {
      byte[] re;
      try { re = c.encodeFacade(c.decodeBytes(EXPECT_A, EXPECT_A.length)); }
      catch (Exception e) { throw new IllegalStateException(e); }
      if (!Arrays.equals(re, EXPECT_A)) fail(c.name + ": P2.2 decoded and re-encoded in its mode is "
          + re.length + " B and differs from the body");
    }
  }

  /** Decision 11 rule 3, after a cell's run: 0 buffers alive, none left or reclaimed. */
  static void leakCheck() {
    long live = ak.Variant.UNKNOWN_FIELDS ? Native.unkLive() : 0L, reclaimed = UNK_RECLAIMED.get(), left = UNK_LEFT.get();
    if (live != 0 || reclaimed != 0 || left != 0)
      fail("unknown-field leak counters are not 0: alive " + live + ", reclaimed " + reclaimed + ", left " + left);
  }


  /** Req 19 as amended (R-H31): crossings per call for cells B, C, D and E, from the
   *  counting shim (-DAK_HOST_COUNT: every JNI entry into the core, the RPC ones included)
   *  over the counting core (the codec's upcalls). One untimed call first per (cell, dir),
   *  then one counted call. Retain mode arms every position with the shim's grow (the
   *  timed build's geometric grow, req 19 as amended) and no pre-placed buffer. */
  static void countCalls(List<Cell> cells) throws Exception {
    System.out.println("# rpc crossings per call: cell dir forward(host, every core entry point) reverse(core upcalls) grow");
    for (Cell c : cells) {
      if (c.name.startsWith("A") || c.name.startsWith("F")) continue;
      for (String d : new String[] {"a", "b", "c/P5.3", "c/P5.4", "d/4MiB", "d/16MiB"}) {
        Object req = c.fresh();
        runOne(c, d, req);
        req = c.fresh();
        long[] hc = new long[2], ec = new long[6], dc = new long[6];
        Binding b = c.codec == FFI ? c.bind() : null;
        if (b != null) {
          Native.encCountersReset(b.encCtx);
          Native.decCountersReset(b.contextOf(Arms.root(PAYLOAD)));
        }
        Native.hostCountsReset();
        runOne(c, d, req);
        Native.hostCounts(hc);
        long rev = 0;
        if (b != null) {
          Native.encCounters(b.encCtx, ec);
          Native.decCounters(b.contextOf(Arms.root(PAYLOAD)), dc);
          rev = ec[1] + dc[1];
        }
        System.out.println(String.format("RPC %-9s %-2s %8d %8d %6d", c.name, d, hc[0], rev, hc[1]));
      }
    }
  }

  /** The request's send path, per sample: the core's reference or framed path (and, for C,
   *  the move path ak_call_unary_enc / ak_call_send_enc, or Cc's copy path), or grpc-java's. */
  static String sendPath(Cell c) {
    if (!(c instanceof CoreCell)) return "grpc-java";
    CoreCell k = (CoreCell) c;
    String p = k.framed ? "framed" : "reference";
    if (k.codec == FFI) p += k.copyPath ? "+copy" : "+move";
    return p;
  }

  /** One call of `c` in the counting mode's direction names. */
  static void runOne(Cell c, String d, Object req) {
    switch (d) {
      case "a": c.callA(false); break;
      case "b": c.callB(req); break;
      case "c/P5.3": c.callC(0); break;
      case "c/P5.4": c.callC(1); break;
      case "d/4MiB": c.callD(0, false); break;
      default: c.callD(1, false); break;
    }
  }

  /** Req 14 (c), (d): the uploads, built and checked before any call. (c)'s requests are the
   *  payload builders' P5.3 / P5.4, checked against the manifest's sha256. */
  static void uploads() {
    for (int k = 0; k < C_PAYLOADS.length; k++) {
      C_FACADE[k] = Arms.build(C_PAYLOADS[k], Values.ASCII);
      C_PB[k] = PbArms.build(C_PAYLOADS[k], Values.ASCII);
      Enc e = new Enc(Arms.R_SITES);
      Arms.encodeR(C_PAYLOADS[k], C_FACADE[k], e);
      if (!Values.sha256Hex(e.toBytes()).equals(Payloads.row(C_PAYLOADS[k]).sha256))
        fail("(c): " + C_PAYLOADS[k] + " differs from the manifest");
    }
    for (int k = 0; k < D_CHUNKS.length; k++) STREAMS[k] = new StreamPayload(D_CHUNKS[k]);
  }

  public static void main(String[] args) throws Exception {
    if (args.length >= 1 && args[0].equals("--combos")) {
      // The JMH `combo` params (req 22a as decided e6c909630): one fork per (cell, combination).
      StringBuilder sb = new StringBuilder();
      for (String[] x : combos()) sb.append(sb.length() == 0 ? "" : ",").append(x[0]).append('/').append(x[3]);
      System.out.println(sb);
      return;
    }
    if (args.length >= 1 && args[0].equals("--list")) {
      // The JMH `cell` params of this build and transport, rotated one step per launch (req 22).
      List<String> names = Campaign.rotate(cellNames(), Integer.getInteger("ak.camp.launch", 1) - 1);
      StringBuilder sb = new StringBuilder();
      for (String nm : names) sb.append(sb.length() == 0 ? "" : ",").append(nm);
      System.out.println(sb);
      return;
    }
    String sock = System.getProperty("ak.camp.socket");
    if (sock == null) throw new IllegalStateException("-Dak.camp.socket (the server's socket) is unset");
    String transport = System.getProperty("ak.camp.transport", "pinned");
    boolean pinned = transport.equals("pinned");
    NativeRpc.ensureBound();
    EXPECT_A = PbArms.build(PAYLOAD, Values.ASCII).toByteArray();

    uploads();
    EpollEventLoopGroup elg = new EpollEventLoopGroup(EVENT_LOOPS);
    List<Cell> cells = cells(sock, elg, pinned);
    if ("1".equals(System.getProperty("ak.camp.uploadcheck"))) {
      // Req 18 for (c) and (d), before any timing (the gate runs it on both builds, and with
      // -Dak.camp.plant=1 as the control that must fail): every cell's unary uploads, and
      // every cell's streamed uploads through the digest path, count and SHA-256 compared.
      for (Cell c : cells) {
        for (int k = 0; k < C_PAYLOADS.length; k++) c.callC(k);
        for (int k = 0; k < D_CHUNKS.length; k++) c.callD(k, true);
        System.out.println("  upload check " + c.name + ": (c) P5.3, P5.4 accepted; (d) 4 MiB, 16 MiB: count and SHA-256 identical");
      }
      System.out.println("UPLOAD CHECK PASSED (" + cells.size() + " cells; TCP_NODELAY read back on " + checkNodelay(sock) + " live sockets)");
      for (Cell c : cells) c.close();
      releaseThread();
      System.exit(0);
    }
    if ("1".equals(System.getProperty("ak.camp.count"))) {
      if (Native.hostCounting() != 1) fail("ak.camp.count needs the counting shim (-DAK_HOST_COUNT)");
      countCalls(cells);
      for (Cell c : cells) c.close();
      releaseThread();
      System.exit(0);
    }

    throw new IllegalArgumentException("timing runs on JMH (ak.RpcJmh); this client is --list,"
        + " ak.camp.count or ak.camp.uploadcheck");
  }
}
