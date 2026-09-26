package ak;

import ak.shapes.Arms;
import ak.shapes.Binding;
import ak.shapes.FfiArms;
import ak.shapes.PbArms;
import ak.shapes.PbWalk;
import ak.shapes.Walk;
import com.google.protobuf.Message;
import io.grpc.Drainable;
import io.grpc.MethodDescriptor;
import io.grpc.protobuf.lite.ProtoLiteUtils;
import java.io.InputStream;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * design/CAMPAIGN.md section 4.1, the codec suite, for the JVM. One process, pinned to
 * {@code AK_CPU_CLIENT} by the runner ({@code gen/run_campaign.sh}); no server.
 *
 * <pre>
 *   arm             unknown_mode  what one iteration is
 *   incumbent-prod  default       grpc-java's marshaller (ProtoLiteUtils): stream() drained
 *                                 into a reused sink / parse() of a KnownLength stream (R14)
 *   incumbent-best  default       toByteArray() / parseFrom(byte[])
 *   core-ffi        drop          the generated binding through the C ABI, push decode;
 *                                 encode = ak_encode_* + ak_enc_take into the reused sink
 *   core-ffi-pull   drop          decode only: ak_parse_* + the drained replay
 *   host-gen        drop, retain  arm R (Codec / CodecRetain) + copy into the reused sink
 *   core-ffi        retain        every position armed (decision 11), u-group encode
 * </pre>
 *
 * <p><b>The no-unknown build</b> (FIX-PLAN WP5 step 10, req 10's third mode): the same class
 * compiled against the tree generated from the plan relowered with unknown="drop"
 * ({@code ak.Variant.UNKNOWN_FIELDS == false}), linked to the core built without the
 * {@code unknown-fields} feature. Its cells are {@code core-ffi|no-unknown},
 * {@code core-ffi-pull|no-unknown} and {@code host-gen|no-unknown} (arm R in drop mode over a
 * facade with no {@code unknownFields} member), with the incumbent arms beside them. JMH
 * forks one JVM per cell, so no arm shares a process with another in either build: the
 * incumbent arms are a control across the two builds' invocations, not an in-process one,
 * and ratios are formed from per-launch medians (req 30, owner R-H24).
 *
 * <p>Directions (req 9): {@code encode}, {@code decode} (the bare call) and
 * {@code decode-read} (decode, then {@code Walk}/{@code PbWalk} read every field).
 *
 * <p>Req 11: every encode iteration serialises a DIFFERENT object graph, built fresh before
 * the timed loop (a pool of {@code iters} objects per sample, untimed), so protobuf-java's
 * per-instance memoised size is never amortised. The facade arms get the same treatment.
 *
 * <p>Req 10: {@code core-ffi} runs in drop and retain (ABI v1 decision 11, WP5 step 9: the
 * binding arms every position with the shim's grow and re-encodes through the u-groups);
 * host-gen in drop and retain; the incumbent in protobuf-java's default mode (it retains).
 *
 * <p><b>Timed by JMH</b> (req 22a, owner 2026-09-25): {@code src/jmh/ak/CodecJmh.java} runs
 * one cell per JMH benchmark (one fork per cell), using {@link #make}, {@link #canonical},
 * {@link #check} and {@link #iters} from here. {@code main} lists the cells for a launch
 * (properties ak.camp.ids, ak.camp.sets, ak.camp.launch). The self-timed loop this class
 * had (the first WP3 smoke run) is retired. Properties read by the arms: ak.camp.budget
 * (bytes serialised per sample, default 32 MiB) and ak.camp.maxiters (4000).
 */
public final class CampaignCodec {
  static final String[] SET_NAMES = {"ascii", "latin1", "wide"};
  static long sink;

  /** grpc's shape with none of its allocator: a reused array (RunR14's). */
  static final RunR14.Sink SINK = new RunR14.Sink();
  static final RunR14.KnownStream KS = new RunR14.KnownStream();

  interface CArm {
    String name();
    String mode();
    boolean encodes();
    /** Untimed: build `n` fresh objects for an encode sample (one when the input is hot). */
    void prepare(String id, int cs, int n);
    /** Timed. */
    void encode(String id, int n) throws Exception;
    void decode(String id, byte[] w, int n, boolean read) throws Exception;
    /** For the pre-timing check: one encode of object 0 in the current end state, as bytes. */
    byte[] once(String id) throws Exception;
    /** Req 11's encode variant (R-H29): `hot` = one graph re-encoded `n` times, else a pool of
     *  `n` distinct graphs; `transport` = the end state the arm's RPC path hands its transport,
     *  else the bytes in a reused buffer. */
    default void variant(boolean hot, boolean transport) {}
  }

  /** Req 11 (R-H29): the encode variants, as the `dir` of a cell. */
  static final String[] ENC_DIRS = {"encode", "encode-hot", "encode-transport", "encode-transport-hot"};

  /** Base of every arm's variant switch. */
  abstract static class Var {
    boolean hot, transport;
    public void variant(boolean hot, boolean transport) { this.hot = hot; this.transport = transport; }
  }

  // ---- protobuf-java ---------------------------------------------------------------

  static final Map<String, MethodDescriptor.Marshaller<Message>> MARSHAL =
      new HashMap<String, MethodDescriptor.Marshaller<Message>>();

  static MethodDescriptor.Marshaller<Message> marshaller(String id) {
    MethodDescriptor.Marshaller<Message> m = MARSHAL.get(id);
    if (m == null) {
      Message proto = PbArms.build(id, Values.ASCII);
      if (proto == null) proto = parseProto(id);
      m = ProtoLiteUtils.marshaller(proto.getDefaultInstanceForType());
      MARSHAL.put(id, m);
    }
    return m;
  }

  static Message parseProto(String id) {
    try {
      return PbArms.parseArray(id, Payloads.vector(id));
    } catch (Exception e) {
      throw new IllegalStateException(e);
    }
  }

  static final class Pbj extends Var implements CArm {
    final boolean prod;
    Message[] pool = new Message[0];
    Pbj(boolean prod) { this.prod = prod; }
    public String name() { return prod ? "incumbent-prod" : "incumbent-best"; }
    public String mode() { return "default"; }
    public boolean encodes() { return true; }
    public void prepare(String id, int cs, int n) {
      if (hot) n = 1;
      if (pool.length < n) pool = new Message[n];
      for (int i = 0; i < n; i++) pool[i] = PbArms.build(id, cs);
    }
    /** End states. incumbent-prod: (i) `stream()` drained into the reused sink; (ii)
     *  `stream()` drained into a freshly allocated exact-size array (grpc-java's framer
     *  drains the stream into buffers it allocates; the allocation is modelled by the array).
     *  incumbent-best: (i) `writeTo` a CodedOutputStream over the reused sink's array; (ii)
     *  `toByteArray()`. */
    public void encode(String id, int n) throws Exception {
      if (prod) {
        MethodDescriptor.Marshaller<Message> m = marshaller(id);
        for (int i = 0; i < n; i++) {
          InputStream in = m.stream(pool[hot ? 0 : i]);
          if (transport) {
            RunR14.Sink fresh = new RunR14.Sink();
            fresh.reset(in.available());
            sink += ((Drainable) in).drainTo(fresh);
          } else {
            SINK.n = 0;
            sink += ((Drainable) in).drainTo(SINK);
          }
        }
      } else if (transport) {
        for (int i = 0; i < n; i++) sink += pool[hot ? 0 : i].toByteArray().length;
      } else {
        for (int i = 0; i < n; i++) {
          com.google.protobuf.CodedOutputStream cos = com.google.protobuf.CodedOutputStream.newInstance(SINK.buf);
          pool[hot ? 0 : i].writeTo(cos);
          sink += cos.getTotalBytesWritten();
        }
      }
    }
    public void decode(String id, byte[] w, int n, boolean read) throws Exception {
      MethodDescriptor.Marshaller<Message> m = prod ? marshaller(id) : null;
      String root = Arms.root(id);
      for (int i = 0; i < n; i++) {
        Message x = prod ? m.parse(KS.set(w, 0, w.length)) : PbArms.parseArray(id, w);
        sink += read ? PbWalk.walk(root, x) : System.identityHashCode(x);
      }
    }
    public byte[] once(String id) throws Exception {
      if (prod) {
        SINK.reset(pool[0].getSerializedSize() + 16);
        ((Drainable) marshaller(id).stream(pool[0])).drainTo(SINK);
        return Arrays.copyOf(SINK.buf, SINK.n);
      }
      if (!transport) {
        SINK.reset(pool[0].getSerializedSize() + 16);
        com.google.protobuf.CodedOutputStream cos = com.google.protobuf.CodedOutputStream.newInstance(SINK.buf);
        pool[0].writeTo(cos);
        return Arrays.copyOf(SINK.buf, cos.getTotalBytesWritten());
      }
      return pool[0].toByteArray();
    }
  }

  // ---- the facade arms ---------------------------------------------------------------

  abstract static class Fac extends Var implements CArm {
    Object[] pool = new Object[0];
    public boolean encodes() { return true; }
    public void prepare(String id, int cs, int n) {
      if (hot) n = 1;
      if (pool.length < n) pool = new Object[n];
      for (int i = 0; i < n; i++) pool[i] = Arms.build(id, cs);
    }
  }

  static final class HostGen extends Fac {
    final boolean retain;
    final Enc e = new Enc(Arms.R_SITES);
    final Dec d = new Dec();
    HostGen(boolean retain) { this.retain = retain; }
    public String name() { return "host-gen"; }
    public String mode() { return retain ? "retain" : ak.Variant.UNKNOWN_FIELDS ? "drop" : "no-unknown"; }
    /** End states: (i) the Enc's bytes copied into the reused sink; (ii) `Enc.toBytes()`, the
     *  fresh array cells E and F hand their transport. */
    public void encode(String id, int n) {
      for (int i = 0; i < n; i++) {
        Object o = pool[hot ? 0 : i];
        if (retain) Arms.encodeRRetain(id, o, e); else Arms.encodeR(id, o, e);
        if (transport) {
          sink += e.toBytes().length;
        } else {
          SINK.n = 0;
          SINK.write(e.buf, 0, e.len);
          sink += SINK.n;
        }
      }
    }
    public void decode(String id, byte[] w, int n, boolean read) {
      String root = Arms.root(id);
      for (int i = 0; i < n; i++) {
        Object x = retain ? Arms.decodeRRetain(id, d, w, 0, w.length) : Arms.decodeR(id, d, w, 0, w.length);
        sink += read ? Walk.walk(root, x) : System.identityHashCode(x);
      }
    }
    public byte[] once(String id) {
      if (retain) Arms.encodeRRetain(id, pool[0], e); else Arms.encodeR(id, pool[0], e);
      return e.toBytes();
    }
  }

  static final class Ffi extends Fac {
    final Binding b = new Binding();
    final boolean pull;
    final boolean retain;
    Ffi(boolean pull) { this(pull, false); }
    Ffi(boolean pull, boolean retain) { this.pull = pull; this.retain = retain; FfiArms.setRetain(b, retain); }
    public String name() { return pull ? "core-ffi-pull" : "core-ffi"; }
    public String mode() { return retain ? "retain" : ak.Variant.UNKNOWN_FIELDS ? "drop" : "no-unknown"; }
    @Override public boolean encodes() { return !pull; }
    /** End states: (i) `ak_enc_take` into the reused sink; (ii) `take()`, the fresh array
     *  cells C and D hand their transport. */
    public void encode(String id, int n) {
      for (int i = 0; i < n; i++) {
        FfiArms.encode(b, id, pool[hot ? 0 : i]);
        if (transport) {
          sink += b.take().length;
        } else {
          SINK.n = Native.encTake(b.encCtx, SINK.buf);   // the sink is pre-sized per sample
          sink += SINK.n;
        }
      }
    }
    public void decode(String id, byte[] w, int n, boolean read) {
      String root = Arms.root(id);
      for (int i = 0; i < n; i++) {
        Object x = pull ? FfiArms.parse(b, id, w, 0, w.length) : FfiArms.decode(b, id, w, 0, w.length);
        sink += read ? Walk.walk(root, x) : System.identityHashCode(x);
      }
    }
    public byte[] once(String id) {
      FfiArms.encode(b, id, pool[0]);
      return b.take();
    }
  }

  // ---- what the JMH suite (ak.CodecJmh) uses ----------------------------------------
  //
  // Req 22a (owner, 2026-09-25): the codec suite is timed by JMH. This class keeps the arms
  // and the pre-timing check; the self-timed loop it had is retired. `main` now only lists
  // the cells (arm|mode|payload|content|dir) the runner hands JMH as the `cell` parameter.

  static final String[] ARMS = ak.Variant.UNKNOWN_FIELDS
      ? new String[] {"incumbent-prod|default", "incumbent-best|default",
          "core-ffi|drop", "core-ffi|retain", "core-ffi-pull|drop", "host-gen|drop", "host-gen|retain"}
      : new String[] {"incumbent-prod|default", "incumbent-best|default",   // WP5 step 10
          "core-ffi|no-unknown", "core-ffi-pull|no-unknown", "host-gen|no-unknown"};

  static CArm make(String arm, String mode) {
    if (arm.equals("incumbent-prod")) return new Pbj(true);
    if (arm.equals("incumbent-best")) return new Pbj(false);
    if (arm.equals("core-ffi")) return new Ffi(false, mode.equals("retain"));
    if (arm.equals("core-ffi-pull")) return new Ffi(true, mode.equals("retain"));
    if (arm.equals("host-gen")) return new HostGen(mode.equals("retain"));
    throw new IllegalArgumentException("no arm " + arm + "/" + mode);
  }

  /** The canonical wire of (payload, content): arm R's encoding, checked against the
   *  manifest's sha256 on the ASCII set; P7.1 (decode-only) is its committed vector. */
  static byte[] canonical(String id, int cs) {
    byte[] w;
    if (!Arms.encodable(id)) {
      if (cs != Values.ASCII) throw new IllegalArgumentException(id + " is ASCII only");
      w = Payloads.vector(id);
    } else {
      HostGen ref = new HostGen(false);
      ref.prepare(id, cs, 1);
      w = ref.once(id);
    }
    if (cs == Values.ASCII && !Values.sha256Hex(w).equals(Payloads.row(id).sha256))
      throw new IllegalStateException(id + ": arm R's encoding differs from the manifest (sha256)");
    return w;
  }

  /** Req 26 inside the harness: an encoding arm reproduces the canonical bytes (for
   *  protobuf-java, bytes that arm R reads back to the canonical form, because P2.5 is written
   *  in protobuf-java's both-fields form); a decoding arm reads the canonical bytes to an
   *  object that arm R re-encodes to them. Throws on any mismatch. */
  static void check(CArm a, String id, int cs, byte[] w, String dir) throws Exception {
    HostGen ref = new HostGen(false);
    if (dir.startsWith("encode")) {
      a.prepare(id, cs, 1);
      byte[] got = a.once(id);
      if (a instanceof Pbj) {
        ref.pool = new Object[] {Arms.decodeR(id, new Dec(), got, 0, got.length)};
        got = ref.once(id);
      }
      if (!Arrays.equals(got, w))
        throw new IllegalStateException(id + "/" + cs + " " + a.name() + "/" + a.mode() + ": encoding differs");
    } else {
      // Decode: compared with arm R's own reading of the same bytes, re-encoded, not with the
      // bytes themselves -- P7.1 is interleaved on purpose and no writer reproduces it.
      ref.pool = new Object[] {Arms.decodeR(id, new Dec(), w, 0, w.length)};
      byte[] want = ref.once(id);
      checkDecode(a, id, cs, w, want, ref);
    }
  }

  static void checkDecode(CArm a, String id, int cs, byte[] w, byte[] want, HostGen ref) throws Exception {
    if (!(a instanceof Pbj)) {
      Object x = a instanceof Ffi ? (((Ffi) a).pull ? FfiArms.parse(((Ffi) a).b, id, w, 0, w.length)
                                                    : FfiArms.decode(((Ffi) a).b, id, w, 0, w.length))
          : ((HostGen) a).retain ? Arms.decodeRRetain(id, new Dec(), w, 0, w.length)
                                 : Arms.decodeR(id, new Dec(), w, 0, w.length);
      ref.pool = new Object[] {x};
      if (!Arrays.equals(ref.once(id), want))
        throw new IllegalStateException(id + "/" + cs + " " + a.name() + ": decode does not round-trip");
    } else {
      Message m = PbArms.parseArray(id, w);
      byte[] pb = m.toByteArray();
      ref.pool = new Object[] {Arms.decodeR(id, new Dec(), pb, 0, pb.length)};
      if (!Arrays.equals(ref.once(id), want))
        throw new IllegalStateException(id + "/" + cs + " " + a.name() + ": decode does not round-trip");
    }
  }

  static int iters(int len) {
    long budget = Long.getLong("ak.camp.budget", 32L << 20);
    int maxIters = Integer.getInteger("ak.camp.maxiters", 4000);
    return (int) Math.max(1, Math.min(maxIters, budget / Math.max(1, len)));
  }

  /** Req 11's pool input (R-H29): enough distinct graphs that their serialised bytes alone
   *  exceed `ak.camp.poolbytes` (the runner passes 2 x the machine's last-level cache,
   *  AK_LLC_BYTES in ffi/campaign.machine or its default 13.75 MB), so the pool cannot sit in
   *  the LLC; never fewer than `iters`. */
  static long POOL_BYTES = Long.getLong("ak.camp.poolbytes", 2L * 14417920L);

  static int poolIters(int len) {
    return (int) Math.max(iters(len), (POOL_BYTES + len - 1) / Math.max(1, len));
  }

  /** The cell list, one per line, arms rotated by `ak.camp.launch` inside each
   *  (payload, content, dir) block (req 22: arm order rotated between launches). */
  public static void main(String[] args) {
    String[] ids = System.getProperty("ak.camp.ids", String.join(",", Arms.IDS)).split(",");
    String[] sets = System.getProperty("ak.camp.sets", "0,1,2").split(",");
    int launch = Campaign.LAUNCH;
    List<String> arms = new ArrayList<String>(Arrays.asList(ARMS));
    StringBuilder sb = new StringBuilder();
    for (String id : ids)
      for (String c : sets) {
        int cs = Integer.parseInt(c.trim());
        if (!Arms.encodable(id) && cs != Values.ASCII) continue;
        for (String dir : new String[] {"encode", "encode-hot", "encode-transport", "encode-transport-hot",
                                        "decode", "decode-read"}) {
          if (dir.startsWith("encode") && !Arms.encodable(id)) continue;
          for (String a : Campaign.rotate(arms, launch - 1)) {
            if (dir.startsWith("encode") && a.startsWith("core-ffi-pull")) continue;
            sb.append(a).append('|').append(id).append('|').append(SET_NAMES[cs]).append('|')
              .append(dir).append('\n');
          }
        }
      }
    // Req 7 (R-H27): the U-* rows at the shapes core's roots, timed in the main invocation
    // (the shapes core's shim); `ak.camp.urows` thins them in a smoke run like the extras.
    {
      StringBuilder u = new StringBuilder();
      listU(u, launch, true);
      sb.append(thin(u, Integer.getInteger("ak.camp.urows", 0)));
    }
    if (System.getProperty("ak.camp.unknown", "0").equals("1")) {
      sb.setLength(0);
      listU(sb, launch);
      StringBuilder t = thin(sb, Integer.getInteger("ak.camp.urows", 0));
      sb = t;
    }
    System.out.print(sb);
  }

  /** Smoke: keep `lim` rows (payload ids) spread evenly; 0 keeps all. */
  static StringBuilder thin(StringBuilder sb, int lim) {
    if (lim <= 0 || sb.length() == 0) return sb;
    List<String> rows = new ArrayList<String>();
    for (String line : sb.toString().split("\n")) {
      String row = line.split("\\|")[2];
      if (!rows.contains(row)) rows.add(row);
    }
    java.util.Set<String> keep = new java.util.HashSet<String>();
    for (int k = 0; k < lim && k < rows.size(); k++) keep.add(rows.get(k * rows.size() / lim));
    StringBuilder t = new StringBuilder();
    for (String line : sb.toString().split("\n"))
      if (keep.contains(line.split("\\|")[2])) t.append(line).append('\n');
    return t;
  }

  // ---- req 7 (amended 0e8e9eb): the corpus's U-* rows of class `unknown` -------------
  //
  // Decoded and re-encoded through the CORPUS description (ak.corpus), which covers every
  // root; `Nest` is outside the C ABI, so the core-ffi arms skip its rows. The incumbent arms
  // run on the rows whose root protoc generated in `ak.pb` (shapes.proto; identical to
  // corpus.proto for those messages, CONTRACT.md rule 0, checked in re4-corpus-armR.log).
  // A cell names a row as payload `U-...` and content `corpus:<Root>`. The encode direction
  // serialises objects the same arm decoded from the row (untimed), so retain re-emits.

  static final class UArm implements CArm {
    final String arm, mode, root;
    /** Through the shapes description and the timed shapes core (req 7, R-H27), else the
     *  corpus description and the corpus core (a labelled extra). */
    final boolean shapes;
    Object[] pool = new Object[0];
    ak.corpus.Binding b;
    ak.shapes.Binding sb;
    Message proto;
    MethodDescriptor.Marshaller<Message> marsh;
    byte[] row;

    UArm(String arm, String mode, String root, byte[] row) throws Exception { this(arm, mode, root, row, false); }

    UArm(String arm, String mode, String root, byte[] row, boolean shapes) throws Exception {
      this.arm = arm; this.mode = mode; this.root = root; this.row = row; this.shapes = shapes;
      if (arm.startsWith("core-ffi")) {
        if (shapes) {
          sb = new ak.shapes.Binding();
          ak.shapes.Dispatch.setRetain(sb, mode.equals("retain"));
        } else {
          b = new ak.corpus.Binding();
          ak.corpus.Dispatch.setRetain(b, mode.equals("retain"));   // decision 11 (WP5 step 9)
        }
      } else if (arm.startsWith("incumbent")) {
        proto = (Message) Class.forName("ak.pb." + root).getMethod("getDefaultInstance").invoke(null);
        marsh = ProtoLiteUtils.marshaller(proto);
      }
    }

    public String name() { return arm; }
    public String mode() { return mode; }
    public boolean encodes() { return !arm.equals("core-ffi-pull"); }

    Object dec(byte[] w) throws Exception {
      if (shapes) switch (arm) {
        case "host-gen": return mode.equals("retain") ? ak.shapes.Dispatch.decRRetain(root, w) : ak.shapes.Dispatch.decR(root, w);
        case "core-ffi": return ak.shapes.Dispatch.decFfi(sb, root, w);
        case "core-ffi-pull": return ak.shapes.Dispatch.parseFfi(sb, root, w);
        default: break;
      }
      switch (arm) {
        case "host-gen": return mode.equals("retain") ? ak.corpus.Dispatch.decRRetain(root, w) : ak.corpus.Dispatch.decR(root, w);
        case "core-ffi": return ak.corpus.Dispatch.decFfi(b, root, w);
        case "core-ffi-pull": return ak.corpus.Dispatch.parseFfi(b, root, w);
        case "incumbent-prod": return marsh.parse(KS.set(w, 0, w.length));
        default: return proto.getParserForType().parseFrom(w);
      }
    }

    byte[] enc(Object o) throws Exception {
      if (shapes) switch (arm) {
        case "host-gen": return mode.equals("retain") ? ak.shapes.Dispatch.encRRetain(root, o) : ak.shapes.Dispatch.encR(root, o);
        case "core-ffi": return ak.shapes.Dispatch.encFfi(sb, root, o);
        default: break;
      }
      switch (arm) {
        case "host-gen": return mode.equals("retain") ? ak.corpus.Dispatch.encRRetain(root, o) : ak.corpus.Dispatch.encR(root, o);
        case "core-ffi": return ak.corpus.Dispatch.encFfi(b, root, o);
        case "incumbent-prod": {
          SINK.n = 0;
          ((Drainable) marsh.stream((Message) o)).drainTo(SINK);
          return null;
        }
        default: return ((Message) o).toByteArray();
      }
    }

    public void prepare(String id, int cs, int n) {
      if (pool.length < n) pool = new Object[n];
      try {
        for (int i = 0; i < n; i++) pool[i] = dec(row);
      } catch (Exception e) {
        throw new IllegalStateException(e);
      }
    }

    public void encode(String id, int n) throws Exception {
      for (int i = 0; i < n; i++) {
        byte[] w = enc(pool[i]);
        sink += w == null ? SINK.n : w.length;
      }
    }

    public void decode(String id, byte[] w, int n, boolean read) throws Exception {
      for (int i = 0; i < n; i++) {
        Object x = dec(w);
        sink += !read ? System.identityHashCode(x)
            : arm.startsWith("incumbent") ? ak.shapes.PbWalk.walk(root, x)
            : shapes ? ak.shapes.Walk.walk(root, x) : ak.corpus.Walk.walk(root, x);
      }
    }

    public byte[] once(String id) throws Exception {
      prepare(id, 0, 1);
      if (arm.equals("incumbent-prod")) return ((Message) pool[0]).toByteArray();
      return enc(pool[0]);
    }
  }

  static final String CORPUS_DIR = new java.io.File(Payloads.SCHEMA_DIR, "../../corpus/generated").getPath();

  static byte[] corpusRow(String id) {
    return Payloads.readFile(new java.io.File(CORPUS_DIR, "vectors/" + id + ".bin"));
  }

  /** Req 26 for a U-* cell: what the arm reads from the row, written back (by its own
   *  encoder; the pull arm through the push encoder), read by arm R in DROP mode and
   *  re-encoded, equals arm R's drop reading of the row. */
  static void checkU(UArm a) throws Exception {
    String root = a.root;
    boolean sh = a.shapes;
    byte[] want = sh ? ak.shapes.Dispatch.encR(root, ak.shapes.Dispatch.decR(root, a.row))
                     : ak.corpus.Dispatch.encR(root, ak.corpus.Dispatch.decR(root, a.row));
    Object x = a.dec(a.row);
    byte[] w = a.arm.startsWith("incumbent") ? ((Message) x).toByteArray()
        : a.arm.startsWith("core-ffi") ? (sh ? ak.shapes.Dispatch.encFfi(a.sb, root, x) : ak.corpus.Dispatch.encFfi(a.b, root, x))
        : a.enc(x);
    byte[] mine = sh ? ak.shapes.Dispatch.encR(root, ak.shapes.Dispatch.decR(root, w))
                     : ak.corpus.Dispatch.encR(root, ak.corpus.Dispatch.decR(root, w));
    if (!Arrays.equals(mine, want))
      throw new IllegalStateException(a.arm + "/" + a.mode + " " + root + ": does not read the row as arm R does");
  }

  /** The U-* cells: every row of class unknown, not disputed, whose root this slice
   *  implements (all of them through ak.corpus; the incumbent where ak.pb has the root). */
  static void listU(StringBuilder sb, int launch) { listU(sb, launch, false); }

  /** `shapes`: req 7 as amended (R-H27): the ACCEPTED rows of class unknown at the shapes
   *  core's ABI roots, through the timed shapes core (content `shapes:<Root>`); otherwise
   *  every non-disputed row of class unknown through the corpus description and the corpus
   *  core, a labelled extra (content `corpus:<Root>`). */
  static void listU(StringBuilder sb, int launch, boolean shapes) {
    @SuppressWarnings("unchecked")
    java.util.Map<String, Object> vs = (java.util.Map<String, Object>) ((java.util.Map<String, Object>) Json.parse(
        new String(Payloads.readFile(new java.io.File(CORPUS_DIR, "manifest.json")),
            java.nio.charset.Charset.forName("UTF-8")))).get("vectors");
    java.util.Set<String> abi = new java.util.HashSet<String>(Arrays.asList(
        shapes ? ak.shapes.Dispatch.ABI : ak.corpus.Dispatch.ABI));
    List<String> arms = new ArrayList<String>(Arrays.asList(ARMS));
    for (java.util.Map.Entry<String, Object> e : new java.util.TreeMap<String, Object>(vs).entrySet()) {
      @SuppressWarnings("unchecked")
      java.util.Map<String, Object> r = (java.util.Map<String, Object>) e.getValue();
      if (!e.getKey().startsWith("U-") || !"unknown".equals(r.get("class")) || "disputed".equals(r.get("verdict")))
        continue;
      String root = (String) r.get("root");
      if (shapes && (!"accept".equals(r.get("expect")) || !abi.contains(root))) continue;
      boolean pb;
      try { Class.forName("ak.pb." + root); pb = true; } catch (ClassNotFoundException x) { pb = false; }
      for (String dir : new String[] {"encode", "decode", "decode-read"})
        for (String a : Campaign.rotate(arms, launch - 1)) {
          if (a.startsWith("incumbent") && !pb) continue;
          if (a.startsWith("core-ffi") && !abi.contains(root)) continue;
          if (dir.equals("encode") && a.startsWith("core-ffi-pull")) continue;
          sb.append(a).append('|').append(e.getKey()).append(shapes ? "|shapes:" : "|corpus:").append(root).append('|')
            .append(dir).append('\n');
        }
    }
  }
}
