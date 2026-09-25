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
 *   core-ffi        retain        PENDING the decision 11 port (see below): not run
 * </pre>
 *
 * <p>Directions (req 9): {@code encode}, {@code decode} (the bare call) and
 * {@code decode-read} (decode, then {@code Walk}/{@code PbWalk} read every field).
 *
 * <p>Req 11: every encode iteration serialises a DIFFERENT object graph, built fresh before
 * the timed loop (a pool of {@code iters} objects per sample, untimed), so protobuf-java's
 * per-instance memoised size is never amortised. The facade arms get the same treatment.
 *
 * <p>Req 10: {@code core-ffi} in retain mode needs ABI v1 decision 11's mechanism (options
 * entries armed per position), which is being ported into poc/codec; the binding does not
 * render it yet. The hook is the {@code unknown_mode} of an arm: the run records
 * {@code core-ffi/retain} as pending in a meta line and produces no sample for it.
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
    /** Untimed: build `n` fresh objects for an encode sample. */
    void prepare(String id, int cs, int n);
    /** Timed. */
    void encode(String id, int n) throws Exception;
    void decode(String id, byte[] w, int n, boolean read) throws Exception;
    /** For the pre-timing check: one encode of object 0, as bytes. */
    byte[] once(String id) throws Exception;
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

  static final class Pbj implements CArm {
    final boolean prod;
    Message[] pool = new Message[0];
    Pbj(boolean prod) { this.prod = prod; }
    public String name() { return prod ? "incumbent-prod" : "incumbent-best"; }
    public String mode() { return "default"; }
    public boolean encodes() { return true; }
    public void prepare(String id, int cs, int n) {
      if (pool.length < n) pool = new Message[n];
      for (int i = 0; i < n; i++) pool[i] = PbArms.build(id, cs);
    }
    public void encode(String id, int n) throws Exception {
      if (prod) {
        MethodDescriptor.Marshaller<Message> m = marshaller(id);
        for (int i = 0; i < n; i++) {
          SINK.n = 0;
          InputStream in = m.stream(pool[i]);
          sink += ((Drainable) in).drainTo(SINK);
        }
      } else {
        for (int i = 0; i < n; i++) sink += pool[i].toByteArray().length;
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
      return pool[0].toByteArray();
    }
  }

  // ---- the facade arms ---------------------------------------------------------------

  abstract static class Fac implements CArm {
    Object[] pool = new Object[0];
    public boolean encodes() { return true; }
    public void prepare(String id, int cs, int n) {
      if (pool.length < n) pool = new Object[n];
      for (int i = 0; i < n; i++) pool[i] = Arms.build(id, cs);
    }
  }

  static final class HostGen extends Fac {
    final boolean retain;
    final Enc e = new Enc(ak.shapes.Codec.SITES > ak.shapes.CodecRetain.SITES
        ? ak.shapes.Codec.SITES : ak.shapes.CodecRetain.SITES);
    final Dec d = new Dec();
    HostGen(boolean retain) { this.retain = retain; }
    public String name() { return "host-gen"; }
    public String mode() { return retain ? "retain" : "drop"; }
    public void encode(String id, int n) {
      for (int i = 0; i < n; i++) {
        if (retain) Arms.encodeRRetain(id, pool[i], e); else Arms.encodeR(id, pool[i], e);
        SINK.n = 0;
        SINK.write(e.buf, 0, e.len);
        sink += SINK.n;
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
    Ffi(boolean pull) { this.pull = pull; }
    public String name() { return pull ? "core-ffi-pull" : "core-ffi"; }
    public String mode() { return "drop"; }
    @Override public boolean encodes() { return !pull; }
    public void encode(String id, int n) {
      for (int i = 0; i < n; i++) {
        FfiArms.encode(b, id, pool[i]);
        SINK.n = Native.encTake(b.encCtx, SINK.buf);   // the sink is pre-sized per sample
        sink += SINK.n;
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

  static final String[] ARMS = {"incumbent-prod|default", "incumbent-best|default",
      "core-ffi|drop", "core-ffi-pull|drop", "host-gen|drop", "host-gen|retain"};

  static CArm make(String arm, String mode) {
    if (arm.equals("incumbent-prod")) return new Pbj(true);
    if (arm.equals("incumbent-best")) return new Pbj(false);
    if (arm.equals("core-ffi") && mode.equals("drop")) return new Ffi(false);
    if (arm.equals("core-ffi-pull")) return new Ffi(true);
    if (arm.equals("host-gen")) return new HostGen(mode.equals("retain"));
    // core-ffi/retain: ABI v1 decision 11's options are not rendered by the Java backend yet.
    throw new IllegalArgumentException("no arm " + arm + "/" + mode + " (core-ffi/retain is pending decision 11)");
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
    if (dir.equals("encode")) {
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
        for (String dir : new String[] {"encode", "decode", "decode-read"}) {
          if (dir.equals("encode") && !Arms.encodable(id)) continue;
          for (String a : Campaign.rotate(arms, launch - 1)) {
            if (dir.equals("encode") && a.startsWith("core-ffi-pull")) continue;
            sb.append(a).append('|').append(id).append('|').append(SET_NAMES[cs]).append('|')
              .append(dir).append('\n');
          }
        }
      }
    System.out.print(sb);
  }
}
