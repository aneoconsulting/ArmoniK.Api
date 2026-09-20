package ak;

import ak.shapes.Arms;
import ak.shapes.Binding;
import ak.shapes.Codec;
import ak.shapes.FfiArms;
import ak.shapes.PbArms;
import com.google.protobuf.Message;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

/**
 * The measurement. Read the method before the numbers.
 *
 * <h2>How a figure here is formed</h2>
 *
 * <p><b>Paired ratios, inside one round.</b> R4: "Every ratio is formed inside one process
 * on one runtime", and its sharpened form, "where a question can be asked as a delta
 * between two arms in the same interleaved rounds, ask it that way -- it survives things a
 * ratio to a third arm does not." So every arm runs once per round, the ratio to the
 * incumbent is computed from that round's two numbers, and what is reported is the
 * min/median/max of the per-round ratios. A ratio of two medians taken from different
 * rounds is not reported anywhere.
 *
 * <p><b>Rotating arm order.</b> A fixed order produced a monotone artifact in both the
 * rust and cpp slices: the always-first arm read 0.778 in one invocation and 0.88 in the
 * next. The order rotates by round.
 *
 * <p><b>Signs, not decimals.</b> Absolutes are being re-taken on a controlled physical
 * machine (README R13), so every delta prints whether its lo and hi share a sign. A range
 * that straddles zero is reported as straddling zero and is not quoted over the half that
 * agrees with a conclusion.
 *
 * <h2>Two hazards this harness is built around</h2>
 *
 * <p><b>No formatting until the last measurement is taken.</b> On JDK 21 and later one
 * {@code String.format} with a numeric conversion permanently deoptimises every char
 * narrowing loop in the process, and protobuf-java's own encoder is such a loop (README
 * R9). Everything here accumulates into arrays and formats at the end. {@code -Dak.deopt=1}
 * deliberately triggers it first, so the hazard is measured on this JDK rather than
 * inherited from a report.
 *
 * <p><b>The incumbent is not handicapped.</b> The cpp slice's review moved its encode
 * column by about eight points because its harness cleared and resized protobuf's output
 * every iteration and hand-rolled a stream instead of calling the library's own entry
 * point. So protobuf-java appears here three ways -- {@code toByteArray}, a reused
 * {@code CodedOutputStream}, and the same with deterministic serialization -- and the
 * comparison that carries the verdict is the one where every arm writes into a buffer it
 * already owns.
 */
public final class Bench {

  // ------------------------------------------------------------------ the arm interface

  abstract static class Arm {
    final String name;
    final String note;
    Arm(String name, String note) { this.name = name; this.note = note; }
    /** Run `n` operations on `id`. Return something the JIT cannot discard. */
    abstract long run(String id, int n);
    boolean covers(String id) { return true; }
    /** Called before every round, OUTSIDE the timed region, for an arm that needs state
     *  it may only use once. Every arm's prepare runs before any arm's timed run in that
     *  round, so the heap an arm meets does not depend on where it sits in the rotation. */
    void beforeRound(String id, int n) {}
    /** An upper bound on the iterations this arm can run in one round. The pooled
     *  baseline has one: a message may be serialised once, so the pool is the limit and
     *  the pool is bounded by memory. */
    int maxIters(String id) { return Integer.MAX_VALUE; }
    void close() {}
  }

  // ------------------------------------------------------------------ state per payload

  static Object facade;
  static Message pbmsg;
  static byte[] wire;

  /**
   * One source object per iteration, for EVERY encode arm.
   *
   * <p>The baseline has to walk a fresh message per operation, because protobuf-java
   * memoizes its serialized size. Left there, that would have been a second bias in the
   * opposite direction: the baseline streaming N distinct object trees through the cache
   * while every other arm re-read one hot one. On P1.1 that is 4,096 small messages
   * against a single object, which is a memory-system result and not a codec one.
   *
   * <p>So every encode arm reads {@code facadePool[i]} or {@code pbPool[i]} with the same
   * N. The facade pool is built once per payload -- a facade object can be encoded twice
   * with no state carried between -- and the protobuf pool is rebuilt every round,
   * because a protobuf message cannot.
   */
  static Object[] facadePool = new Object[0];
  static Message[] pbPool = new Message[0];
  static Message[] parsedPool = new Message[0];
  static int poolN;

  public static void main(String[] args) throws Exception {
    final int ROUNDS = Integer.getInteger("ak.rounds", 11);
    final long TARGET_NS = Long.getLong("ak.roundns", 20_000_000L);
    final String only = System.getProperty("ak.only");
    // design/SHAPES.md's three content sets. ASCII is what the manifest pins and what
    // every correctness check runs on; the other two are where a narrowing transcoder has
    // real work to do, and on the JVM they are also where the target's LATIN1 fast path
    // stops applying. A string-path number without one of these named is half a number.
    CS = Integer.getInteger("ak.cs", Values.ASCII);
    final boolean decode = !"0".equals(System.getProperty("ak.decode", "1"));
    final boolean encode = !"0".equals(System.getProperty("ak.encode", "1"));

    // README R9's hazard, triggered ON PURPOSE so it can be measured rather than assumed.
    // The modes exist because the first measurement of mode 1 moved protobuf-java's encode
    // by a factor of two IN THE WRONG DIRECTION, and a mystery that large has to be
    // narrowed to a mechanism before it can be reported.
    //
    //   0  nothing
    //   1  one String.format with a numeric conversion, which is what R9 names
    //   2  read a WIDE String through charAt, and nothing else
    //   3  read a LATIN1 String through charAt, and nothing else
    //
    // Modes 2 and 3 carry no Formatter, no narrowing loop and no numeric conversion. If
    // mode 2 reproduces mode 1 then the mechanism is the TYPE PROFILE of the compact-string
    // branch, and the sign of the effect is a fact about what the process read first.
    int deopt = Integer.getInteger("ak.deopt", 0);
    if (deopt == 1) {
      System.err.print(String.format("%d%n", 1));
    } else if (deopt == 2 || deopt == 3) {
      String probe = deopt == 2 ? new String(new char[] {0x4E2D, 0x6587, 0x4E00})
                                : new String(new char[] {0x00E9, 0x00FF, 0x0061});
      long h = 0;
      for (int i = 0; i < 20000; i++) h += probe.charAt(i % probe.length());
      if (h == -1) System.err.println(h);
    }

    Binding bFfi = new Binding();
    Binding bNoBatch = new Binding(); bNoBatch.batch = false;
    Binding bZeroed = new Binding(); bZeroed.zeroed = true;
    Binding bPull = new Binding();
    Binding bPullWalk = new Binding(); bPullWalk.pullWalk = true;
    ak.borrow.Binding bBorrow = new ak.borrow.Binding();
    PbArm pb = new PbArm();
    Enc enc = new Enc(Codec.SITES);
    Dec dec = new Dec();

    List<Arm> encArms = new ArrayList<Arm>();

    // The baseline is a message serialised ONCE, which is what an application does and
    // what a loop over one message does not. protobuf-java memoizes getSerializedSize()
    // on the instance, and both toByteArray and writeTo call it, so from iteration two
    // the size pass -- a full walk computing every string's UTF-8 length -- is free for
    // the incumbent and for nobody else. `logs/java/baseline.log` prices it at 1.5 to 2.8
    // times the write on every element-bearing payload, which is larger than the effect
    // this slice exists to measure.
    encArms.add(new Arm("pbj", "toByteArray on a message serialised ONCE, which is what"
        + " an application does: the size pass is paid, not memoized away") {
      long run(String id, int n) {
        long s = 0;
        for (int i = 0; i < n; i++) s += pbPool[i].toByteArray().length;
        return s;
      }
    });
    encArms.add(new Arm("pbj-reused-out", "a fresh message, written into a buffer the"
        + " caller owns: the exact pair for `R` and `ffi`") {
      long run(String id, int n) {
        long s = 0;
        for (int i = 0; i < n; i++) s += pb.writeReused(pbPool[i]);
        return s;
      }
    });
    encArms.add(new Arm("pbj-parsed", "toByteArray on a message that was PARSED rather"
        + " than built: its string fields hold ByteString, so it re-serialises with no"
        + " UTF-8 transcoding at all") {
      long run(String id, int n) {
        long s = 0;
        for (int i = 0; i < n; i++) s += parsedPool[i].toByteArray().length;
        return s;
      }
    });
    encArms.add(new Arm("pbj-loop", "the same object serialised again and again, with the"
        + " size memoized: what a loop benchmark reports, and not an application's cost") {
      long run(String id, int n) {
        long s = 0;
        for (int i = 0; i < n; i++) s += pb.toByteArray(pbPool[0]).length;
        return s;
      }
    });
    List<Arm> decArms = new ArrayList<Arm>();

    // ---- encode, all writing into a buffer the arm already owns -----------------------
    encArms.add(new Arm("pbj-reuse", "writeTo a reused CodedOutputStream, size memoized:"
        + " the fastest thing protobuf-java can be made to do here") {
      long run(String id, int n) {
        long s = 0;
        for (int i = 0; i < n; i++) s += pb.writeReused(pbPool[0]);
        return s;
      }
    });
    encArms.add(new Arm("pbj-det", "pbj-reuse with deterministic serialization, which is"
        + " what sorts map entries and is what byte identity needs") {
      long run(String id, int n) {
        long s = 0;
        for (int i = 0; i < n; i++) s += pb.writeReusedDeterministic(pbPool[0]);
        return s;
      }
    });
    encArms.add(new Arm("R", "the generated pure-Java codec: README R3's no-boundary"
        + " control, and section 13 outcome 2's architecture") {
      long run(String id, int n) {
        long s = 0;
        for (int i = 0; i < n; i++) { Arms.encodeR(id, facadePool[i], enc); s += enc.len; }
        return s;
      }
    });
    encArms.add(new Arm("ffi", "the C ABI, batched, the core transcoding") {
      boolean covers(String id) { return FfiArms.encodable(id); }
      long run(String id, int n) {
        long s = 0;
        for (int i = 0; i < n; i++) s += FfiArms.encode(bFfi, id, facadePool[i]);
        return s;
      }
    });
    encArms.add(new Arm("ffi-nobatch", "the same, with the host declining to batch"
        + " (ABI v1 section 6 allows it)") {
      boolean covers(String id) { return FfiArms.encodable(id); }
      long run(String id, int n) {
        long s = 0;
        for (int i = 0; i < n; i++) s += FfiArms.encode(bNoBatch, id, facadePool[i]);
        return s;
      }
    });
    encArms.add(new Arm("ffi-zeroed", "open decision 9's candidate: bulk-clear the chunk,"
        + " assign only what differs") {
      boolean covers(String id) { return FfiArms.encodable(id); }
      long run(String id, int n) {
        long s = 0;
        for (int i = 0; i < n; i++) s += FfiArms.encode(bZeroed, id, facadePool[i]);
        return s;
      }
    });
    encArms.add(new Arm("R-take", "arm R plus a fresh output array, so it and `pbj`"
        + " deliver the same thing") {
      long run(String id, int n) {
        long s = 0;
        for (int i = 0; i < n; i++) {
          Arms.encodeR(id, facadePool[i], enc);
          s += enc.toBytes().length;
        }
        return s;
      }
    });
    encArms.add(new Arm("ffi-take", "the C ABI, batched, plus a fresh output array, so it"
        + " and `pbj` deliver the same thing") {
      boolean covers(String id) { return FfiArms.encodable(id); }
      long run(String id, int n) {
        long s = 0;
        for (int i = 0; i < n; i++) {
          FfiArms.encode(bFfi, id, facadePool[i]);
          s += bFfi.take().length;
        }
        return s;
      }
    });

    // ---- decode, all building a fresh object graph ------------------------------------
    decArms.add(new Arm("pbj", "parseFrom") {
      long run(String id, int n) {
        long s = 0;
        try {
          // identityHashCode, NOT hashCode. `Message.hashCode()` walks the whole
          // decoded tree, so using it to keep the parse alive would have charged the
          // incumbent a second full traversal that no other arm pays. This harness's
          // own C7, found by looking for the cpp slice's.
          for (int i = 0; i < n; i++)
            s += System.identityHashCode(PbArms.parse(id, wire, 0, wire.length));
        } catch (Exception e) { throw new IllegalStateException(e); }
        return s;
      }
    });
    decArms.add(new Arm("R", "the generated pure-Java codec") {
      long run(String id, int n) {
        long s = 0;
        for (int i = 0; i < n; i++)
          s += Arms.decodeR(id, dec, wire, 0, wire.length) == null ? 0 : 1;
        return s;
      }
    });
    decArms.add(new Arm("ffi", "the C ABI, push family, wire copied once into native"
        + " scratch and spans resolved against the byte[] the host holds") {
      long run(String id, int n) {
        long s = 0;
        for (int i = 0; i < n; i++)
          s += FfiArms.decode(bFfi, id, wire, 0, wire.length) == null ? 0 : 1;
        return s;
      }
    });
    // ABI v1 7.1's PULL family. Zero reverse calls on every payload (logs/java/counts.log),
    // which is the whole reason it exists on this runtime: the push arm's decode regression
    // decomposes into 7.004 upcalls per element at about 80 ns. Two deliveries, differing
    // by exactly the drain copy, so the family's cost splits into "materialise the records"
    // and "copy them to the host" instead of arriving as one number.
    decArms.add(new Arm("ffi-pull", "7.1 pull, drained into host memory in 32 KB chunks:"
        + " one forward crossing per chunk, no reverse call, wire never copied") {
      long run(String id, int n) {
        long s = 0;
        bPull.pullWalk = false;
        for (int i = 0; i < n; i++)
          s += FfiArms.parse(bPull, id, wire, 0, wire.length) == null ? 0 : 1;
        return s;
      }
    });
    decArms.add(new Arm("ffi-pull-walk", "the same, read in place through ak_bdr_ptr:"
        + " two forward crossings for the whole response and no intermediate") {
      long run(String id, int n) {
        long s = 0;
        bPullWalk.pullWalk = true;
        for (int i = 0; i < n; i++)
          s += FfiArms.parse(bPullWalk, id, wire, 0, wire.length) == null ? 0 : 1;
        return s;
      }
    });
    decArms.add(new Arm("ffi-borrow", "ABI v1 open decision 13: the facade holds views"
        + " over the host's own buffer rather than String") {
      long run(String id, int n) {
        long s = 0;
        for (int i = 0; i < n; i++)
          s += ak.borrow.FfiArms.decode(bBorrow, id, wire, 0, wire.length) == null ? 0 : 1;
        return s;
      }
    });

    List<String> ids = new ArrayList<String>();
    for (String id : Arms.IDS) if (only == null || only.equals(id)) ids.add(id);

    List<Table> out = new ArrayList<Table>();
    if (encode) out.add(measure("ENCODE", ids, encArms, ROUNDS, TARGET_NS, true));
    if (decode) out.add(measure("DECODE", ids, decArms, ROUNDS, TARGET_NS, false));

    // ---- everything below here runs after the last measurement ------------------------
    StringBuilder sb = new StringBuilder();
    header(sb, ROUNDS, TARGET_NS, bFfi);
    for (Table t : out) t.render(sb);
    System.out.print(sb);
  }

  static void header(StringBuilder sb, int rounds, long targetNs, Binding b) {
    sb.append("== the java slice's codec bench ==\n");
    sb.append("java.version=").append(System.getProperty("java.version"))
      .append("  vm=").append(System.getProperty("java.vm.version")).append('\n');
    sb.append("protobuf-java=").append(RunUnknown.protobufVersion()).append('\n');
    sb.append("lib=").append(System.getProperty("ak.lib")).append('\n');
    sb.append("compact strings in the binding: ")
      .append(b.usingCompactStrings() ? "yes (JDK 9+ String.value/coder)" : "NO").append('\n');
    sb.append("rounds requested=").append(rounds)
      .append(" (rounded up to a multiple of the arm count), per-round target ")
      .append(targetNs / 1_000_000).append(" ms, arm order rotates every round\n");
    sb.append("content set: ")
      .append(CS == Values.ASCII ? "ASCII (design/SHAPES.md's default; every id in the"
              + " real schema is an ASCII GUID)"
              : CS == Values.LATIN1 ? "LATIN1 but not ASCII -- one code point per input"
              + " character, U+00A0 to U+00FF. On the JVM this is still the compact"
              + " coder, so the target's fast path applies and the CORE's transcoder"
              + " changes from latin1 to latin1 with two-byte output"
              : "above U+00FF -- U+4E00 and up, inside the BMP so one char per code"
              + " point. This is where a JVM String stops being LATIN1, so the binding's"
              + " staging doubles and the core transcodes UTF-16")
      .append("\n");
    sb.append("ak.deopt=").append(System.getProperty("ak.deopt", "0"))
      .append("  (0 nothing; 1 one String.format with a numeric conversion, which is what")
      .append(" R9 names;\n           2 read a WIDE String through charAt; 3 read a")
      .append(" LATIN1 String through charAt)\n");
    sb.append("\nEvery ratio is PAIRED: computed inside one round against that round's\n")
      .append("own baseline, then reported as min/median/max across rounds. A ratio of\n")
      .append("two medians from different rounds appears nowhere.\n");
  }

  // ------------------------------------------------------------------ the measurement

  static final class Table {
    final String dir;
    final List<String> ids = new ArrayList<String>();
    final List<Arm> arms;
    // [payload][arm][round] nanoseconds per operation, scaled by 1000 to keep integers
    final List<long[][]> ps = new ArrayList<long[][]>();
    final List<int[]> iters = new ArrayList<int[]>();
    Table(String dir, List<Arm> arms) { this.dir = dir; this.arms = arms; }

    void render(StringBuilder sb) {
      sb.append("\n\n=== ").append(dir).append(" ===\n");
      for (int a = 0; a < arms.size(); a++)
        sb.append("  ").append(pad(arms.get(a).name, 12)).append(arms.get(a).note)
          .append('\n');
      sb.append("\nComparable pairs. `pbj`, `R-take` and `ffi-take` all hand back a\n")
        .append("fresh byte[] and all pay their own size pass, so those three are the\n")
        .append("apples-to-apples set. `pbj-reuse`, `pbj-det`, `R`, `ffi`, `ffi-nobatch`\n")
        .append("and `ffi-zeroed` all write into a buffer they already own -- but the\n")
        .append("protobuf ones memoize the size and the others do not, so read those\n")
        .append("against each other only through the deltas below.\n");
      sb.append("\nBaseline: ").append(arms.get(0).name)
        .append(". ns/op is that arm's own median. Each cell is min / median / max of\n")
        .append("the PAIRED per-round ratio. Read the median; the min is the least\n")
        .append("contaminated estimate and the max is usually one collection.\n\n");
      sb.append(pad("id", 7)).append(pad("bytes", 9)).append(pad("base ns/op", 12));
      for (int a = 1; a < arms.size(); a++) sb.append(pad(arms.get(a).name, 22));
      sb.append('\n');
      for (int p = 0; p < ids.size(); p++) {
        long[][] r = ps.get(p);
        if (r == null) continue;
        sb.append(pad(ids.get(p), 7));
        sb.append(pad(Long.toString(Payloads.row(ids.get(p)).bytes), 9));
        long[] base = r[0];
        if (base == null) { sb.append("(not covered)\n"); continue; }
        sb.append(pad(fmt(median(base) / 1000.0, 1), 12));
        for (int a = 1; a < arms.size(); a++) {
          if (r[a] == null) { sb.append(pad("-", 22)); continue; }
          double[] q = ratios(r[a], base);
          sb.append(pad(fmt(q[0], 3) + " " + fmt(q[1], 3) + " " + fmt(q[2], 3), 22));
        }
        sb.append('\n');
      }
      renderDeltas(sb);
    }

    /** The questions that are a DELTA between two arms rather than a ratio to a third. */
    void renderDeltas(StringBuilder sb) {
      String[][] pairs = {
        {"ffi-nobatch", "ffi", "the batching predicate (ABI v1 section 6): positive means"
            + " batching is FASTER"},
        {"ffi", "ffi-zeroed", "open decision 9's fill: positive means the sparse fill is"
            + " FASTER"},
        {"pbj-reuse", "pbj-det", "what deterministic serialization costs the incumbent,"
            + " priced separately rather than charged to the headline. Both arms are one"
            + " message into a reused buffer, so this is the sort and nothing else"},
        {"ffi-take", "ffi", "copying the encoded bytes back into a fresh Java array"},
        {"R-take", "R", "the same copy, for arm R"},
        {"pbj", "pbj-loop", "what protobuf-java's memoized size hides from a loop"
            + " benchmark: positive means the loop is faster than an application"},
        {"R-take", "ffi-take", "the architecture question, both handing back a fresh"
            + " array: positive means the C ABI is FASTER than a generated Java codec"},
        {"R", "ffi", "the same question, both writing into a buffer they own"},
        {"pbj", "pbj-parsed", "what a PARSED protobuf message saves by holding ByteString"
            + " instead of String: positive means parsed is faster, and a harness whose"
            + " pool comes from parseFrom measures that instead of the codec"},
        {"pbj", "pbj-reused-out", "what the output allocation costs the incumbent"},
        {"ffi", "ffi-borrow", "open decision 13: positive means the borrowed facade is"
            + " FASTER"},
        {"ffi", "ffi-pull", "ABI v1 7.1, the FAMILY question: positive means the pull"
            + " family is faster than the push family. Both deliver through the same"
            + " per-slot host code, so this is two deliveries of one traversal"},
        {"ffi-pull", "ffi-pull-walk", "what the drain copy costs: positive means reading"
            + " the records in place is faster. The C# slice estimated this at 12 to 19"
            + " percent of a parse; here it is measured"},
        {"R", "ffi-pull", "the architecture question again, against the no-boundary"
            + " control: positive means the C ABI's pull family beats a generated Java"
            + " codec, which the push family does on no payload at all"},
      };
      boolean any = false;
      for (String[] pr : pairs) {
        int a = indexOf(pr[0]), b = indexOf(pr[1]);
        if (a < 0 || b < 0) continue;
        if (!any) {
          sb.append("\n").append(dir).append(" deltas, paired inside each round")
            .append(" (R4's sharpened form)\n");
          any = true;
        }
        sb.append("\n  ").append(pr[0]).append(" - ").append(pr[1]).append("   ")
          .append(pr[2]).append('\n');
        sb.append("  ").append(pad("id", 7)).append(pad("ns/op lo", 12))
          .append(pad("ns/op hi", 12)).append(pad("per element", 14))
          .append("sign\n");
        for (int p = 0; p < ids.size(); p++) {
          long[][] r = ps.get(p);
          if (r == null || r[a] == null || r[b] == null) continue;
          int n = Math.min(r[a].length, r[b].length);
          double[] d = new double[n];
          for (int i = 0; i < n; i++) d[i] = (r[a][i] - r[b][i]) / 1000.0;
          Arrays.sort(d);
          double lo = d[0], hi = d[n - 1];
          int el = Math.max(Payloads.row(ids.get(p)).elements, 1);
          boolean same = (lo > 0 && hi > 0) || (lo < 0 && hi < 0);
          double md = d[n / 2];
          sb.append("  ").append(pad(ids.get(p), 7)).append(pad(fmt(lo, 1), 12))
            .append(pad(fmt(md, 1), 12)).append(pad(fmt(hi, 1), 12))
            .append(pad(fmt(md / el, 3), 16))
            .append(same ? (lo > 0 ? "+" : "-") : "STRADDLES ZERO").append('\n');
        }
      }
    }

    int indexOf(String n) {
      for (int i = 0; i < arms.size(); i++) if (arms.get(i).name.equals(n)) return i;
      return -1;
    }
  }

  static Table measure(String dir, List<String> ids, List<Arm> arms, int rounds,
                       long targetNs, boolean isEncode) {
    // Rounded UP to a multiple of the arm count. The order rotates, so this makes every
    // arm occupy every position in the round exactly the same number of times -- the
    // position bias is removed rather than averaged over, which with 8 arms and 15 rounds
    // it is not.
    rounds = ((rounds + arms.size() - 1) / arms.size()) * arms.size();
    Table t = new Table(dir, arms);
    for (String id : ids) {
      // Build the objects once, outside every timed region.
      Payloads.Row row = Payloads.row(id);
      facade = Arms.build(id, CS);
      pbmsg = PbArms.build(id, CS);
      // The committed vector is the ASCII set by construction, so any other content set
      // has to be encoded here. Otherwise a "wide" decode row would be decoding ASCII.
      wire = CS == Values.ASCII ? Payloads.vector(id) : null;
      if (wire == null) {
        if (pbmsg == null) { t.ids.add(id); t.ps.add(null); t.iters.add(null); continue; }
        wire = new PbArm().deterministic(pbmsg);
      }
      if (isEncode && (facade == null || pbmsg == null)) {
        t.ids.add(id); t.ps.add(null); t.iters.add(null); continue;
      }

      // ONE iteration count for the whole payload, from the baseline, capped by the
      // pool. Every arm then streams the same number of distinct source objects, so the
      // cache behaviour is a property of the payload rather than of the arm. A fast arm
      // gets a shorter round and a slow one a longer round, which is the price of that
      // and is smaller than the bias it removes.
      // Two stages, because the pool has to exist before the baseline can be timed and
      // its size is what the timing decides. Eight operations give a first estimate;
      // building the cap outright would be tens of seconds of sha256 on the payloads
      // whose round needs a dozen.
      int n0;
      if (isEncode) {
        final int probe = 8;
        buildPools(id, true, probe);
        arms.get(0).run(id, probe);                       // warm the path, not the clock
        refreshPbPool(id, probe);
        long t0 = System.nanoTime();
        arms.get(0).run(id, probe);
        long est = Math.max(1, (System.nanoTime() - t0) / probe);
        n0 = (int) Math.max(probe, Math.min(poolCap(id), targetNs / est));
        buildPools(id, true, n0);
      } else {
        n0 = calibrate(arms.get(0), id, targetNs);
      }
      long[][] r = new long[arms.size()][];
      int[] its = new int[arms.size()];
      // Warm every arm on this payload past C2's thresholds before any round is kept.
      for (int a = 0; a < arms.size(); a++) {
        if (!arms.get(a).covers(id)) continue;
        int w = Math.max(1, Math.min(
            Math.max(200, (int) (2_000_000L / Math.max(row.bytes, 64))),
            isEncode ? n0 : Integer.MAX_VALUE));
        arms.get(a).run(id, w);
        if (isEncode) buildPools(id, true, n0);
        arms.get(a).run(id, w);
        its[a] = isEncode ? n0 : calibrate(arms.get(a), id, targetNs);
        r[a] = new long[rounds];
      }
      for (int round = 0; round < rounds; round++) {
        // Rebuild the protobuf pool first, then collect, then time. It happens outside
        // every timed region and before any arm in this round runs, so where an arm sits
        // in the rotation does not decide what heap it meets.
        if (isEncode) buildPools(id, true, n0);
        System.gc();
        try { Thread.sleep(8); } catch (InterruptedException ignored) { }
        for (int j = 0; j < arms.size(); j++) {
          int a = (j + round) % arms.size();     // rotate: a fixed order is an artifact
          if (r[a] == null) continue;
          long t0 = System.nanoTime();
          long s = arms.get(a).run(id, its[a]);
          long dt = System.nanoTime() - t0;
          SINK += s;
          r[a][round] = dt * 1000L / its[a];
        }
      }
      t.ids.add(id);
      t.ps.add(r);
      t.iters.add(its);
    }
    return t;
  }

  static long SINK;

  static int CS = Values.ASCII;   // -Dak.cs

  static int poolCap(String id) {
    int bytes = Math.max(Payloads.row(id).bytes, 64);
    return Math.max(8, (int) Math.min(8192, (128L << 20) / (bytes * 5L)));
  }

  /**
   * Both pools, rebuilt together before every round.
   *
   * <p>The facade pool does not NEED rebuilding -- a facade object carries no state
   * between encodes -- and it is rebuilt anyway, for symmetry. Left alone it would sit in
   * the old generation while the protobuf pool was freshly allocated in the young one
   * every round, and the arms would then differ in where their input lives as well as in
   * what they do with it.
   */
  static void buildPools(String id, boolean isEncode, int n) {
    if (!isEncode) return;
    if (facadePool.length < n) facadePool = new Object[n];
    for (int i = 0; i < n; i++) facadePool[i] = Arms.build(id, CS);
    refreshPbPool(id, n);
  }

  /**
   * Rebuilt every round: a protobuf message memoizes its size after the first write.
   *
   * <p><b>BUILT, not parsed, and the distinction is worth about as much as everything else
   * in this harness put together.</b> protobuf-java's string fields hold a
   * {@code java.lang.Object}: a {@code String} when the message was built, a
   * {@code ByteString} when it was parsed. {@code writeTo} writes the RAW object, so a
   * parsed message re-serialises with <b>no UTF-8 transcoding at all</b>, and
   * {@code getSerializedSize} on it is a length lookup rather than a scan. A pool made by
   * {@code parseFrom} would have handed the incumbent a free pass on the one cost
   * design/SHAPES.md says dominates this schema ("so this is a string codec"). A server
   * builds its messages from Strings, so the pool does too.
   */
  static void refreshPbPool(String id, int n) {
    if (pbPool.length < n) pbPool = new Message[n];
    for (int i = 0; i < n; i++) pbPool[i] = PbArms.build(id, CS);
    if (parsedPool.length < n) parsedPool = new Message[n];
    try {
      for (int i = 0; i < n; i++) parsedPool[i] = PbArms.parse(id, wire, 0, wire.length);
    } catch (Exception e) {
      throw new IllegalStateException(id, e);
    }
  }



  static int calibrate(Arm arm, String id, long targetNs) {
    // Capped by the arm's own bound: the pooled baseline may not run more operations than
    // it has fresh messages, and a calibration that overran the pool would index past it.
    final int cap = arm.maxIters(id);
    int n = Math.min(4, cap);
    for (;;) {
      long t0 = System.nanoTime();
      arm.run(id, n);
      long dt = System.nanoTime() - t0;
      if (dt >= targetNs || n >= (1 << 26) || n >= cap) return Math.min(n, cap);
      long want = n * (targetNs / Math.max(dt, 1000L));
      n = (int) Math.max(n * 2L, Math.min(want, n * 64L));
      if (n <= 0 || n > cap) return cap;
    }
  }

  static long median(long[] v) {
    long[] c = v.clone();
    Arrays.sort(c);
    return c[c.length / 2];
  }

  /** Paired: the ratio is formed inside each round, then ordered. */
  static double[] ratios(long[] arm, long[] base) {
    int n = Math.min(arm.length, base.length);
    double[] q = new double[n];
    for (int i = 0; i < n; i++) q[i] = arm[i] / (double) base[i];
    Arrays.sort(q);
    return new double[] {q[0], q[n / 2], q[n - 1]};
  }

  static String pad(String s, int w) {
    StringBuilder b = new StringBuilder(s);
    while (b.length() < w) b.append(' ');
    return b.toString();
  }

  /** No String.format on any path a measurement could still be running through. */
  static String fmt(double v, int dp) {
    boolean neg = v < 0;
    if (neg) v = -v;
    long scale = 1;
    for (int i = 0; i < dp; i++) scale *= 10;
    long x = (long) (v * scale + 0.5);
    StringBuilder b = new StringBuilder();
    if (neg) b.append('-');
    b.append(x / scale);
    if (dp > 0) {
      b.append('.');
      String f = Long.toString(x % scale);
      for (int i = f.length(); i < dp; i++) b.append('0');
      b.append(f);
    }
    return b.toString();
  }
}
