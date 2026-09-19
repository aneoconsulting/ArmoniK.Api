package ak;

/**
 * The encode buffer, the varints and the learned length-placeholder width -- the Java
 * transliteration of the core's {@code ak-rt/src/enc.rs}.
 *
 * <p>Deliberately the same algorithm, not a better one. Arm {@code R} exists to price the
 * boundary and nothing else (README R3), so if it sized its length prefixes differently
 * from the core the delta between them would be a prefix-strategy result wearing an
 * architecture's name. ABI v1 section 6 fixes the strategy: a learned width per site, held
 * per context, never padded, and a miss moves the body.
 *
 * <p>The same reasoning governs the buffer. The core reuses a warm {@code Vec} across
 * encodes; this reuses a warm {@code byte[]} across encodes and grows by doubling. An arm
 * that allocated per call would be measuring an allocator -- the rust slice published a
 * 1.412 that turned out to be exactly that, and it became 1.018 when fixed.
 */
public final class Enc {
  public byte[] buf;
  public int len;
  /** One learned width per length-prefix site. Per instance: ABI v1 section 6 records
   *  that a process-global table made two encoding threads slower than one. */
  private final byte[] widths;
  public int err;

  /** Counting build only (README R5). Never read on the timed path. */
  public long prefixMoves, prefixBytes;
  public boolean counting;

  public Enc(int sites) {
    this.buf = new byte[4096];
    this.widths = new byte[sites];
    java.util.Arrays.fill(this.widths, (byte) 1);
  }

  public void reset() {
    len = 0;
    err = 0;
    // The learned widths deliberately SURVIVE a reset: that is what makes them learned.
  }

  public byte[] toBytes() {
    byte[] out = new byte[len];
    System.arraycopy(buf, 0, out, 0, len);
    return out;
  }

  public void ensure(int extra) {
    if (len + extra > buf.length) grow(len + extra);
  }

  private void grow(int need) {
    int n = buf.length;
    while (n < need) n <<= 1;
    byte[] nb = new byte[n];
    System.arraycopy(buf, 0, nb, 0, len);
    buf = nb;
  }

  public void varint(long v) {
    ensure(10);
    byte[] b = buf;
    int p = len;
    while ((v & ~0x7FL) != 0) {
      b[p++] = (byte) (((int) v) | 0x80);
      v >>>= 7;
    }
    b[p++] = (byte) v;
    len = p;
  }

  public void key(int tag, int wire) { varint(((long) tag << 3) | wire); }

  public void varintField(int tag, long v) { key(tag, 0); varint(v); }

  public void f64Field(int tag, double v) {
    key(tag, 1);
    ensure(8);
    long bits = Double.doubleToRawLongBits(v);
    for (int i = 0; i < 8; i++) buf[len++] = (byte) (bits >>> (8 * i));
  }

  /** A blob whose length is known before the body is written, which is every blob the
   *  host hands over by value. */
  public void blobField(int tag, byte[] b) {
    key(tag, 2);
    varint(b.length);
    ensure(b.length);
    System.arraycopy(b, 0, buf, len, b.length);
    len += b.length;
  }

  /** A `string` field: the length is not known until it is encoded, so this is the
   *  two-pass write ABI v1 section 4's note is about, done the cheap way -- the host
   *  holds the bytes and can measure them. */
  public void stringField(int tag, String s) {
    key(tag, 2);
    int n = Utf8.length(s);
    varint(n);
    ensure(n);
    len = Utf8.encode(s, buf, len);
  }

  public static int varintLen(long v) {
    int n = 1;
    while ((v & ~0x7FL) != 0) { v >>>= 7; n++; }
    return n;
  }

  /** Open a length-delimited field whose body length is not yet known. Returns a mark
   *  packed as (site &lt;&lt; 40) | (width &lt;&lt; 32) | header position. */
  public long begin(int tag, int site) {
    key(tag, 2);
    int w = widths[site];
    int hdr = len;
    ensure(w);
    for (int i = 0; i < w; i++) buf[len++] = 0;
    return ((long) site << 40) | ((long) w << 32) | (hdr & 0xFFFFFFFFL);
  }

  /** Close it, resolving the prefix width now that the body is written. Never padded:
   *  padding to a learned width makes the encoder's output depend on its own history,
   *  which ABI v1 section 6 refuses outright. */
  public void end(long mark) {
    int hdr = (int) (mark & 0xFFFFFFFFL);
    int w = (int) ((mark >>> 32) & 0xFF);
    int site = (int) (mark >>> 40);
    int body = len - hdr - w;
    int need = varintLen(body);
    if (need != w) {
      resizePrefix(site, hdr, w, body, need);
      w = need;
    }
    long v = body;
    int i = hdr;
    while ((v & ~0x7FL) != 0) { buf[i++] = (byte) (((int) v) | 0x80); v >>>= 7; }
    buf[i] = (byte) v;
  }

  private void resizePrefix(int site, int hdr, int w, int body, int need) {
    if (counting) { prefixMoves++; prefixBytes += body; }
    widths[site] = (byte) need;
    int src = hdr + w;
    if (need > w) ensure(need - w);
    System.arraycopy(buf, src, buf, hdr + need, body);
    len = hdr + need + body;
  }
}
