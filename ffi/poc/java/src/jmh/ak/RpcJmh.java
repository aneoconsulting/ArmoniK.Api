package ak;

import java.util.List;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.Semaphore;
import java.util.concurrent.TimeUnit;

import io.grpc.netty.shaded.io.netty.channel.epoll.EpollEventLoopGroup;
import org.openjdk.jmh.annotations.AuxCounters;
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
 * design/CAMPAIGN.md section 4.2, the RPC grid, on JMH (req 22a as amended 2026-09-27, FIX-PLAN
 * WP9). The server is the runner's separate process, started and warmed before JMH runs
 * (req 13, 24). The cells, directions and checks are {@code ak.CampaignRpc}'s.
 *
 * <p>What is JMH's, used as it is: the forks (one JVM per {@code cell} value, {@code -f 1}),
 * warm-up and measurement iterations ({@code -wi}, {@code -i}) and their durations
 * ({@code -w}, {@code -r}), the invocation loop and its clock (Mode.AverageTime with a
 * {@code Level.Invocation} setup: JMH times every invocation and leaves the setup out), the
 * stop on first error ({@code -foe true}), and the raw export ({@code -rf json}: every
 * measurement iteration's score and the {@code @AuxCounters} below, per iteration).
 *
 * <p>What is custom, and the requirement it serves:
 * <ul>
 * <li><b>The combination cycle</b> (req 13's one channel per cell per process, and the fork
 * count): JMH forks once per parameter combination, so a (cell, direction, payload, in-flight)
 * parameter set would open 17 processes and 17 channels per cell. One fork per cell instead,
 * and JMH iteration i runs combination {@code (i + launch - 1) mod 17} of
 * {@link CampaignRpc#combos} (warm-up and measurement counted separately), so every round
 * visits every combination, interleaved within the fork, the start rotated per launch
 * (req 22). JMH's per-benchmark summary score therefore averages unlike combinations and is
 * not used; its raw per-iteration scores are.
 * <li><b>k calls in flight</b> (req 14, 15; WP9 item 2): one invocation is one batch of k
 * calls, call 0 on the benchmark thread and calls 1..k-1 on persistent helper threads started
 * in the trial setup; the invocation returns when all k have. JMH's {@code -t} threads would
 * each run whole invocations with their own state and cannot put k calls of one batch in
 * flight on one channel.
 * <li><b>Process CPU</b> (req 21; JMH measures wall only): the process's CPU clock
 * (CLOCK_PROCESS_CPUTIME_ID, {@code Campaign.processCpuNs}, the codec suite's clock) is read
 * at the start and end of every invocation and summed into the {@code rpcCpuNs} aux counter.
 * <li><b>Checks</b> (req 18): every call is checked by the cell; a failed check throws
 * ({@code CampaignRpc.THROW_ON_FAIL}), JMH records the error and, with {@code -foe true},
 * stops. The trial setup runs the cell's pre-check (every direction once, d through the
 * SHA-256 path, P2.2 re-encoded byte-identical); the trial teardown requires decision 11's
 * leak counters to read 0.
 * <li><b>Labels</b> (section 7): the trial setup prints one {@code RPCJMH-CELL} line (the
 * cell's mode, codec, send path, transport kind, threads) and every iteration setup one
 * {@code RPCJMH-ITER} line (its combination), which gen/rpc_jmh_to_jsonl.py joins to JMH's
 * JSON by cell and measurement-iteration index.
 * </ul>
 *
 * <p>Counters per iteration (JMH {@code @AuxCounters}, exported per iteration): {@code calls}
 * (OPERATIONS: JMH reports the time per call, k per invocation), {@code callsMade} and
 * {@code rpcCpuNs} (EVENTS: totals). The iteration's wall time is JMH's primary score (ns per
 * invocation) times its invocations ({@code callsMade / k}).
 */
@State(Scope.Benchmark)
@BenchmarkMode(Mode.AverageTime)
@OutputTimeUnit(TimeUnit.NANOSECONDS)
public class RpcJmh {
  @Param({"A"})
  public String cell;

  /** The combination this fork runs (CAMPAIGN req 22a, owner e6c909630: the campaign's
   *  default, JMH's own isolation, one fork per (cell, combination)), as
   *  {@code <dir key>/<k>} ("a/1", "c:0/8", ...), or {@code cycle}: every combination in one
   *  fork, JMH iteration i running combination (i + launch - 1) mod 17 (the runner's
   *  AK_RPC_GROUP=1, for smoke and small exploration runs only). */
  @Param({"cycle"})
  public String combo;

  CampaignRpc.Cell c;
  EpollEventLoopGroup elg;
  List<String[]> combos;
  int launch;

  // the current iteration's combination
  String key;
  int k;
  int warmIdx, measIdx;
  /** Req 24 as amended (8c02e7c58): warm-up batches run before the first measured value;
   *  each batch is one call on each of the k calling threads (the same threads measure). */
  long warmBatches;
  boolean measuring;
  final Object[] reqs = new Object[16];

  // helpers 1..15 of a batch
  Thread[] workers;
  final Semaphore[] start = new Semaphore[16];
  volatile CountDownLatch done;
  volatile boolean stop;
  volatile Throwable err;

  /** Per-iteration totals, exported by JMH as secondary metrics. */
  @AuxCounters(AuxCounters.Type.EVENTS)
  @State(Scope.Thread)
  public static class Totals {
    public long rpcCpuNs, callsMade, rpcTaskClockNs, softirqTicks, minflt;
    private long irq0;
    @Setup(Level.Iteration)
    public void reset() { rpcCpuNs = 0; callsMade = 0; rpcTaskClockNs = 0; minflt = 0; irq0 = softirq(); }
    /** Req 21 as amended: softirq time on the CLIENT CPUs over the iteration (/proc/stat, in
     *  USER_HZ ticks), read outside the timed invocations. */
    @TearDown(Level.Iteration)
    public void irq() { softirqTicks = softirq() - irq0; }
  }

  /** The CLIENT CPUs (-Dak.camp.clientcpus, a cpu list like "1-4,11-14"; unset: every CPU). */
  static final java.util.BitSet CLIENT = cpuList(System.getProperty("ak.camp.clientcpus", ""));
  static java.util.BitSet cpuList(String l) {
    java.util.BitSet b = new java.util.BitSet();
    for (String p : l.split(",")) {
      if (p.isEmpty()) continue;
      String[] r = p.split("-");
      for (int i = Integer.parseInt(r[0]); i <= Integer.parseInt(r[r.length - 1]); i++) b.set(i);
    }
    return b;
  }
  /** The summed softirq column of /proc/stat's cpuN lines for the CLIENT CPUs. */
  static long softirq() {
    long t = 0;
    try (java.io.BufferedReader r = new java.io.BufferedReader(new java.io.FileReader("/proc/stat"))) {
      for (String l; (l = r.readLine()) != null; ) {
        if (!l.startsWith("cpu") || l.startsWith("cpu ")) continue;
        String[] f = l.trim().split("\\s+");
        int cpu = Integer.parseInt(f[0].substring(3));
        if (CLIENT.isEmpty() || CLIENT.get(cpu)) t += Long.parseLong(f[7]);
      }
    } catch (java.io.IOException e) {
      throw new IllegalStateException(e);
    }
    return t;
  }

  /** The calls as JMH operations (k per invocation): JMH reports the time per call. */
  @AuxCounters(AuxCounters.Type.OPERATIONS)
  @State(Scope.Thread)
  public static class Calls {
    public long calls;
    @Setup(Level.Iteration)
    public void reset() { calls = 0; }
  }

  @Setup(Level.Trial)
  public void trial() throws Exception {
    CampaignRpc.THROW_ON_FAIL = true;
    NativeRpc.ensureBound();
    Native.ensureBound();
    String sock = System.getProperty("ak.camp.socket");
    if (sock == null) throw new IllegalStateException("-Dak.camp.socket is unset");
    String transport = System.getProperty("ak.camp.transport", "pinned");
    launch = Integer.getInteger("ak.camp.launch", 1);
    CampaignRpc.EXPECT_A = ak.shapes.PbArms.build(CampaignRpc.PAYLOAD, Values.ASCII).toByteArray();
    CampaignRpc.uploads();
    elg = new EpollEventLoopGroup(CampaignRpc.EVENT_LOOPS);
    c = CampaignRpc.cell(cell, sock, elg, transport.equals("pinned"));
    System.out.println(CampaignAlloc.startup());        // req 25 / D9: refuse a wrong allocator mode
    CampaignRpc.precheck(c);
    TaskClock.ensureOpen();                          // req 21 as amended: refuse without it
    String nodelay = CampaignRpc.checkNodelay(sock); // req 17 as amended: on the live sockets
    combos = CampaignRpc.combos();
    System.out.println("RPCJMH-CELL\t" + cell + "\t" + c.mode() + "\t"
        + (c.codec == CampaignRpc.INC ? "incumbent" : c.codec == CampaignRpc.FFI ? "core-ffi" : "host-gen")
        + "\t" + CampaignRpc.sendPath(c) + "\t" + (c instanceof CampaignRpc.CoreCell ? "core" : "grpc")
        + "\t" + transport + "\t" + ak.Variant.NAME + "\t" + CampaignRpc.threadsJson()
        + "\t" + System.getProperty("ak.camp.h2", "unknown") + "\t" + nodelay
        + "\t" + System.getProperty("ak.camp.alloc", "default"));
    workers = new Thread[15];
    for (int t = 0; t < 15; t++) {
      final int w = t + 1;
      start[w] = new Semaphore(0);
      workers[t] = new Thread(() -> {
        try {
          for (;;) {
            start[w].acquire();
            if (stop) break;
            try {
              CampaignRpc.call(c, key, reqs[w]);
            } catch (Throwable e) {
              err = e;
            } finally {
              done.countDown();
            }
          }
        } catch (InterruptedException e) {
          // trial teardown
        } finally {
          CampaignRpc.releaseThread();
        }
      }, "rpc-" + cell + "-" + w);
      workers[t].setDaemon(true);
      workers[t].start();
    }
  }

  @Setup(Level.Iteration)
  public void iteration(IterationParams ip) {
    boolean m = ip.getType() == IterationType.MEASUREMENT;
    if (m && measIdx == 0)   // the record the 20-calls-per-thread rule is checked against
      System.out.println("RPCJMH-WARM\t" + cell + "|" + combo + "\t" + warmBatches);
    measuring = m;
    int i = m ? measIdx++ : warmIdx++;
    String[] cb = null;
    if (combo.equals("cycle")) cb = combos.get((i + launch - 1) % combos.size());
    else for (String[] x : combos) if ((x[0] + "/" + x[3]).equals(combo)) cb = x;
    if (cb == null) throw new IllegalArgumentException("no combination " + combo);
    key = cb[0];
    k = Integer.parseInt(cb[3]);
    // The label trail (untimed): what this iteration runs, joined to JMH's raw data.
    System.out.println("RPCJMH-ITER\t" + cell + "|" + combo + "\t" + (ip.getType() == IterationType.MEASUREMENT ? "m" : "w")
        + "\t" + i + "\t" + cb[0] + "\t" + cb[1] + "\t" + cb[2] + "\t" + k);
  }

  @Setup(Level.Invocation)
  public void requests() {
    if (key.equals("b")) for (int i = 0; i < k; i++) reqs[i] = c.fresh();   // req 11, untimed
  }

  @Benchmark
  public void batch(Totals tot, Calls ops, Blackhole bh) throws Throwable {
    long c0 = Campaign.processCpuNs(), k0 = TaskClock.ns(), f0 = Native.minorFaults();
    if (k == 1) {
      CampaignRpc.call(c, key, reqs[0]);
    } else {
      done = new CountDownLatch(k - 1);
      for (int w = 1; w < k; w++) start[w].release();   // helpers 1..k-1 start their call
      try {
        CampaignRpc.call(c, key, reqs[0]);             // the benchmark thread makes call 0
      } finally {
        done.await();
      }
      if (err != null) throw err;
    }
    tot.minflt += Native.minorFaults() - f0;          // req 25: faults over the measured span
    tot.rpcTaskClockNs += TaskClock.ns() - k0;
    tot.rpcCpuNs += Campaign.processCpuNs() - c0;
    tot.callsMade += k;
    if (!measuring) warmBatches++;
    ops.calls += k;
    bh.consume(CampaignRpc.sinkv);
  }

  @TearDown(Level.Trial)
  public void close() throws Exception {
    stop = true;
    for (int w = 1; w < 16; w++) start[w].release();
    for (Thread t : workers) t.join(5000);
    c.close();
    CampaignRpc.releaseThread();
    elg.shutdownGracefully(0, 0, TimeUnit.SECONDS);
    CampaignRpc.leakCheck();                         // decision 11 rule 3, fails the run
  }
}
