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
public final class AllocCheck {
  private AllocCheck() {}
  public static final String PINNED =
      "glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432";

  /** Returns the readback line; throws on any disagreement. ak.lib must be loaded. */
  public static String verify() {
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
    return line;
  }

  public static void main(String[] a) {
    Native.ensureBound();
    try {
      System.out.println(verify());
    } catch (IllegalStateException e) {
      System.out.println("ALLOC-CHECK FAILED: " + e.getMessage());
      System.exit(4);
    }
  }
}
