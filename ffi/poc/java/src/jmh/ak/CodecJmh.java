package ak;

import java.io.FileOutputStream;
import java.io.OutputStreamWriter;
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.TimeUnit;
import org.openjdk.jmh.annotations.Benchmark;
import org.openjdk.jmh.annotations.BenchmarkMode;
import org.openjdk.jmh.annotations.Level;
import org.openjdk.jmh.annotations.Mode;
import org.openjdk.jmh.annotations.OutputTimeUnit;
import org.openjdk.jmh.annotations.Param;
import org.openjdk.jmh.annotations.Scope;
import org.openjdk.jmh.annotations.Setup;
import org.openjdk.jmh.annotations.State;
import org.openjdk.jmh.annotations.TearDown;
import org.openjdk.jmh.infra.Blackhole;
import org.openjdk.jmh.infra.IterationParams;
import org.openjdk.jmh.runner.IterationType;

/**
 * The codec suite on JMH (design/CAMPAIGN.md req 22a, owner 2026-09-25).
 *
 * <p><b>One JMH iteration is one sample</b> of section 7: {@code Mode.SingleShotTime}, one
 * invocation per iteration, and the invocation runs {@code iters} operations of the cell
 * (the same count the self-timed harness used: {@code ak.camp.budget} bytes per sample,
 * clamped to {@code ak.camp.maxiters}). Req 11: the {@code iters} fresh object graphs an
 * encode sample serialises are built in {@code @Setup(Level.Iteration)}, which JMH does not
 * time. JMH's raw per-iteration wall time is the {@code wall_ns} of the sample.
 *
 * <p><b>CPU time</b> (req 21): JMH measures wall time only. The benchmark method reads the
 * measuring thread's CPU clock (ThreadMXBean, CLOCK_THREAD_CPUTIME_ID) at its start and end
 * and the iteration teardown writes it, tagged warm-up or measurement, to
 * {@code ak.jmh.cpuout}; gen/jmh_to_jsonl.py joins it to JMH's rawData by cell and
 * measurement-iteration index. The two reads are inside the timed region, so wall_ns carries
 * them (tens of ns against a sample of milliseconds).
 *
 * <p><b>The cell</b> is one parameter, {@code arm|mode|payload|content|dir}, listed by
 * {@code ak.CampaignCodec} (rotated per launch) and passed with {@code -p cell=...}, so no
 * invalid combination (encode on a decode-only arm, P7.1 off ASCII) ever runs. The Trial
 * setup runs the cell's correctness check (req 26) and throws on a mismatch, which with
 * {@code -foe true} stops the whole run before a figure is written.
 *
 * <p><b>Blackhole</b>: every operation folds its result into {@code CampaignCodec.sink}
 * (identity hashes, lengths, walked sums), and the method hands that to the Blackhole.
 */
@State(Scope.Thread)
@BenchmarkMode(Mode.SingleShotTime)
@OutputTimeUnit(TimeUnit.NANOSECONDS)
public class CodecJmh {
  @Param({"host-gen|drop|P1.1|ascii|encode"})
  public String cell;

  CampaignCodec.CArm arm;
  String id, dir;
  int cs, n;
  byte[] wire;
  boolean measuring;
  long cpu;
  final StringBuilder cpuLines = new StringBuilder();
  int measured;

  @Setup(Level.Trial)
  public void trial() throws Exception {
    String[] c = cell.split("\\|");
    id = c[2];
    dir = c[4];
    if (c[3].startsWith("corpus:")) {
      // Req 7 (amended): a corpus U-* row, through the corpus description.
      wire = CampaignCodec.corpusRow(id);
      CampaignCodec.UArm u = new CampaignCodec.UArm(c[0], c[1], c[3].substring(7), wire);
      CampaignCodec.checkU(u);
      arm = u;
      cs = 0;
    } else {
      arm = CampaignCodec.make(c[0], c[1]);
      cs = java.util.Arrays.asList(CampaignCodec.SET_NAMES).indexOf(c[3]);
      wire = CampaignCodec.canonical(id, cs);
      CampaignCodec.check(arm, id, cs, wire, dir);
    }
    n = CampaignCodec.iters(wire.length);
  }

  @Setup(Level.Iteration)
  public void iteration(IterationParams ip) {
    measuring = ip.getType() == IterationType.MEASUREMENT;
    if (dir.equals("encode")) arm.prepare(id, cs, n);       // untimed (req 11)
    CampaignCodec.SINK.reset(2 * wire.length + 4096);      // untimed: no arm grows the sink
  }

  @Benchmark
  public void sample(Blackhole bh) throws Exception {
    long c0 = Campaign.threadCpuNs();
    if (dir.equals("encode")) arm.encode(id, n);
    else arm.decode(id, wire, n, dir.equals("decode-read"));
    cpu = Campaign.threadCpuNs() - c0;
    bh.consume(CampaignCodec.sink);
  }

  @TearDown(Level.Iteration)
  public void after() {
    cpuLines.append(cell).append('\t').append(measuring ? "m" : "w").append('\t')
        .append(measuring ? measured++ : -1).append('\t').append(cpu).append('\t').append(n).append('\n');
  }

  @TearDown(Level.Trial)
  public void done() throws Exception {
    String path = System.getProperty("ak.jmh.cpuout");
    if (path == null) return;
    synchronized (CodecJmh.class) {
      PrintWriter w = new PrintWriter(new OutputStreamWriter(new FileOutputStream(path, true),
          StandardCharsets.UTF_8));
      w.print(cpuLines);
      w.close();
    }
  }
}
