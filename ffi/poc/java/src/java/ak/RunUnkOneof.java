package ak;

import java.io.ByteArrayOutputStream;
import java.util.Arrays;

/**
 * Decision 11 rule 4 on the JVM (FIX-PLAN R-H8): a oneof is one position with one buffer,
 * handed once in the root's options; after decoding it is found in the ACTIVE member's
 * decode group, and a buffer left in an inactive member's slot (after a switch to a scalar
 * member) is the host's to free. The corpus has no row with unknowns inside a oneof member,
 * so this runs the C++ slice's three sequences (poc/cpp/src/conformance.cpp, d11_oneof) on
 * ListProbeResponse { probes: [ Probe { id "p", body ... } ] }, each message member's body
 * being one unknown run (field 100, varint i+1):
 *
 * <pre>
 *   stamp -> nothing            final member as_nothing, bag = run(2)
 *   stamp -> nothing -> stamp   final member as_stamp,   bag = run(3)
 *   stamp -> as_int (scalar)    final member as_int = 5, no bag; the emptied buffer freed
 * </pre>
 *
 * through the retain arms ffi-retain (push), ffi-pull-retain (drain) and ffi-pull-walk-retain.
 * Checked per (sequence, arm): the decode succeeds, the final case and bag are exact, the
 * binding left no buffer undelivered ({@code unkLeftAfterSuccess == 0}) and the shim holds
 * no live buffer afterwards ({@code Native.unkLive() == 0}). Full build only.
 * {@code -Dak.unk.oneofplant=1} is a planted defect: the decode is not armed (drop mode), so
 * every message member's bag is missing and the control must fail.
 */
public final class RunUnkOneof {
  static void ld(int tag, byte[] body, ByteArrayOutputStream out) {
    long k = ((long) tag << 3) | 2;
    while (k >= 0x80) { out.write((int) (k | 0x80)); k >>>= 7; }
    out.write((int) k);
    long n = body.length;
    while (n >= 0x80) { out.write((int) (n | 0x80)); n >>>= 7; }
    out.write((int) n);
    out.write(body, 0, body.length);
  }

  static byte[] run(int x) { return new byte[] {(byte) 0xa0, 0x06, (byte) x}; }

  public static void main(String[] args) {
    String[] names = {"stamp -> nothing", "stamp -> nothing -> stamp", "stamp -> as_int (scalar)"};
    int[][] tags = {{13, 14}, {13, 14, 13}, {13, 10}};
    int[] last = {2, 3, 0};
    String[] arms = {"ffi-retain", "ffi-pull-retain", "ffi-pull-walk-retain"};
    boolean ok = true;
    new ak.shapes.Binding().close();                 // loads and binds the shim
    for (int s = 0; s < 3; s++) {
      ByteArrayOutputStream p = new ByteArrayOutputStream();
      ld(1, "p".getBytes(java.nio.charset.StandardCharsets.UTF_8), p);
      for (int i = 0; i < tags[s].length; i++) {
        if (tags[s][i] == 10) { p.write(10 << 3); p.write(5); }
        else ld(tags[s][i], run(i + 1), p);
      }
      ByteArrayOutputStream b = new ByteArrayOutputStream();
      ld(1, p.toByteArray(), b);
      byte[] w = b.toByteArray();
      for (String arm : arms) {
        ak.shapes.Binding bd = new ak.shapes.Binding();
        ak.shapes.FfiArms.setRetain(bd, !"1".equals(System.getProperty("ak.unk.oneofplant")));
        bd.pullWalk = arm.equals("ffi-pull-walk-retain");
        long before = Native.unkLive();
        String why = null;
        try {
          ak.shapes.ListProbeResponse r = arm.equals("ffi-retain")
              ? bd.decodeListProbeResponse(w, 0, w.length) : bd.parseListProbeResponse(w, 0, w.length);
          ak.shapes.Probe pr = r.probes.size() == 1 ? r.probes.get(0) : null;
          if (pr == null) why = "probes: " + r.probes.size();
          else if (last[s] == 0) {
            if (pr.body_case != ak.shapes.Probe.BODY_CASE_AS_INT || pr.body_as_int != 5) why = "case " + pr.body_case;
          } else {
            int want = last[s] == 2 ? ak.shapes.Probe.BODY_CASE_AS_NOTHING : ak.shapes.Probe.BODY_CASE_AS_STAMP;
            byte[] bag = want == ak.shapes.Probe.BODY_CASE_AS_NOTHING
                ? (pr.body_as_nothing == null ? null : pr.body_as_nothing.unknownFields)
                : (pr.body_as_stamp == null ? null : pr.body_as_stamp.unknownFields);
            if (pr.body_case != want) why = "case " + pr.body_case;
            else if (!Arrays.equals(bag, run(last[s]))) why = "bag " + (bag == null ? "null" : Payloads.hex(bag));
          }
        } catch (RuntimeException e) {
          why = e.toString();
        }
        long left = ak.shapes.FfiArms.unkCounters(bd)[1];
        long live = Native.unkLive() - before;
        bd.close();
        if (why == null && (left != 0 || live != 0)) why = "left undelivered " + left + ", alive " + live;
        System.out.println(String.format("  oneof %-26s %-21s %s", names[s], arm,
            why == null ? "final case and bag exact; left undelivered 0; alive 0" : "FAIL: " + why));
        ok &= why == null;
      }
    }
    System.out.println(ok ? "ONEOF CONTROL PASSED" : "ONEOF CONTROL FAILED");
    System.exit(ok ? 0 : 1);
  }
}
