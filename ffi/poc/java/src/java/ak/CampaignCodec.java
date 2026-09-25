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
 * <p>Properties: ak.camp.ids (payload ids, default all 16), ak.camp.sets (content sets,
 * default 0,1,2), ak.camp.rounds (5), ak.camp.launch, ak.camp.budget (bytes serialised per
 * sample, default 32 MiB, which sets iters = budget / payload size clamped to
 * [1, ak.camp.maxiters=4000]), ak.camp.warm (warm-up samples per cell before round 1,
 * default 5), ak.camp.coder (label of the String coder state of this JVM: compact | utf16,
 * set by the runner with -XX:-CompactStrings), ak.camp.out (the log to append to).
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

  // ---- the run -------------------------------------------------------------------

  public static void main(String[] args) throws Exception {
    String[] ids = System.getProperty("ak.camp.ids", String.join(",", Arms.IDS)).split(",");
    String[] setsS = System.getProperty("ak.camp.sets", "0,1,2").split(",");
    long budget = Long.getLong("ak.camp.budget", 32L << 20);
    int maxIters = Integer.getInteger("ak.camp.maxiters", 4000);
    int warm = Integer.getInteger("ak.camp.warm", 5);
    String coder = System.getProperty("ak.camp.coder", "compact");
    String out = System.getProperty("ak.camp.out");

    List<CArm> arms = new ArrayList<CArm>();
    arms.add(new Pbj(true));
    arms.add(new Pbj(false));
    arms.add(new Ffi(false));
    arms.add(new Ffi(true));
    arms.add(new HostGen(false));
    arms.add(new HostGen(true));
    Campaign.meta("{\"suite\":\"codec\",\"pending\":\"core-ffi/retain: ABI v1 decision 11's"
        + " mechanism is not rendered by the Java binding yet (req 10)\"}");

    int[] sets = new int[setsS.length];
    for (int i = 0; i < sets.length; i++) sets[i] = Integer.parseInt(setsS[i].trim());

    // ---- the canonical wire per (payload, content), and the pre-timing check (req 26):
    // every encoding arm reproduces the canonical bytes (protobuf-java: the prod and best
    // entry points agree with each other, and arm R reads them back to the canonical form,
    // because P2.5 is written in protobuf-java's both-fields form), and every decoding arm
    // reads the canonical bytes back to them.
    Map<String, byte[]> wire = new HashMap<String, byte[]>();
    Map<String, Integer> iters = new HashMap<String, Integer>();
    HostGen ref = new HostGen(false);
    for (String id : ids) {
      for (int cs : sets) {
        String key = id + "/" + cs;
        byte[] w;
        if (!Arms.encodable(id)) {
          if (cs != Values.ASCII) continue;         // P7.1: decode-only, one committed vector
          w = Payloads.vector(id);
        } else {
          ref.prepare(id, cs, 1);
          w = ref.once(id);
        }
        if (cs == Values.ASCII && !Values.sha256Hex(w).equals(Payloads.row(id).sha256))
          Campaign.abort(id + ": arm R's encoding differs from the manifest (sha256)");
        wire.put(key, w);
        iters.put(key, (int) Math.max(1, Math.min(maxIters, budget / Math.max(1, w.length))));
        for (CArm a : arms) {
          if (a.encodes() && Arms.encodable(id)) {
            a.prepare(id, cs, 1);
            byte[] got = a.once(id);
            if (a instanceof Pbj) {
              Object back = Arms.decodeR(id, new Dec(), got, 0, got.length);
              ref.pool[0] = back;
              if (!Arrays.equals(ref.once(id), w)) Campaign.abort(id + "/" + cs + " " + a.name() + ": encoding not equivalent");
            } else if (!Arrays.equals(got, w)) {
              Campaign.abort(id + "/" + cs + " " + a.name() + "/" + a.mode() + ": encoding differs");
            }
          }
        }
      }
    }

    List<String[]> cells = new ArrayList<String[]>();   // {id, cs, dir}
    for (String id : ids)
      for (int cs : sets)
        if (wire.containsKey(id + "/" + cs))
          for (String dir : new String[] {"encode", "decode", "decode-read"})
            if (!dir.equals("encode") || Arms.encodable(id))
              cells.add(new String[] {id, "" + cs, dir});

    // ---- warm-up (req 24): `warm` untimed samples of every (cell, arm) before round 1,
    // the same number for every arm.
    for (int k = 0; k < warm; k++)
      for (String[] c : cells)
        for (CArm a : arms)
          sample(a, c, wire, iters, false, 0, coder);
    Campaign.meta("{\"warmup_samples_per_cell\":" + warm + ",\"jit_ms_after_warmup\":" + Campaign.jitMs() + "}");

    for (int r = 1; r <= Campaign.ROUNDS; r++) {
      for (String[] c : cells)
        for (CArm a : Campaign.rotate(arms, r))
          sample(a, c, wire, iters, true, r, coder);
      Campaign.meta("{\"round\":" + r + ",\"jit_ms\":" + Campaign.jitMs() + "}");
    }
    Campaign.meta("{\"sink\":" + sink + "}");
    Campaign.flush(out);
  }

  static void sample(CArm a, String[] c, Map<String, byte[]> wire, Map<String, Integer> iters,
                     boolean record, int round, String coder) throws Exception {
    String id = c[0], dir = c[2];
    int cs = Integer.parseInt(c[1]);
    if (dir.equals("encode") && !a.encodes()) return;
    String key = id + "/" + cs;
    int n = iters.get(key);
    byte[] w = wire.get(key);
    if (dir.equals("encode")) a.prepare(id, cs, n);
    SINK.reset(2 * w.length + 4096);                   // untimed: no arm grows the sink
    long c0 = Campaign.threadCpuNs(), t0 = System.nanoTime();
    if (dir.equals("encode")) a.encode(id, n);
    else a.decode(id, w, n, dir.equals("decode-read"));
    long t1 = System.nanoTime(), c1 = Campaign.threadCpuNs();
    if (!record) return;
    Campaign.add(new Campaign.Sample().s("suite", "codec").s("arm", a.name()).s("payload", id)
        .s("content", SET_NAMES[cs]).s("dir", dir).s("unknown_mode", a.mode())
        .s("coder", coder).n("launch", Campaign.LAUNCH).n("round", round)
        .n("cpu_ns", c1 - c0).n("wall_ns", t1 - t0).n("iters", n));
  }
}
