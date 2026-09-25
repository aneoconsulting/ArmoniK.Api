package ak;

import java.io.FileOutputStream;
import java.io.OutputStreamWriter;
import java.io.PrintWriter;
import java.lang.management.ManagementFactory;
import java.lang.management.ThreadMXBean;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;

/**
 * What every campaign harness of this slice shares (design/CAMPAIGN.md sections 5 and 7).
 *
 * <ul>
 *   <li>clocks: the measuring thread's CPU time (ThreadMXBean, which HotSpot reads with
 *       {@code clock_gettime(CLOCK_THREAD_CPUTIME_ID)}, ns) for codec samples, req 21;</li>
 *   <li>the JSON-lines body of req 28, buffered in memory and written only when the run
 *       completes, so a run that aborts (a failed check, req 18 and 26) writes no figure;</li>
 *   <li>the rotation of req 22.</li>
 * </ul>
 */
public final class Campaign {
  private Campaign() {}

  static final ThreadMXBean TMX = ManagementFactory.getThreadMXBean();

  static {
    if (!TMX.isCurrentThreadCpuTimeSupported())
      throw new IllegalStateException("req 21: no thread CPU clock on this JVM");
    TMX.setThreadCpuTimeEnabled(true);
  }

  public static long threadCpuNs() { return TMX.getCurrentThreadCpuTime(); }

  public static final int LAUNCH = Integer.getInteger("ak.camp.launch", 1);
  public static final int ROUNDS = Integer.getInteger("ak.camp.rounds", 5);

  /** Req 22: arm/cell order for round r, rotated by r. */
  public static <T> List<T> rotate(List<T> xs, int r) {
    List<T> out = new ArrayList<T>(xs.size());
    int k = xs.isEmpty() ? 0 : r % xs.size();
    for (int i = 0; i < xs.size(); i++) out.add(xs.get((i + k) % xs.size()));
    return out;
  }

  /** One JSON object per sample (req 28), fields not applicable left out. */
  public static final class Sample {
    final StringBuilder sb = new StringBuilder("{\"slice\":\"java\"");
    public Sample s(String k, String v) {
      if (v != null) sb.append(",\"").append(k).append("\":\"").append(v).append('"');
      return this;
    }
    public Sample n(String k, long v) {
      sb.append(",\"").append(k).append("\":").append(v);
      return this;
    }
    @Override public String toString() { return sb.toString() + "}"; }
  }

  static final List<String> LINES = new ArrayList<String>();

  public static synchronized void add(Sample s) { LINES.add(s.toString()); }

  /** A meta line (not a sample): what a reader needs to judge the samples, e.g. the JIT's
   *  cumulative compile time at each round boundary (req 24, "recorded"). */
  public static synchronized void meta(String json) { LINES.add("{\"meta\":" + json + "}"); }

  public static long jitMs() {
    java.lang.management.CompilationMXBean c = ManagementFactory.getCompilationMXBean();
    return c != null && c.isCompilationTimeMonitoringSupported() ? c.getTotalCompilationTime() : -1;
  }

  /** Written once, at the end, only if the run completed (req 18, 26). */
  public static void flush(String path) throws Exception {
    PrintWriter w = new PrintWriter(new OutputStreamWriter(
        path == null ? System.out : new FileOutputStream(path, true), StandardCharsets.UTF_8));
    for (String l : LINES) w.println(l);
    w.flush();
    if (path != null) w.close();
  }

  public static void abort(String why) {
    System.err.println("CAMPAIGN ABORT (no figure written): " + why);
    System.exit(3);
  }
}
