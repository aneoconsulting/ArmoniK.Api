// A crossing and nothing else, so the slice prices its own boundary before it designs
// around a published one (README R13, and the rust slice's "an arm is not what its name
// says until the artifact agrees").
//
// Four things are timed:
//   forward        a static native method that does nothing
//   forward+arg    the same with a direct-buffer address argument, which is what a real
//                  entry point takes
//   reverse        native calling back into Java through CallStaticIntMethod, which is
//                  what an ABI v1 loop slot costs
//   reverse-cached the same with the jmethodID and the class ref cached, which is what a
//                  generated binding would actually emit
//
// No formatting happens until every measurement is done: on JDK 21+ a single
// String.format with a numeric conversion permanently deoptimises every char narrowing
// loop in the process (README R9), and this process later hosts protobuf-java's encoder
// in the real harness. Here it is cheap insurance; in `bench` it is load-bearing.
public final class Probe {
  static { System.load(System.getProperty("ak.lib")); }

  static native long fwd(long x);
  static native long fwdBuf(long addr, long x);
  static native long rev(long x);        // resolves the callback every call
  static native long revCached(long x);  // cached class + methodID, the generated shape
  static native long revObj(Object o, long x);  // upcall passing a Java object through

  // The callback targets. Kept trivial so what is measured is the transition.
  public static int cb(long x) { return (int) (x ^ 1); }
  public static int cbObj(Object o, long x) { return (int) (x ^ 1) + (o == null ? 0 : 1); }

  static long sink;

  public static void main(String[] args) throws Exception {
    final int ROUNDS = Integer.getInteger("ak.rounds", 9);
    final long TARGET_NS = 30_000_000L;

    String[] names = {"fwd", "fwd+buf", "rev", "rev-cached", "rev-obj"};
    long[][] ps = new long[names.length][ROUNDS];  // picoseconds per call

    java.nio.ByteBuffer buf = java.nio.ByteBuffer.allocateDirect(64);
    long addr = addressOf(buf);

    // Warm every path well past C2's thresholds before any round is recorded.
    for (int w = 0; w < 400_000; w++) { sink += fwd(w) + fwdBuf(addr, w) + rev(w) + revCached(w) + revObj(buf, w); }

    // Calibrate an iteration count per path so every round takes about the same time.
    int[] iters = new int[names.length];
    for (int k = 0; k < names.length; k++) iters[k] = calibrate(k, addr, buf, TARGET_NS);

    // Rotating order: a fixed order produced a monotone artifact in the cpp slice.
    for (int r = 0; r < ROUNDS; r++) {
      for (int j = 0; j < names.length; j++) {
        int k = (j + r) % names.length;
        long t = System.nanoTime();
        run(k, iters[k], addr, buf);
        long dt = System.nanoTime() - t;
        ps[k][r] = dt * 1000L / iters[k];
      }
    }

    // ---- everything below here is after the last measurement ----
    StringBuilder sb = new StringBuilder();
    sb.append("== JNI crossing price, this machine, this process ==\n");
    sb.append("java.version=").append(System.getProperty("java.version"))
      .append("  vm=").append(System.getProperty("java.vm.version")).append('\n');
    sb.append("rounds=").append(ROUNDS).append("  sink=").append(sink).append('\n');
    sb.append(String.format("%-12s %10s %8s %8s %8s%n", "path", "iters", "min_ns", "med_ns", "max_ns"));
    for (int k = 0; k < names.length; k++) {
      long[] v = ps[k].clone();
      java.util.Arrays.sort(v);
      sb.append(String.format("%-12s %10d %8.3f %8.3f %8.3f%n", names[k], iters[k],
          v[0] / 1000.0, v[v.length / 2] / 1000.0, v[v.length - 1] / 1000.0));
    }
    System.out.print(sb);
  }

  static void run(int k, int n, long addr, Object o) {
    long s = 0;
    switch (k) {
      case 0: for (int i = 0; i < n; i++) s += fwd(i); break;
      case 1: for (int i = 0; i < n; i++) s += fwdBuf(addr, i); break;
      case 2: for (int i = 0; i < n; i++) s += rev(i); break;
      case 3: for (int i = 0; i < n; i++) s += revCached(i); break;
      case 4: for (int i = 0; i < n; i++) s += revObj(o, i); break;
    }
    sink += s;
  }

  static int calibrate(int k, long addr, Object o, long targetNs) {
    int n = 1 << 14;
    for (;;) {
      long t = System.nanoTime();
      run(k, n, addr, o);
      long dt = System.nanoTime() - t;
      if (dt >= targetNs || n >= (1 << 28)) return n;
      n = (int) Math.max(n * 2L, n * (targetNs / Math.max(dt, 1)) );
      if (n <= 0) return 1 << 24;
    }
  }

  static long addressOf(java.nio.ByteBuffer b) throws Exception {
    java.lang.reflect.Field f = java.nio.Buffer.class.getDeclaredField("address");
    f.setAccessible(true);
    return f.getLong(b);
  }
}
