package ak;

import java.io.File;
import java.io.FileInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/** The manifest as this slice's oracle: the id, the root, the expected size and hash, and
 *  the committed vector where there is one. */
public final class Payloads {
  public static final class Row {
    public final String id, root, sha256, vector;
    public final int bytes, elements, strings;
    public final String mode;
    Row(String id, Map<String, Object> m) {
      this.id = id;
      this.root = (String) m.get("root");
      this.sha256 = (String) m.get("sha256");
      this.bytes = (int) (long) (Long) m.get("bytes");
      this.elements = (int) (long) (Long) m.get("elements");
      this.strings = (int) (long) (Long) m.get("strings");
      this.mode = (String) m.get("mode");
      this.vector = (String) m.get("vector");
    }
  }

  public static final File SCHEMA_DIR = schemaDir();
  private static Map<String, Row> rows;

  private static File schemaDir() {
    String p = System.getProperty("ak.schema");
    if (p != null) return new File(p);
    // Walk up from the working directory to `ffi/schema/generated`.
    File d = new File(".").getAbsoluteFile();
    while (d != null) {
      File c = new File(d, "ffi/schema/generated");
      if (c.isDirectory()) return c;
      c = new File(d, "schema/generated");
      if (c.isDirectory()) return c;
      d = d.getParentFile();
    }
    throw new IllegalStateException("cannot find ffi/schema/generated; pass -Dak.schema=");
  }

  @SuppressWarnings("unchecked")
  public static synchronized Map<String, Row> all() {
    if (rows != null) return rows;
    Map<String, Object> top = (Map<String, Object>) Json.parse(
        new String(readFile(new File(SCHEMA_DIR, "manifest.json")),
                   java.nio.charset.Charset.forName("UTF-8")));
    Map<String, Object> ps = (Map<String, Object>) top.get("payloads");
    Map<String, Row> out = new LinkedHashMap<String, Row>();
    for (Map.Entry<String, Object> e : ps.entrySet())
      out.put(e.getKey(), new Row(e.getKey(), (Map<String, Object>) e.getValue()));
    rows = out;
    return out;
  }

  public static Row row(String id) {
    Row r = all().get(id);
    if (r == null) throw new IllegalArgumentException("no payload " + id);
    return r;
  }

  /** The committed bytes, where the manifest carries a vector (<= 64 KB). */
  public static byte[] vector(String id) {
    Row r = row(id);
    if (r.vector == null) return null;
    return readFile(new File(SCHEMA_DIR, r.vector));
  }

  public static List<String> ids() {
    return new ArrayList<String>(all().keySet());
  }

  public static byte[] readFile(File f) {
    try {
      InputStream in = new FileInputStream(f);
      try {
        byte[] out = new byte[(int) f.length()];
        int n = 0;
        while (n < out.length) {
          int k = in.read(out, n, out.length - n);
          if (k < 0) break;
          n += k;
        }
        return out;
      } finally {
        in.close();
      }
    } catch (IOException e) {
      throw new IllegalStateException("reading " + f, e);
    }
  }

  public static String hex(byte[] b) {
    char[] H = "0123456789abcdef".toCharArray();
    char[] out = new char[b.length * 2];
    for (int i = 0; i < b.length; i++) {
      out[2 * i] = H[(b[i] >> 4) & 0xF];
      out[2 * i + 1] = H[b[i] & 0xF];
    }
    return new String(out);
  }
}
