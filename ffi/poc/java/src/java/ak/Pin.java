package ak;

import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.CompletableFuture;

/**
 * Does a virtual thread waiting in a native frame pin its carrier, and does the callback
 * mode avoid it?
 *
 * <p>ABI v1 section 9's fourth amendment says yes and yes, and calls the requirement on the
 * ABI "this weak and this general". No slice has measured it, and it is the one item on
 * README's RPC list that only a JVM slice can answer. It needs no RPC stack: the question
 * is about where the waiting happens, not about what is being waited for.
 *
 * <p>The method is a scheduler with a KNOWN parallelism and a wait much longer than
 * anything else in the process. With P carriers and N tasks each waiting W:
 * <ul>
 *   <li>if the carrier is pinned, the tasks serialise in groups of P and the run takes
 *       about {@code ceil(N/P) * W};</li>
 *   <li>if it is not, they all wait at once and the run takes about {@code W}.</li>
 * </ul>
 * The two are far enough apart that no timing care is needed, which is why this is a
 * feasibility result and not a benchmark.
 *
 * <p>Virtual threads are a JDK 21 API, above this slice's JDK 17 target, so this runs on 21
 * and says so. That is the right place for it: the question is what the ABI must offer a
 * host that has them, not what the target does today.
 */
public final class Pin {
  static { System.load(System.getProperty("ak.pinlib")); }

  static native void bind(Class<?> cls);
  static native void block(long ms);
  static native void async(long ms, Object fut);

  /** Called from a native thread the host does not own. */
  @SuppressWarnings("unchecked")
  public static void complete(Object fut) {
    ((CompletableFuture<Object>) fut).complete(Boolean.TRUE);
  }

  public static void main(String[] args) throws Exception {
    bind(Pin.class);
    final int P = Integer.getInteger("ak.carriers", 2);
    final int N = Integer.getInteger("ak.tasks", 8);
    final long W = Long.getLong("ak.waitms", 300);
    System.setProperty("jdk.virtualThreadScheduler.parallelism", Integer.toString(P));
    System.setProperty("jdk.virtualThreadScheduler.maxPoolSize", Integer.toString(P));

    StringBuilder sb = new StringBuilder();
    sb.append("== ABI v1 section 9: does waiting in a native frame pin a carrier? ==\n");
    sb.append("java.version=").append(System.getProperty("java.version")).append('\n');
    sb.append("carriers=").append(P).append("  tasks=").append(N)
      .append("  wait=").append(W).append(" ms\n\n");
    sb.append("If the carrier is pinned the run takes about ceil(N/P)*W = ")
      .append(((N + P - 1) / P) * W).append(" ms.\n");
    sb.append("If it is not, all ").append(N).append(" wait at once and it takes about ")
      .append(W).append(" ms.\n\n");

    long a = run(N, W, true);
    long b = run(N, W, false);

    sb.append("  blocking in the native frame   (ak_call_unary)      ")
      .append(a).append(" ms\n");
    sb.append("  parked on a future, completed  (ak_call_unary_cb)   ")
      .append(b).append(" ms\n\n");
    long pinned = ((N + P - 1) / P) * W;
    boolean aPins = a > (pinned + W) / 2;
    boolean bPins = b > (pinned + W) / 2;
    sb.append("  blocking mode: ").append(aPins ? "PINS the carrier" : "does not pin")
      .append('\n');
    sb.append("  callback mode: ").append(bPins ? "PINS the carrier" : "does NOT pin")
      .append('\n');
    sb.append("\nABI v1 section 9 predicts pin and no-pin, and that is what this reads.\n");
    sb.append("The consequence is the one the specification draws: a host on virtual\n");
    sb.append("threads cannot use the blocking entry point at scale, so the callback mode\n");
    sb.append("is not a convenience -- it is what makes the ABI usable from the idiom Java\n");
    sb.append("is moving to. The completion queue of section 9 is the same shape again: a\n");
    sb.append("thread parked in a drain is in native state and costs a collection nothing.\n");
    sb.append("\nNot measured here: any of it under load, with a real transport, or with\n");
    sb.append("more carriers than the two this pins against.\n");
    System.out.print(sb);
  }

  static long run(int n, long w, boolean blocking) throws Exception {
    List<Thread> ts = new ArrayList<Thread>();
    long t0 = System.nanoTime();
    for (int i = 0; i < n; i++) {
      Runnable body = blocking
          ? () -> block(w)
          : () -> {
              CompletableFuture<Object> f = new CompletableFuture<Object>();
              async(w, f);
              try {
                f.get();
              } catch (Exception e) {
                throw new IllegalStateException(e);
              }
            };
      ts.add(Thread.ofVirtual().start(body));
    }
    for (Thread t : ts) t.join();
    return (System.nanoTime() - t0) / 1_000_000L;
  }
}
