"""The campaign harness's shared parts (design/CAMPAIGN.md, W11): pinning, clocks, the log
header (requirement 27) and the one-JSON-object-per-sample body (requirement 28).

Nothing here is a measurement policy of its own: every constant a sample depends on is
printed in the header, and every sample is written, not a summary.

Clocks (requirement 21):
  codec   CLOCK_THREAD_CPUTIME_ID of the measuring thread (the codec suite is one thread)
  rpc     CLOCK_PROCESS_CPUTIME_ID of the client process (every client thread counted; the
          server is another process), with wall (perf_counter_ns) beside it
Both are nanosecond clocks; nothing coarser than 1 us is used.
"""
import json
import os
import platform
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
FFI = os.path.dirname(os.path.dirname(HERE))
SLICE = "python"


def thread_cpu_ns():
    return time.clock_gettime_ns(time.CLOCK_THREAD_CPUTIME_ID)


def proc_cpu_ns():
    return time.clock_gettime_ns(time.CLOCK_PROCESS_CPUTIME_ID)


def wall_ns():
    return time.perf_counter_ns()


def cpuset(var):
    """`AK_CPU_CLIENT=0-3` or `0,1,2,3` -> {0, 1, 2, 3}; None when unset."""
    v = os.environ.get(var, "").strip()
    if not v:
        return None
    out = set()
    for part in v.split(","):
        if "-" in part:
            a, b = part.split("-")
            out.update(range(int(a), int(b) + 1))
        elif part:
            out.add(int(part))
    return out


def pin(var):
    """Pin THIS process (and every thread it creates afterwards) to the set in `var`.
    Called first, before any thread exists. Returns the affinity actually in force."""
    s = cpuset(var)
    if s:
        os.sched_setaffinity(0, s)
    return sorted(os.sched_getaffinity(0))


def _read(path, default="unknown"):
    try:
        with open(path) as f:
            return f.read().strip()
    except OSError:
        return default


def machine():
    model = "unknown"
    for ln in _read("/proc/cpuinfo", "").splitlines():
        if ln.startswith("model name"):
            model = ln.split(":", 1)[1].strip()
            break
    turbo = _read("/sys/devices/system/cpu/intel_pstate/no_turbo", None)
    if turbo is not None:
        turbo = "off" if turbo == "1" else "on"
    else:
        b = _read("/sys/devices/system/cpu/cpufreq/boost", None)
        turbo = ("on" if b == "1" else "off") if b is not None else "unknown (no intel_pstate/boost knob)"
    cmdline = _read("/proc/cmdline", "")
    iso = [t for t in cmdline.split() if t.startswith(("isolcpus", "nohz_full"))]
    return {
        "cpu_model": model,
        "cpus_online": os.cpu_count(),
        "smt": _read("/sys/devices/system/cpu/smt/active"),
        "governor": _read("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor"),
        "turbo": turbo,
        "kernel": platform.release(),
        "isolation": os.environ.get("AK_ISOLATION") or (" ".join(iso) if iso else
                                                         "none detected (isolated=%s)" % _read("/sys/devices/system/cpu/isolated", "?")),
        "cpu_client": os.environ.get("AK_CPU_CLIENT", "unset"),
        "cpu_server": os.environ.get("AK_CPU_SERVER", "unset"),
        "cpu_os": os.environ.get("AK_CPU_OS", "unset (everything not in CLIENT or SERVER)"),
    }


def commit_state():
    """(short sha, dirty?) of the tree the run executes. The core is built from the
    AK_SNAPSHOT archive when set, so only poc/python can be dirty."""
    sha = subprocess.run(["git", "-C", HERE, "rev-parse", "--short", "HEAD"],
                         capture_output=True, text=True).stdout.strip()
    paths = [HERE]
    if not os.environ.get("AK_SNAPSHOT"):
        paths.append(os.path.join(FFI, "poc", "codec"))
    dirty = subprocess.run(["git", "-C", HERE, "status", "--porcelain", "--"] + paths,
                           capture_output=True, text=True).stdout.strip()
    return sha, bool(dirty)


def versions():
    out = {"python": sys.version.split()[0], "implementation": platform.python_implementation()}
    try:
        import google.protobuf as pb
        from google.protobuf.internal import api_implementation
        out["protobuf"] = "%s (%s)" % (pb.__version__, api_implementation.Type())
    except Exception as e:  # noqa: BLE001
        out["protobuf"] = "absent: %s" % e
    try:
        import grpc
        out["grpcio"] = grpc.__version__
    except Exception:  # noqa: BLE001
        out["grpcio"] = "absent"
    return out


def buildinfo():
    p = os.path.join(HERE, "build", "py%d.%d" % sys.version_info[:2], "buildinfo.json")
    try:
        with open(p) as f:
            return json.load(f)
    except OSError:
        return {"missing": p}


def _snapshot():
    """The RESOLVED commit the core was built from (AK_SNAPSHOT may say HEAD, which moves)."""
    v = os.environ.get("AK_SNAPSHOT")
    if not v:
        return "none (working tree)"
    sha = subprocess.run(["git", "-C", HERE, "rev-parse", "--short", v], capture_output=True, text=True).stdout.strip()
    return "%s (%s)" % (sha, v)


class Log:
    """Header lines start with '#'; every other line is one JSON sample. Samples are held
    until `close(ok=True)`: a run that aborts writes its header, the abort, and NO sample
    (requirement 18)."""

    def __init__(self, path, suite, allow_dirty=False, smoke=False, build="full"):
        """`build`: "full" (unknown-fields on) or "nounk" (the no-unknown build, WP5 step
        10); written into EVERY sample so a summary never pools or pairs across builds
        (R-H1)."""
        if build not in ("full", "nounk"):
            raise ValueError("build must be full or nounk, not %r" % build)
        self.path, self.suite, self.smoke, self.build = path, suite, smoke, build
        self.samples = []
        self.head = []
        sha, dirty = commit_state()
        if dirty and not allow_dirty:
            raise SystemExit("REFUSED: the tree is dirty (requirement 27); commit first, or "
                             "--allow-dirty for a smoke run only")
        self.commit = sha + (" DIRTY (smoke only)" if dirty else "")

    def header(self, **kv):
        m = machine()
        base = {"slice": SLICE, "suite": self.suite, "commit": self.commit,
                "date": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                "snapshot": _snapshot()}
        for block in (base, m, versions(), {"build": buildinfo()}, kv):
            for k, v in block.items():
                self.head.append("# %-16s %s" % (k + ":", v if not isinstance(v, (dict, list)) else json.dumps(v, sort_keys=True)))
        if self.smoke:
            self.head.append("# INSTRUMENTATION: a container smoke run (CAMPAIGN.md requirement 32). "
                             "No figure below is a result.")

    def sample(self, **kv):
        d = {"slice": SLICE, "suite": self.suite, "build": self.build}
        d.update({k: v for k, v in kv.items() if v is not None})
        self.samples.append(d)

    def note(self, s):
        self.head.append("# " + s)

    def close(self, ok=True, why=""):
        os.makedirs(os.path.dirname(os.path.abspath(self.path)), exist_ok=True)
        with open(self.path, "w") as f:
            f.write("\n".join(self.head) + "\n")
            if not ok:
                f.write("# ABORTED, NO FIGURE: %s\n" % why)
                return
            for s in self.samples:
                f.write(json.dumps(s, sort_keys=True) + "\n")
            f.write("# samples: %d\n" % len(self.samples))


def rotated(seq, r):
    """Requirement 22: the order of the arms is rotated by one each round."""
    k = r % len(seq) if seq else 0
    return list(seq[k:]) + list(seq[:k])
