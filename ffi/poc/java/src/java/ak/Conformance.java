package ak;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * Correctness before timing (README R2). Nothing in this slice is measured until this
 * passes.
 *
 * <p>What it checks, per payload and per arm:
 * <ol>
 *   <li><b>byte identity against the manifest</b>, which the rust slice validated against
 *       prost and a second independent encoder;</li>
 *   <li><b>byte identity between this slice's own arms</b>, which is what R2 is actually
 *       protecting;</li>
 *   <li><b>a decode round trip</b> that re-encodes to the arm's own form;</li>
 *   <li><b>the committed vector</b>, where the manifest carries one, so a hash collision
 *       is not the only thing standing between this slice and a wrong byte.</li>
 * </ol>
 *
 * <h2>P2.5 and the two valid encodings</h2>
 *
 * <p>An empty map <i>value</i> is an implicit-presence leaf holding the proto zero, so the
 * canonical form omits it and protobuf C++ and upb both write it at 2 B per entry
 * (design/SHAPES.md). On P2.5 that is 40 emptied values across 20 elements: 19,712 B where
 * the manifest records 19,632. Both are valid proto3 and neither encoder is wrong, so
 * "every arm produces the same bytes" is <b>false here and taken literally would raise a
 * false defect on the one payload whose purpose is the absent path</b>.
 *
 * <p>What replaces it is stronger than a waiver, because SHAPES.md also requires a slice
 * to <i>parse</i> both. Where two arms write different forms, this checks the full cross
 * product: each arm parses the other's bytes and re-encodes to its own form. A wrong byte
 * in either form fails that, and so does an arm that silently ignores the field.
 */
public final class Conformance {
  /** One arm under test. `encode` returns null where the arm cannot encode a payload,
   *  which P7.1 is: no canonical writer can produce interleaved repeated fields. */
  public interface Arm {
    String name();
    byte[] encode(String id, int cs);
    /** Decode `wire`, then re-encode in this arm's own form. */
    byte[] roundTrip(String id, byte[] wire);
  }

  public static final class Result {
    public int checked, failed;
    public final List<String> notes = new ArrayList<String>();
    /** Which form each arm wrote, for the payloads that have more than one. */
    public final Map<String, String> forms = new LinkedHashMap<String, String>();
  }

  /** The payloads for which more than one encoding is valid, and by how much the
   *  alternative differs. design/SHAPES.md names P2.5 and says why. */
  static int altDelta(String id) {
    return id.equals("P2.5") ? 80 : -1;
  }

  public static Result run(List<Arm> arms, Appendable log, boolean verbose) {
    return run(arms, log, verbose, Values.ASCII);
  }

  /**
   * With a content set other than ASCII there is no manifest to check against:
   * `schema/` emits the ASCII set only. design/SHAPES.md says what replaces it -- "a slice
   * checks a content set by byte identity of all its arms against the INCUMBENT arm, which
   * the manifest validated on ASCII, plus a decode round trip per set" -- and that is what
   * the cross-arm and round-trip halves below already do. Only the manifest comparison is
   * skipped, and it is skipped loudly.
   */
  public static Result run(List<Arm> arms, Appendable log, boolean verbose, int cs) {
    Result res = new Result();
    final boolean manifest = cs == Values.ASCII;
    for (Map.Entry<String, Payloads.Row> e : Payloads.all().entrySet()) {
      String id = e.getKey();
      Payloads.Row row = e.getValue();
      Map<String, byte[]> wrote = new LinkedHashMap<String, byte[]>();
      Map<String, String> form = new LinkedHashMap<String, String>();

      for (Arm a : arms) {
        byte[] got = a.encode(id, cs);
        if (got == null) {
          note(log, verbose, id + "  " + a.name() + "  encode: not applicable");
          continue;
        }
        res.checked++;
        String h = Payloads.hex(Values.sha256(got));
        if (!manifest) {
          // No oracle for this set. The arms still have to agree with each other, which is
          // what the cross product below checks, and each still has to round-trip.
          // With no manifest the two valid encodings of P2.5 still exist -- an empty map
          // value is zero-length in every content set, so protobuf-java is still +2 B per
          // emptied value -- and they cannot be told apart by comparing with an ASCII
          // length. So the form is the length itself: arms that wrote the same number of
          // bytes must be byte-identical, and arms that did not must each parse the
          // other's bytes and return their own. That is the same check, keyed differently.
          form.put(a.name(), "len:" + got.length);
          wrote.put(a.name(), got);
          note(log, verbose, id + "  " + a.name() + "  encode " + got.length
              + " B (no manifest for this content set)");
          continue;
        }
        if (h.equals(row.sha256)) {
          form.put(a.name(), "canonical");
        } else if (altDelta(id) > 0 && got.length == row.bytes + altDelta(id)) {
          form.put(a.name(), "both-fields-written");
        } else {
          fail(res, log, id + "  " + a.name() + "  ENCODE MISMATCH: " + got.length
              + " B sha " + h.substring(0, 16) + " against manifest " + row.bytes
              + " B sha " + row.sha256.substring(0, 16));
          continue;
        }
        byte[] vec = Payloads.vector(id);
        if (vec != null && "canonical".equals(form.get(a.name()))
            && !java.util.Arrays.equals(vec, got)) {
          fail(res, log, id + "  " + a.name() + "  VECTOR MISMATCH at byte "
              + firstDiff(vec, got));
          continue;
        }
        wrote.put(a.name(), got);
        res.forms.put(id + "/" + a.name(), form.get(a.name()));
        note(log, verbose, id + "  " + a.name() + "  encode ok (" + form.get(a.name())
            + ", " + got.length + " B)");
      }

      // ---- cross-arm: same form must be identical, different forms must interoperate
      for (Map.Entry<String, byte[]> x : wrote.entrySet()) {
        for (Map.Entry<String, byte[]> y : wrote.entrySet()) {
          if (x.getKey().compareTo(y.getKey()) >= 0) continue;
          res.checked++;
          boolean sameForm = form.get(x.getKey()).equals(form.get(y.getKey()));
          if (sameForm) {
            if (!java.util.Arrays.equals(x.getValue(), y.getValue()))
              fail(res, log, id + "  " + x.getKey() + " vs " + y.getKey()
                  + "  DISAGREE at byte " + firstDiff(x.getValue(), y.getValue()));
            else
              note(log, verbose, id + "  " + x.getKey() + " == " + y.getKey());
          } else {
            // Both forms are valid, so each arm must PARSE the other's and come back
            // with its own. That is the check design/SHAPES.md actually asks for.
            crossParse(res, log, verbose, id, arms, x.getKey(), y.getValue(),
                       x.getValue(), form);
            crossParse(res, log, verbose, id, arms, y.getKey(), x.getValue(),
                       y.getValue(), form);
          }
        }
      }

      // ---- round trip, against the manifest's own bytes where they exist
      byte[] wire = manifest ? Payloads.vector(id) : null;
      if (wire == null && !wrote.isEmpty()) {
        for (Map.Entry<String, byte[]> w : wrote.entrySet())
          if (!manifest || "canonical".equals(form.get(w.getKey()))) {
            wire = w.getValue();
            break;
          }
      }
      if (wire == null) continue;
      for (Arm a : arms) {
        byte[] again = a.roundTrip(id, wire);
        if (again == null) {
          note(log, verbose, id + "  " + a.name() + "  round trip: not applicable");
          continue;
        }
        res.checked++;
        byte[] want = wrote.get(a.name());
        if (want == null) want = wire;      // an arm that cannot build it still decodes it
        if (java.util.Arrays.equals(want, again)) {
          note(log, verbose, id + "  " + a.name() + "  round trip ok");
        } else if (id.equals("P7.1") && Triples.samePermutation(wire, again)) {
          // P7.1 re-encodes contiguously, so it is a permutation of the same triples
          // rather than the same bytes. design/SHAPES.md asks for exactly that.
          note(log, verbose, id + "  " + a.name()
              + "  round trip ok (permutation of the same triples)");
        } else {
          fail(res, log, id + "  " + a.name() + "  ROUND TRIP MISMATCH at byte "
              + firstDiff(want, again) + " (" + wire.length + " B in, " + again.length
              + " B out, own form " + want.length + " B)");
        }
      }
    }
    return res;
  }

  /** `who` parses `other`'s bytes and must come back with its own form. */
  static void crossParse(Result res, Appendable log, boolean verbose, String id,
                         List<Arm> arms, String who, byte[] foreign, byte[] own,
                         Map<String, String> form) {
    for (Arm a : arms) {
      if (!a.name().equals(who)) continue;
      res.checked++;
      byte[] back = a.roundTrip(id, foreign);
      if (back == null) return;
      if (!java.util.Arrays.equals(own, back))
        fail(res, log, id + "  " + who + "  CANNOT PARSE THE " + otherForm(form, who)
            + " FORM: re-encoded " + back.length + " B, own form " + own.length
            + " B, first difference at " + firstDiff(own, back));
      else
        note(log, verbose, id + "  " + who + "  parses the other valid form and returns"
            + " its own (" + own.length + " B)");
    }
  }

  static String otherForm(Map<String, String> form, String who) {
    return "canonical".equals(form.get(who)) ? "both-fields-written" : "canonical";
  }

  static int firstDiff(byte[] a, byte[] b) {
    int n = Math.min(a.length, b.length);
    for (int i = 0; i < n; i++) if (a[i] != b[i]) return i;
    return n;
  }

  static void note(Appendable log, boolean verbose, String s) {
    if (verbose) append(log, "  " + s + "\n");
  }

  static void fail(Result r, Appendable log, String s) {
    r.failed++;
    r.notes.add(s);
    append(log, "  FAIL " + s + "\n");
  }

  static void append(Appendable log, String s) {
    try {
      log.append(s);
    } catch (java.io.IOException e) {
      throw new IllegalStateException(e);
    }
  }
}
