package ak;

import ak.shapes.Binding;
import ak.shapes.FfiArms;

/**
 * ABI v1 section 7.1's <b>pull</b> family under the correctness gate.
 *
 * <p>The oracle is byte identity after a round trip, which is the right one here: the pull
 * arm and the push arm replay through the <em>same</em> per-slot host code, so if the
 * record stream were misordered, mis-slotted or missing a token the object graph would
 * differ and the re-encode would say so. A record stream is the call sequence push would
 * have made, and this gate is what checks that claim on this host rather than quoting it.
 *
 * <p>Registered twice, drained and walked, for the reason {@link FfiArm} is registered four
 * times: an arm that is byte-identical because it fell back to the other path would pass a
 * gate that only ran one of them.
 */
public final class FfiPullArm implements Conformance.Arm {
  final Binding b = new Binding();
  private final String label;

  public FfiPullArm(String label, boolean walk) {
    this.label = label;
    b.pullWalk = walk;
  }

  @Override public String name() { return label; }

  /** Encode is not this arm's question: the family is a decode delivery. */
  @Override public byte[] encode(String id, int cs) { return null; }

  @Override public byte[] roundTrip(String id, byte[] wire) {
    Object o = FfiArms.parse(b, id, wire, 0, wire.length);
    FfiArms.encode(b, id, o);
    return b.take();
  }
}
