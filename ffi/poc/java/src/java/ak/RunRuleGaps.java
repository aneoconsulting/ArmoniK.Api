package ak;

import java.io.FileOutputStream;

/**
 * R-E4, the rule gaps the corpus cannot reach in arm R's scope, run rather than read.
 *
 * <p>The corpus has no vector for a singular message field (or a oneof message member)
 * arriving twice, and no encode-side vector at all, so {@code gen/corpus_r.py} cannot
 * confirm two of the review's five gaps. This prints what arm R does with:
 * <ol>
 *   <li>{@code ResultRaw.created_at} on the wire twice, {@code {seconds:1}} then
 *       {@code {nanos:2}}. protobuf MERGES a repeated singular message field, so a
 *       conformant reader holds both; the vector is written to {@code args[0]} so the
 *       caller can ask protobuf C++ ({@code protoc --decode}) the same question;</li>
 *   <li>{@code Probe.as_stamp} (a oneof message member) twice, the same way;</li>
 *   <li>{@code Probe} with {@code body_case = 99}, a case the schema does not declare,
 *       encoded. A conformant generator refuses it (FIX-PLAN WP5, "a refusal for an
 *       unknown case"); this prints whether arm R refuses or writes something.</li>
 * </ol>
 * It judges nothing; the log states the expected behaviour beside the observed one.
 */
public final class RunRuleGaps {
  public static void main(String[] args) throws Exception {
    Class<?> codec = Class.forName("ak.shapes.Codec");
    java.lang.reflect.Method decRR = codec.getDeclaredMethod("decResultRaw", Dec.class);
    java.lang.reflect.Method decP = codec.getDeclaredMethod("decProbe", Dec.class);
    java.lang.reflect.Method encP = codec.getDeclaredMethod("encProbe", Enc.class,
        Class.forName("ak.shapes.Probe"));
    decRR.setAccessible(true);
    decP.setAccessible(true);
    encP.setAccessible(true);

    // 1. ResultRaw.created_at (field 5) twice
    byte[] rr = {0x2a, 0x02, 0x08, 0x01, 0x2a, 0x02, 0x10, 0x02};
    Object o = decRR.invoke(null, new Dec().reset(rr, 0, rr.length));
    StringBuilder sb = new StringBuilder();
    RunCorpusR.project(o, sb);
    System.out.println("merge-singular  wire=2a020801 2a021002  armR=" + sb);
    if (args.length > 0) {
      FileOutputStream f = new FileOutputStream(args[0] + "/merge-singular.bin");
      f.write(rr);
      f.close();
    }

    // 2. Probe.as_stamp (field 13, oneof member) twice
    byte[] pb = {0x6a, 0x02, 0x08, 0x01, 0x6a, 0x02, 0x10, 0x02};
    o = decP.invoke(null, new Dec().reset(pb, 0, pb.length));
    sb.setLength(0);
    RunCorpusR.project(o, sb);
    System.out.println("merge-oneof     wire=6a020801 6a021002  armR=" + sb);
    if (args.length > 0) {
      FileOutputStream f = new FileOutputStream(args[0] + "/merge-oneof.bin");
      f.write(pb);
      f.close();
    }

    // 3. an undeclared oneof case, encoded
    Object p = Class.forName("ak.shapes.Probe").getDeclaredConstructor().newInstance();
    p.getClass().getField("id").set(p, "x");
    p.getClass().getField("body_case").setInt(p, 99);
    Enc e = new Enc(codec.getField("SITES").getInt(null));
    e.reset();
    String what;
    try {
      encP.invoke(null, e, p);
      byte[] w = e.toBytes();
      StringBuilder h = new StringBuilder();
      for (byte b : w) h.append(String.format("%02x", b & 0xff));
      what = "no refusal; wrote " + w.length + " B " + h;
    } catch (java.lang.reflect.InvocationTargetException x) {
      what = "refused: " + x.getCause();
    }
    System.out.println("unknown-case    body_case=99  armR=" + what);

    // 4. E6: `Dec.readLen` checks `pos + n > limit` in int. A length of 2^31 - 1 after a
    //    one-byte key makes pos + n wrap negative, so the check passes. The corpus's
    //    X-lenwrap rows wrap 2^64, whose low 32 bits are negative as an int and are
    //    refused by `n < 0`, so none of them reaches this.
    java.lang.reflect.Method decL = codec.getDeclaredMethod("decListResultsResponse",
        Dec.class);
    decL.setAccessible(true);
    byte[] lw = {0x0a, (byte) 0xff, (byte) 0xff, (byte) 0xff, (byte) 0xff, 0x07};
    try {
      decL.invoke(null, new Dec().reset(lw, 0, lw.length));
      what = "ACCEPTED";
    } catch (java.lang.reflect.InvocationTargetException x) {
      what = "threw " + x.getCause();
    }
    System.out.println("int-lenwrap     wire=0affffffff07 (ListResultsResponse)  armR=" + what
        + "   (a conformant refusal is Dec.Malformed)");
  }
}
