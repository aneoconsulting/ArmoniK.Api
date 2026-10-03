package ak;

/**
 * CAMPAIGN req 25 / D9 as amended by the owner 2026-10-03: the allocator mode of a measured
 * JVM process, checked before any timing. {@code -Dak.camp.alloc} (the runner's
 * AK_CAMPAIGN_ALLOC: default | pinned) must agree with the process's GLIBC_TUNABLES (default:
 * no malloc tunable; pinned: exactly {@link #PINNED}) and with what glibc does: one 16 MiB
 * malloc through the shim ({@code Native.allocProbe}) must read "mmapped" in default and
 * "heap" in pinned. Any disagreement throws: in a JMH fork the trial setup fails and, with
 * -foe true, no sample is written; run as a main it exits non-zero.
 */
public final class CampaignAlloc {
  private CampaignAlloc() {}
  public static final String PINNED =
      "glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432";

  private static String done;

  /** Returns the readback line; throws on any disagreement. ak.lib must be loaded. Probes
   *  once per process (JMH may run a trial setup more than once in a fork), the result
   *  cached: see Native.allocProbe. */
  public static synchronized String verify() {
    if (done != null) return done;
    String mode = System.getProperty("ak.camp.alloc", "default");
    String env = System.getenv("GLIBC_TUNABLES");
    if (env == null) env = "";
    if (!mode.equals("default") && !mode.equals("pinned"))
      throw new IllegalStateException("alloc check: ak.camp.alloc=" + mode + " is neither default nor pinned");
    boolean envPinned = env.equals(PINNED);
    if (mode.equals("pinned") != envPinned || (mode.equals("default") && env.contains("glibc.malloc.")))
      throw new IllegalStateException("alloc check: AK_CAMPAIGN_ALLOC=" + mode + " disagrees with GLIBC_TUNABLES='" + env + "'");
    int r = Native.allocProbe();
    String got = r == 1 ? "mmapped" : r == 0 ? "heap" : "malloc failed";
    String want = mode.equals("default") ? "mmapped" : "heap";
    String line = "ALLOC-CHECK alloc=" + mode + " GLIBC_TUNABLES='" + env + "' 16MiB-malloc=" + got;
    if (!got.equals(want))
      throw new IllegalStateException("alloc check: " + line + ", expected " + want);
    done = line;
    return line;
  }

  /** The startup line for a log: verify()'s readback. (The heap pre-grow that followed it
   *  was reverted by the owner, 2026-10-03: JMH's warm-up runs the real call path on every
   *  thread, and the per-sample minflt shows whether it sufficed.) */
  public static String startup() {
    return verify();
  }

  public static void main(String[] a) {
    Native.ensureBound();
    try {
      System.out.println(startup());
    } catch (IllegalStateException e) {
      System.out.println("ALLOC-CHECK FAILED: " + e.getMessage());
      System.exit(4);
    }
  }
}
