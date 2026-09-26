package ak;

import ak.shapes.Arms;
import ak.shapes.Binding;
import ak.shapes.FfiArms;

/**
 * README R5: count the crossings, do not infer them.
 *
 * <p>Counted in the CORE, from a build with the `count` feature, so a call the optimiser
 * removed is not counted. The rust slice established that a counter alone is not enough --
 * with the core in the crate graph as an rlib, rustc inlined every entry point and the
 * counters kept reporting the right numbers because the counting code inlined with them.
 * The JVM is the one host that cannot be got that way: a managed caller cannot inline
 * across JNI at all, and `gen/boundary.sh` shows the entry points as undefined imports of
 * the shim anyway, because "an arm is not what its name says until the artifact agrees".
 *
 * <p>The counts are the part of this slice that is already final. They are a property of
 * the interface rather than of the machine -- the cpp slice reproduced the rust slice's to
 * the digit -- so they survive the controlled rerun that every absolute here does not.
 */
public final class RunCounts {
  public static void main(String[] args) {
    StringBuilder log = new StringBuilder();
    log.append("== crossing counts, from the counting core (README R5) ==\n");
    log.append("java.version=").append(System.getProperty("java.version"))
       .append("  lib=").append(System.getProperty("ak.lib")).append('\n');
    log.append("\nforward = a JNI call into the core; reverse = an upcall out of it.\n");
    log.append("\n");

    String[] cfg = {"batched", "unbatched"};
    log.append(String.format("%-6s %-10s %9s %9s %9s %9s %9s %9s %9s%n",
        "id", "config", "elements", "enc.fwd", "enc.rev", "enc/elem", "dec.fwd",
        "dec.rev", "dec/elem"));

    for (String id : Arms.IDS) {
      Payloads.Row row = Payloads.row(id);
      byte[] wire = Payloads.vector(id);
      for (int c = 0; c < cfg.length; c++) {
        Binding b = new Binding();
        b.batch = c == 0;
        long[] enc = new long[6];
        long[] dec = new long[6];
        if (FfiArms.encodable(id)) {
          Object o = Arms.build(id, Values.ASCII);
          Native.encCountersReset(b.encCtx);
          FfiArms.encode(b, id, o);
          Native.encCounters(b.encCtx, enc);
          if (wire == null) wire = b.take();
        }
        if (wire != null) {
          Native.decCountersReset(b.contextOf(Arms.root(id)));
          FfiArms.decode(b, id, wire, 0, wire.length);
          Native.decCounters(b.decCtx, dec);
        }
        int n = Math.max(row.elements, 1);
        log.append(String.format("%-6s %-10s %9d %9d %9d %9.3f %9d %9d %9.3f%n",
            id, cfg[c], row.elements, enc[0], enc[1], (enc[0] + enc[1]) / (double) n,
            dec[0], dec[1], (dec[0] + dec[1]) / (double) n));
        b.close();
      }
    }

    // ---- ABI v1 7.1's pull family: the claim is that it removes the upcalls rather
    // than reducing them, and a count is the only thing that can say which. The drain
    // arm's forward crossings are the HOST's, so the shim reports them through
    // `ak_bdr_count_forward` rather than the core inferring them (R5: count, do not
    // infer). `footprint` is what the family materialises, which is the cost the
    // crossing count does not show.
    log.append("\n== the pull family (ABI v1 7.1) ==\n");
    log.append("\npush.rev is the same column as dec.rev above, for the comparison the\n");
    log.append("family exists to make. chunk is 32 KB + one header, the ABI's minimum.\n\n");
    log.append(String.format("%-6s %-10s %9s %9s %9s %9s %12s %10s%n",
        "id", "delivery", "elements", "fwd", "rev", "push.rev", "footprint", "bytes/elem"));
    for (String id : Arms.IDS) {
      Payloads.Row row = Payloads.row(id);
      byte[] wire = Payloads.vector(id);
      if (wire == null) {
        Binding w = new Binding();
        Object o = Arms.build(id, Values.ASCII);
        if (!FfiArms.encodable(id)) { w.close(); continue; }
        FfiArms.encode(w, id, o);
        wire = w.take();
        w.close();
      }
      long pushRev;
      {
        Binding b = new Binding();
        long[] d = new long[6];
        Native.decCountersReset(b.contextOf(Arms.root(id)));
        FfiArms.decode(b, id, wire, 0, wire.length);
        Native.decCounters(b.decCtx, d);
        pushRev = d[1];
        b.close();
      }
      for (int w = 0; w < 2; w++) {
        Binding b = new Binding();
        b.pullWalk = w == 1;
        long[] d = new long[6];
        Native.decCountersReset(b.contextOf(Arms.root(id)));
        FfiArms.parse(b, id, wire, 0, wire.length);
        Native.decCounters(b.decCtx, d);
        long fp = b.bdrFootprint();
        int n = Math.max(row.elements, 1);
        log.append(String.format("%-6s %-10s %9d %9d %9d %9d %12d %10.1f%n",
            id, w == 1 ? "pull-walk" : "pull-drain", row.elements, d[0], d[1], pushRev,
            fp, fp / (double) n));
        b.close();
      }
    }

    // The length-prefix mechanism of ABI v1 section 6 and open decision 5, which the rust
    // slice answered and which a second host either reproduces or contradicts.
    log.append("\n== the learned length-placeholder width (open decision 5) ==\n");
    log.append(String.format("%-6s %12s %12s %12s%n",
        "id", "prefix_moves", "prefix_bytes", "grows"));
    for (String id : Arms.IDS) {
      if (!FfiArms.encodable(id)) continue;
      Binding b = new Binding();
      Object o = Arms.build(id, Values.ASCII);
      // WARM, not cold: the widths survive a reset, which is what makes them learned, and
      // a cold context reports a miss per site that a running application never pays.
      FfiArms.encode(b, id, o);
      Native.encCountersReset(b.encCtx);
      FfiArms.encode(b, id, o);
      long[] k = new long[6];
      Native.encCounters(b.encCtx, k);
      log.append(String.format("%-6s %12d %12d %12d%n", id, k[3], k[4], k[5]));
      b.close();
    }

    // The transcoder count is the one number that says which string path an arm took, and
    // it is the only way to tell the core-transcodes arm from a host-transcodes one by
    // measurement rather than by reading the build flags.
    log.append("\n== transcoder invocations, which name the string path ==\n");
    log.append(String.format("%-6s %10s %12s%n", "id", "strings", "transcodes"));
    for (String id : Arms.IDS) {
      if (!FfiArms.encodable(id)) continue;
      Binding b = new Binding();
      Object o = Arms.build(id, Values.ASCII);
      Native.encCountersReset(b.encCtx);
      FfiArms.encode(b, id, o);
      long[] k = new long[6];
      Native.encCounters(b.encCtx, k);
      log.append(String.format("%-6s %10d %12d%n", id, Payloads.row(id).strings, k[2]));
      b.close();
    }
    entryPoints(log);
    System.out.print(log);
  }

  // ---- CAMPAIGN req 19 as amended 2026-09-26 (R-H31) ---------------------------------
  //
  // Every exported entry point the timed loop calls, resets included, counted by the shim
  // (-DAK_HOST_COUNT: every JNI entry that calls the core); beside it the core's own forward
  // count, the reverse crossings (upcalls, counted by the core) and the grow calls (the core
  // calling the shim's C grow). Per payload and direction in every mode of this build, and
  // the U-* rows at the shapes core's roots. Retain mode: no pre-placed buffer, the shim's
  // grow allocating exactly what the core asks for. One untimed operation first per row
  // (context creation and the first learned widths are not the timed loop's).
  //
  // Where the resets fall (each counted in `host`): encode -- ak_enc_reset before every
  // encode; decode, drop and no-unknown -- none (the root's context is reused, ak_decode_*
  // starts clean); decode, retain -- ak_dec_reset_<Root>(opts) before and
  // ak_dec_reset_<Root>(NULL) after every decode; decode-pull -- none (ak_parse_* resets
  // the record buffer at entry), retain arms and disarms around it as in push.

  interface Op { void run(); }

  static String row(String input, String dir, String mode, Binding b, Op op, boolean dec, String root) {
    op.run();                                                   // untimed first operation
    long[] h = new long[2], c = new long[6];
    if (dec) Native.decCountersReset(b.contextOf(root)); else Native.encCountersReset(b.encCtx);
    Native.hostCountsReset();
    op.run();
    Native.hostCounts(h);
    if (dec) Native.decCounters(b.contextOf(root), c); else Native.encCounters(b.encCtx, c);
    return String.format("EP %-26s %-21s %-10s host %6d core %6d rev %6d grow %4d%n",
        input, dir, mode, h[0], c[0], c[1], h[1]);
  }

  static void entryPoints(StringBuilder log) {
    log.append("\n== every entry point per operation (CAMPAIGN req 19, R-H31) ==\n");
    if (Native.hostCounting() != 1) {
      log.append("  (not the counting shim: -DAK_HOST_COUNT absent, section skipped)\n");
      return;
    }
    if (ak.Variant.UNKNOWN_FIELDS) Native.unkGrowExact(true);
    log.append("# host = JNI entries into the core (resets, take, drain, walk included); core = the\n"
        + "# core's own forward count; rev = upcalls; grow = calls of the shim's C grow.\n");
    String[] modes = ak.Variant.UNKNOWN_FIELDS ? new String[] {"drop", "retain"} : new String[] {"no-unknown"};
    for (String id : Arms.IDS) {
      final String root = Arms.root(id);
      byte[] w0 = Payloads.vector(id);
      if (w0 == null) {
        Binding t = new Binding();
        FfiArms.encode(t, id, Arms.build(id, Values.ASCII));
        w0 = t.take();
        t.close();
      }
      final byte[] w = w0;
      for (String m : modes) {
        final Binding b = new Binding();
        FfiArms.setRetain(b, m.equals("retain"));
        if (FfiArms.encodable(id)) {
          final Object o = Arms.build(id, Values.ASCII);
          final byte[] sinkBuf = new byte[w.length + 64];
          log.append(row(id, "encode (buf)", m, b, () -> { FfiArms.encode(b, id, o); Native.encTake(b.encCtx, sinkBuf); }, false, root));
          log.append(row(id, "encode (transport)", m, b, () -> { FfiArms.encode(b, id, o); b.take(); }, false, root));
        }
        log.append(row(id, "decode", m, b, () -> FfiArms.decode(b, id, w, 0, w.length), true, root));
        b.pullWalk = false;
        log.append(row(id, "decode-pull (drain)", m, b, () -> FfiArms.parse(b, id, w, 0, w.length), true, root));
        b.pullWalk = true;
        log.append(row(id, "decode-pull (walk)", m, b, () -> FfiArms.parse(b, id, w, 0, w.length), true, root));
        b.close();
      }
    }
    // The U-* rows at the shapes core's ABI roots (req 7 as amended), accepted, class unknown.
    java.io.File dir = new java.io.File(Payloads.SCHEMA_DIR, "../../corpus/generated");
    @SuppressWarnings("unchecked")
    java.util.Map<String, Object> vs = (java.util.Map<String, Object>) ((java.util.Map<String, Object>) Json.parse(
        new String(Payloads.readFile(new java.io.File(dir, "manifest.json")),
            java.nio.charset.Charset.forName("UTF-8")))).get("vectors");
    java.util.Set<String> abi = new java.util.HashSet<String>(java.util.Arrays.asList(ak.shapes.Dispatch.ABI));
    for (java.util.Map.Entry<String, Object> e : new java.util.TreeMap<String, Object>(vs).entrySet()) {
      @SuppressWarnings("unchecked")
      java.util.Map<String, Object> r = (java.util.Map<String, Object>) e.getValue();
      final String root = (String) r.get("root");
      if (!e.getKey().startsWith("U-") || !"unknown".equals(r.get("class")) || "disputed".equals(r.get("verdict"))
          || !"accept".equals(r.get("expect")) || !abi.contains(root)) continue;
      final byte[] w = Payloads.readFile(new java.io.File(dir, "vectors/" + e.getKey() + ".bin"));
      for (String m : modes) {
        final Binding b = new Binding();
        FfiArms.setRetain(b, m.equals("retain"));
        final Object o = ak.shapes.Dispatch.decFfi(b, root, w);
        log.append(row(e.getKey(), "encode (transport)", m, b, () -> ak.shapes.Dispatch.encFfi(b, root, o), false, root));
        log.append(row(e.getKey(), "decode", m, b, () -> ak.shapes.Dispatch.decFfi(b, root, w), true, root));
        b.pullWalk = false;
        log.append(row(e.getKey(), "decode-pull (drain)", m, b, () -> ak.shapes.Dispatch.parseFfi(b, root, w), true, root));
        b.close();
      }
    }
    if (ak.Variant.UNKNOWN_FIELDS) Native.unkGrowExact(false);
  }
}
