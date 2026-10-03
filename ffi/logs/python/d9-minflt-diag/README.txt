# Where Cf-drop's ~514 minor faults per call come from (pinned, d/16MiB, k = 1). Diagnosis only.
# Owner request, 2026-10-03. Container instrumentation: one container, a handful of runs, no timing.
# Script: probe.py.txt (a throwaway copy of scratchpad/flt/probe.py, never in the campaign path).
#   It mirrors a pyperf worker's setup (allocator readback with the block kept, cells(keys={d:16MiB}),
#   the checked setup calls), then per call: process minflt, per-thread minflt from
#   /proc/self/task/*/stat grouped by thread name, and mallinfo2 deltas. With "pool", the calls run
#   on a k = 1 camp_rpc.Pool thread, as the worker runs them. Server: the shared Rust rpc_server
#   over TCP (AK_SERVER_TCP=0). grpc-core sized to 8 by the sysconf shim, as in the grid.
# Files:
#   {default,pinned}-{A,Cf-drop}.json   calls on the MAIN thread after 20 warm-ups (6 calls)
#   {default,pinned}-Cf-w0.json         calls on the MAIN thread from the 3rd call on (12 calls)
#   {default,pinned}-Cf-pool.json       calls on a POOL thread, from its 1st call (14 calls)
#   {default,pinned}-Cf-st.json + .strace   the same as -pool, under strace -f -e
#       mmap,munmap,madvise,brk,mprotect,mremap,write (a CALLMARK write before each call)
#
# Findings:
# 1. It decays; it is not steady per call. On a pool thread both modes take one burst of about
#    512 faults (2 MiB of first-touched pages) in the first calls, then 0-2 per call. Pinned:
#    2,582 at call 1, then 513 at call 10 (first run) or 514 at call 2 (strace run), then 0-2.
#    Default: 3,086 at call 1, then 513 or 514 at call 4 in both runs, then 0-2. After 20
#    warm-ups on the main thread: A 1-3 per call in both modes; Cf-drop 0-1 in both modes.
# 2. Thread: the CALLER thread (comm "python3.12", the pool thread that runs the cell), not a
#    core worker. tokio-rt-worker threads take 0-11 per call in both modes.
# 3. glibc malloc, on the caller thread's own arena:
#    - pinned: each burst coincides with mallinfo2 arena +4,194,304 and uordblks +4,194,976, and
#      with mprotect(..., 4194304, PROT_READ|PROT_WRITE): growth of the thread arena's heap.
#      That is a new ~4 MiB block that stays allocated.
#    - default: the blocks are mmapped (4,198,400 B each, 5 or 6 at call 1, one more at call 3).
#      The 512-fault burst of call 4 has no syscall and no mallinfo change: the first touch of
#      a block mapped earlier.
#    The block size (4 MiB + 4 KiB) and its persistence fit the core's encode buffers for a 2 MiB
#    chunk, kept in the per-context spare ring (SPARES = 6, p1): new ones are allocated until
#    ring plus in-flight cover the stream. This is inferred from size and timing, not traced
#    into the core.
#    The encode context is per caller thread, and a new pool thread gets a fresh glibc arena, so
#    the setup's calls on the main thread do not warm it. On the main thread after the setup's
#    two calls: no burst over 11 per call in either mode.
# 4. Why pinned and not default in the smoke: not a different mechanism. Both modes take the
#    same kind of burst within the first few pool-thread calls. With pyperf's 1 warm-up of 1
#    loop, the value is the pool thread's 2nd call: pinned's burst landed there (3 smokes, and
#    this strace run), default's landed at call 3-4 (here) or 1 (the smoke). Which call takes it
#    varies between runs; in default the first-touch is deferred to a later call after an mmap.
#    So the 514 is a warm-up that ended too early, in pinned's case on the timed value.
# Unknown: why the burst lands on a given call (transport timing holding buffers is the likely
# variable, not measured); whether the campaign machine shows the same spacing; the exact core
# allocation (not traced with ltrace or a core build).
# One-line fix, not applied: warm each RPC benchmark up for enough calls on its own pool thread
# (several, not 1) before the first value, so the encode ring and the thread arena are full.
