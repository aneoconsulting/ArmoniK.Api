package ak;

/**
 * A borrowed view over UTF-8 bytes the host already holds: ABI v1 open decision 13's
 * measurement arm, on the JVM.
 *
 * <p><b>Why this needs no ABI change.</b> {@code ak_span} is already an offset into the
 * buffer the host handed in (section 4), and 7.4 already tells a facade author to resolve
 * it against a base pointer it holds. So the only thing that changes is the facade's own
 * promise about the string, which is exactly what decision 13 says is open.
 *
 * <p><b>What it isolates.</b> The copy, and nothing else. The UTF-8 policy is unchanged --
 * the bytes are still validated, and still rejected when malformed -- so a delta against
 * the owning facade is the cost of building a {@code java.lang.String}, which on the JVM
 * is an allocation plus a transcode into the String's own array. Lists, maps and message
 * children are still constructed, so the residual is those.
 *
 * <p><b>Why it is not a design.</b> The view is valid only while the input buffer lives,
 * and this class cannot enforce that: it holds the array and an offset, and nothing stops
 * a caller keeping it past the next decode into the same buffer. That lifetime contract is
 * the substance of decision 13 and the branch has never written it down. The C++ slice
 * said the same thing about its own arm and it is worth repeating rather than softening.
 *
 * <p>The hash and equality are over the BYTES, so a view and another view of the same
 * content agree; {@code TreeMap} needs the ordering too, and it is the byte ordering,
 * which for UTF-8 is also the code-point ordering.
 */
public final class Utf8View implements Comparable<Utf8View>, CharSequence {
  public static final Utf8View EMPTY = new Utf8View(new byte[0], 0, 0);

  public final byte[] buf;
  public final int off;
  public final int len;

  private Utf8View(byte[] b, int off, int len) {
    this.buf = b;
    this.off = off;
    this.len = len;
  }

  /** Validate and borrow. Rejecting is ABI v1 decision 3 and it is what protobuf-java's
   *  generated proto3 code does, so the two columns stay the same work; what this arm
   *  removes is the COPY, never the check. */
  public static Utf8View of(byte[] b, int off, int len) {
    if (len == 0) return EMPTY;
    if (!valid(b, off, len)) throw new Utf8.Malformed("malformed UTF-8 in a decoded string");
    return new Utf8View(b, off, len);
  }

  /** Borrow a String's UTF-8 form. Used only where a builder hands the borrowed facade a
   *  value it constructed in Java, which is the encode direction; the encode column of
   *  this arm is therefore not quoted. */
  public static Utf8View of(String s) {
    byte[] b = new byte[Utf8.length(s)];
    Utf8.encode(s, b, 0);
    return new Utf8View(b, 0, b.length);
  }

  static boolean valid(byte[] b, int off, int len) {
    int i = off, end = off + len;
    while (i < end && b[i] >= 0) i++;
    if (i == end) return true;
    while (i < end) {
      int b0 = b[i++] & 0xFF;
      if (b0 < 0xC2) return false;
      if (b0 < 0xE0) {
        if (i >= end || (b[i++] & 0xC0) != 0x80) return false;
      } else if (b0 < 0xF0) {
        if (i + 1 >= end) return false;
        int b1 = b[i++] & 0xFF, b2 = b[i++] & 0xFF;
        if ((b1 & 0xC0) != 0x80 || (b2 & 0xC0) != 0x80) return false;
        int cp = ((b0 & 0x0F) << 12) | ((b1 & 0x3F) << 6) | (b2 & 0x3F);
        if (cp < 0x800 || (cp >= 0xD800 && cp <= 0xDFFF)) return false;
      } else if (b0 < 0xF5) {
        if (i + 2 >= end) return false;
        int b1 = b[i++] & 0xFF, b2 = b[i++] & 0xFF, b3 = b[i++] & 0xFF;
        if ((b1 & 0xC0) != 0x80 || (b2 & 0xC0) != 0x80 || (b3 & 0xC0) != 0x80) return false;
        int cp = ((b0 & 0x07) << 18) | ((b1 & 0x3F) << 12) | ((b2 & 0x3F) << 6) | (b3 & 0x3F);
        if (cp < 0x10000 || cp > 0x10FFFF) return false;
      } else {
        return false;
      }
    }
    return true;
  }

  /** Materialise. The whole point of the arm is that a facade consumer may never call
   *  this; the correctness gate does, which is how the borrowed arm is checked against
   *  the same manifest as every other. */
  public String str() { return Utf8.decode(buf, off, len); }

  public int byteLength() { return len; }

  @Override public int length() { return str().length(); }
  @Override public char charAt(int i) { return str().charAt(i); }
  @Override public CharSequence subSequence(int a, int b) { return str().subSequence(a, b); }
  @Override public String toString() { return str(); }

  @Override public boolean equals(Object o) {
    if (!(o instanceof Utf8View)) return false;
    Utf8View v = (Utf8View) o;
    if (v.len != len) return false;
    for (int i = 0; i < len; i++) if (buf[off + i] != v.buf[v.off + i]) return false;
    return true;
  }

  @Override public int hashCode() {
    int h = 1;
    for (int i = 0; i < len; i++) h = 31 * h + buf[off + i];
    return h;
  }

  /** Byte order, which for UTF-8 is also code-point order -- and therefore the order the
   *  canonical form's "map entries are sorted by key" means. */
  @Override public int compareTo(Utf8View o) {
    int n = Math.min(len, o.len);
    for (int i = 0; i < n; i++) {
      int a = buf[off + i] & 0xFF, b = o.buf[o.off + i] & 0xFF;
      if (a != b) return a - b;
    }
    return len - o.len;
  }
}
