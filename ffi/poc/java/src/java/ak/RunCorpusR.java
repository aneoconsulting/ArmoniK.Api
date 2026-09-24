package ak;

import java.io.BufferedReader;
import java.io.FileInputStream;
import java.io.InputStreamReader;
import java.io.PrintStream;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;

/**
 * R-E4: arm R (the generated pure-Java codec) against the conformance corpus.
 *
 * <p>The Java half of {@code gen/corpus_r.py}, which owns every decision about what a row
 * means (ffi/corpus/CONTRACT.md). This file only does, per row: decode the vector with arm
 * R's decoder for that root, project the decoded object, re-encode it, and print one line.
 * It judges nothing.
 *
 * <p><b>Reached by reflection</b> on {@code ak.shapes.Codec}'s package-private
 * {@code dec<Root>(Dec)} and {@code enc<Root>(Enc, T)}, so every message the codec has is a
 * root here, not only the seven with a public entry point. Nothing in the codec is changed
 * or re-derived.
 *
 * <p><b>Projection</b> (CONTRACT.md section 3) walks the facade by reflection, using only
 * the facade's own conventions, which {@code gen/javanames.py} states: a {@code has_x} flag
 * is explicit presence, a field whose fresh default is {@code null} is present iff
 * non-null, a {@code <o>_case} int with {@code <O>_CASE_*} constants is a oneof, and
 * anything else is implicit presence (present iff not the default). A double is printed as
 * its raw bits, {@code "D:<hex>"}, and the Python side renders it with {@code %.17g} so the
 * formatting is C's rather than Java's.
 *
 * <p><b>Per-row timeout</b>: every row runs on its own thread, joined with a timeout
 * ({@code -Dak.corpus.timeoutms}, default 5000). A decoder that loops on a wrapping length
 * is recorded as {@code timeout} and the thread is stopped, which JDK 8 and 17 both still
 * allow.
 *
 * <p>Output, tab separated, one line per row:
 * {@code R id status errclass sha256 nbytes projectionJSON}, where status is
 * {@code accept}, {@code reject} (the decoder threw), {@code timeout}, {@code encfail}
 * (decoded, re-encode threw) or {@code noroot}.
 */
public final class RunCorpusR {
  static final Class<?> CODEC;
  static {
    try {
      CODEC = Class.forName("ak.shapes.Codec");
    } catch (ClassNotFoundException e) {
      throw new RuntimeException(e);
    }
  }

  public static void main(String[] args) throws Exception {
    PrintStream out = new PrintStream(System.out, true, "UTF-8");
    if (args.length > 0 && args[0].equals("--roots")) {
      for (Method m : CODEC.getDeclaredMethods()) {
        Class<?>[] p = m.getParameterTypes();
        if (m.getName().startsWith("dec") && p.length == 1 && p[0] == Dec.class) {
          out.println(m.getName().substring(3));
        }
      }
      return;
    }
    long timeout = Long.parseLong(System.getProperty("ak.corpus.timeoutms", "5000"));
    out.println("# java.version=" + System.getProperty("java.version")
        + "  codec=" + CODEC.getProtectionDomain().getCodeSource().getLocation()
        + "  timeout_ms=" + timeout);
    BufferedReader r = new BufferedReader(
        new InputStreamReader(new FileInputStream(args[0]), "UTF-8"));
    String line;
    while ((line = r.readLine()) != null) {
      if (line.isEmpty()) continue;
      String[] f = line.split("\t");
      final String id = f[0], root = f[1];
      final byte[] buf = Files.readAllBytes(Paths.get(f[2]));
      final String[] res = new String[1];
      Thread t = new Thread(new Runnable() {
        @Override public void run() { res[0] = one(id, root, buf); }
      }, "row-" + id);
      t.setDaemon(true);
      t.start();
      t.join(timeout);
      if (t.isAlive()) {
        stop(t);
        out.println("R\t" + id + "\ttimeout\t-\t-\t0\t-");
      } else {
        out.println(res[0] != null ? res[0]
            : "R\t" + id + "\treject\tno-result\t-\t0\t-");
      }
    }
  }

  @SuppressWarnings("deprecation")
  static void stop(Thread t) {
    try {
      t.stop();
    } catch (Throwable e) {
      // JDK 20+ throws UnsupportedOperationException; the thread is a daemon, so it
      // cannot keep the process alive, only a core busy.
    }
  }

  static String one(String id, String root, byte[] buf) {
    Method dec, enc;
    try {
      Class<?> type = Class.forName("ak.shapes." + root);
      dec = CODEC.getDeclaredMethod("dec" + root, Dec.class);
      enc = CODEC.getDeclaredMethod("enc" + root, Enc.class, type);
      dec.setAccessible(true);
      enc.setAccessible(true);
    } catch (Exception e) {
      return "R\t" + id + "\tnoroot\t" + e.getClass().getSimpleName() + "\t-\t0\t-";
    }
    Object o;
    try {
      Dec d = new Dec();
      d.reset(buf, 0, buf.length);
      o = dec.invoke(null, d);
    } catch (java.lang.reflect.InvocationTargetException e) {
      Throwable c = e.getCause();
      String m = c.getClass().getSimpleName()
          + (c.getMessage() != null ? ":" + c.getMessage().replace('\t', ' ') : "");
      return "R\t" + id + "\treject\t" + m + "\t-\t0\t-";
    } catch (Throwable e) {
      return "R\t" + id + "\treject\t" + e.getClass().getSimpleName() + "\t-\t0\t-";
    }
    String proj;
    try {
      StringBuilder sb = new StringBuilder();
      project(o, sb);
      proj = sb.toString();
    } catch (Throwable e) {
      proj = "PROJECTION-ERROR:" + e;
    }
    try {
      Enc e = new Enc(CODEC.getField("SITES").getInt(null));
      e.reset();
      enc.invoke(null, e, o);
      byte[] w = e.toBytes();
      String dir = System.getProperty("ak.corpus.out");
      if (dir != null) Files.write(Paths.get(dir, id + ".bin"), w);
      return "R\t" + id + "\taccept\t-\t" + sha(w) + "\t" + w.length + "\t" + proj;
    } catch (Throwable t) {
      Throwable c = t instanceof java.lang.reflect.InvocationTargetException
          ? t.getCause() : t;
      return "R\t" + id + "\tencfail\t" + c.getClass().getSimpleName() + "\t-\t0\t" + proj;
    }
  }

  static String sha(byte[] w) throws Exception {
    byte[] h = MessageDigest.getInstance("SHA-256").digest(w);
    StringBuilder s = new StringBuilder();
    for (byte b : h) s.append(String.format("%02x", b & 0xff));
    return s.toString();
  }

  // ---- projection, CONTRACT.md section 3 ------------------------------------------------

  static void project(Object o, StringBuilder sb) throws Exception {
    Class<?> c = o.getClass();
    Object fresh = c.getDeclaredConstructor().newInstance();
    List<Field> fields = new ArrayList<Field>();
    for (Field f : c.getDeclaredFields()) {
      if (Modifier.isStatic(f.getModifiers())) continue;
      fields.add(f);
    }
    sb.append('{');
    boolean first = true;
    for (Field f : fields) {
      String n = f.getName();
      if (n.startsWith("has_")) continue;
      Object v = f.get(o);
      // a oneof: `<o>_case` plus members `<o>_<m>`, named by `<O>_CASE_<M>` constants
      if (n.endsWith("_case") && f.getType() == int.class) {
        String p = n.substring(0, n.length() - 5);
        int cs = (Integer) v;
        if (cs == 0) continue;
        for (Field k : c.getDeclaredFields()) {
          String kn = k.getName();
          String pre = p.toUpperCase() + "_CASE_";
          if (!Modifier.isStatic(k.getModifiers()) || !kn.startsWith(pre)
              || kn.equals(pre + "NONE")) continue;
          if (k.getInt(null) != cs) continue;
          String member = kn.substring(pre.length()).toLowerCase();
          Field mf = c.getDeclaredField(p + "_" + member);
          first = key(sb, first, member);
          value(mf.get(o), mf.getType(), sb);
        }
        continue;
      }
      if (isOneofMember(c, n)) continue;
      Field has = null;
      try {
        has = c.getDeclaredField("has_" + n);
      } catch (NoSuchFieldException e) {
        // implicit presence, or a nullable one
      }
      boolean present;
      if (has != null) {
        present = has.getBoolean(o);
      } else if (f.get(fresh) == null) {
        present = v != null;           // explicit presence carried as a null default
      } else {
        present = !isDefault(v);
      }
      if (!present) continue;
      first = key(sb, first, n);
      value(v, f.getType(), sb);
    }
    sb.append('}');
  }

  static boolean isOneofMember(Class<?> c, String n) {
    for (Field f : c.getDeclaredFields()) {
      String fn = f.getName();
      if (!Modifier.isStatic(f.getModifiers()) && fn.endsWith("_case")
          && f.getType() == int.class) {
        String p = fn.substring(0, fn.length() - 5) + "_";
        if (n.startsWith(p)) return true;
      }
    }
    return false;
  }

  static boolean key(StringBuilder sb, boolean first, String k) {
    if (!first) sb.append(',');
    str(k, sb);
    sb.append(':');
    return false;
  }

  static boolean isDefault(Object v) {
    if (v == null) return true;
    if (v instanceof String) return ((String) v).isEmpty();
    if (v instanceof Integer) return (Integer) v == 0;
    if (v instanceof Long) return (Long) v == 0L;
    if (v instanceof Boolean) return !(Boolean) v;
    if (v instanceof Double) return Double.doubleToRawLongBits((Double) v) == 0L;
    if (v instanceof byte[]) return ((byte[]) v).length == 0;
    if (v instanceof int[]) return ((int[]) v).length == 0;
    if (v instanceof long[]) return ((long[]) v).length == 0;
    if (v instanceof double[]) return ((double[]) v).length == 0;
    if (v instanceof boolean[]) return ((boolean[]) v).length == 0;
    if (v instanceof List) return ((List<?>) v).isEmpty();
    if (v instanceof Map) return ((Map<?, ?>) v).isEmpty();
    return false;                      // a message that is not null is present
  }

  static void value(Object v, Class<?> t, StringBuilder sb) throws Exception {
    if (v == null) { sb.append("null"); return; }
    if (v instanceof String) { str((String) v, sb); return; }
    if (v instanceof Integer || v instanceof Long) { str(v.toString(), sb); return; }
    if (v instanceof Boolean) { sb.append(v.toString()); return; }
    if (v instanceof Double) { dbl((Double) v, sb); return; }
    if (v instanceof byte[]) { hex((byte[]) v, sb); return; }
    if (v instanceof int[]) {
      sb.append('[');
      int[] a = (int[]) v;
      for (int i = 0; i < a.length; i++) { if (i > 0) sb.append(','); str("" + a[i], sb); }
      sb.append(']');
      return;
    }
    if (v instanceof long[]) {
      sb.append('[');
      long[] a = (long[]) v;
      for (int i = 0; i < a.length; i++) { if (i > 0) sb.append(','); str("" + a[i], sb); }
      sb.append(']');
      return;
    }
    if (v instanceof double[]) {
      sb.append('[');
      double[] a = (double[]) v;
      for (int i = 0; i < a.length; i++) { if (i > 0) sb.append(','); dbl(a[i], sb); }
      sb.append(']');
      return;
    }
    if (v instanceof boolean[]) {
      sb.append('[');
      boolean[] a = (boolean[]) v;
      for (int i = 0; i < a.length; i++) { if (i > 0) sb.append(','); sb.append(a[i]); }
      sb.append(']');
      return;
    }
    if (v instanceof List) {
      sb.append('[');
      boolean first = true;
      for (Object x : (List<?>) v) {
        if (!first) sb.append(',');
        first = false;
        value(x, x == null ? Object.class : x.getClass(), sb);
      }
      sb.append(']');
      return;
    }
    if (v instanceof Map) {
      sb.append('{');
      boolean first = true;
      for (Map.Entry<?, ?> e : ((Map<?, ?>) v).entrySet()) {
        first = key(sb, first, String.valueOf(e.getKey()));
        Object x = e.getValue();
        value(x, x == null ? Object.class : x.getClass(), sb);
      }
      sb.append('}');
      return;
    }
    project(v, sb);
  }

  static void dbl(double d, StringBuilder sb) {
    str("D:" + Long.toHexString(Double.doubleToRawLongBits(d)), sb);
  }

  static void hex(byte[] b, StringBuilder sb) {
    StringBuilder h = new StringBuilder();
    for (byte x : b) h.append(String.format("%02x", x & 0xff));
    str(h.toString(), sb);
  }

  static void str(String s, StringBuilder sb) {
    sb.append('"');
    for (int i = 0; i < s.length(); i++) {
      char ch = s.charAt(i);
      if (ch == '"' || ch == '\\') sb.append('\\').append(ch);
      else if (ch < 0x20 || ch > 0x7e) sb.append(String.format("\\u%04x", (int) ch));
      else sb.append(ch);
    }
    sb.append('"');
  }
}
