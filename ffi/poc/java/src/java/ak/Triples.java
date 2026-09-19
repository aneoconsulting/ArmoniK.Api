package ak;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

/**
 * P7.1's oracle. `DualResponse` interleaves two repeated fields of the same message type,
 * which is legal wire that no canonical writer produces: a writer emitting each field
 * contiguously cannot interleave two of them, and that is the entire point of the control.
 *
 * <p>design/SHAPES.md therefore asks a slice to validate it by decoding it and re-encoding
 * contiguously to <b>a permutation of the same (tag, wire type, body) triples</b>, and
 * never by reproducing its bytes.
 */
public final class Triples {
  private Triples() {}

  public static boolean samePermutation(byte[] a, byte[] b) {
    List<String> x = scan(a), y = scan(b);
    if (x.size() != y.size()) return false;
    Collections.sort(x);
    Collections.sort(y);
    return x.equals(y);
  }

  private static List<String> scan(byte[] b) {
    List<String> out = new ArrayList<String>();
    Dec d = new Dec().reset(b, 0, b.length);
    while (!d.done()) {
      int t = d.readTag();
      int start = d.pos;
      d.skip(t);
      out.add(t + ":" + Payloads.hex(java.util.Arrays.copyOfRange(b, start, d.pos)));
    }
    return out;
  }
}
