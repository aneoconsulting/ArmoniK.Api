package ak;

import ak.shapes.PbArms;
import com.google.protobuf.CodedOutputStream;
import com.google.protobuf.Message;
import java.util.Arrays;

/**
 * Is the incumbent handicapped, or flattered? The check the cpp slice's review says every
 * managed slice must run before it reports.
 *
 * <p>Its finding was a baseline doing EXTRA work -- a zero-filled output buffer and a
 * hand-rolled stream -- worth about eight points of every encode ratio. This slice looked
 * for the same class and found the opposite sign, which is the more dangerous one because
 * it flatters the incumbent and a slice has no reason to go looking.
 *
 * <p><b>protobuf-java memoizes {@code getSerializedSize()} on the message instance.</b>
 * Both {@code toByteArray} and {@code writeTo} call it, so in a loop over ONE message the
 * size pass -- a full walk of the tree computing every string's UTF-8 length -- is paid
 * once and amortised over every iteration. Every other arm here pays its own equivalent
 * on every iteration. An application serialises a message once, so the production cost
 * includes it; a loop benchmark excludes it, for the incumbent alone.
 *
 * <p>This measures what that hides, per payload, by timing {@code getSerializedSize()} on
 * messages that have never been serialised. The figure is a correction to add to the
 * incumbent's encode column, not a defect in protobuf-java.
 */
public final class RunBaseline {
  static final int POOL = 24;
  static long sink;

  public static void main(String[] args) throws Exception {
    final int ROUNDS = Integer.getInteger("ak.rounds", 9);
    String[] ids = ak.shapes.Arms.IDS;

    // Measure first, format later (README R9).
    long[][] cold = new long[ids.length][ROUNDS];
    long[][] warm = new long[ids.length][ROUNDS];
    long[][] ser = new long[ids.length][ROUNDS];
    boolean[] ok = new boolean[ids.length];

    for (int p = 0; p < ids.length; p++) {
      String id = ids[p];
      byte[] wire = Payloads.vector(id);
      if (wire == null) {
        Message m0 = PbArms.build(id, Values.ASCII);
        if (m0 == null) continue;
        wire = new PbArm().deterministic(m0);
      }
      ok[p] = true;
      byte[] scratch = new byte[wire.length + 4096];
      // Warm the JIT on a pool that is thrown away.
      for (int w = 0; w < 3; w++) {
        Message[] pool = fresh(id, wire);
        for (Message m : pool) sink += m.getSerializedSize();
        for (Message m : pool) sink += writeInto(m, scratch);
      }
      for (int r = 0; r < ROUNDS; r++) {
        Message[] pool = fresh(id, wire);
        long t0 = System.nanoTime();
        for (Message m : pool) sink += m.getSerializedSize();   // never serialised before
        cold[p][r] = (System.nanoTime() - t0) * 1000L / POOL;
        t0 = System.nanoTime();
        for (Message m : pool) sink += m.getSerializedSize();   // memoized now
        warm[p][r] = (System.nanoTime() - t0) * 1000L / POOL;
        t0 = System.nanoTime();
        for (Message m : pool) sink += writeInto(m, scratch);   // the write alone
        ser[p][r] = (System.nanoTime() - t0) * 1000L / POOL;
      }
    }

    StringBuilder sb = new StringBuilder();
    sb.append("== is the incumbent flattered? protobuf-java's memoized serialized size ==\n");
    sb.append("java.version=").append(System.getProperty("java.version"))
      .append("  protobuf-java=").append(RunUnknown.protobufVersion()).append('\n');
    sb.append("pool=").append(POOL).append(" freshly parsed messages per round, rounds=")
      .append(ROUNDS).append("  sink=").append(sink).append("\n\n");
    sb.append("`size cold` is getSerializedSize() on a message that has never been\n")
      .append("serialised; `size warm` is the same call once it is memoized; `write` is\n")
      .append("writeTo into a buffer the caller owns, which is what the bench's `pbj` arm\n")
      .append("measures on iteration 2 and after.\n\n");
    sb.append("A loop over ONE message therefore reports `write` where an application\n")
      .append("that serialises each message once pays `size cold` + `write`. The last\n")
      .append("column is the correction the bench's pbj column needs.\n\n");
    sb.append(pad("id", 7)).append(pad("size cold", 12)).append(pad("size warm", 12))
      .append(pad("write", 12)).append(pad("cold+write", 12)).append("hidden\n");
    for (int p = 0; p < ids.length; p++) {
      if (!ok[p]) continue;
      double c = med(cold[p]) / 1000.0, w = med(warm[p]) / 1000.0, s = med(ser[p]) / 1000.0;
      sb.append(pad(ids[p], 7)).append(pad(f(c), 12)).append(pad(f(w), 12))
        .append(pad(f(s), 12)).append(pad(f(c + s), 12))
        .append(f((c + s) / Math.max(s, 1e-9))).append("x\n");
    }
    sb.append("\nRead the last column as: the bench's protobuf-java encode figure should\n")
      .append("be multiplied by this to describe an application, and every ratio against\n")
      .append("it divided by it. The bench reports the UNCORRECTED figure and says so,\n")
      .append("because which of the two a reader wants depends on the question.\n");
    System.out.print(sb);
  }

  static Message[] fresh(String id, byte[] wire) throws Exception {
    Message[] pool = new Message[POOL];
    for (int i = 0; i < POOL; i++) pool[i] = PbArms.parse(id, wire, 0, wire.length);
    return pool;
  }

  static int writeInto(Message m, byte[] scratch) throws java.io.IOException {
    CodedOutputStream cos = CodedOutputStream.newInstance(scratch);
    m.writeTo(cos);
    return cos.getTotalBytesWritten();
  }

  static long med(long[] v) {
    long[] c = v.clone();
    Arrays.sort(c);
    return c[c.length / 2];
  }

  static String pad(String s, int w) {
    StringBuilder b = new StringBuilder(s);
    while (b.length() < w) b.append(' ');
    return b.toString();
  }

  static String f(double v) { return Bench.fmt(v, 1); }
}
