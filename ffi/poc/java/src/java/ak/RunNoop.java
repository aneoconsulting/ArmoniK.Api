package ak;

/**
 * A crossing and nothing else, through the shim every arm uses.
 *
 * <p>"When a change does not do what it should, the first hypothesis is that it is not
 * running." If this measures zero the arms are not crossing anything and every figure
 * taken from them is a figure about the optimiser. The JVM cannot inline across JNI, so
 * this is a control rather than a hazard here -- but the rust slice's FFI arm turned out
 * to be the no-boundary control with extra struct copies, and its counters said otherwise
 * the whole time.
 */
public final class RunNoop {
  static long sink;

  public static void main(String[] args) {
    Native.ensureBound();
    for (int w = 0; w < 2_000_000; w++) sink += Native.noop(w);
    final int N = 8_000_000;
    long[] ps = new long[9];
    for (int r = 0; r < ps.length; r++) {
      long t0 = System.nanoTime();
      long s = 0;
      for (int i = 0; i < N; i++) s += Native.noop(i);
      ps[r] = (System.nanoTime() - t0) * 1000L / N;
      sink += s;
    }
    java.util.Arrays.sort(ps);
    StringBuilder sb = new StringBuilder();
    sb.append("  ak_noop through the slice's own shim: min ")
      .append(Bench.fmt(ps[0] / 1000.0, 3)).append(" med ")
      .append(Bench.fmt(ps[ps.length / 2] / 1000.0, 3)).append(" max ")
      .append(Bench.fmt(ps[ps.length - 1] / 1000.0, 3)).append(" ns  (sink=")
      .append(sink).append(")\n");
    System.out.print(sb);
  }
}
