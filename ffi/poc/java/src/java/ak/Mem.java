package ak;

import java.lang.reflect.Field;

/**
 * Off-heap memory, through {@code sun.misc.Unsafe}.
 *
 * <p><b>Why Unsafe and not a direct ByteBuffer.</b> A by-value ABI group is a C struct, and
 * a Java binding fills it by writing scalars at computed offsets. A direct
 * {@code ByteBuffer} can do that, with a bounds check and an {@code Buffer} object per
 * region; {@code Unsafe} does it with the store alone. The choice matters because it is
 * the thing being measured: protobuf-java itself reaches for {@code Unsafe} on exactly
 * these paths, so an arm that used the slower mechanism would be reporting a handicap it
 * chose as an ABI cost. It is also identical on Java 8 and JDK 17, so it is one of the few
 * places the floor and the target do NOT diverge.
 *
 * <p><b>What it costs to say that.</b> {@code sun.misc.Unsafe} lives in the
 * {@code jdk.unsupported} module and the {@code theUnsafe} handle below works unchanged
 * from Java 8 through 21. It is terminally deprecated from JDK 23 and its memory-access
 * methods warn at run time from JDK 24, whose replacement is FFM -- a JDK 22 API, which is
 * above this slice's target. So the mechanism a Java binding would use today is on a
 * removal path and its successor is above the floor. That is a packaging finding
 * (README 5.1.3 and open question 6), not a benchmark result, and STATE.md carries it.
 */
public final class Mem {
  private Mem() {}

  public static final sun.misc.Unsafe U = unsafe();
  public static final long BYTE_BASE = U.arrayBaseOffset(byte[].class);
  public static final long CHAR_BASE = U.arrayBaseOffset(char[].class);
  public static final long INT_BASE = U.arrayBaseOffset(int[].class);
  public static final long LONG_BASE = U.arrayBaseOffset(long[].class);
  public static final long DOUBLE_BASE = U.arrayBaseOffset(double[].class);
  public static final long BOOL_BASE = U.arrayBaseOffset(boolean[].class);
  public static final long BOOL_SCALE = U.arrayIndexScale(boolean[].class);

  private static sun.misc.Unsafe unsafe() {
    try {
      Field f = sun.misc.Unsafe.class.getDeclaredField("theUnsafe");
      f.setAccessible(true);
      return (sun.misc.Unsafe) f.get(null);
    } catch (Exception e) {
      throw new IllegalStateException(
          "sun.misc.Unsafe is unreachable; this binding writes C structs by offset and has"
          + " no fallback that is not a different measurement", e);
    }
  }

  public static long alloc(long n) {
    long p = U.allocateMemory(n);
    U.setMemory(p, n, (byte) 0);
    return p;
  }

  public static void free(long p) { U.freeMemory(p); }

  public static void zero(long p, long n) { U.setMemory(p, n, (byte) 0); }

  public static void copyFromBytes(byte[] src, int off, long dst, int n) {
    U.copyMemory(src, BYTE_BASE + off, null, dst, n);
  }

  public static void copyFromChars(char[] src, int off, long dst, int n) {
    U.copyMemory(src, CHAR_BASE + (long) off * 2, null, dst, (long) n * 2);
  }

  public static void copyToBytes(long src, byte[] dst, int off, int n) {
    U.copyMemory(null, src, dst, BYTE_BASE + off, n);
  }

  /** `boolean[]` is one byte per element in HotSpot, which is what `uint8_t` in the group
   *  needs, so a packed `bool` field is the host's own array and crosses without a
   *  materialising copy. Asserted rather than assumed: see {@link #checkBoolScale}. */
  public static void checkBoolScale() {
    if (BOOL_SCALE != 1)
      throw new IllegalStateException(
          "boolean[] has scale " + BOOL_SCALE + ", so a packed bool field is not the"
          + " host's own array on this VM and ABI v1 section 6's 'hands over a pointer and"
          + " copies nothing' does not hold here");
  }
}
