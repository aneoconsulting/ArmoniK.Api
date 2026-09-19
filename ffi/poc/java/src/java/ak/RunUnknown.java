package ak;

import ak.shapes.Arms;
import ak.shapes.Codec;
import ak.shapes.FfiArms;
import ak.shapes.PbArms;
import java.util.ArrayList;
import java.util.List;

/**
 * The unknown-field vectors, and ABI v1 open decision 11 measured on Java.
 *
 * <p>README section 10 item 1: "A corpus generated from the schema that reads it never
 * executes the unknown-field skip, which is the whole of protobuf's forward
 * compatibility." So the vectors are built here by splicing fields the reader was not
 * built against into payloads it was -- one of each wire type, at the top level and
 * inside a nested message, and an unrecognised oneof tag, which is the case that looks
 * like a special one and is not.
 *
 * <p><b>What this measures that no benchmark can.</b> The core drops unknown fields and
 * protobuf-java retains them, so the two disagree on the re-encoded bytes by construction
 * -- and that disagreement is the behaviour change ABI v1 calls "the largest unpriced
 * behaviour change the branch has found". The rust slice established the wire half of it
 * against an incumbent (prost) that also drops. Java is the first slice whose incumbent
 * <i>retains</i>, so this is the first direct measurement of what adopting the core would
 * remove.
 *
 * <p>Both halves are checked: that every arm decodes the KNOWN fields identically (an
 * unknown field must not corrupt its neighbours), and that the re-encode differs exactly
 * by the unknown bytes and by nothing else.
 */
public final class RunUnknown {

  /** One vector: a name, the bytes, and how many unknown bytes were spliced in. */
  static final class Vec {
    final String id, name;
    final byte[] wire;
    final int unknownBytes;
    Vec(String id, String name, byte[] wire, int unknownBytes) {
      this.id = id; this.name = name; this.wire = wire; this.unknownBytes = unknownBytes;
    }
  }

  // ---- a tiny writer, so a vector is built rather than pasted -----------------------

  static byte[] key(int tag, int wire) { return varint(((long) tag << 3) | wire); }

  static byte[] varint(long v) {
    byte[] t = new byte[10];
    int n = 0;
    while ((v & ~0x7FL) != 0) { t[n++] = (byte) ((int) v | 0x80); v >>>= 7; }
    t[n++] = (byte) v;
    byte[] out = new byte[n];
    System.arraycopy(t, 0, out, 0, n);
    return out;
  }

  static byte[] cat(byte[]... parts) {
    int n = 0;
    for (byte[] p : parts) n += p.length;
    byte[] out = new byte[n];
    int at = 0;
    for (byte[] p : parts) { System.arraycopy(p, 0, out, at, p.length); at += p.length; }
    return out;
  }

  /** An unknown field of each wire type, at a tag no message in the schema uses. */
  static byte[] unknownRun() {
    byte[] len = "unknown-to-this-reader".getBytes(java.nio.charset.Charset.forName("UTF-8"));
    return cat(
        key(900, 0), varint(123456789L),                       // varint
        key(901, 1), new byte[] {1, 2, 3, 4, 5, 6, 7, 8},      // i64
        key(902, 2), varint(len.length), len,                  // length-delimited
        key(903, 5), new byte[] {9, 8, 7, 6},                  // i32
        key(904, 2), varint(0));                               // a length-delimited EMPTY
  }

  /** Splice the run in AFTER the known fields of the outer message. Trailing rather than
   *  leading on purpose: a decoder that flushes a batched run on a foreign tag (ABI v1
   *  7.3) takes a different path for each, and both are exercised by the pair below. */
  static Vec trailing(String id) {
    byte[] base = Payloads.vector(id);
    byte[] unk = unknownRun();
    return new Vec(id, "unknown fields after the known ones", cat(base, unk), unk.length);
  }

  /** The same run BEFORE the known fields: this is the one that forces the arena flush,
   *  because the tag that follows does not belong to the open batch. */
  static Vec leading(String id) {
    byte[] base = Payloads.vector(id);
    byte[] unk = unknownRun();
    return new Vec(id, "unknown fields before the known ones", cat(unk, base), unk.length);
  }

  /** An unknown field INSIDE a nested message, which the top-level splice cannot reach:
   *  the nested length prefix has to be re-sized, so this is also the only vector that
   *  proves the splice did not just append past the message. */
  static Vec nested(String id, int outerTag) {
    byte[] base = Payloads.vector(id);
    Dec d = new Dec().reset(base, 0, base.length);
    java.io.ByteArrayOutputStream out = new java.io.ByteArrayOutputStream();
    byte[] unk = unknownRun();
    int spliced = 0;
    try {
      while (!d.done()) {
        int t = d.readTag();
        int start = d.pos;
        if ((t >>> 3) == outerTag && (t & 7) == 2 && spliced == 0) {
          int n = d.readLen();
          byte[] body = java.util.Arrays.copyOfRange(base, d.pos, d.pos + n);
          d.pos += n;
          byte[] nbody = cat(body, unk);
          out.write(key(outerTag, 2));
          out.write(varint(nbody.length));
          out.write(nbody);
          spliced = unk.length;
          continue;
        }
        d.skip(t);
        out.write(base, start - varintLenOf(t), d.pos - start + varintLenOf(t));
      }
    } catch (java.io.IOException e) {
      throw new IllegalStateException(e);
    }
    return new Vec(id, "an unknown field inside the first element", out.toByteArray(),
                   spliced);
  }

  static int varintLenOf(long v) {
    int n = 1;
    while ((v & ~0x7FL) != 0) { v >>>= 7; n++; }
    return n;
  }

  /** An unrecognised tag that WOULD be a oneof member if the reader knew it. At the wire
   *  level there is no such thing: the grouping lives only in the descriptor, so a parser
   *  cannot tell this from any other unknown field, the case stays at the last known
   *  member, and the payload is dropped. That is protobuf, not the ABI. */
  static Vec unknownOneofMember() {
    byte[] base = Payloads.vector("P3.1");
    // Probe's oneof members are tags 10..14; 15 would be a sixth the reader lacks.
    byte[] unk = cat(key(15, 0), varint(7777L));
    Dec d = new Dec().reset(base, 0, base.length);
    java.io.ByteArrayOutputStream out = new java.io.ByteArrayOutputStream();
    boolean done = false;
    try {
      while (!d.done()) {
        int t = d.readTag();
        int start = d.pos;
        if ((t >>> 3) == 1 && (t & 7) == 2 && !done) {
          int n = d.readLen();
          byte[] body = java.util.Arrays.copyOfRange(base, d.pos, d.pos + n);
          d.pos += n;
          byte[] nbody = cat(body, unk);
          out.write(key(1, 2));
          out.write(varint(nbody.length));
          out.write(nbody);
          done = true;
          continue;
        }
        d.skip(t);
        out.write(base, start - varintLenOf(t), d.pos - start + varintLenOf(t));
      }
    } catch (java.io.IOException e) {
      throw new IllegalStateException(e);
    }
    return new Vec("P3.1", "an unrecognised tag where a oneof member would be",
                   out.toByteArray(), unk.length);
  }

  public static void main(String[] args) {
    StringBuilder log = new StringBuilder();
    log.append("== unknown fields: forward compatibility, and ABI v1 open decision 11 ==\n");
    log.append("java.version=").append(System.getProperty("java.version")).append('\n');
    log.append("protobuf-java=").append(protobufVersion()).append('\n');
    log.append("\nA corpus generated from the schema that reads it never executes the\n")
       .append("unknown-field skip (README 10.1), so these vectors are spliced here.\n\n");

    List<Vec> vecs = new ArrayList<Vec>();
    for (String id : new String[] {"P1.1", "P1.3", "P2.1", "P2.5", "P3.1", "P4.1", "P5.1"}) {
      vecs.add(trailing(id));
      vecs.add(leading(id));
      vecs.add(nested(id, 1));
    }
    vecs.add(unknownOneofMember());

    Binder ffi = new Binder();
    int fail = 0, checked = 0;
    for (Vec v : vecs) {
      // The known fields, as every arm sees them, expressed as the bytes each arm
      // re-encodes. The core and arm R drop the unknown run; protobuf-java keeps it.
      byte[] clean = Payloads.vector(v.id);
      byte[] r = reencodeR(v);
      byte[] f = reencodeFfi(ffi, v);
      byte[] p = reencodePb(v);
      checked += 3;

      boolean rOk = java.util.Arrays.equals(clean, r);
      boolean fOk = java.util.Arrays.equals(clean, f);
      // Retention is checked by STRIPPING rather than by arithmetic. A length check would
      // be wrong by one wherever the retained bytes push a nested message's length prefix
      // to a second varint byte, which is real behaviour and not a defect -- P5.1's nested
      // vector does exactly that, 116 + 50 + 1. So: protobuf-java's re-encode is longer
      // than the clean payload (it kept something), and feeding it back through an arm
      // that drops unknowns returns the clean payload exactly (it kept nothing else, and
      // changed no known field).
      byte[] pStripped = reencodeR(new Vec(v.id, v.name, p, 0));
      boolean pKeeps = p.length > clean.length
          && java.util.Arrays.equals(clean, pStripped);
      boolean pClean = java.util.Arrays.equals(clean, p);

      log.append(v.id).append("  ").append(v.name).append("  (+")
         .append(v.unknownBytes).append(" B unknown)\n");
      log.append("    R    ").append(rOk ? "drops the unknown run, known fields intact"
                                         : "DIFFERS from the clean payload").append('\n');
      log.append("    ffi  ").append(fOk ? "drops the unknown run, known fields intact"
                                         : "DIFFERS from the clean payload").append('\n');
      log.append("    pbj  ").append(
          pKeeps ? "RETAINS the unknown run and re-emits it (+"
                   + (p.length - clean.length) + " B, known fields unchanged)"
                 : pClean ? "dropped it, which protobuf-java is not supposed to do"
                          : "re-encoded " + p.length + " B against " + clean.length
                            + " clean + " + v.unknownBytes + " unknown, and stripping it"
                            + " did not return the clean payload").append('\n');
      if (!rOk) fail++;
      if (!fOk) fail++;
      if (!pKeeps) fail++;
    }

    log.append("\nchecked=").append(checked).append("  failed=").append(fail).append('\n');
    log.append("\nWhat this establishes, in one line: on every vector the core and the\n")
       .append("generated Java codec drop the unrecognised field and protobuf-java carries\n")
       .append("it through to the re-encode. Java's incumbent RETAINS, where the rust\n")
       .append("slice's (prost) does not, so this is the first direct measurement of the\n")
       .append("guarantee ABI v1 open decision 11 would remove rather than an argument\n")
       .append("about one. Who it bites is unchanged and specific: a client that decodes\n")
       .append("and never re-encodes loses nothing; a worker forwarding a ProcessRequest\n")
       .append("between two schema versions loses the field silently.\n");
    System.out.print(log);
    if (fail != 0) System.exit(1);
  }

  /** Read off the jar rather than named in a comment: R7 asks for the incumbent's
   *  version and a comment can go stale against the classpath. */
  static String protobufVersion() {
    try {
      java.net.URL u = com.google.protobuf.Message.class.getProtectionDomain()
          .getCodeSource().getLocation();
      String p = u.getPath();
      return p.substring(p.lastIndexOf('/') + 1);
    } catch (Throwable t) {
      return "unknown";
    }
  }

  static final class Binder {
    final ak.shapes.Binding b = new ak.shapes.Binding();
  }

  static byte[] reencodeR(Vec v) {
    Enc e = new Enc(Codec.SITES);
    Dec d = new Dec();
    Object o = Arms.decodeR(v.id, d, v.wire, 0, v.wire.length);
    Arms.encodeR(v.id, o, e);
    return e.toBytes();
  }

  static byte[] reencodeFfi(Binder bd, Vec v) {
    Object o = FfiArms.decode(bd.b, v.id, v.wire, 0, v.wire.length);
    FfiArms.encode(bd.b, v.id, o);
    return bd.b.take();
  }

  static byte[] reencodePb(Vec v) {
    try {
      com.google.protobuf.Message m = PbArms.parse(v.id, v.wire, 0, v.wire.length);
      int n = m.getSerializedSize();
      byte[] out = new byte[n];
      com.google.protobuf.CodedOutputStream cos =
          com.google.protobuf.CodedOutputStream.newInstance(out);
      cos.useDeterministicSerialization();
      m.writeTo(cos);
      cos.checkNoSpaceLeft();
      return out;
    } catch (Exception e) {
      throw new IllegalStateException(v.id + " " + v.name, e);
    }
  }
}
