package ak;

/**
 * A protobuf reader over a {@code byte[]} -- the decode half of arm {@code R}'s runtime.
 *
 * <p>No streaming, no chunking, no input-stream abstraction: every payload in
 * design/SHAPES.md is one contiguous buffer, and an abstraction the incumbent does not
 * have to pay for would be this arm's own handicap.
 */
public final class Dec {
  public byte[] b;
  public int pos;
  public int limit;

  public static final class Malformed extends RuntimeException {
    private static final long serialVersionUID = 1L;
    public Malformed(String m) { super(m); }
  }

  public Dec reset(byte[] buf, int off, int len) {
    this.b = buf;
    this.pos = off;
    this.limit = off + len;
    return this;
  }

  public boolean done() { return pos >= limit; }

  public int readTag() {
    if (pos >= limit) return 0;
    return (int) readVarint();
  }

  public long readVarint() {
    long out = 0;
    int shift = 0;
    while (true) {
      if (pos >= limit) throw new Malformed("truncated varint");
      int x = b[pos++];
      out |= ((long) (x & 0x7F)) << shift;
      if (x >= 0) return out;
      shift += 7;
      if (shift > 63) throw new Malformed("varint longer than 10 bytes");
    }
  }

  public int readVarint32() { return (int) readVarint(); }

  public double readDouble() {
    if (pos + 8 > limit) throw new Malformed("truncated double");
    long v = 0;
    for (int i = 7; i >= 0; i--) v = (v << 8) | (b[pos + i] & 0xFFL);
    pos += 8;
    return Double.longBitsToDouble(v);
  }

  public int readLen() {
    int n = (int) readVarint();
    if (n < 0 || pos + n > limit) throw new Malformed("length past end of buffer");
    return n;
  }

  public String readString() {
    int n = readLen();
    String s = Utf8.decode(b, pos, n);
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

  /** Enter a length-delimited submessage. Returns the outer limit to restore. */
  public int push() {
    int n = readLen();
    int old = limit;
    limit = pos + n;
    return old;
  }

  public void pop(int old) {
    if (pos != limit) throw new Malformed("submessage body not fully consumed");
    limit = old;
  }

  /**
   * Skip a field the reader does not know.
   *
   * <p>This is the whole of protobuf's forward compatibility and a corpus generated from
   * the schema that reads it never executes it (README section 10 item 1), so the unknown
   * -field vectors run against exactly this method. Nothing is retained: ABI v1 open
   * decision 11 records that as a behaviour change from protobuf-java, which does retain,
   * and this arm follows the core rather than the incumbent on purpose -- the two columns
   * would otherwise not be the same work.
   */
  /** The core bounds group nesting at 100 and returns `AK_ERR_DEPTH`; this decoder
   *  recursed without a bound, so a payload of start tags was a StackOverflowError
   *  rather than a refusal. Caught by reading the core's D7 fix against this file, and
   *  the corpus has the two vectors for it (`X-depth-101`, `X-depth-300`). The JVM makes
   *  the unbounded form survivable where a native core makes it a crash inside the host's
   *  process, which is ABI v1 open decision 7 -- but "survivable" is not "refused". */
  static final int MAX_GROUP_DEPTH = 100;

  public void skip(int tag) { skip(tag, 0); }

  private void skip(int tag, int depth) {
    switch (tag & 7) {
      case 0: readVarint(); break;
      case 1: if (pos + 8 > limit) throw new Malformed("truncated i64"); pos += 8; break;
      // `pos += readLen()` would be WRONG and was: Java evaluates the left
      // operand of a compound assignment FIRST, so the varint readLen
      // consumed is discarded and the skip lands inside the body. It only
      // shows on an unknown LEN field, which no payload generated from the
      // schema that reads it contains -- README section 10 item 1 exactly.
      case 2: { int n = readLen(); pos += n; break; }
      case 5: if (pos + 4 > limit) throw new Malformed("truncated i32"); pos += 4; break;
      case 3: {  // a start group: legal wire, and the schema has none, so refuse loudly
        if (depth >= MAX_GROUP_DEPTH) throw new Malformed("group nesting past 100");
        int field = tag >>> 3;
        while (true) {
          int t = readTag();
          if (t == 0) throw new Malformed("unterminated group");
          if ((t & 7) == 4) {
            // The END_GROUP's field number must MATCH the one that opened it. Counting
            // depth instead accepts a mismatched end and mis-nests every group after it,
            // which is what the core's D7 fix says and what this already did.
            if ((t >>> 3) != field) throw new Malformed("mismatched end group");
            break;
          }
          skip(t, depth + 1);
        }
        break;
      }
      default: throw new Malformed("wire type " + (tag & 7));
    }
  }
}
