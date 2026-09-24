package ak;

/**
 * A protobuf reader over a {@code byte[]} -- the decode primitives of arm {@code R}'s runtime.
 *
 * <p>Primitives only. Which (field number, wire type) pairs a field accepts, merge versus
 * replace, the unknown-field behaviour, tag 0, the depth limit and the UTF-8 policy are the
 * PLAN's ({@code poc/codec/gen/plan.py}) and are rendered into the generated codec by
 * {@code poc/codec/gen/java_rcodec.py}; this class reads varints, lengths and fixed-width
 * values and skips an unknown field, each exactly as {@code ak-rt/src/dec.rs} does for the
 * core, so arm R and the core cannot disagree on a primitive.
 *
 * <p><b>R-G8 / R-D1, the length rule.</b> A length prefix is read as the full unsigned
 * 64-bit varint -- never narrowed to {@code int} before the check -- and accepted only if
 * {@code n <= limit - pos} (unsigned), never tested as {@code pos + n > limit}, which wraps.
 * The pre-WP5 {@code readLen} cast to {@code int} first and checked {@code pos + n > limit}
 * in {@code int}: a length of 2^31 - 1 wrapped the check and was refused only later by the
 * submessage-consumed check ({@code logs/java/re4-corpus-armR.log}).
 *
 * <p>Errors are exceptions carrying the ABI's code, so a decode stops at the first one and
 * nothing after it is delivered (plan rule, R-G6): the generated decoder returns no object.
 */
public final class Dec {
  public static final int ERR_MALFORMED = -2;
  public static final int ERR_TRUNCATED = -3;
  public static final int ERR_DEPTH = -4;
  public static final int ERR_TRANSCODE = -6;

  public byte[] b;
  public int pos;
  public int limit;

  public static final class Malformed extends RuntimeException {
    private static final long serialVersionUID = 1L;
    public final int code;
    public Malformed(String m) { this(ERR_MALFORMED, m); }
    public Malformed(int code, String m) { super(m); this.code = code; }
  }

  public static Malformed err(int code, String m) { return new Malformed(code, m); }

  public Dec reset(byte[] buf, int off, int len) {
    this.b = buf;
    this.pos = off;
    this.limit = off + len;
    return this;
  }

  public boolean done() { return pos >= limit; }

  public long readVarint() {
    long out = 0;
    int shift = 0;
    while (true) {
      if (pos >= limit) throw new Malformed(ERR_TRUNCATED, "truncated varint");
      int x = b[pos++];
      out |= ((long) (x & 0x7F)) << shift;
      if (x >= 0) return out;
      shift += 7;
      if (shift > 63) throw new Malformed(ERR_MALFORMED, "varint longer than 10 bytes");
    }
  }

  public double readDouble() {
    if (limit - pos < 8) throw new Malformed(ERR_TRUNCATED, "truncated fixed64");
    long v = 0;
    for (int i = 7; i >= 0; i--) v = (v << 8) | (b[pos + i] & 0xFFL);
    pos += 8;
    return Double.longBitsToDouble(v);
  }

  public int readFixed32() {
    if (limit - pos < 4) throw new Malformed(ERR_TRUNCATED, "truncated fixed32");
    int v = (b[pos] & 0xFF) | (b[pos + 1] & 0xFF) << 8 | (b[pos + 2] & 0xFF) << 16
        | (b[pos + 3] & 0xFF) << 24;
    pos += 4;
    return v;
  }

  /** R-G8: 64-bit, unsigned, against the REMAINING bytes. */
  public int readLen() {
    long n = readVarint();
    if (Long.compareUnsigned(n, (long) (limit - pos)) > 0)
      throw new Malformed(ERR_TRUNCATED, "length past end of buffer");
    return (int) n;
  }

  public String readString() {
    int n = readLen();
    String s;
    try {
      s = Utf8.decode(b, pos, n);
    } catch (Utf8.Malformed e) {
      throw new Malformed(ERR_TRANSCODE, e.getMessage());
    }
    pos += n;
    return s;
  }

  public byte[] readBytes() {
    int n = readLen();
    byte[] out = new byte[n];
    System.arraycopy(b, pos, out, 0, n);
    pos += n;
    return out;
  }

  /** Enter a length-delimited body. Returns the outer limit to restore. */
  public int push() {
    int n = readLen();
    int old = limit;
    limit = pos + n;
    return old;
  }

  public void pop(int old) {
    if (pos != limit) throw new Malformed(ERR_MALFORMED, "body not fully consumed");
    limit = old;
  }

  /** The bytes from {@code start} to the current position, for a retain-mode capture. */
  public static byte[] append(byte[] bag, byte[] src, int start, int end) {
    int n = end - start;
    if (bag == null) {
      byte[] out = new byte[n];
      System.arraycopy(src, start, out, 0, n);
      return out;
    }
    byte[] out = java.util.Arrays.copyOf(bag, bag.length + n);
    System.arraycopy(src, start, out, bag.length, n);
    return out;
  }

  // ---- the unknown-field skip, as ak-rt/src/dec.rs `skip` and `skip_group` ------------
  //
  // The two limits are the PLAN's (plan.MAX_FIELD_NUMBER, plan.GROUP_DEPTH_LIMIT), rendered
  // into every generated codec as constants and passed in here, so this runtime holds no
  // limit of its own. D38: before this, a key inside a group was checked for field number 0
  // but not for a number above 2^29 - 1 (probe row P-field-maxplus1-in-group), where the
  // core refuses it; and the depth was a constant of this file.

  /** Skip one field whose key has been read. {@code tag} is the field number the wire
   *  type arrived with: a GROUP carries no length, so its end is the END_GROUP whose field
   *  number MATCHES the one that opened it. */
  public void skip(int tag, int wire, long maxField, int groupDepth) {
    switch (wire) {
      case 0: readVarint(); break;
      case 1: pos += 8; break;
      case 2: { int n = readLen(); pos += n; break; }
      case 3: skipGroup(tag, 0, maxField, groupDepth); break;
      case 5: pos += 4; break;
      // 4 is END_GROUP with nothing open; 6 and 7 do not exist.
      default: throw new Malformed(ERR_MALFORMED, "wire type " + wire);
    }
    if (pos > limit) {
      pos = limit;
      throw new Malformed(ERR_TRUNCATED, "truncated field of wire type " + wire);
    }
  }

  private void skipGroup(int tag, int depth, long maxField, int groupDepth) {
    if (depth >= groupDepth) throw new Malformed(ERR_DEPTH, "group nesting past " + groupDepth);
    while (true) {
      if (pos >= limit) throw new Malformed(ERR_TRUNCATED, "unterminated group");
      long k = readVarint();
      long fn = k >>> 3;
      int w = (int) (k & 7);
      if (fn == 0 || Long.compareUnsigned(fn, maxField) > 0)
        throw new Malformed(ERR_MALFORMED, "field number 0 or above 2^29-1 inside a group");
      int t = (int) fn;
      if (w == 4) {
        if (t != tag) throw new Malformed(ERR_MALFORMED, "mismatched end group");
        return;
      }
      if (w == 3) {
        skipGroup(t, depth + 1, maxField, groupDepth);
        continue;
      }
      skip(t, w, maxField, groupDepth);
    }
  }

  // ---- the pre-WP5 key-at-a-time API, kept for the hand-written harnesses -----------
  // (`Triples`, `RunUnknown`), which walk wire they built themselves. Not used by any
  // generated codec.

  /** The next key as an int, or 0 at the end. */
  public int readTag() {
    if (pos >= limit) return 0;
    return (int) readVarint();
  }

  public void skip(int key) {
    skip(key >>> 3, key & 7, ak.shapes.Codec.MAX_FIELD_NUMBER, ak.shapes.Codec.GROUP_DEPTH_LIMIT);
  }
}
