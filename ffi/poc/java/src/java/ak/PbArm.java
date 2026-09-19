package ak;

import ak.shapes.PbArms;
import com.google.protobuf.CodedOutputStream;
import com.google.protobuf.Message;

/**
 * The incumbent under the correctness gate, and the object the bench measures.
 *
 * <p>Deterministic serialization is on for the gate because the canonical form of
 * {@code schema/README.md} sorts map entries and protobuf-java does not unless asked.
 * What it costs is measured as its own row rather than folded into the headline.
 */
public final class PbArm implements Conformance.Arm {
  private byte[] scratch = new byte[1 << 16];

  @Override public String name() { return "pbj"; }

  @Override public byte[] encode(String id, int cs) {
    Message m = PbArms.build(id, cs);
    if (m == null) return null;
    return deterministic(m);
  }

  @Override public byte[] roundTrip(String id, byte[] wire) {
    try {
      return deterministic(PbArms.parse(id, wire, 0, wire.length));
    } catch (com.google.protobuf.InvalidProtocolBufferException e) {
      throw new IllegalStateException(id, e);
    }
  }

  /** Sorted map entries, which is what the manifest's canonical form requires. */
  public byte[] deterministic(Message m) {
    try {
      int n = m.getSerializedSize();
      byte[] out = new byte[n];
      CodedOutputStream cos = CodedOutputStream.newInstance(out);
      cos.useDeterministicSerialization();
      m.writeTo(cos);
      cos.checkNoSpaceLeft();
      return out;
    } catch (java.io.IOException e) {
      throw new IllegalStateException(e);
    }
  }

  /** The library's own entry point, and what an application calls. */
  public byte[] toByteArray(Message m) { return m.toByteArray(); }

  /** The same work into a REUSED buffer, which is what the core arms get. Returns the
   *  byte count; the bytes are in {@link #buffer()}. */
  public int writeReused(Message m) {
    try {
      int n = m.getSerializedSize();
      if (scratch.length < n) scratch = new byte[Integer.highestOneBit(n - 1) * 2];
      CodedOutputStream cos = CodedOutputStream.newInstance(scratch, 0, n);
      m.writeTo(cos);
      cos.checkNoSpaceLeft();
      return n;
    } catch (java.io.IOException e) {
      throw new IllegalStateException(e);
    }
  }

  public int writeReusedDeterministic(Message m) {
    try {
      int n = m.getSerializedSize();
      if (scratch.length < n) scratch = new byte[Integer.highestOneBit(n - 1) * 2];
      CodedOutputStream cos = CodedOutputStream.newInstance(scratch, 0, n);
      cos.useDeterministicSerialization();
      m.writeTo(cos);
      cos.checkNoSpaceLeft();
      return n;
    } catch (java.io.IOException e) {
      throw new IllegalStateException(e);
    }
  }

  public byte[] buffer() { return scratch; }
}
