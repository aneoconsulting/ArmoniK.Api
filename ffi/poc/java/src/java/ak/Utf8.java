package ak;

/**
 * UTF-8 in and out of a {@code java.lang.String}, shared by the no-boundary control (arm
 * {@code R}) and by the host-transcoder arm of the binding.
 *
 * <p>Shared on purpose. If the two arms decoded strings differently, the delta between
 * them would be a measurement of two string decoders wearing an architecture's name, and
 * that is the exact class of defect the cpp slice's review found twice. protobuf-java has
 * its own and keeps it; that comparison is codec against codec, which is what the
 * incumbent column is for.
 *
 * <p><b>Decode rejects.</b> ABI v1 open decision 3 is settled that way, proto3 requires a
 * parser to validate, and protobuf-java's generated proto3 code calls
 * {@code readStringRequireUtf8}, so rejecting is also what makes the two columns the same
 * work. {@code new String(bytes, UTF_8)} substitutes U+FFFD silently and would have been
 * the obvious thing to write.
 *
 * <p>Java 8 clean.
 */
public final class Utf8 {
  private Utf8() {}

  /** Thrown rather than returned so a decode cannot continue past bad input by accident. */
  public static final class Malformed extends RuntimeException {
    private static final long serialVersionUID = 1L;
    public Malformed(String m) { super(m); }
  }

  private static final ThreadLocal<char[]> SCRATCH = new ThreadLocal<char[]>() {
    @Override protected char[] initialValue() { return new char[256]; }
  };

  private static char[] scratch(int n) {
    char[] c = SCRATCH.get();
    if (c.length < n) {
      c = new char[Integer.highestOneBit(n - 1) * 2];
      SCRATCH.set(c);
    }
    return c;
  }

  /**
   * Decode `len` bytes at `off`, rejecting malformed input.
   *
   * <p>The ASCII fast path is not an optimisation for its own sake: every id, session id,
   * task id and result id in the real schema is an ASCII GUID (design/SHAPES.md), so it is
   * the path the measured payloads take, and a decoder without it would measure a branch
   * mispredict per character against an incumbent that has one.
   */
  public static String decode(byte[] b, int off, int len) {
    if (len == 0) return "";
    int i = off, end = off + len;
    while (i < end && b[i] >= 0) i++;
    if (i == end) {
      // Pure ASCII. The (byte[], int, int, String) constructor is the JDK's own fused
      // Latin-1 path and allocates the String's array once.
      return newAscii(b, off, len);
    }
    char[] c = scratch(len);
    int n = 0;
    for (int k = off; k < off + (i - off); k++) c[n++] = (char) b[k];
    i = off + (i - off);
    while (i < end) {
      int b0 = b[i++] & 0xFF;
      if (b0 < 0x80) {
        c[n++] = (char) b0;
      } else if (b0 < 0xC2) {
        throw new Malformed("continuation or overlong lead byte");
      } else if (b0 < 0xE0) {
        if (i >= end) throw new Malformed("truncated 2-byte sequence");
        int b1 = b[i++] & 0xFF;
        if ((b1 & 0xC0) != 0x80) throw new Malformed("bad continuation");
        c[n++] = (char) (((b0 & 0x1F) << 6) | (b1 & 0x3F));
      } else if (b0 < 0xF0) {
        if (i + 1 >= end) throw new Malformed("truncated 3-byte sequence");
        int b1 = b[i++] & 0xFF, b2 = b[i++] & 0xFF;
        if ((b1 & 0xC0) != 0x80 || (b2 & 0xC0) != 0x80) throw new Malformed("bad continuation");
        int cp = ((b0 & 0x0F) << 12) | ((b1 & 0x3F) << 6) | (b2 & 0x3F);
        if (cp < 0x800) throw new Malformed("overlong 3-byte sequence");
        if (cp >= 0xD800 && cp <= 0xDFFF) throw new Malformed("encoded surrogate");
        c[n++] = (char) cp;
      } else if (b0 < 0xF5) {
        if (i + 2 >= end) throw new Malformed("truncated 4-byte sequence");
        int b1 = b[i++] & 0xFF, b2 = b[i++] & 0xFF, b3 = b[i++] & 0xFF;
        if ((b1 & 0xC0) != 0x80 || (b2 & 0xC0) != 0x80 || (b3 & 0xC0) != 0x80)
          throw new Malformed("bad continuation");
        int cp = ((b0 & 0x07) << 18) | ((b1 & 0x3F) << 12) | ((b2 & 0x3F) << 6) | (b3 & 0x3F);
        if (cp < 0x10000 || cp > 0x10FFFF) throw new Malformed("out of range 4-byte sequence");
        cp -= 0x10000;
        c[n++] = (char) (0xD800 | (cp >> 10));
        c[n++] = (char) (0xDC00 | (cp & 0x3FF));
      } else {
        throw new Malformed("lead byte above F4");
      }
    }
    return new String(c, 0, n);
  }

  @SuppressWarnings("deprecation")
  private static String newAscii(byte[] b, int off, int len) {
    // On JDK 9+ a String whose bytes are all < 0x80 is stored LATIN1, and this constructor
    // is the one-copy path to it. On Java 8 it is also one copy, into a char[].
    return new String(b, 0, off, len);
  }

  /** How many UTF-8 bytes `s` needs. Unpaired surrogates count as 3, matching what
   *  {@link #encode} writes for them (U+FFFD). */
  public static int length(String s) {
    int n = s.length(), out = 0;
    for (int i = 0; i < n; i++) {
      char c = s.charAt(i);
      if (c < 0x80) out += 1;
      else if (c < 0x800) out += 2;
      else if (Character.isHighSurrogate(c) && i + 1 < n
               && Character.isLowSurrogate(s.charAt(i + 1))) { out += 4; i++; }
      else out += 3;
    }
    return out;
  }

  /**
   * Encode into `dst` at `at`, returning the new position.
   *
   * <p>An unpaired surrogate becomes U+FFFD. That is <b>not</b> what protobuf-java does --
   * it writes `?` -- and the divergence is real, silent and named in ABI v1 section 7 as a
   * migration note rather than a defect. It is unreachable from the manifest's content
   * sets, all three of which are well-formed, so it changes no measured byte here; it is
   * written down because README section 10 item 4 makes C# and Java the two slices that
   * have to run the transcode pair, and this is the Java half of the disagreement.
   */
  public static int encode(String s, byte[] dst, int at) {
    int n = s.length(), p = at;
    for (int i = 0; i < n; i++) {
      char c = s.charAt(i);
      if (c < 0x80) {
        dst[p++] = (byte) c;
      } else if (c < 0x800) {
        dst[p++] = (byte) (0xC0 | (c >> 6));
        dst[p++] = (byte) (0x80 | (c & 0x3F));
      } else if (Character.isHighSurrogate(c) && i + 1 < n
                 && Character.isLowSurrogate(s.charAt(i + 1))) {
        int cp = Character.toCodePoint(c, s.charAt(++i));
        dst[p++] = (byte) (0xF0 | (cp >> 18));
        dst[p++] = (byte) (0x80 | ((cp >> 12) & 0x3F));
        dst[p++] = (byte) (0x80 | ((cp >> 6) & 0x3F));
        dst[p++] = (byte) (0x80 | (cp & 0x3F));
      } else {
        int cp = (c >= 0xD800 && c <= 0xDFFF) ? 0xFFFD : c;
        dst[p++] = (byte) (0xE0 | (cp >> 12));
        dst[p++] = (byte) (0x80 | ((cp >> 6) & 0x3F));
        dst[p++] = (byte) (0x80 | (cp & 0x3F));
      }
    }
    return p;
  }
}
