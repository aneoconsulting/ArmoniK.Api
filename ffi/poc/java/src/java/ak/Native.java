package ak;

/**
 * The declared half of the JNI boundary. One native method per ABI entry point, matching
 * one C function in {@code native/generated/shim.c}; never a dispatch table, because the
 * thing this slice measures is a difference in forward crossing counts and a shared switch
 * would inflate it.
 *
 * <p>Hand-written rather than generated, and it is the one place in the slice where that
 * is a risk: a signature here and a C function there could drift. What catches it is the
 * JVM itself -- an unbound native method throws {@code UnsatisfiedLinkError} on first call
 * -- plus {@code ensureBound}, which resolves every one of them at load rather than at
 * first use, so a drift is a startup failure and not a benchmark that silently skipped an
 * arm.
 */
public final class Native {
  private Native() {}

  public static final int OK = 0;
  public static final int ERR_HOST = -1;
  public static final int ERR_TRANSCODE = -6;
  public static final int ERR_CAPACITY = -7;
  public static final int ERR_ABI = -11;
  public static final byte[] NO_BYTES = new byte[0];

  private static boolean loaded;

  public static synchronized void load(String path) {
    if (loaded) return;
    System.load(path);
    loaded = true;
  }

  public static synchronized void ensureBound() {
    if (!loaded) {
      String p = System.getProperty("ak.lib");
      if (p == null) throw new IllegalStateException("-Dak.lib is not set");
      load(p);
    }
    bind(ak.Callbacks.class);
    if (abiVersion() != 1)
      throw new IllegalStateException("ak_abi_version() is " + abiVersion() + ", not 1");
    // `ak_init` (R-G7) and the layout guard (ABI v1 section 10) are per DESCRIPTION, so
    // they are rendered into each generated package: `<pkg>.Binding`'s static initialiser
    // calls `<entry>.ensureInit()` and `<pkg>.Layout.assertAgreement()` (FIX-PLAN WP5).
  }

  /** Whether `ak_init` has returned successfully in this process (`ak_initialized`). */
  public static native int initialized();

  /** Caches the reverse-call method ids from the INTERFACE, so one shim serves both the
   *  owning facade's Binding and the borrowed facade's. */
  static native void bind(Class<?> callbacks);

  public static native int abiVersion();
  public static native long encCtxNew();
  public static native void encCtxFree(long ctx);
  public static native void encReset(long ctx);
  public static native int encErr(long ctx);
  public static native int encTake(long ctx, byte[] dst);
  public static native int encLen(long ctx);
  // No untyped `decCtxNew`: decision 11 rule 6 binds a context to its root
  // (`<entry>.decCtxNew<Root>(opts)`, rendered per description).
  /** Decision 11: the shim's grow (malloc/realloc), and a delivered buffer copied and freed,
   *  or freed. */
  public static native long unkGrow();
  public static native byte[] unkTake(long data, int len);
  public static native void unkFree(long data);
  /** The per-options tracking list, the reclaim of what was never delivered, the live count. */
  public static native long unkListNew();
  public static native int unkReclaim(long list);
  public static native void unkListFree(long list);
  public static native long unkLive();
  /** Counting harness (CAMPAIGN req 19): the shim's grow allocates exactly what is asked. */
  public static native void unkGrowExact(boolean on);
  /** Counting shim (-DAK_HOST_COUNT): {JNI entries into the core, grow calls}. */
  public static native void hostCounts(long[] out);
  public static native void hostCountsReset();
  public static native int hostCounting();
  /** CLOCK_PROCESS_CPUTIME_ID in ns (tax.c, linked into every shim; CAMPAIGN req 21). */
  public static native long processCpuNs();
  public static native void decCtxFree(long ctx);
  public static native int decErr(long ctx);
  public static native void decErrReset(long ctx);
  public static native void fail(long ctx, int code);

  // ABI v1 7.1's record buffer: the pull family's forward half.
  public static native void bdrReset(long ctx);
  public static native long bdrFootprint(long ctx);
  public static native int bdrReserve(long ctx, long bytes);
  public static native int bdrPtr(long ctx, long[] out);
  public static native long bdrDrain(long ctx, long dst, long cap, long[] cursor);

  public static native long tcUtf16();
  public static native long tcLatin1();
  public static native long tcBytes();
  public static native long tcUtf8();

  public static native void encCounters(long ctx, long[] out);
  public static native void encCountersReset(long ctx);
  public static native void decCounters(long ctx, long[] out);
  public static native void decCountersReset(long ctx);
  public static native int layoutFacts(int[] out);
  public static native long noop(long x);

  // Forward entry points. Instance methods on Binding would be one field load cheaper to
  // call from Java but would put `self` in the argument list of every forward crossing;
  // the reverse calls need `self` and get it from the frame the entry pushed.
  public static native int blobRun(long ctx, long p, int n);
  public static native int runI32(long ctx, long p, long n);
  public static native int runI64(long ctx, long p, long n);
  public static native int runF64(long ctx, long p, long n);
  public static native int runU8(long ctx, long p, long n);
}
