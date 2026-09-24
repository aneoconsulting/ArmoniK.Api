"""Why the bench runs with the collector ON, and what turning it off was worth.

Work unit 1's harness disables the garbage collector for the measured rounds. For a
microbenchmark of a C-API primitive that is the right call: it removes a source of
variance that has nothing to do with what is being measured.

**For a codec arm it is a systematic bias in favour of exactly one arm of exactly one
column.** A facade decode of P2.2 builds on the order of ten thousand GC-tracked objects
-- 500 `TaskDetailed`, their nested children, their lists and their dicts, and every
generated C extension type carries `Py_TPFLAGS_HAVE_GC` because it holds `PyObject *`
members that can form cycles. `FromString` builds a upb arena and one Python wrapper. So
the collector's work is the facade's work, and subtracting it subtracts from one side.

This script measures how much, for both arms and both directions, in one process with
everything else held still. `bench.py` passes `gc_enabled=True` because of it; `mech/`
keeps the old default, because README R4 says a frozen column's text does not move.

**It was the RPC arm that found this**, and it could not have been anything else in the
slice: a real gRPC server cannot run with the collector off, so the RPC arm's in-process
control disagreed with `bench.py` on the same payload in the same interpreter, and the
only thing between them was `gc.disable()`. Defect D11.

Usage:  python gcbias.py
"""

import gc
import os
import statistics
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import allocator  # noqa: E402
_WARM = allocator.warm_up()

import arms  # noqa: E402

ARMS = ("upb (incumbent)", "core-ffi / C ext type")
ROUNDS = 9


def _t(f, reps, rounds=ROUNDS):
    v = []
    for _ in range(rounds):
        t0 = time.perf_counter_ns()
        for _ in range(reps):
            f()
        v.append((time.perf_counter_ns() - t0) / reps)
    v.sort()
    return statistics.median(v), v[0], v[-1]


def main():
    out = sys.stdout
    print("# python slice: what disabling the collector was worth, and to whom", file=out)
    print("# interpreter:  %s" % sys.version.replace("\n", " "), file=out)
    print("# allocator:    pinned, as in bench.py (%s)"
          % ("applied" if _WARM else "not glibc"), file=out)
    print("#", file=out)
    print("# Same process, same payloads, same order; the ONLY difference is gc.disable().", file=out)
    print("# Interleaved per arm rather than one block each, so a drift in the machine", file=out)
    print("# cannot be read as a difference between the two settings.", file=out)
    print("", file=out)
    print("   %-5s %-7s %-24s %12s %12s %8s"
          % ("", "dir", "arm", "GC off ns", "GC on ns", "on/off"), file=out)
    worst = (0.0, "")
    for pid in arms.PAYLOADS:
        n = len(arms.reference(pid))
        reps = 64 if n < 5_000 else (8 if n < 300_000 else 4)
        for dname, mk in (("encode", arms.encode_arms), ("decode", arms.decode_arms)):
            for nm, f in mk(pid):
                if nm not in ARMS:
                    continue
                f()
                gc.disable()
                off, offlo, offhi = _t(f, reps)
                gc.enable()
                on, onlo, onhi = _t(f, reps)
                r = on / off
                flag = ""
                if r > 1.05 and onlo > offhi:
                    flag = "  <--"
                    if r > worst[0]:
                        worst = (r, "%s %s %s" % (pid, dname, nm))
                print("   %-5s %-7s %-24s %12.0f %12.0f %8.2f%s"
                      % (pid, dname, nm, off, on, r, flag), file=out)

    print("", file=out)
    print("# Flagged where the two spreads do not touch and the ratio is over 1.05.", file=out)
    print("# Every flagged row is the FACADE's DECODE and nothing else: an encode writes", file=out)
    print("# one buffer and allocates no graph, and upb's decode allocates an arena the", file=out)
    print("# collector never walks. Worst row: %s at %.2f." % (worst[1], worst[0]), file=out)
    print("# So `gc.disable()` is not neutral tuning: it removes the collector's work from", file=out)
    print("# one arm of one column. `bench.py` KEEPS the collector off for its interleaved", file=out)
    print("# rounds (enabling it there mis-attributes each collection to whichever case", file=out)
    print("# trips the threshold; JOURNAL J28), so this script is where that cost is shown,", file=out)
    print("# one payload and one direction at a time, nothing interleaved.", file=out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
