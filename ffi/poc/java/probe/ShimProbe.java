// What a generated C shim would pay per operation on the JVM, priced before the arm is
// built (README 9.1's shape, applied to Java; see probe/shimprobe.c for why).
//
// The decode regression this is aimed at is 1.22 to 1.62 on every M2 payload, and the
// slice decomposed it: 7.004 reverse calls per element at about 80 ns is 560 ns of it.
// A shim that writes facade fields through the JNI API removes those upcalls. Whether it
// wins is arithmetic and this prices both sides of it.
//
// No formatting until the last measurement is taken (README R9).
public final class ShimProbe {
  static { System.load(System.getProperty("ak.lib")); }

  /** Stands in for a facade element: two reference fields and a primitive. */
  public static final class Node {
    public String a = "abcdef0123456789";
    public String b = "";
    public int s = 0;
  public String f0 = "";
  public String f1 = "";
  public String f2 = "";
  public String f3 = "";
  public String f4 = "";
  public String f5 = "";
  public String f6 = "";
  public String f7 = "";
  public String f8 = "";
  public String f9 = "";
  public String f10 = "";
  public String f11 = "";
  public String f12 = "";
  public String f13 = "";
  public String f14 = "";
  public String f15 = "";
  }

  static native void bind(Class<?> node, Class<?> list);
  static native void bind2(Class<?> probe);
  static native void bindFields(Object[] values);

  static native long opNone(Object o, int n);
  static native long opGetObj(Object o, int n);
  static native long opSetObj(Object o, int n);
  static native long opSetInt(Object o, int n);
  static native long opNewObj(Object o, int n);
  static native long opAllocObj(Object o, int n);
  static native long opListSet(Object list, int n);
  static native long opArrSet(Object arr, int n);
  static native long opNewArr(Object o, int n);
  static native long opNewStr(Object o, int n);

  static native long revApply(Object o, int k, int n);
  static native long shimApply(Object o, int k, int n);

  /** The callee of the upcall: k straight-line stores of k DISTINCT values into k
   *  DISTINCT fields, which is what a generated `apply` emits. The first cut of this
   *  probe stored one value into one field k times, which C2 reduces to a single store --
   *  a defective control, and it was in the arm deciding whether to build the shim. */
  static final String[] V = new String[16];
  static {
    for (int i = 0; i < 16; i++) V[i] = "abcdef012345678" + (char) ('a' + i);
  }

  public static void apply(Node o, int k) {
    switch (k) {
      case 16: o.f15 = V[15];
      case 15: o.f14 = V[14];
      case 14: o.f13 = V[13];
      case 13: o.f12 = V[12];
      case 12: o.f11 = V[11];
      case 11: o.f10 = V[10];
      case 10: o.f9 = V[9];
      case 9:  o.f8 = V[8];
      case 8:  o.f7 = V[7];
      case 7:  o.f6 = V[6];
      case 6:  o.f5 = V[5];
      case 5:  o.f4 = V[4];
      case 4:  o.f3 = V[3];
      case 3:  o.f2 = V[2];
      case 2:  o.f1 = V[1];
      case 1:  o.f0 = V[0];
      default:
    }
  }

  static final String[] OPS = {
    "none (control)", "GetObjectField", "SetObjectField", "SetIntField",
    "NewObject", "AllocObject", "List.set upcall", "SetObjectArrayElement",
    "NewObjectArray(3)", "NewString(16)",
  };

  static long call(int op, Node node, java.util.List<String> list, Object[] arr, int n) {
    switch (op) {
      case 0: return opNone(node, n);
      case 1: return opGetObj(node, n);
      case 2: return opSetObj(node, n);
      case 3: return opSetInt(node, n);
      case 4: return opNewObj(node, n);
      case 5: return opAllocObj(node, n);
      case 6: return opListSet(list, n);
      case 7: return opArrSet(arr, n);
      case 8: return opNewArr(node, n);
      case 9: return opNewStr(node, n);
      default: throw new IllegalArgumentException();
    }
  }

  static long sink;

  public static void main(String[] args) throws Exception {
    final int ROUNDS = Integer.getInteger("ak.rounds", 11);
    final int N = Integer.getInteger("ak.n", 4096);
    final int[] KS = {1, 2, 4, 8, 16};

    Node node = new Node();
    java.util.List<String> list = new java.util.ArrayList<String>();
    list.add("x");
    Object[] arr = new Object[8];
    java.util.Arrays.fill(arr, "x");

    bind(Node.class, java.util.List.class);
    bindFields(V);
    bind2(ShimProbe.class);

    // Warm every path well past C2's thresholds before any round is recorded.
    for (int w = 0; w < 2_000; w++) {
      for (int op = 0; op < OPS.length; op++) sink += call(op, node, list, arr, N);
      for (int k : KS) sink += revApply(node, k, 64) + shimApply(node, k, 64);
    }

    long[][] ps = new long[OPS.length][ROUNDS];
    long[][] rev = new long[KS.length][ROUNDS];
    long[][] shm = new long[KS.length][ROUNDS];

    // Rotating order: a fixed order produced a monotone artifact in the cpp slice.
    for (int r = 0; r < ROUNDS; r++) {
      for (int j = 0; j < OPS.length; j++) {
        int op = (j + r) % OPS.length;
        long t = System.nanoTime();
        sink += call(op, node, list, arr, N);
        ps[op][r] = (System.nanoTime() - t) * 1000L / N;
      }
      for (int j = 0; j < KS.length; j++) {
        int i = (j + r) % KS.length;
        long t = System.nanoTime();
        sink += revApply(node, KS[i], N);
        rev[i][r] = (System.nanoTime() - t) * 1000L / N;
        t = System.nanoTime();
        sink += shimApply(node, KS[i], N);
        shm[i][r] = (System.nanoTime() - t) * 1000L / N;
      }
    }

    // ---- everything below here is after the last measurement ----
    StringBuilder sb = new StringBuilder();
    sb.append("== what a C shim pays per operation, this machine, this process ==\n");
    sb.append("java.version=").append(System.getProperty("java.version"))
      .append("  vm=").append(System.getProperty("java.vm.version")).append('\n');
    sb.append("rounds=").append(ROUNDS).append("  n per call=").append(N)
      .append("  sink=").append(sink).append('\n');
    sb.append("\nEach op is looped INSIDE one native call and reported net of the control,\n")
      .append("because a shim is already in C when it pays these. Compare against this\n")
      .append("machine's cached upcall of about 80 ns (logs/java/crossing.log).\n\n");
    sb.append(String.format("%-24s %9s %9s %9s%n", "op", "min_ns", "med_ns", "max_ns"));
    long base = med(ps[0]);
    for (int op = 0; op < OPS.length; op++) {
      long[] v = ps[op].clone();
      java.util.Arrays.sort(v);
      double lo = (v[0] - base) / 1000.0, md = (v[ROUNDS / 2] - base) / 1000.0;
      double hi = (v[ROUNDS - 1] - base) / 1000.0;
      sb.append(String.format("%-24s %9.3f %9.3f %9.3f%n", OPS[op], lo, md, hi));
    }

    sb.append("\nOne run of k stores, the two ways. `upcall` is what the binding does\n")
      .append("today: one cached reverse call, the stores in bytecode inside it. `shim`\n")
      .append("is k SetObjectField calls and no reverse call. Nanoseconds per run.\n\n");
    sb.append(String.format("%-6s %12s %12s %12s%n", "k", "upcall", "shim", "shim-upcall"));
    for (int i = 0; i < KS.length; i++) {
      double a = med(rev[i]) / 1000.0, b = med(shm[i]) / 1000.0;
      sb.append(String.format("%-6d %12.2f %12.2f %12.2f%n", KS[i], a, b, b - a));
    }
    System.out.print(sb);
  }

  static long med(long[] v) {
    long[] c = v.clone();
    java.util.Arrays.sort(c);
    return c[c.length / 2];
  }
}
