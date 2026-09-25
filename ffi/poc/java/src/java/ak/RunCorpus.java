package ak;

import java.io.BufferedReader;
import java.io.FileInputStream;
import java.io.InputStreamReader;
import java.io.PrintStream;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.security.MessageDigest;
import java.util.Arrays;
import java.util.HashSet;
import java.util.Set;

/**
 * FIX-PLAN WP5 step 3's corpus gate: ffi/corpus against every Java arm that decodes, over
 * the CORPUS description ({@code ak.corpus}, generated from the corpus's reader schema by
 * the Java backend in poc/codec/gen), in both unknown-field modes where the arm has them.
 *
 * <p>The Java half of {@code gen/corpus.py}, which owns every judgement (CONTRACT.md).
 * This file only does, per (arm, row): decode, project, re-encode, and print one line.
 *
 * <pre>
 *   R           arm R, unknown fields dropped   (ak.corpus.Codec)
 *   R-retain    arm R, unknown fields retained  (ak.corpus.CodecRetain)
 *   ffi         the core through JNI, push family decode, push encode
 *   ffi-pull    pull family (ak_parse_* + drain), re-encoded with the push encode
 *   ffi-pull-walk  pull family read in place (ak_bdr_ptr)
 *   ffi-borrow  decision 13's borrowed facade over the same binding
 * </pre>
 *
 * <p>The ffi arms need {@code -Dak.lib} = the shim linked against the core built with the
 * {@code corpus} and {@code init-guard} features; a row whose root the C ABI cannot carry
 * (plan.check_expressible: {@code Nest}) is {@code noabi}. Every row runs on its own
 * thread under {@code -Dak.corpus.timeoutms}; a timeout is recorded, the thread is
 * abandoned (a daemon) and the arm's binding is replaced.
 *
 * <p>Planted controls ({@code -Dak.corpus.plant}), each of which gen/corpus.py must see
 * FAIL: {@code proj} adds a key to every projection (C2), {@code reenc} appends a byte to
 * every re-encoding (C3), {@code accept} turns every refusal into an acceptance (C4).
 * {@code -Dak.skipInit=1} skips {@code ak_init} (every ffi row must then fail).
 *
 * <p>Output, tab separated: {@code R arm id status err sha256 nbytes projection}, status
 * one of accept, reject, timeout, encfail, noabi.
 */
public final class RunCorpus {
  static final String PLANT = System.getProperty("ak.corpus.plant", "");
  static final Set<String> ABI = new HashSet<String>(Arrays.asList(ak.corpus.Dispatch.ABI));

  interface Arm {
    Object decode(String root, byte[] w);
    byte[] encode(String root, Object o);
    String project(String root, Object o);
  }

  static Arm arm(String name) {
    if (name.equals("R"))
      return new Arm() {
        public Object decode(String r, byte[] w) { return ak.corpus.Dispatch.decR(r, w); }
        public byte[] encode(String r, Object o) { return ak.corpus.Dispatch.encR(r, o); }
        public String project(String r, Object o) { return ak.corpus.Project.project(r, o); }
      };
    if (name.equals("R-retain"))
      return new Arm() {
        public Object decode(String r, byte[] w) { return ak.corpus.Dispatch.decRRetain(r, w); }
        public byte[] encode(String r, Object o) { return ak.corpus.Dispatch.encRRetain(r, o); }
        public String project(String r, Object o) { return ak.corpus.Project.project(r, o); }
      };
    if (name.equals("ffi") || name.equals("ffi-pull") || name.equals("ffi-pull-walk")
        || name.equals("ffi-retain") || name.equals("ffi-pull-retain") || name.equals("ffi-pull-walk-retain")) {
      final ak.corpus.Binding b = new ak.corpus.Binding();
      final boolean pull = name.startsWith("ffi-pull");
      b.pullWalk = name.startsWith("ffi-pull-walk");
      // Decision 11 (WP5 step 9): every position armed, u-group encode.
      b.retain = name.endsWith("-retain");
      return new Arm() {
        public Object decode(String r, byte[] w) {
          return pull ? ak.corpus.Dispatch.parseFfi(b, r, w) : ak.corpus.Dispatch.decFfi(b, r, w);
        }
        public byte[] encode(String r, Object o) { return ak.corpus.Dispatch.encFfi(b, r, o); }
        public String project(String r, Object o) { return ak.corpus.Project.project(r, o); }
      };
    }
    if (name.equals("ffi-borrow") || name.equals("ffi-borrow-retain")) {
      final ak.corpus.borrow.Binding b = new ak.corpus.borrow.Binding();
      b.retain = name.endsWith("-retain");
      return new Arm() {
        public Object decode(String r, byte[] w) { return ak.corpus.borrow.Dispatch.decFfi(b, r, w); }
        public byte[] encode(String r, Object o) { return ak.corpus.borrow.Dispatch.encFfi(b, r, o); }
        public String project(String r, Object o) { return ak.corpus.borrow.Project.project(r, o); }
      };
    }
    throw new IllegalArgumentException("no arm " + name);
  }

  public static void main(String[] args) throws Exception {
    PrintStream out = new PrintStream(System.out, true, "UTF-8");
    long timeout = Long.parseLong(System.getProperty("ak.corpus.timeoutms", "5000"));
    String[] arms = System.getProperty("ak.corpus.arms", "R,R-retain").split(",");
    String outDir = System.getProperty("ak.corpus.out");
    out.println("# java.version=" + System.getProperty("java.version") + "  timeout_ms=" + timeout
        + "  arms=" + String.join(",", arms) + "  plant=" + (PLANT.isEmpty() ? "none" : PLANT)
        + "  ak.lib=" + System.getProperty("ak.lib", "unset")
        + "  skipInit=" + System.getProperty("ak.skipInit", "0"));
    out.println("# codec LIMIT=" + ak.corpus.Codec.LIMIT + " utf8=" + ak.corpus.Codec.UTF8_POLICY
        + "  C ABI roots " + ABI.size() + " of " + ak.corpus.Dispatch.ALL.length + " messages");
    java.util.List<String[]> rows = new java.util.ArrayList<String[]>();
    BufferedReader r = new BufferedReader(new InputStreamReader(new FileInputStream(args[0]), "UTF-8"));
    String line;
    while ((line = r.readLine()) != null) if (!line.isEmpty()) rows.add(line.split("\t"));
    r.close();
    for (String an : arms) {
      Arm[] a = new Arm[1];
      try {
        a[0] = arm(an);
      } catch (Throwable t) {
        // A binding that cannot even be constructed (ak_init refused, layout guard fired,
        // library missing) fails EVERY row of the arm, loudly.
        String why = oneLine(t);
        out.println("# arm " + an + " could not be constructed: " + why);
        for (String[] f : rows)
          out.println("R\t" + an + "\t" + f[0] + "\treject\tARM-UNAVAILABLE:" + why + "\t-\t0\t-");
        continue;
      }
      boolean ffi = an.startsWith("ffi");
      for (String[] f : rows) {
        final String id = f[0], root = f[1];
        if (ffi && !ABI.contains(root)) {
          out.println("R\t" + an + "\t" + id + "\tnoabi\t-\t-\t0\t-");
          continue;
        }
        final byte[] buf = Files.readAllBytes(Paths.get(f[2]));
        final String[] res = new String[1];
        final Arm arm0 = a[0];
        final String an0 = an;
        Thread t = new Thread(new Runnable() {
          @Override public void run() { res[0] = one(an0, arm0, id, root, buf, outDir); }
        }, "row-" + an + "-" + id);
        t.setDaemon(true);
        t.start();
        t.join(timeout);
        if (t.isAlive()) {
          stop(t);
          out.println("R\t" + an + "\t" + id + "\ttimeout\t-\t-\t0\t-");
          try { a[0] = arm(an); } catch (Throwable e) { /* the next row reports it */ }
        } else {
          out.println(res[0] != null ? res[0] : "R\t" + an + "\t" + id + "\treject\tno-result\t-\t0\t-");
        }
      }
    }
  }

  @SuppressWarnings("deprecation")
  static void stop(Thread t) {
    try {
      t.stop();
    } catch (Throwable e) {
      // JDK 20+ refuses; the thread is a daemon and cannot keep the process alive.
    }
  }

  static String oneLine(Throwable t) {
    Throwable c = t;
    while (c.getCause() != null && c.getCause() != c) c = c.getCause();
    String m = c.getClass().getSimpleName() + (c.getMessage() != null ? ":" + c.getMessage() : "");
    return m.replace('\t', ' ').replace('\n', ' ');
  }

  static String one(String an, Arm arm, String id, String root, byte[] buf, String outDir) {
    Object o;
    try {
      o = arm.decode(root, buf);
    } catch (Throwable e) {
      if (PLANT.equals("accept"))
        return "R\t" + an + "\t" + id + "\taccept\tplanted\t-\t0\t{}";
      return "R\t" + an + "\t" + id + "\treject\t" + oneLine(e) + "\t-\t0\t-";
    }
    String proj;
    try {
      proj = arm.project(root, o);
      if (PLANT.equals("proj")) proj = proj.equals("{}") ? "{\"planted\":\"1\"}"
          : "{\"planted\":\"1\"," + proj.substring(1);
    } catch (Throwable e) {
      proj = "PROJECTION-ERROR:" + oneLine(e);
    }
    try {
      byte[] w = arm.encode(root, o);
      if (PLANT.equals("reenc")) w = Arrays.copyOf(w, w.length + 1);
      if (outDir != null) Files.write(Paths.get(outDir, an + "." + id + ".bin"), w);
      return "R\t" + an + "\t" + id + "\taccept\t-\t" + sha(w) + "\t" + w.length + "\t" + proj;
    } catch (Throwable t) {
      return "R\t" + an + "\t" + id + "\tencfail\t" + oneLine(t) + "\t-\t0\t" + proj;
    }
  }

  static String sha(byte[] w) throws Exception {
    byte[] h = MessageDigest.getInstance("SHA-256").digest(w);
    StringBuilder s = new StringBuilder();
    for (byte b : h) s.append(String.format("%02x", b & 0xff));
    return s.toString();
  }
}
