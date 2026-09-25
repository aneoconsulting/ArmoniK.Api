/**
 * design/CAMPAIGN.md req 20, the reverse direction on the JVM: a JNI upcall into a static
 * Java method with a cached class and method id (the generated shim's shape), and the bare
 * JNI forward call beside it, timed per round on the thread CPU clock. JSON lines (req 28).
 * Reuses `Probe`'s natives (probe/probe.c).
 */
public final class CampaignRev {
  static long sink;

  public static void main(String[] args) throws Exception {
    java.lang.management.ThreadMXBean tmx = java.lang.management.ManagementFactory.getThreadMXBean();
    int n = Integer.getInteger("ak.camp.calibiters", 20_000_000);
    int rounds = Integer.getInteger("ak.camp.rounds", 5);
    int launch = Integer.getInteger("ak.camp.launch", 1);
    StringBuilder sb = new StringBuilder();
    for (int w = 0; w < 3; w++) for (int i = 0; i < n; i++) sink += Probe.fwd(i) + Probe.revCached(i);
    for (int r = 1; r <= rounds; r++) {
      for (int k = 0; k < 2; k++) {
        int kk = (k + r) % 2;
        long c0 = tmx.getCurrentThreadCpuTime(), t0 = System.nanoTime();
        long s = 0;
        if (kk == 0) for (int i = 0; i < n; i++) s += Probe.fwd(i);
        else for (int i = 0; i < n; i++) s += Probe.revCached(i);
        long t1 = System.nanoTime(), c1 = tmx.getCurrentThreadCpuTime();
        sink += s;
        sb.append("{\"slice\":\"java\",\"suite\":\"calib\",\"arm\":\"")
          .append(kk == 0 ? "crossing-forward-jni" : "crossing-reverse-jni-upcall")
          .append("\",\"launch\":").append(launch).append(",\"round\":").append(r)
          .append(",\"cpu_ns\":").append(c1 - c0).append(",\"wall_ns\":").append(t1 - t0)
          .append(",\"iters\":").append(n).append("}\n");
      }
    }
    sb.append("{\"meta\":{\"sink\":").append(sink).append("}}\n");
    System.out.print(sb);
  }
}
