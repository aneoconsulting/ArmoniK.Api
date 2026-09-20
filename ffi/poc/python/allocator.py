"""Why a 218 KiB encode has two different absolutes, and neither of them is the codec.

Work unit 2 measured `upb` encoding P1.2 at 344-358 us and the composed arm at 243-248
us, so the composed arm came out at **0.68-0.72 of the incumbent**.  Work unit 3, same
machine, same core, same shim, same bytes, measured the composed arm at 247-248 us -- and
the incumbent at 195-197 us, so **1.26**.  One arm moved by 1.8x between two runs of the
same benchmark and the other did not move at all.

It is not the fixture (`build_upb` was rewritten between the two runs; this script's
sibling experiment in JOURNAL.md J26 shows the hand-written copy, the plan-driven copy
and `FromString` all serialise within 1% of each other).  It is the process's malloc
state.  glibc gives back a buffer above `M_MMAP_THRESHOLD` (128 KiB by default) when it
is freed, either by `munmap` or by trimming the top of the heap, and the next call faults
it in again.  Once the process has allocated and freed something LARGER, glibc raises its
own thresholds and the same buffer is recycled warm.  So the encode absolute for a
payload above ~200 KiB depends on what the process allocated earlier -- and in a
benchmark, that means it depends on which payloads are in the run and on the calibration
order, neither of which is a property of the codec.

Work unit 2's bench carried M1 only, so nothing in it ever allocated more than 218 KiB
and the incumbent ran cold.  Work unit 3's bench carries M2 as well, `harness.calibrate`
touches every case before the first round, and P2.4's 979 KiB output warms the allocator
for everything that follows.

This script makes the switch explicit instead of accidental: `mallopt(M_TOP_PAD, 8 MiB)`
in one process and not in the other, everything else identical.  It is the control that
`bench.py` now applies to every run, and the reason `bench.py` applies it is that WARM is
both the state a long-lived gRPC server is actually in and the state that flatters the
incumbent more than it flatters us (see the table it prints).

Usage:  python allocator.py            # runs both halves, in two subprocesses
        python allocator.py cold|warm  # one half, for a by-hand check
"""

import ctypes
import os
import statistics
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))

M_TRIM_THRESHOLD, M_TOP_PAD, M_MMAP_THRESHOLD = -1, -2, -3
TOP_PAD = 8 << 20

# Only these two arms.  The question is whether the INCUMBENT's absolute is stable, and
# the composed arm is here to answer "does it move the same way" -- adding the other six
# arms would not change the answer and would triple the runtime.
ARMS = ("upb (incumbent)", "core-ffi / C ext type")
ROUNDS = 9


def _mallopt(param, value):
    """Returns True if glibc took it.  Not glibc -> no mallopt -> the run is 'cold'."""
    try:
        return bool(ctypes.CDLL("libc.so.6").mallopt(param, value))
    except OSError:
        return False


def warm_up():
    """Put the allocator in the state a long-lived process is in.

    `M_TOP_PAD` is the knob rather than `M_MMAP_THRESHOLD` because the measurement says
    so: raising the mmap threshold alone recovers nothing (347 us), raising the trim
    threshold alone recovers a third (296 us), and either `M_TOP_PAD` alone or both of
    the others together recovers all of it (194-195 us).  So the cost is the heap being
    trimmed back to the OS between calls, not the buffer being mmap'd.
    """
    return _mallopt(M_TOP_PAD, TOP_PAD)


def _t(f, reps, rounds=ROUNDS):
    v = []
    for _ in range(rounds):
        t0 = time.perf_counter_ns()
        for _ in range(reps):
            f()
        v.append((time.perf_counter_ns() - t0) / reps)
    v.sort()
    return statistics.median(v), v[0], v[-1]


def half(mode):
    """One process.  Nothing is imported before the mallopt: `arms` allocates."""
    if mode == "warm":
        print("# mallopt(M_TOP_PAD, %d) -> %s" % (TOP_PAD, warm_up()))
    else:
        print("# no mallopt: glibc's defaults, and its own dynamic adjustment")
    sys.path.insert(0, HERE)
    import arms as A

    for pid in A.PAYLOADS:
        n = len(A.reference(pid))
        reps = 256 if n < 5_000 else (64 if n < 300_000 else 16)
        for dname, mk in (("encode", A.encode_arms), ("decode", A.decode_arms)):
            for nm, f in mk(pid):
                if nm not in ARMS:
                    continue
                f(), f()          # one warm-up call, outside the clock
                med, lo, hi = _t(f, reps)
                print("%s\t%s\t%s\t%d\t%.0f\t%.0f\t%.0f"
                      % (pid, dname, nm, n, med, lo, hi))


def both():
    rows = {}
    for mode in ("cold", "warm"):
        r = subprocess.run([sys.executable, os.path.abspath(__file__), mode],
                           capture_output=True, text=True, cwd=HERE)
        if r.returncode:
            sys.stderr.write(r.stdout + r.stderr)
            return 1
        for ln in r.stdout.splitlines():
            if ln.startswith("#"):
                print("# %-5s %s" % (mode, ln[2:]))
                continue
            pid, d, nm, n, med, lo, hi = ln.split("\t")
            rows.setdefault((pid, d, nm, int(n)), {})[mode] = (
                float(med), float(lo), float(hi))

    print("\n# cold: a fresh process that has allocated nothing larger than the payload.")
    print("# warm: the same process with mallopt(M_TOP_PAD, 8 MiB) set before the first")
    print("#       allocation.  Identical bytes, identical arms, identical order.")
    print("#")
    print("%-5s %-7s %-24s %8s %11s %11s %11s %11s %6s"
          % ("", "", "arm", "bytes", "cold med", "cold min", "warm med", "warm max",
             "c/w"))
    for (pid, d, nm, n) in sorted(rows):
        c, w = rows[(pid, d, nm, n)]["cold"], rows[(pid, d, nm, n)]["warm"]
        # Flagged only when the two spreads do not touch: a ratio on its own picks
        # up the jitter of a 2 us row as readily as a real 2x on a 400 us one.
        flag = "  <-- " if (c[0] / w[0] > 1.15 and c[1] > w[2]) else ""
        print("%-5s %-7s %-24s %8d %11.0f %11.0f %11.0f %11.0f %6.2f%s"
              % (pid, d, nm, n, c[0], c[1], w[0], w[2], c[0] / w[0], flag))

    print("\n# The rows marked <-- are the ones where the ANSWER depends on the state of")
    print("# the allocator rather than on the codec.  Every one of them is an encode")
    print("# whose output is larger than anything the process had allocated BEFORE it:")
    print("# P1.2 at 218 KiB, which is the first thing past the 128 KiB threshold, and")
    print("# P2.4 at 979 KiB, which is the first thing past what P1.2 raised it to.")
    print("# P2.2 and P2.3 sit between the two and are clean, because by the time they")
    print("# run glibc has already adjusted -- which is the whole point: the figure")
    print("# depends on WHAT RAN BEFORE IT, so it is a property of the case list and not")
    print("# of the codec.  Every decode is clean at every size, because a decode")
    print("# allocates many small objects rather than one large buffer.")
    return 0


if __name__ == "__main__":
    if len(sys.argv) > 1:
        half(sys.argv[1])
        sys.exit(0)
    sys.exit(both())
