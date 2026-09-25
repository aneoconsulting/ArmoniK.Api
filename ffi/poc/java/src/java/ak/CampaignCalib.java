package ak;

/**
 * design/CAMPAIGN.md req 20, the host's half: the FORWARD crossing through the same JNI shim
 * and the same core the arms use (`ak_noop`), timed per round on the thread CPU clock and
 * reported per iteration by the reader. The REVERSE crossing (a JNI upcall into Java) is
 * `probe/CampaignRev.java`, and the Rust slice's own crossing benchmark is run beside both
 * by `gen/run_campaign.sh --suite calib`, which wraps each in `perf stat` when perf exists.
 */
public final class CampaignCalib {
  static long sink;

  public static void main(String[] args) throws Exception {
    Native.ensureBound();
    int n = Integer.getInteger("ak.camp.calibiters", 20_000_000);
    String out = System.getProperty("ak.camp.out");
    for (int w = 0; w < 3; w++) for (int i = 0; i < n; i++) sink += Native.noop(i);
    for (int r = 1; r <= Campaign.ROUNDS; r++) {
      long c0 = Campaign.threadCpuNs(), t0 = System.nanoTime();
      long s = 0;
      for (int i = 0; i < n; i++) s += Native.noop(i);
      long t1 = System.nanoTime(), c1 = Campaign.threadCpuNs();
      sink += s;
      Campaign.add(new Campaign.Sample().s("suite", "calib").s("arm", "crossing-forward-core-shim")
          .n("launch", Campaign.LAUNCH).n("round", r).n("cpu_ns", c1 - c0).n("wall_ns", t1 - t0)
          .n("iters", n));
    }
    Campaign.meta("{\"sink\":" + sink + "}");
    Campaign.flush(out);
  }
}
