"""ABI v1 obligation 12.5, and README section 9's question: can it survive the GIL?

**No slice in the branch has a concurrency suite**, and 12.5 is the obligation with the
most evidence behind it and the least existence: the Rust slice found a shared-mutable
client defect *by accident*, because stage 4 happened to ask for eight calls in flight.
This is the suite it asks for, in Python:

*   **at least two payload shapes** -- P1.2 (M1, a leaf element) and P2.2 (M2, a non-leaf
    element with a map). One shape reports zero wrong bytes with a per-thread-state defect
    present and absent alike;
*   **threads run in sequence as well as together**, so "it works" is distinguished from
    "it works when nothing else is running";
*   **every encode asserted against a reference**, never counted. A suite that counts
    completions passes with a defect that corrupts every byte.

And two things that are Python's and nobody else's.

**The shared-facade case.** The Rust defect was one client shared across threads. The
analogue here is one FACADE shared across threads, which is what an application does when
it caches a response and serialises it from a pool. Both are run: per-thread objects and
one shared object, and the second is the one that finds a shim keeping state anywhere but
its own stack.

**The GIL.** `SerializeToString` holds the GIL, and so does this shim -- it cannot let go,
because the core calls back into the host for every element and every one of those reads a
Python object. So the scaling table below is not expected to show a speedup, and what it is
FOR is to say which of the two is worse and by how much: a lock held across a call that
also does I/O-free CPU work is the shape that decides whether a native codec can ever be
put behind a thread pool in this host. Measured against a control that DOES release it.

Usage:  python concurrency.py [--threads 1,2,4] [--iters N]
"""

import os
import sys
import threading
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import allocator  # noqa: E402  (before `arms`: the same pin the bench uses)
_WARM = allocator.warm_up()

import arms  # noqa: E402

SHAPES = ["P1.2", "P2.2"]          # 12.5's "at least two payload shapes"
THREADS = [1, 2, 4]
ITERS = 40


def _reference(pid):
    return arms.reference(pid)


def _encoders(pid):
    """(name, factory) where factory() -> a callable returning bytes.

    A FACTORY rather than a callable, so the per-thread case can build one facade per
    thread and the shared case can build one and hand it to all of them. The difference
    between those two is the whole point of the shared-mutable half of 12.5.
    """
    root = arms.ROOT_OF[pid]
    out = []
    if arms._pb2 is not None:
        out.append(("upb (incumbent)",
                    lambda: arms.build_upb_native(pid).SerializeToString))
    if arms._ffi is not None:
        def mk_cext(_p=pid, _r=root):
            fc = arms.build_facade(_p, arms.CT_CEXT)
            return lambda: arms._ffi.encode("cext", _r, fc)
        out.append(("core-ffi / C ext type", mk_cext))
    return out


def _run(factory, iters, want, shared=None):
    """One thread's work. Returns the number of WRONG results, not the number of calls."""
    fn = shared if shared is not None else factory()
    bad = 0
    for _ in range(iters):
        if fn() != want:
            bad += 1
    return bad


def correctness(out):
    print("\n## 12.5: every encode asserted, two shapes, sequential and concurrent", file=out)
    print("#  `shared` is one facade object handed to every thread, which is the analogue", file=out)
    print("#  of the shared client the rust slice tripped over. `per-thread` is one each.", file=out)
    print("#", file=out)
    print("#  The reference is THE ARM'S OWN output, taken once before any thread starts,", file=out)
    print("#  and `canon` says whether that equals the manifest's canonical bytes. It is", file=out)
    print("#  not the manifest itself, because `SerializeToString` does not sort map", file=out)
    print("#  entries and the canonical form does, so the incumbent legally writes another", file=out)
    print("#  accepted form on M2 -- which conformance.py checks and this suite must not", file=out)
    print("#  re-litigate. The question here is whether CONCURRENCY changes the bytes.", file=out)
    print("   %-6s %-24s %-5s %-11s %-7s %8s %8s"
          % ("shape", "arm", "canon", "objects", "threads", "encodes", "wrong"), file=out)
    fails = 0
    for pid in SHAPES:
        canonical = _reference(pid)
        for name, factory in _encoders(pid):
            want = factory()()
            canon = "yes" if want == canonical else "alt"
            for mode in ("per-thread", "shared"):
                shared = factory() if mode == "shared" else None
                for n in THREADS:
                    bad = [0] * n
                    if n == 1:
                        # In sequence: the same total work on one thread, so a failure
                        # that is really a defect is told from one that is a race.
                        bad[0] = _run(factory, ITERS * len(THREADS), want, shared)
                        total = ITERS * len(THREADS)
                    else:
                        ths = []
                        for i in range(n):
                            def work(i=i):
                                bad[i] = _run(factory, ITERS, want, shared)
                            ths.append(threading.Thread(target=work))
                        for t in ths:
                            t.start()
                        for t in ths:
                            t.join()
                        total = ITERS * n
                    w = sum(bad)
                    fails += bool(w)
                    print("   %-6s %-24s %-5s %-11s %-7s %8d %8d%s"
                          % (pid, name, canon, mode, "seq" if n == 1 else n, total, w,
                             "   WRONG BYTES" if w else ""), file=out)
    return fails


def scaling(out):
    print("\n## the GIL: wall-clock throughput against one thread", file=out)
    print("#  Both arms hold the GIL for the whole call, so 1.0 is the expected figure and", file=out)
    print("#  anything above it would be the surprise. The control at the foot releases it.", file=out)
    print("   %-6s %-24s %-7s %10s %8s" % ("shape", "arm", "threads", "ns/encode",
                                           "speedup"), file=out)
    for pid in SHAPES:
        for name, factory in _encoders(pid):
            base = None
            for n in THREADS:
                fns = [factory() for _ in range(n)]
                for f in fns:
                    f()                      # warm, outside the clock
                barrier = threading.Barrier(n)
                t0 = [0.0]

                def work(f=None):
                    barrier.wait()
                    for _ in range(ITERS):
                        f()

                ths = [threading.Thread(target=work, kwargs={"f": fns[i]})
                       for i in range(n)]
                start = time.perf_counter_ns()
                for t in ths:
                    t.start()
                for t in ths:
                    t.join()
                dt = time.perf_counter_ns() - start
                per = dt / (ITERS * n)
                base = base if base is not None else per
                print("   %-6s %-24s %-7d %10.0f %8.2f"
                      % (pid, name, n, per, base / per), file=out)
    return 0


def gil_control(out):
    """What a CPU-bound call that DOES release the GIL scales like, same machine.

    `hashlib` releases the GIL for a buffer over 2 KiB, so a 4 MB digest is real CPU work
    with the lock dropped -- no C of this slice's own required, and nothing about the
    codec. Work unit 1 priced `Py_BEGIN_ALLOW_THREADS` at 34.2-34.7 ns as a pair of calls;
    this is the other half of that number, and it is what says whether a 1.0 in the table
    above is "the GIL" or "this container has one core to spare".
    """
    import hashlib
    buf = b"\x5a" * (4 << 20)
    print("\n## the control: CPU-bound work that RELEASES the GIL, same threads", file=out)
    print("#  hashlib drops the lock above 2 KiB, so this is the parallelism the machine", file=out)
    print("#  actually has. Read the table above against THIS, not against 1.0.", file=out)
    print("   %-7s %12s %8s" % ("threads", "ns/digest", "speedup"), file=out)
    base = None
    for n in THREADS:
        hashlib.sha256(buf).digest()
        barrier = threading.Barrier(n)
        reps = 8

        def work():
            barrier.wait()
            for _ in range(reps):
                hashlib.sha256(buf).digest()

        ths = [threading.Thread(target=work) for _ in range(n)]
        start = time.perf_counter_ns()
        for t in ths:
            t.start()
        for t in ths:
            t.join()
        per = (time.perf_counter_ns() - start) / (reps * n)
        base = base if base is not None else per
        print("   %-7d %12.0f %8.2f" % (n, per, base / per), file=out)
    return 0


def main():
    global THREADS, ITERS
    if "--threads" in sys.argv:
        THREADS = [int(x) for x in sys.argv[sys.argv.index("--threads") + 1].split(",")]
    if "--iters" in sys.argv:
        ITERS = int(sys.argv[sys.argv.index("--iters") + 1])
    out = sys.stdout
    print("# python slice: concurrency (ABI v1 obligation 12.5, README section 9)", file=out)
    print("# interpreter:  %s" % sys.version.replace("\n", " "), file=out)
    print("# shapes:       %s" % ", ".join(SHAPES), file=out)
    print("# threads:      %s, and `seq` is the same total work on one thread"
          % ", ".join(str(t) for t in THREADS), file=out)
    print("# allocator:    pinned, as in bench.py (%s)"
          % ("applied" if _WARM else "not glibc"), file=out)
    for a in arms.absent:
        print("# ARM ABSENT: %s" % a, file=out)
    fails = correctness(out)
    scaling(out)
    gil_control(out)
    print("\n%s" % ("CONCURRENCY SUITE PASSES" if not fails
                    else "%d ROW(S) PRODUCED WRONG BYTES" % fails), file=out)
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
