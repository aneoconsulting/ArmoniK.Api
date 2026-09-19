package ak;

import ak.shapes.Arms;
import ak.shapes.Codec;
import java.util.ArrayList;
import java.util.List;

/** Correctness gate. Arms register here; nothing is measured that this has not checked. */
public final class RunConformance {
  public static void main(String[] args) throws Exception {
    boolean verbose = args.length > 0 && args[0].equals("-v");
    StringBuilder log = new StringBuilder();
    log.append("== conformance: byte identity across every arm ==\n");
    log.append("java.version=").append(System.getProperty("java.version"))
       .append("  schema=").append(Payloads.SCHEMA_DIR).append('\n');

    List<Conformance.Arm> arms = new ArrayList<Conformance.Arm>();
    arms.add(new RArm());
    if (System.getProperty("ak.lib") != null) {
      // Every configuration of the binding is gated, not just the default. "When a change
      // does not do what it should, the first hypothesis is that it is not running", and
      // an arm that agreed because it silently fell back to the other path would pass a
      // gate that only ran one of them. Three real defects in this branch were found by
      // asking that question.
      arms.add(new FfiArm("ffi", true, false));
      arms.add(new FfiArm("ffi-nobatch", false, false));
      arms.add(new FfiArm("ffi-zeroed", true, true));
      arms.add(new FfiArm("ffi-nobatch-zeroed", false, true));
      // Decision 13's arm is emitted for the TARGET only, so the floor build registers
      // nothing here rather than failing to compile. Reflection, so a missing arm is a
      // stated skip and never a silent pass.
      try {
        arms.add((Conformance.Arm) Class.forName("ak.BorrowArm").getDeclaredConstructor()
                 .newInstance());
      } catch (ClassNotFoundException e) {
        log.append("NOTE: ak.BorrowArm is absent from this build (open decision 13's arm"
                   + " is emitted at the target level only).\n");
      }
      log.append("NOTE: compact strings (JDK 9+ String.value/coder) are ")
         .append(new FfiArm().b.usingCompactStrings() ? "IN USE" : "NOT available")
         .append(" in this build.\n");
    } else {
      log.append("NOTE: -Dak.lib is unset, so the core-ffi arms are NOT in this gate.\n");
    }
    if (System.getProperty("ak.pb", "1").equals("1")) {
      arms.add((Conformance.Arm) Class.forName("ak.PbArm").getDeclaredConstructor()
               .newInstance());
    }

    Conformance.Result r = Conformance.run(arms, log, verbose);
    log.append("checked=").append(r.checked).append("  failed=").append(r.failed)
       .append('\n');
    for (String n : r.notes) log.append("  ! ").append(n).append('\n');
    System.out.print(log);
    if (r.failed != 0) System.exit(1);
  }

  /** Arm R: the generated pure-Java codec. */
  static final class RArm implements Conformance.Arm {
    private final Enc e = new Enc(Codec.SITES);
    private final Dec d = new Dec();

    @Override public String name() { return "R"; }

    @Override public byte[] encode(String id, int cs) {
      if (!Arms.encodable(id)) return null;
      Object o = Arms.build(id, cs);
      Arms.encodeR(id, o, e);
      return e.toBytes();
    }

    @Override public byte[] roundTrip(String id, byte[] wire) {
      Object o = Arms.decodeR(id, d, wire, 0, wire.length);
      Arms.encodeR(id, o, e);
      return e.toBytes();
    }
  }
}
