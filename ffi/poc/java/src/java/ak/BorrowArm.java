package ak;

/**
 * ABI v1 open decision 13's arm under the correctness gate.
 *
 * <p>It is gated for one reason: a borrowed view is only worth measuring if it holds the
 * right bytes, and the only way to know is to round-trip it against the same manifest as
 * every other arm. The C++ slice's equivalent arm reported a decode 0.234 of protobuf and
 * the number is only meaningful because the bytes came back.
 *
 * <p>Its ENCODE column is not quoted anywhere: a `Utf8View` already holds UTF-8, so it
 * stages as a memcpy where the owning facade transcodes from UTF-16. That is a different
 * mechanism, and decision 13 is a decode question.
 */
public final class BorrowArm implements Conformance.Arm {
  final ak.borrow.Binding b = new ak.borrow.Binding();

  @Override public String name() { return "ffi-borrow"; }

  @Override public byte[] encode(String id, int cs) {
    // The borrowed facade has no payload builder: its values would come from the owning
    // builder and be re-encoded, which measures nothing and proves nothing. It joins the
    // gate on the round trip, which is the direction it exists for.
    return null;
  }

  @Override public byte[] roundTrip(String id, byte[] wire) {
    Object o = ak.borrow.FfiArms.decode(b, id, wire, 0, wire.length);
    ak.borrow.FfiArms.encode(b, id, o);
    return b.take();
  }
}
