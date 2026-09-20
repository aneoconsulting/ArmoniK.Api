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
    log.append("This machine's prices, measured in `logs/java/crossing.log`:\n");
    log.append("  forward about 11 ns, reverse about 80 ns with the method id cached.\n\n");

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
          Native.decCountersReset(b.decCtx);
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
        Native.decCountersReset(b.decCtx);
        FfiArms.decode(b, id, wire, 0, wire.length);
        Native.decCounters(b.decCtx, d);
        pushRev = d[1];
        b.close();
      }
      for (int w = 0; w < 2; w++) {
        Binding b = new Binding();
        b.pullWalk = w == 1;
        long[] d = new long[6];
        Native.decCountersReset(b.decCtx);
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
    System.out.print(log);
  }
}
