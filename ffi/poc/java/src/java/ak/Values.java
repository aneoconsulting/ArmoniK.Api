package ak;

import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;

/**
 * The deterministic field values of {@code ffi/schema/emit/values.py}, hand-re-derived in
 * Java.
 *
 * <p>No RNG anywhere: a value is a pure function of (field path, element index), so five
 * slices in five languages produce identical bytes by implementing the same function
 * rather than by sharing a data file. The manifest pins the outcome, and this file is
 * wrong if {@code ffi/schema/generated/manifest.json} disagrees -- which is what
 * {@code Conformance} checks before anything is timed.
 *
 * <p>Java 8 clean: nothing here is newer than {@code Long.remainderUnsigned}, which is
 * Java 8. The unsigned arithmetic is not decoration -- {@code h64} is the low eight bytes
 * of a digest read little-endian, which Python treats as an unsigned integer and Java's
 * {@code long} does not.
 */
public final class Values {
  private Values() {}

  public static final String[] VOCAB = {
      "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel",
      "india", "juliet", "kilo", "lima", "mike", "november", "oscar", "papa"};

  public static final int ASCII = 0, LATIN1 = 1, WIDE = 2;

  private static final ThreadLocal<MessageDigest> MD = new ThreadLocal<MessageDigest>() {
    @Override protected MessageDigest initialValue() {
      try {
        return MessageDigest.getInstance("SHA-256");
      } catch (NoSuchAlgorithmException e) {
        throw new IllegalStateException(e);
      }
    }
  };

  private static final char[] HEX = "0123456789abcdef".toCharArray();

  public static byte[] sha256(byte[] in) {
    MessageDigest md = MD.get();
    md.reset();
    return md.digest(in);
  }

  public static String sha256Hex(byte[] in) {
    byte[] d = sha256(in);
    char[] out = new char[d.length * 2];
    for (int i = 0; i < d.length; i++) {
      out[2 * i] = HEX[(d[i] >> 4) & 0xF];
      out[2 * i + 1] = HEX[d[i] & 0xF];
    }
    return new String(out);
  }

  private static byte[] ascii(String s) {
    byte[] b = new byte[s.length()];
    for (int i = 0; i < b.length; i++) b[i] = (byte) s.charAt(i);
    return b;
  }

  /** The low eight bytes of sha256("path#idx"), little-endian, as Python reads them. */
  public static long h64(String path, int idx) {
    byte[] d = sha256(ascii(path + "#" + idx));
    long h = 0;
    for (int i = 7; i >= 0; i--) h = (h << 8) | (d[i] & 0xFFL);
    return h;
  }

  /** 36 ASCII characters, the shape of every id in the real schema. */
  public static String guid(String path, int idx) {
    String d = sha256Hex(ascii(path + "#" + idx));
    return d.substring(0, 8) + "-" + d.substring(8, 12) + "-" + d.substring(12, 16) + "-"
        + d.substring(16, 20) + "-" + d.substring(20, 32);
  }

  public static String word(String path, int idx) {
    long h = h64(path, idx);
    return VOCAB[(int) Long.remainderUnsigned(h, VOCAB.length)]
        + Long.remainderUnsigned(h, 1000);
  }

  public static String sentence(String path, int idx) {
    long h = h64(path, idx);
    StringBuilder sb = new StringBuilder(40);
    for (int i = 0; i < 5; i++) {
      if (i > 0) sb.append(' ');
      sb.append(VOCAB[(int) ((h >>> (4 * i)) & 15)]);
    }
    return sb.toString();
  }

  public static byte[] blob(String path, int idx, int n) {
    byte[] out = new byte[n];
    int filled = 0, i = 0;
    while (filled < n) {
      byte[] d = sha256(ascii(path + "#" + idx + "#" + i));
      int take = Math.min(d.length, n - filled);
      System.arraycopy(d, 0, out, filled, take);
      filled += take;
      i++;
    }
    return out;
  }

  private static byte[] bulkCache;
  private static int bulkCacheN = -1;

  /** A multi-megabyte body. Cached: P5.4 is 4 MB and rebuilding it per call would time
   *  the digest rather than the codec. */
  public static synchronized byte[] bulk(int n) {
    if (bulkCacheN == n) return bulkCache;
    byte[] out = new byte[n];
    int filled = 0, i = 0;
    while (filled < n) {
      byte[] d = sha256(ascii("bulk#" + i));
      int take = Math.min(d.length, n - filled);
      System.arraycopy(d, 0, out, filled, take);
      filled += take;
      i++;
    }
    bulkCache = out;
    bulkCacheN = n;
    return out;
  }

  public static int scalarI32(String path, int idx) {
    return (int) Long.remainderUnsigned(h64(path, idx), 100000L);
  }

  public static long scalarI64(String path, int idx) {
    return Long.remainderUnsigned(h64(path, idx), 1000000000000L);
  }

  public static boolean scalarBool(String path, int idx) {
    return (h64(path, idx) & 1L) != 0;
  }

  public static double scalarDouble(String path, int idx) {
    return Long.remainderUnsigned(h64(path, idx), 1000000L) / 1000.0;
  }

  /** Cycles the declared values so that a payload with enough elements reaches the large
   *  one: ResultStatus 127 is a two-byte varint. */
  public static int enumValue(int[] declared, int idx) {
    int i = idx % declared.length;
    if (i < 0) i += declared.length;
    return declared[i];
  }

  public static long stampSeconds(int idx) { return 1700000000L + (long) idx * 37L; }
  public static int stampNanos(int idx) { return (int) (((long) idx * 7919L) % 1000000000L); }
  public static long durSeconds(int idx) { return idx % 3600; }
  public static int durNanos(int idx) { return (int) (((long) idx * 104729L) % 1000000000L); }

  /**
   * The three content sets of design/SHAPES.md, one code point per input character.
   *
   * <p>What this prices differs by host and a column must not be read across: on the JVM
   * the facade holds UTF-16, so LATIN1 and WIDE make a narrowing transcoder do real work,
   * where a Rust `String` is already UTF-8 and only its width changes. WIDE also stays
   * inside the BMP on purpose -- one `char` per code point -- so what it measures is the
   * transcoder and not surrogate pairing, which is a separate question and is the one the
   * corpus's transcode pair (README 10.4) is about.
   */
  public static String recode(String s, int cs) {
    if (cs == ASCII) return s;
    int n = s.length();
    char[] out = new char[n];
    for (int i = 0; i < n; i++) {
      int c = s.charAt(i) & 0xFF;
      out[i] = cs == LATIN1 ? (char) (0xA0 + (c % 0x60)) : (char) (0x4E00 + (c % 0x1000));
    }
    return new String(out);
  }
}
