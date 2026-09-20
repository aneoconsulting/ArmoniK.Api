package ak;

import ak.shapes.Arms;
import ak.shapes.Binding;
import ak.shapes.FfiArms;
import ak.shapes.PbArms;
import com.google.protobuf.Message;
import io.grpc.Drainable;
import io.grpc.KnownLength;
import io.grpc.MethodDescriptor;
import io.grpc.protobuf.lite.ProtoLiteUtils;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

/**
 * R14: the baseline is what ArmoniK runs, not the library's best entry point.
 *
 * <p>"Almost nothing in `packages/` serialises directly; gRPC's generated marshaller does,
 * so that is the path every ratio is against." This slice's existing tables are against
 * {@code toByteArray}, which is the idiomatic entry point and is <b>not</b> the one
 * production takes. This re-takes the headline against the real marshaller -- the actual
 * {@code ProtoLiteUtils} object, not an imitation of it.
 *
 * <h2>What the marshaller actually does, read from the bytecode rather than remembered</h2>
 *
 * <p><b>Encode.</b> {@code marshaller.stream(msg)} returns a {@code ProtoInputStream}, and
 * grpc's framer drains it: {@code ProtoInputStream.drainTo} calls
 * {@code getSerializedSize()} and then {@code writeTo(OutputStream)}. So production pays
 * the size pass -- this slice's `pbj` arm already does, and `pbj-loop` is why that matters
 * -- and then writes through a 4 KB {@code CodedOutputStream} that flushes to the stream,
 * where `toByteArray` allocates an exact array and writes it in one go. Those are different
 * costs and R14 says the first one is the headline.
 *
 * <p><b>Decode.</b> {@code marshaller.parse(in)} has a fast path that returns the very same
 * object when handed back its own {@code ProtoInputStream}, which is not a parse at all and
 * must not be measured. For a real {@code KnownLength} stream it reads the whole message
 * into a thread-local reusable {@code byte[]} and parses from that array -- so it is this
 * slice's existing {@code parseFrom} baseline plus one copy, and the decode tables did not
 * need re-taking. The arm is here anyway, because "did not need re-taking" is a claim and
 * this makes it a measurement.
 *
 * <h2>The sink, and why it is not a ByteArrayOutputStream</h2>
 *
 * <p>grpc writes into pooled {@code WritableBuffer}s from netty. Draining into a
 * {@code ByteArrayOutputStream} would charge the incumbent a growth policy this harness
 * invented; draining into a pre-sized reused array charges it only what the marshaller
 * itself does, which is the size pass and the 4 KB chunked write. **Every arm here writes
 * into the same sink**, so whatever the sink costs, it costs all of them equally.
 */
public final class RunR14 {

  /** A sink with grpc's shape and none of its allocator: a reused array. */
  static final class Sink extends OutputStream {
    byte[] buf = new byte[1 << 16];
    int n;
    void reset(int need) {
      if (buf.length < need) buf = new byte[Integer.highestOneBit(need - 1) * 2];
      n = 0;
    }
    @Override public void write(int b) { buf[n++] = (byte) b; }
    @Override public void write(byte[] b, int off, int len) {
      System.arraycopy(b, off, buf, n, len);
      n += len;
    }
  }

  /** What the transport hands `parse`: a stream that knows its length. */
  static final class KnownStream extends InputStream implements KnownLength {
    byte[] b; int pos, end;
    KnownStream set(byte[] b, int off, int len) { this.b = b; pos = off; end = off + len; return this; }
    @Override public int read() { return pos < end ? (b[pos++] & 0xFF) : -1; }
    @Override public int read(byte[] d, int off, int len) {
      int k = Math.min(len, end - pos);
      if (k <= 0) return -1;
      System.arraycopy(b, pos, d, off, k);
      pos += k;
      return k;
    }
    @Override public int available() { return end - pos; }
  }

  static long sink;

  public static void main(String[] args) throws Exception {
    final int ROUNDS = Integer.getInteger("ak.rounds", 24);
    final long TARGET = Long.getLong("ak.roundns", 40_000_000L);
    final int CS = Integer.getInteger("ak.cs", Values.ASCII);

    Binding b = new Binding();
    Enc enc = new Enc(ak.shapes.Codec.SITES);
    Dec dec = new Dec();
    Sink out = new Sink();
    KnownStream ks = new KnownStream();
    PbArm pb = new PbArm();

    List<String> ids = new ArrayList<String>();
    String only = System.getProperty("ak.only");
    for (String id : Arms.IDS) if (only == null || only.equals(id)) ids.add(id);

    String[] encNames = {"pbj-grpc", "pbj-tba", "R-sink", "ffi-sink"};
    String[] decNames = {"pbj-grpc", "pbj-parseFrom", "R", "ffi"};
    long[][][] e = new long[ids.size()][][];
    long[][][] d = new long[ids.size()][][];
    int[] eN = new int[ids.size()], dN = new int[ids.size()];

    for (int p = 0; p < ids.size(); p++) {
      String id = ids.get(p);
      if (!FfiArms.encodable(id)) continue;
      @SuppressWarnings("unchecked")
      MethodDescriptor.Marshaller<Message> m =
          (MethodDescriptor.Marshaller<Message>) (MethodDescriptor.Marshaller<?>)
              ProtoLiteUtils.marshaller(PbArms.build(id, CS));

      // The pool: a protobuf message memoizes its serialized size, so each is used once.
      int cap = Math.max(8, (int) Math.min(4096,
          (128L << 20) / (Math.max(Payloads.row(id).bytes, 64) * 5L)));
      Object[] fac = new Object[cap];
      Message[] msgs = new Message[cap];
      for (int i = 0; i < cap; i++) fac[i] = Arms.build(id, CS);
      refill(msgs, id, CS, cap);
      out.reset(Payloads.row(id).bytes + 4096);

      // ---- encode, one iteration count for every arm
      long t0 = System.nanoTime();
      encGrpc(m, msgs, 8, out);
      long est = Math.max(1, (System.nanoTime() - t0) / 8);
      int n = (int) Math.max(8, Math.min(cap, TARGET / est));
      eN[p] = n;
      for (int w = 0; w < 3; w++) {
        refill(msgs, id, CS, n);
        encGrpc(m, msgs, n, out);
        encTba(msgs, n);
        encR(id, fac, n, enc, out);
        encFfi(id, fac, n, b, out);
      }
      long[][] er = new long[4][ROUNDS];
      for (int r = 0; r < ROUNDS; r++) {
        refill(msgs, id, CS, n);
        System.gc();
        Thread.sleep(6);
        for (int j = 0; j < 4; j++) {
          int a = (j + r) % 4;
          out.reset(Payloads.row(id).bytes + 4096);
          long s = System.nanoTime();
          switch (a) {
            case 0: encGrpc(m, msgs, n, out); break;
            case 1: encTba(msgs, n); break;
            case 2: encR(id, fac, n, enc, out); break;
            default: encFfi(id, fac, n, b, out); break;
          }
          er[a][r] = (System.nanoTime() - s) * 1000L / n;
        }
      }
      e[p] = er;

      // ---- decode
      byte[] wire = CS == Values.ASCII ? Payloads.vector(id) : null;
      if (wire == null) wire = pb.deterministic(PbArms.build(id, CS));
      final byte[] w2 = wire;
      t0 = System.nanoTime();
      decGrpc(m, ks, w2, 8);
      est = Math.max(1, (System.nanoTime() - t0) / 8);
      int dn = (int) Math.max(8, TARGET / est);
      dN[p] = dn;
      for (int w = 0; w < 2; w++) {
        decGrpc(m, ks, w2, dn); decPf(id, w2, dn); decR(id, dec, w2, dn); decFfi(id, b, w2, dn);
      }
      long[][] dr = new long[4][ROUNDS];
      for (int r = 0; r < ROUNDS; r++) {
        System.gc();
        Thread.sleep(6);
        for (int j = 0; j < 4; j++) {
          int a = (j + r) % 4;
          long s = System.nanoTime();
          switch (a) {
            case 0: decGrpc(m, ks, w2, dn); break;
            case 1: decPf(id, w2, dn); break;
            case 2: decR(id, dec, w2, dn); break;
            default: decFfi(id, b, w2, dn); break;
          }
          dr[a][r] = (System.nanoTime() - s) * 1000L / dn;
        }
      }
      d[p] = dr;
    }

    // ---- after the last measurement -------------------------------------------------
    StringBuilder sb = new StringBuilder();
    sb.append("== R14: the headline against gRPC's marshaller, not against toByteArray ==\n");
    sb.append("java.version=").append(System.getProperty("java.version"))
      .append("  protobuf-java=").append(RunUnknown.protobufVersion()).append('\n');
    sb.append("grpc-java=1.74.0 (io.grpc.protobuf.lite.ProtoLiteUtils, the real object)\n");
    sb.append("content set=").append(CS == 0 ? "ASCII" : CS == 1 ? "LATIN1" : "above U+00FF")
      .append("  rounds=").append(ROUNDS).append("  sink=").append(sink).append("\n\n");
    sb.append("Every arm writes into the same reused sink and every protobuf message is\n");
    sb.append("serialised once. `pbj-grpc` is marshaller.stream() drained the way grpc's\n");
    sb.append("framer drains it; `pbj-tba` is toByteArray, which is what this slice's other\n");
    sb.append("tables are against and is kept so the two can be read together.\n");
    table(sb, "ENCODE", ids, e, eN, encNames);
    table(sb, "DECODE", ids, d, dN, decNames);
    System.out.print(sb);
  }

  static void table(StringBuilder sb, String dir, List<String> ids, long[][][] v, int[] iters,
                    String[] names) {
    sb.append("\n\n=== ").append(dir).append(" ===\n");
    sb.append("Baseline: ").append(names[0])
      .append(". Cells are min / median / max of the PAIRED per-round ratio.\n\n");
    sb.append(pad("id", 7)).append(pad("base ns/op", 13));
    for (int a = 1; a < names.length; a++) sb.append(pad(names[a], 24));
    sb.append('\n');
    for (int p = 0; p < ids.size(); p++) {
      if (v[p] == null) continue;
      sb.append(pad(ids.get(p), 7)).append(pad(Bench.fmt(med(v[p][0]) / 1000.0, 1), 13));
      for (int a = 1; a < names.length; a++) {
        double[] q = ratio(v[p][a], v[p][0]);
        sb.append(pad(Bench.fmt(q[0], 3) + " " + Bench.fmt(q[1], 3) + " "
                      + Bench.fmt(q[2], 3), 24));
      }
      sb.append('\n');
    }
  }

  static void refill(Message[] m, String id, int cs, int n) {
    for (int i = 0; i < n; i++) m[i] = PbArms.build(id, cs);
  }

  static void encGrpc(MethodDescriptor.Marshaller<Message> m, Message[] msgs, int n, Sink out)
      throws IOException {
    for (int i = 0; i < n; i++) {
      out.n = 0;
      InputStream in = m.stream(msgs[i]);
      sink += ((Drainable) in).drainTo(out);
    }
  }

  static void encTba(Message[] msgs, int n) {
    for (int i = 0; i < n; i++) sink += msgs[i].toByteArray().length;
  }

  static void encR(String id, Object[] fac, int n, Enc e, Sink out) {
    for (int i = 0; i < n; i++) {
      Arms.encodeR(id, fac[i], e);
      out.n = 0;
      out.write(e.buf, 0, e.len);
      sink += out.n;
    }
  }

  static void encFfi(String id, Object[] fac, int n, Binding b, Sink out) {
    for (int i = 0; i < n; i++) {
      FfiArms.encode(b, id, fac[i]);
      out.n = 0;
      out.n = Native.encTake(b.encCtx, out.buf);
      sink += out.n;
    }
  }

  static void decGrpc(MethodDescriptor.Marshaller<Message> m, KnownStream ks, byte[] w, int n) {
    for (int i = 0; i < n; i++)
      sink += System.identityHashCode(m.parse(ks.set(w, 0, w.length)));
  }

  static void decPf(String id, byte[] w, int n) {
    try {
      for (int i = 0; i < n; i++)
        sink += System.identityHashCode(PbArms.parse(id, w, 0, w.length));
    } catch (Exception ex) { throw new IllegalStateException(ex); }
  }

  static void decR(String id, Dec d, byte[] w, int n) {
    for (int i = 0; i < n; i++) sink += Arms.decodeR(id, d, w, 0, w.length) == null ? 0 : 1;
  }

  static void decFfi(String id, Binding b, byte[] w, int n) {
    for (int i = 0; i < n; i++) sink += FfiArms.decode(b, id, w, 0, w.length) == null ? 0 : 1;
  }

  static long med(long[] x) { long[] c = x.clone(); Arrays.sort(c); return c[c.length / 2]; }

  static double[] ratio(long[] a, long[] base) {
    double[] q = new double[a.length];
    for (int i = 0; i < a.length; i++) q[i] = a[i] / (double) base[i];
    Arrays.sort(q);
    return new double[] {q[0], q[q.length / 2], q[q.length - 1]};
  }

  static String pad(String s, int w) {
    StringBuilder b = new StringBuilder(s);
    while (b.length() < w) b.append(' ');
    return b.toString();
  }
}
