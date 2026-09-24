package ak;

import java.io.FileOutputStream;

/**
 * R-E4 and R-G8: the rule gaps the corpus does not reach directly, run on arm R and (when
 * {@code -Dak.lib} is set) on the ffi arm, over the corpus description ({@code ak.corpus}).
 *
 * <ol>
 *   <li>merge, singular: {@code ResultRaw.created_at} twice ({@code {seconds:1}} then
 *       {@code {nanos:2}}): protobuf MERGES, so a conformant reader holds both;</li>
 *   <li>merge, oneof: {@code Probe.as_stamp} twice, the same way;</li>
 *   <li>unknown oneof case: {@code Probe} with {@code body_case = 99} encoded; the plan
 *       REFUSES it ({@code oneof_checks}, ERR_ABI);</li>
 *   <li>-0.0: {@code WireZoo.v_double = -0.0} encoded; the plan's implicit-presence test is
 *       the bit pattern, so it is written ({@code 21 0000000000000080});</li>
 *   <li>R-G8, the int length wrap: {@code 0a ffffffff07} on {@code ListResultsResponse}, a
 *       length of 2^31 - 1 with 4 bytes left: refused as past the end of the buffer.</li>
 * </ol>
 * The vectors of 1 and 2 are written to {@code args[0]} so the caller can ask protobuf C++
 * ({@code protoc --decode}) the same question. It prints; gen/corpus.sh reads it.
 */
public final class RunRuleGaps {
  static String hex(byte[] w) {
    StringBuilder h = new StringBuilder();
    for (byte b : w) h.append(String.format("%02x", b & 0xff));
    return h.toString();
  }

  interface Codec {
    Object dec(String root, byte[] w);
    byte[] enc(String root, Object o);
  }

  public static void main(String[] args) throws Exception {
    byte[] rr = {0x2a, 0x02, 0x08, 0x01, 0x2a, 0x02, 0x10, 0x02};
    byte[] pb = {0x6a, 0x02, 0x08, 0x01, 0x6a, 0x02, 0x10, 0x02};
    if (args.length > 0) {
      FileOutputStream f = new FileOutputStream(args[0] + "/merge-singular.bin");
      f.write(rr);
      f.close();
      f = new FileOutputStream(args[0] + "/merge-oneof.bin");
      f.write(pb);
      f.close();
    }
    run("R", new Codec() {
      public Object dec(String r, byte[] w) { return ak.corpus.Dispatch.decR(r, w); }
      public byte[] enc(String r, Object o) { return ak.corpus.Dispatch.encR(r, o); }
    }, rr, pb);
    if (System.getProperty("ak.lib") != null) {
      final ak.corpus.Binding b = new ak.corpus.Binding();
      run("ffi", new Codec() {
        public Object dec(String r, byte[] w) { return ak.corpus.Dispatch.decFfi(b, r, w); }
        public byte[] enc(String r, Object o) { return ak.corpus.Dispatch.encFfi(b, r, o); }
      }, rr, pb);
    }
  }

  static void run(String arm, Codec c, byte[] rr, byte[] pb) {
    System.out.println("merge-singular  " + arm + "  wire=" + hex(rr) + "  read="
        + ak.corpus.Project.project("ResultRaw", c.dec("ResultRaw", rr))
        + "   (protobuf: seconds 1 AND nanos 2)");
    System.out.println("merge-oneof     " + arm + "  wire=" + hex(pb) + "  read="
        + ak.corpus.Project.project("Probe", c.dec("Probe", pb))
        + "   (protobuf: seconds 1 AND nanos 2)");

    ak.corpus.Probe p = new ak.corpus.Probe();
    p.id = "x";
    p.body_case = 99;
    String what;
    try {
      what = "NO REFUSAL; wrote " + hex(c.enc("Probe", p));
    } catch (Throwable t) {
      what = "refused: " + t;
    }
    System.out.println("unknown-case    " + arm + "  body_case=99  " + what);

    ak.corpus.WireZoo z = new ak.corpus.WireZoo();
    z.v_double = -0.0;
    ak.corpus.WireZoo zp = new ak.corpus.WireZoo();
    zp.v_double = 0.0;
    System.out.println("minus-zero      " + arm + "  v_double=-0.0 wrote " + hex(c.enc("WireZoo", z))
        + "   +0.0 wrote '" + hex(c.enc("WireZoo", zp)) + "'   (plan: -0.0 is 210000000000000080, +0.0 nothing)");

    byte[] lw = {0x0a, (byte) 0xff, (byte) 0xff, (byte) 0xff, (byte) 0xff, 0x07};
    try {
      c.dec("ListResultsResponse", lw);
      what = "ACCEPTED";
    } catch (Throwable t) {
      what = "refused: " + t;
    }
    System.out.println("int-lenwrap     " + arm + "  wire=" + hex(lw) + "  " + what
        + "   (R-G8: refused as past the end, at the length)");
  }
}
