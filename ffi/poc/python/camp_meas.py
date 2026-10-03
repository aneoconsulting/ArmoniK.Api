"""What the RPC grid records beside its clock (FIX-PLAN WP13; CAMPAIGN reqs 4, 17, 21 as amended).
No wire rule and no timing loop: counters and facts read from the kernel.

  task_clock()       perf task-clock of the whole process (PERF_TYPE_SOFTWARE,
                     PERF_COUNT_SW_TASK_CLOCK, pid 0, inherit): the process opens it itself
                     BEFORE it starts any thread, so every later thread (the client pool, the core
                     runtime's workers, grpc-core's threads) is counted, and the kernel sums the
                     inherited counters on read (exited threads included). Softirq time run in a
                     thread's context is in it; the process clock misses it under
                     CONFIG_IRQ_TIME_ACCOUNTING (req 21 as amended, findings/physical-probe.md 2).
                     None when perf_event_open refuses (stated by the caller).
  irq_snap(cpus)     /proc/stat irq and softirq (USER_HZ ticks) and /proc/softirqs NET_RX / NET_TX,
                     summed over a CPU set (the CLIENT set: this process's affinity), as the C++
                     slice reads them (poc/cpp/src/campaign_rpc.cpp irq_snap).
  tcp_sockets()      every TCP socket of this process (/proc/self/fd), with TCP_NODELAY read back
                     by getsockopt on the live socket: what grpc-core and the core's tonic client
                     actually set (req 17 as amended).
  thread_classes()   this process's threads by name (/proc/self/task/*/comm), digits folded.
  cpu_facts()        the affinity mask and the two sysconf counts grpc-core sizes itself from.
"""
import ctypes
import os
import re
import socket
import struct

_libc = ctypes.CDLL(None, use_errno=True)
_SYS_PERF_EVENT_OPEN = 298          # x86_64
_TC = []


def task_clock_open():
    """Open the process's task-clock counter (once). Call before any thread is started."""
    if _TC:
        return _TC[0]
    attr = (ctypes.c_uint64 * 16)()
    attr[0] = 1 | (128 << 32)       # type PERF_TYPE_SOFTWARE, size 128
    attr[1] = 1                     # config PERF_COUNT_SW_TASK_CLOCK
    attr[5] = 1 << 1                # inherit; enabled
    fd = _libc.syscall(_SYS_PERF_EVENT_OPEN, attr, 0, -1, -1, 8)   # PERF_FLAG_FD_CLOEXEC
    _TC.append(fd if fd >= 0 else None)
    if fd < 0:
        _TC.append(os.strerror(ctypes.get_errno()))
    return _TC[0]


def task_clock_refusal():
    return _TC[1] if len(_TC) > 1 else None


def task_clock():
    """Task-clock nanoseconds of this process since the counter was opened, or None."""
    fd = _TC[0] if _TC else None
    if fd is None:
        return None
    return struct.unpack("Q", os.read(fd, 8))[0]


def cpu_list(s):
    out = set()
    for part in s.split(","):
        if part:
            a, _, b = part.partition("-")
            out.update(range(int(a), int(b or a) + 1))
    return out


def irq_snap(cpus):
    r = {"irq_ticks": 0, "softirq_ticks": 0, "net_rx": 0, "net_tx": 0}
    with open("/proc/stat") as f:
        for ln in f:
            m = re.match(r"cpu(\d+) (.*)", ln)
            if m and int(m.group(1)) in cpus:
                v = m.group(2).split()
                r["irq_ticks"] += int(v[5])
                r["softirq_ticks"] += int(v[6])
    with open("/proc/softirqs") as f:
        cols = [int(h[3:]) for h in f.readline().split()]
        for ln in f:
            name, *vals = ln.split()
            key = {"NET_RX:": "net_rx", "NET_TX:": "net_tx"}.get(name)
            if key:
                r[key] += sum(int(v) for c, v in zip(cols, vals) if c in cpus)
    return r


def irq_delta(a, b):
    return {k: b[k] - a[k] for k in a}


def tcp_sockets():
    """[(fd, local_port, peer_port, nodelay)] for every TCP socket of this process."""
    out = []
    for name in os.listdir("/proc/self/fd"):
        fd = int(name)
        try:
            if not os.readlink("/proc/self/fd/%d" % fd).startswith("socket:"):
                continue
            s = socket.socket(fileno=os.dup(fd))
        except OSError:
            continue
        try:
            if s.family not in (socket.AF_INET, socket.AF_INET6) or s.type != socket.SOCK_STREAM:
                continue
            try:
                peer = s.getpeername()[1]
            except OSError:
                peer = 0                  # a listening or unconnected socket
            out.append((fd, s.getsockname()[1], peer, s.getsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY)))
        finally:
            s.close()
    return out


def nodelay_summary(port=None):
    """{"tcp": n, "nodelay_on": m, "to_server": k}: sockets connected to `port` (the server's) and
    how many of them have TCP_NODELAY set, read back on the live socket."""
    socks = [x for x in tcp_sockets() if x[2]]
    to = [x for x in socks if port is None or x[2] == port]
    return {"tcp": len(socks), "to_server": len(to), "nodelay_on": sum(1 for x in to if x[3])}


def thread_classes():
    n = {}
    for t in os.listdir("/proc/self/task"):
        try:
            with open("/proc/self/task/%s/comm" % t) as f:
                c = re.sub(r"\d+", "N", f.read().strip())
        except OSError:
            continue
        n[c] = n.get(c, 0) + 1
    return n


def cpu_facts():
    aff = sorted(os.sched_getaffinity(0))
    return {"affinity": ",".join(map(str, aff)), "affinity_count": len(aff),
            "sysconf_nprocessors_conf": os.sysconf("SC_NPROCESSORS_CONF"),
            "sysconf_nprocessors_onln": os.sysconf("SC_NPROCESSORS_ONLN"),
            "ld_preload": os.environ.get("LD_PRELOAD", ""), "ak_shim_ncpus": os.environ.get("AK_SHIM_NCPUS", "")}


D9_TUNABLES = "glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432"


def alloc_readback():
    """D9 (owner, 2026-10-03), the C# slice's startup check: one 16 MiB malloc through glibc,
    with mallinfo2's count of mmapped blocks (hblks) read before and after. Under glibc's
    defaults a 16 MiB block is mmapped (mmap_threshold 128 KiB, at most 32 MiB once raised by a
    free, so the check runs before any timing); under D9's tunables (mmap_threshold 32 MiB) it
    comes from the heap. Returns "mmapped" or "heap"."""
    class MI2(ctypes.Structure):
        _fields_ = [(n, ctypes.c_size_t) for n in ("arena", "ordblks", "smblks", "hblks", "hblkhd", "usmblks",
                                                   "fsmblks", "uordblks", "fordblks", "keepcost")]
    _libc.mallinfo2.restype = MI2
    _libc.malloc.restype = ctypes.c_void_p
    _libc.free.argtypes = [ctypes.c_void_p]
    h0 = _libc.mallinfo2().hblks
    p = _libc.malloc(16 << 20)
    h1 = _libc.mallinfo2().hblks
    _libc.free(p)
    return "mmapped" if h1 > h0 else "heap"


def alloc_check(mode):
    """Refuse (SystemExit, so no sample) unless the process's allocator matches `mode`, both by its
    environment (GLIBC_TUNABLES) and by the readback. Returns the readback for the header."""
    want_env = D9_TUNABLES if mode == "pinned" else None
    if os.environ.get("GLIBC_TUNABLES") != want_env:
        raise SystemExit("allocator: AK_CAMPAIGN_ALLOC=%s but GLIBC_TUNABLES=%r in this process"
                         % (mode, os.environ.get("GLIBC_TUNABLES")))
    rb = alloc_readback()
    want = "heap" if mode == "pinned" else "mmapped"
    if rb != want:
        raise SystemExit("allocator: AK_CAMPAIGN_ALLOC=%s but a 16 MiB malloc came from the %s (want %s)"
                         % (mode, "heap" if rb == "heap" else "mmap path", want))
    return rb
