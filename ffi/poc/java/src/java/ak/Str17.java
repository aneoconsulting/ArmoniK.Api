package ak;

/**
 * Reaching a {@code String}'s code units without a copy, on JDK 9 and later.
 *
 * <p><b>This is the whole floor-to-target divergence in the Java slice</b>, and README 5.1
 * predicted its shape without knowing what it would be: "Java has no preprocessor, so
 * there it is literally one emitted source tree per target level".
 *
 * <p>From JDK 9 a {@code String} is a {@code byte[] value} plus a {@code byte coder}:
 * LATIN1 when every character fits in one byte, UTF16 otherwise. ABI v1 section 4
 * specifies a transcoder for each -- {@code ak_tc_latin1} and {@code ak_tc_utf16} -- and
 * the binding can therefore hand the core ONE bulk copy of the string's own storage. On
 * Java 8 a {@code String} is a {@code char[]} and there is no coder, so the floor stages
 * through {@code getChars} into a scratch array and always uses UTF-16: two copies where
 * the target does one, and twice the bytes for an ASCII id.
 *
 * <p>Every id in the real schema is an ASCII GUID (design/SHAPES.md), so LATIN1 is the
 * common case and the divergence is not a corner: the target stages 36 bytes where the
 * floor stages 72 and copies them twice.
 *
 * <p><b>It is checked, not assumed.</b> {@code Unsafe.objectFieldOffset} reaches a field
 * whose reflective ACCESS is closed, which is why no {@code --add-opens} is needed, but
 * the layout is a JDK internal and a wrong offset would be a wrong byte. The self-test
 * below encodes a known string through the fast path and through {@code getChars}, and
 * refuses the fast path unless they agree; {@link #available()} then says which the
 * binding is running, and the bench prints it.
 */
public final class Str17 {
  private Str17() {}

  public static final byte LATIN1 = 0, UTF16 = 1;

  private static final long VALUE_OFF;
  private static final long CODER_OFF;
  private static final boolean OK;

  static {
    long v = -1, c = -1;
    boolean ok = false;
    try {
      v = Mem.U.objectFieldOffset(String.class.getDeclaredField("value"));
      c = Mem.U.objectFieldOffset(String.class.getDeclaredField("coder"));
      ok = selfTest(v, c);
    } catch (Throwable t) {
      ok = false;
    }
    VALUE_OFF = v;
    CODER_OFF = c;
    OK = ok;
  }

  /** True on JDK 9+ where the compact-string fields were found AND agree with a
   *  {@code getChars} staging of the same input. */
  public static boolean available() { return OK; }

  public static byte coder(String s) { return Mem.U.getByte(s, CODER_OFF); }

  public static byte[] value(String s) { return (byte[]) Mem.U.getObject(s, VALUE_OFF); }

  private static boolean selfTest(long vo, long co) {
    // One ASCII string, one that needs Latin-1 but not UTF-16, and one above U+00FF: the
    // three content sets of design/SHAPES.md, which is also the three cases the coder can
    // take. A JDK that stored something else would disagree on at least one.
    // Built from code points rather than written as literals: the source stays
    // pure ASCII, so no javac -encoding and no editor can change what is probed.
    String[] probes = {"", "abc-0123",
                       new String(new char[] {0x00E9, 0x00FF}),          // Latin-1
                       new String(new char[] {0x4E2D, 0x6587}),          // above U+00FF
                       new String(new char[] {'a', 0x4E2D, 'b'})};      // mixed
    for (String s : probes) {
      byte coder = Mem.U.getByte(s, co);
      byte[] val = (byte[]) Mem.U.getObject(s, vo);
      if (val == null) return false;
      if (coder != LATIN1 && coder != UTF16) return false;
      if (val.length != s.length() * (coder == LATIN1 ? 1 : 2)) return false;
      for (int i = 0; i < s.length(); i++) {
        int ch = coder == LATIN1 ? (val[i] & 0xFF)
                                 : ((val[2 * i] & 0xFF) | ((val[2 * i + 1] & 0xFF) << 8));
        if (ch != s.charAt(i)) return false;
      }
    }
    return true;
  }
}
