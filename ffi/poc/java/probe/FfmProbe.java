// The FFM crossing price, as a SECONDARY arm and never as a target.
//
// README section 5 is explicit: "FFM is a JDK 22 API, so the floor and the target are both
// JNI. FFM is a secondary arm, not a target." And R7 is explicit about what a number from
// it may be used for: "A ratio between an arm on one binding mechanism and an arm on
// another is a comparison of mechanisms, not of ABI shapes, and mistaking one for the
// other has already produced retracted figures."
//
// So this measures one thing -- what a downcall costs on this machine -- so that the
// cross-language crossing table of README section 2 has a Java row taken here rather than
// inherited from a container nobody can identify. It builds no binding and encodes nothing.
//
// It runs on JDK 21 with --enable-preview, because that is the newest JDK this container
// has and `java.lang.foreign` is a preview API there (JEP 442). A preview API is not the
// same artifact as the final one and the number is labelled accordingly.
import java.lang.foreign.Arena;
import java.lang.foreign.FunctionDescriptor;
import java.lang.foreign.Linker;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.SymbolLookup;
import java.lang.foreign.ValueLayout;
import java.lang.invoke.MethodHandle;

public final class FfmProbe {
  static long sink;

  public static void main(String[] args) throws Throwable {
    String lib = System.getProperty("ak.core");
    Arena arena = Arena.global();
    SymbolLookup lookup = SymbolLookup.libraryLookup(java.nio.file.Path.of(lib), arena);
    Linker linker = Linker.nativeLinker();

    MemorySegment noop = lookup.find("ak_noop").orElseThrow();
    FunctionDescriptor fd =
        FunctionDescriptor.of(ValueLayout.JAVA_LONG, ValueLayout.JAVA_LONG);

    // Three shapes, because they are three different prices and a single "FFM crossing"
    // figure hides which one it is.
    MethodHandle plain = linker.downcallHandle(noop, fd);
    MethodHandle trivial = linker.downcallHandle(noop, fd, Linker.Option.isTrivial());

    final int N = 4_000_000;
    final int R = 9;
    long[][] ps = new long[2][R];
    for (int w = 0; w < 2_000_000; w++) {
      sink += (long) plain.invokeExact((long) w);
      sink += (long) trivial.invokeExact((long) w);
    }
    for (int r = 0; r < R; r++) {
      for (int k = 0; k < 2; k++) {
        MethodHandle h = k == 0 ? plain : trivial;
        long t0 = System.nanoTime();
        long s = 0;
        for (int i = 0; i < N; i++) s += (long) h.invokeExact((long) i);
        ps[k][r] = (System.nanoTime() - t0) * 1000L / N;
        sink += s;
      }
    }

    String[] names = {"downcall", "downcall isTrivial()"};
    StringBuilder sb = new StringBuilder();
    sb.append("== FFM downcall price, SECONDARY arm, preview API on JDK 21 ==\n");
    sb.append("java.version=").append(System.getProperty("java.version")).append('\n');
    sb.append("core=").append(lib).append("  sink=").append(sink).append("\n\n");
    sb.append("A JDK 22 API measured on JDK 21 as a preview, because this container has no\n");
    sb.append("JDK 22 and the proxy does not reach a JDK distributor. README 5: FFM is a\n");
    sb.append("secondary arm and never a target, and R7: a ratio between two BINDING\n");
    sb.append("MECHANISMS is not a ratio between ABI shapes.\n\n");
    for (int k = 0; k < 2; k++) {
      long[] v = ps[k].clone();
      java.util.Arrays.sort(v);
      sb.append(String.format("  %-22s min %6.3f  med %6.3f  max %6.3f ns%n", names[k],
          v[0] / 1000.0, v[R / 2] / 1000.0, v[R - 1] / 1000.0));
    }
    sb.append("\n`isTrivial()` tells the linker the call will not block, upcall or trap, so\n");
    sb.append("it may skip the thread-state transition. A codec entry point that makes\n");
    sb.append("reverse calls is NOT trivial and may not use it, which is why both are here:\n");
    sb.append("the cheap number is the one a bulk-bytes path could have and the dear one is\n");
    sb.append("what the element entry points would pay.\n");
    System.out.print(sb);
  }
}
