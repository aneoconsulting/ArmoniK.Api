package ak;

import ak.shapes.Binding;
import ak.shapes.FfiArms;

/**
 * Arm `core-ffi` under the correctness gate.
 *
 * <p>The gate registers it once per configuration -- batched and unbatched, total fill and
 * decision 9's sparse fill -- because "when a change does not do what it should, the first
 * hypothesis is that it is not running", and an arm that is byte-identical because it fell
 * back to the other path would pass a gate that only ran one of them.
 */
public final class FfiArm implements Conformance.Arm {
  final Binding b = new Binding();
  private final String label;

  public FfiArm() { this("ffi", true, false); }

  public FfiArm(String label, boolean batch, boolean zeroed) {
    this.label = label;
    b.batch = batch;
    b.zeroed = zeroed;
  }

  @Override public String name() { return label; }

  @Override public byte[] encode(String id, int cs) {
    if (!FfiArms.encodable(id)) return null;
    Object o = ak.shapes.Arms.build(id, cs);
    FfiArms.encode(b, id, o);
    return b.take();
  }

  @Override public byte[] roundTrip(String id, byte[] wire) {
    Object o = FfiArms.decode(b, id, wire, 0, wire.length);
    FfiArms.encode(b, id, o);
    return b.take();
  }
}
