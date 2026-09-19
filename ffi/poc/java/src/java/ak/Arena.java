package ak;

/**
 * A bump allocator over one off-heap block: where a group and its staged string bytes
 * live for the duration of a codec call.
 *
 * <p><b>Instance state, never static.</b> The slice's own brief calls re-entrancy the
 * thing to settle before anything else is trusted, and the rust slice found the matching
 * defect on the RPC side -- shared mutable state that worked at one call in flight and
 * failed at eight. One Binding owns one Arena and two encoding threads share nothing.
 *
 * <p><b>It never moves.</b> An {@code ak_str} in a group holds the ADDRESS of the staged
 * code units, so a realloc between filling the group and the codec reading it would leave
 * every string pointing at freed memory -- and the payload set would still pass, because
 * the old block is usually still mapped. So the block is sized once and a fill that would
 * overflow it flushes the chunk instead of growing: legal by ABI v1 section 6, "the host
 * may switch forms per field and per element mid-stream, because every entry point
 * appends".
 */
public final class Arena {
  public final long base;
  public final long size;
  private long at;

  public Arena(long size) {
    this.size = size;
    this.base = Mem.alloc(size);
    this.at = 0;
  }

  public long mark() { return at; }

  public void release(long m) { at = m; }

  public void reset() { at = 0; }

  public boolean has(long n) { return at + ((n + 7) & ~7L) <= size; }

  public long alloc(long n) {
    long p = base + at;
    at += (n + 7) & ~7L;       // 8-byte aligned: a group holds int64 and double
    if (at > size) throw new IllegalStateException(
        "arena overflow: wanted " + n + " with " + (size - (at - ((n + 7) & ~7L)))
        + " left of " + size + ". The caller must flush a chunk rather than grow, because"
        + " a group already holds pointers into this block.");
    return p;
  }

  /** Unaligned, for staged string bytes: nothing reads them as a word. */
  public long allocRaw(long n) {
    long p = base + at;
    at += n;
    if (at > size) throw new IllegalStateException("arena overflow staging " + n + " B");
    return p;
  }

  public void close() { Mem.free(base); }
}
