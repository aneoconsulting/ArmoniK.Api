"""The measurement loop every python-slice benchmark shares.

Three properties, and each of them is a rule from `ffi/README.md` section 8
rather than a preference:

*   **Interleaved rounds** (R4).  Every case runs once per round and the round
    is repeated, so a ratio between two cases is formed under the same thermal
    and scheduler conditions rather than across two sequential blocks.
*   **A median with a min and a max** (R4 again, and the "ranges, never a single
    digit" rule).  A single number from a shared four-vCPU container is a
    fiction; the spread is the honest part.
*   **Calibration per case**, so a 2 ns row and a 2 us row both get a round long
    enough to be timed and neither costs the run a minute.

`perf_counter_ns` is the clock: `time.process_time` would hide the GIL-release
question this slice exists to ask, and `timeit` cannot interleave.
"""

import gc
import json
import statistics
import sys
import time

ROUNDS = 11
TARGET_NS = 20_000_000  # 20 ms per case per round


class Case:
    """One measured thing.

    `run(reps)` must do the work `reps` times and nothing else; the harness
    divides by `reps`, so anything `run` does outside the loop lands in every
    per-iteration figure.
    """

    __slots__ = ("group", "arm", "run", "reps", "times", "note", "unit")

    def __init__(self, group, arm, run, note="", unit="call"):
        self.group = group
        self.arm = arm
        self.run = run
        self.note = note
        self.unit = unit
        self.reps = 1
        self.times = []


def calibrate(cases, target_ns=TARGET_NS):
    for c in cases:
        reps = 64
        while True:
            t = time.perf_counter_ns()
            c.run(reps)
            dt = time.perf_counter_ns() - t
            if dt >= target_ns or reps >= 1 << 30:
                break
            # Grow by the measured shortfall rather than by doubling: fewer
            # calibration passes, and a case that is 1000x off does not take ten
            # rounds to find its size.
            grow = max(2.0, min(64.0, target_ns / max(dt, 1)))
            reps = int(reps * grow)
        c.reps = reps


def run(cases, rounds=ROUNDS, target_ns=TARGET_NS, gc_enabled=False):
    """`gc_enabled=False` is work unit 1's behaviour and stays the default, because the
    frozen mechanism columns were measured with it and README R4's last paragraph says
    their text does not move.

    **Nothing passes True, and the flag exists to record why.** Disabling the collector
    does remove a cost that is almost entirely one side's: a facade decode builds ~10,000
    GC-tracked objects per P2.2 call and `FromString` builds an arena and one wrapper.
    Measured in ISOLATION, GC off against GC on: every encode row 0.99-1.02 and upb's
    decode 0.97-1.02, against **1.09 to 1.26 for the facade's decode**.

    But turning it on HERE made things worse, not better. In an interleaved run of ~500
    cases the collector fires wherever the allocation threshold trips, so the cost lands on
    an arbitrary case: P2.2's decode came out at 7.3x the incumbent with this flag set,
    against 1.26x for the identical call measured on its own, and the live set does not
    explain it -- holding all sixteen payloads' fixtures alive makes the isolated figure
    slightly faster. A figure that depends on what else is in the run is the defect
    `allocator.py` exists for, one layer up.

    So: the collector stays off where a RATIO is formed, and `gcbias.py` prices it where it
    can be attributed. Defect D11, found by the RPC arm -- which cannot disable the
    collector, because it runs a real server -- and its first fix was worse than itself.
    """
    gc_was = gc.isenabled()
    if not gc_enabled:
        gc.disable()
    try:
        calibrate(cases, target_ns)
        for _ in range(rounds):
            for c in cases:
                t = time.perf_counter_ns()
                c.run(c.reps)
                c.times.append((time.perf_counter_ns() - t) / c.reps)
    finally:
        if gc_was:
            gc.enable()
    return cases


def stats(c):
    return {
        "group": c.group,
        "arm": c.arm,
        "note": c.note,
        "unit": c.unit,
        "reps": c.reps,
        "median_ns": statistics.median(c.times),
        "min_ns": min(c.times),
        "max_ns": max(c.times),
    }


def report(cases, baseline=None, out=sys.stdout):
    """Print one table per group.

    `baseline` is a callable (group) -> arm name.  Where it returns an arm the
    group carries a ratio column against it; a group whose question is "what
    does this cost" rather than "what does this cost against X" gets none, and
    an invented baseline is worse than no column.
    """
    rows = [stats(c) for c in cases]
    wa = max(len(r["arm"]) for r in rows) + 2
    seen = []
    for r in rows:
        if r["group"] not in seen:
            seen.append(r["group"])
    for g in seen:
        grp = [r for r in rows if r["group"] == g]
        base = baseline(g) if baseline else None
        bref = next((r for r in grp if r["arm"] == base), None)
        print(f"\n## {g}", file=out)
        head = f"{'arm':<{wa}} {'reps':>12} {'median ns':>12} {'min':>10} {'max':>10}"
        if bref:
            head += f" {'/ ' + base:>22}"
        print(head, file=out)
        for r in grp:
            line = (f"{r['arm']:<{wa}} {r['reps']:>12} {r['median_ns']:>12.2f}"
                    f" {r['min_ns']:>10.2f} {r['max_ns']:>10.2f}")
            if bref:
                line += f" {r['median_ns'] / bref['median_ns']:>22.3f}"
            print(line, file=out)
            if r["note"]:
                print(f"{'':<{wa}}   {r['note']}", file=out)
    return rows


def dump_json(rows, path):
    with open(path, "w") as f:
        json.dump(rows, f, indent=1)
        f.write("\n")
