package ak;

import ak.shapes.Arms;
import ak.shapes.Binding;
import ak.shapes.FfiArms;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

/**
 * The questions that are a DELTA between two configurations of one binding, measured on
 * their own.
 *
 * <p>README R4, sharpened: "where a question can be asked as a delta between two arms in
 * the same interleaved rounds, ask it that way -- it survives things a ratio to a third
 * arm does not." In the main bench these deltas straddle zero, and the reason is visible
 * rather than mysterious: that harness rebuilds a pool of protobuf messages between
 * rounds so its baseline is honest, and the collector that follows is larger than a seven
 * percent effect. Here there is no incumbent and no pool churn. The two arms differ by one
 * boolean, read the same objects in the same order, and nothing else in the process
 * allocates.
 *
 * <p>Three questions:
 * <ul>
 *   <li><b>the batching predicate</b> (ABI v1 section 6). The cpp slice priced the
 *       crossover at a forward crossing of roughly 2 to 4 ns; this host's is about 11, so
 *       batching should win here and the size of the win is the point;</li>
 *   <li><b>open decision 9's fill</b>, which Rust and C++ both measured a win and both
 *       said a managed host must decide for itself;</li>
 *   <li><b>the crossing tax curve</b>, with {@code AK_TAX_N} set, which walks the crossover
 *       rather than reporting one point on it.</li>
 * </ul>
 */
public final class RunDelta {

  /** One configuration. `floor` selects the FLOOR binding (README 5.2 arm b): the Java 8
   *  source tree, emitted into a package of its own so it shares this process and this
   *  pool with the target and the ratio between them is paired (R4). */
  static final class Cfg {
    final String name;
    final Binding b;
    final ak.floor.Binding f;
    Cfg(String name, boolean batch, boolean zeroed, boolean floor) {
      this.name = name;
      if (floor) {
        this.b = null;
        this.f = new ak.floor.Binding();
        this.f.batch = batch;
        this.f.zeroed = zeroed;
      } else {
        this.b = new Binding();
        this.b.batch = batch;
        this.b.zeroed = zeroed;
        this.f = null;
      }
    }
  }

  static long sink;

  public static void main(String[] args) {
    final int ROUNDS = Integer.getInteger("ak.rounds", 48);
    final long TARGET = Long.getLong("ak.roundns", 25_000_000L);
    final int POOL = Integer.getInteger("ak.pool", 16);
    final String only = System.getProperty("ak.only");

    List<Cfg> cfgs = new ArrayList<Cfg>();
    cfgs.add(new Cfg("batched", true, false, false));
    cfgs.add(new Cfg("unbatched", false, false, false));
    cfgs.add(new Cfg("batched+zeroed", true, true, false));
    cfgs.add(new Cfg("unbatched+zeroed", false, true, false));
    if ("1".equals(System.getProperty("ak.floor", "0")))
      cfgs.add(new Cfg("floor-batched", true, false, true));

    int rounds = ((ROUNDS + cfgs.size() - 1) / cfgs.size()) * cfgs.size();

    List<String> ids = new ArrayList<String>();
    for (String id : Arms.IDS) if (only == null || only.equals(id)) ids.add(id);

    long[][][] ps = new long[ids.size()][][];
    int[] iters = new int[ids.size()];

    for (int p = 0; p < ids.size(); p++) {
      String id = ids.get(p);
      if (!FfiArms.encodable(id)) continue;
      // A small pool of DISTINCT source objects, so the measurement is not a single hot
      // object, and small enough that it never leaves the cache hierarchy differently for
      // one configuration than for another. It is built once: a facade object carries no
      // state between encodes, so nothing has to be refreshed and nothing allocates
      // between rounds.
      Object[] pool = new Object[POOL];
      for (int i = 0; i < POOL; i++) pool[i] = Arms.build(id, Values.ASCII);

      // Warm to a fixed TIME, not to a fixed count. `for (w < 4000) enc(..., POOL)` ran
      // 64,000 encodes per configuration per payload, which on the two-millisecond rows is
      // ten minutes of warmup for five seconds of measurement. C2 needs about 10,000
      // invocations of the hot methods, and the tiered thresholds are reached long before
      // a second of a two-millisecond operation.
      for (Cfg c : cfgs) {
        long until = System.nanoTime() + 800_000_000L;
        int w = 0;
        while (System.nanoTime() < until && w < 200_000) {
          sink += enc(c, id, pool, 16);
          w += 16;
        }
      }
      int n = calibrate(cfgs.get(0), id, pool, POOL, TARGET);
      iters[p] = n;
      long[][] r = new long[cfgs.size()][rounds];
      for (int round = 0; round < rounds; round++) {
        for (int j = 0; j < cfgs.size(); j++) {
          int a = (j + round) % cfgs.size();
          long t0 = System.nanoTime();
          sink += enc(cfgs.get(a), id, pool, n);
          r[a][round] = (System.nanoTime() - t0) * 1000L / n;
        }
      }
      ps[p] = r;
    }

    // ---- after the last measurement ---------------------------------------------------
    StringBuilder sb = new StringBuilder();
    sb.append("== encode deltas between two configurations of one binding ==\n");
    sb.append("java.version=").append(System.getProperty("java.version"))
      .append("  lib=").append(System.getProperty("ak.lib")).append('\n');
    String tax = System.getenv("AK_TAX_N");
    sb.append("AK_TAX_N=").append(tax == null ? "unset" : tax)
      .append("   (a calibrated delay in front of every FORWARD entry-point call, which"
              + " prices the crossing up)\n");
    sb.append("rounds=").append(rounds).append("  pool=").append(POOL)
      .append(" distinct source objects, built once  sink=").append(sink).append("\n\n");
    sb.append("Nothing allocates between rounds, so a collection is not a third arm.\n\n");

    sb.append(pad("id", 7)).append(pad("iters", 8)).append(pad("batched ns", 13));
    for (int a = 1; a < cfgs.size(); a++) sb.append(pad(cfgs.get(a).name + " /batched", 22));
    sb.append('\n');
    for (int p = 0; p < ids.size(); p++) {
      if (ps[p] == null) continue;
      sb.append(pad(ids.get(p), 7)).append(pad(Integer.toString(iters[p]), 8))
        .append(pad(f(med(ps[p][0]) / 1000.0, 1), 13));
      for (int a = 1; a < cfgs.size(); a++) {
        double[] q = ratio(ps[p][a], ps[p][0]);
        sb.append(pad(f(q[0], 3) + " " + f(q[1], 3) + " " + f(q[2], 3), 22));
      }
      sb.append('\n');
    }

    delta(sb, ids, ps, cfgs, "unbatched", "batched",
          "THE BATCHING PREDICATE (ABI v1 section 6). Positive means batching is faster.");
    delta(sb, ids, ps, cfgs, "batched", "batched+zeroed",
          "OPEN DECISION 9's fill. Positive means the sparse fill is faster.");
    delta(sb, ids, ps, cfgs, "unbatched", "unbatched+zeroed",
          "The same fill question with the host declining to batch, which is where the"
          + " group's fixed cost has the least to hide behind.");
    if (idx(cfgs, "floor-batched") >= 0)
      delta(sb, ids, ps, cfgs, "floor-batched", "batched",
            "README 5.2 ARM B: the floor implementation on the TARGET runtime, paired"
            + " inside one process. Positive means the floor is slower, and the whole"
            + " difference is that the floor stages a String through getChars as UTF-16"
            + " where the target hands the core the string's own compact storage.");
    System.out.print(sb);
  }

  static void delta(StringBuilder sb, List<String> ids, long[][][] ps, List<Cfg> cfgs,
                    String a, String b, String what) {
    int ia = idx(cfgs, a), ib = idx(cfgs, b);
    sb.append("\n\n  ").append(a).append(" - ").append(b).append('\n');
    sb.append("  ").append(what).append('\n');
    sb.append("  ").append(pad("id", 7)).append(pad("elements", 10))
      .append(pad("lo ns/op", 13)).append(pad("med", 13)).append(pad("hi", 13))
      .append(pad("med/element", 14)).append("sign\n");
    for (int p = 0; p < ids.size(); p++) {
      if (ps[p] == null) continue;
      long[] x = ps[p][ia], y = ps[p][ib];
      double[] d = new double[x.length];
      for (int i = 0; i < x.length; i++) d[i] = (x[i] - y[i]) / 1000.0;
      Arrays.sort(d);
      double lo = d[0], md = d[d.length / 2], hi = d[d.length - 1];
      boolean same = (lo > 0 && hi > 0) || (lo < 0 && hi < 0);
      int el = Math.max(Payloads.row(ids.get(p)).elements, 1);
      sb.append("  ").append(pad(ids.get(p), 7)).append(pad(Integer.toString(el), 10))
        .append(pad(f(lo, 1), 13)).append(pad(f(md, 1), 13)).append(pad(f(hi, 1), 13))
        .append(pad(f(md / el, 3), 14))
        .append(same ? (lo > 0 ? "+" : "-") : "STRADDLES ZERO").append('\n');
    }
  }

  static int idx(List<Cfg> c, String n) {
    for (int i = 0; i < c.size(); i++) if (c.get(i).name.equals(n)) return i;
    return -1;
  }

  static long enc(Cfg c, String id, Object[] pool, int n) {
    long s = 0;
    int m = pool.length;
    if (c.f != null) {
      for (int i = 0; i < n; i++) s += ak.floor.FfiArms.encode(c.f, id, pool[i % m]);
      return s;
    }
    for (int i = 0; i < n; i++) s += FfiArms.encode(c.b, id, pool[i % m]);
    return s;
  }

  static int calibrate(Cfg c, String id, Object[] pool, int poolN, long target) {
    int n = 16;
    for (;;) {
      long t0 = System.nanoTime();
      sink += enc(c, id, pool, n);
      long dt = System.nanoTime() - t0;
      if (dt >= target || n >= (1 << 24)) return n;
      n = (int) Math.max(n * 2L, Math.min(n * (target / Math.max(dt, 1000L)), n * 32L));
      if (n <= 0) return 1 << 16;
    }
  }

  static long med(long[] v) { long[] c = v.clone(); Arrays.sort(c); return c[c.length / 2]; }

  static double[] ratio(long[] a, long[] b) {
    double[] q = new double[a.length];
    for (int i = 0; i < a.length; i++) q[i] = a[i] / (double) b[i];
    Arrays.sort(q);
    return new double[] {q[0], q[q.length / 2], q[q.length - 1]};
  }

  static String pad(String s, int w) {
    StringBuilder b = new StringBuilder(s);
    while (b.length() < w) b.append(' ');
    return b.toString();
  }

  static String f(double v, int dp) { return Bench.fmt(v, dp); }
}
