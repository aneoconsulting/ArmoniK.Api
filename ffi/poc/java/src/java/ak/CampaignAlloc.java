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

  private static String grown;
  public static final int PREGROW_CAP = 8;

  /** Req 25 as amended (mechanics): after verify() and before any timing, pre-grow glibc's
   *  heap with rounds of the run's largest payload ({@code -Dak.camp.pregrow}: a byte count,
   *  or "codec-max", the largest canonical payload of the codec suite) until a round faults
   *  nothing, at most 8; throws when the cap is hit. Once per process. */
  public static synchronized String preGrow() {
    if (grown != null) return grown;
    String p = System.getProperty("ak.camp.pregrow", "16777216");
    long bytes = p.equals("codec-max") ? codecMax() : Long.parseLong(p);
    long r = Native.preGrow(bytes, PREGROW_CAP);
    if (r < 0) throw new IllegalStateException("pre-grow: malloc(" + bytes + ") failed");
    long rounds = r >>> 32, faults = r & 0xffffffffL;
    String line = "PREGROW bytes=" + bytes + " rounds=" + rounds + " last_round_faults=" + faults;
    if (faults != 0) throw new IllegalStateException("pre-grow: " + line + ", cap of " + PREGROW_CAP + " rounds hit");
    grown = line;
    return line;
  }

  /** The largest canonical payload of the codec suite (every id, every content set). */
  static long codecMax() {
    long m = 0;
    for (String id : ak.shapes.Arms.IDS)
      for (int cs = 0; cs < CampaignCodec.SET_NAMES.length; cs++) {
        try { m = Math.max(m, CampaignCodec.canonical(id, cs).length); }
        catch (RuntimeException e) { /* a combination the suite does not run (P7.1 off ASCII) */ }
      }
    return m;
  }

  /** verify() then preGrow(): the two lines, for a log. */
  public static String startup() {
    return verify() + " | " + preGrow();
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
