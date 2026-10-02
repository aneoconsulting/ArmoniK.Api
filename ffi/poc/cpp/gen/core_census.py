#!/usr/bin/env python3
"""Which libak_core.so every process of a gate step loaded, from the dynamic loader's own record
(LD_DEBUG=libs, LD_DEBUG_OUTPUT=DIR/ld: one file per pid, appended).

  core_census.py DIR [--forbid CORE_DIR]... [--require CORE_DIR]... [--host PROG]...

Prints, per program (basename) and core path, the number of processes; the sha256 of every core
path seen (read now); then the checks:
  --forbid CORE_DIR   no process of a C++ program loaded a core from CORE_DIR (FAIL otherwise);
  --require CORE_DIR  at least one process loaded a core from CORE_DIR (FAIL otherwise);
  --host PROG         a program outside the C++ slice (the shared Rust server and its warm-up
                      client): listed apart, never failed by --forbid (it is stock by design).
Exit 1 when a check fails. Nothing is timed.
"""
import collections
import hashlib
import os
import re
import sys

LINE = re.compile(r"^\s*(\d+):\s+(.*)$")


def sha(p):
    try:
        with open(p, "rb") as f:
            return hashlib.sha256(f.read()).hexdigest()
    except OSError as e:
        return "unreadable (%s)" % e.strerror


def main(argv):
    d = argv[0]
    forbid, require, host = [], [], []
    i = 1
    while i < len(argv):
        k, v = argv[i], os.path.realpath(argv[i + 1]) if argv[i] != "--host" else argv[i + 1]
        {"--forbid": forbid, "--require": require, "--host": host}[k].append(v)
        i += 2
    seen = collections.Counter()  # (prog, core) -> processes
    procs = 0
    files = sorted(f for f in os.listdir(d) if f.startswith("ld."))
    for fn in files:
        pend = {}  # pid -> core paths initialised before "initialize program"
        for raw in open(os.path.join(d, fn), errors="replace"):
            m = LINE.match(raw.rstrip("\n"))
            if not m:
                continue
            pid, rest = m.group(1), m.group(2)
            if rest.startswith("calling init: ") and rest.endswith("/libak_core.so"):
                pend.setdefault(pid, []).append(os.path.realpath(rest[len("calling init: "):]))
            elif rest.startswith("initialize program: "):
                procs += 1
                prog = os.path.basename(rest[len("initialize program: "):].strip())
                for c in pend.pop(pid, []):
                    seen[(prog, c)] += 1
    print("# core census of %s: %d loader files, %d programs started, %d (program, core) pairs"
          % (d, len(files), procs, len(seen)))
    cores = sorted({c for _, c in seen})
    width = max([len(p) for p, _ in seen] + [8])
    for (prog, c), n in sorted(seen.items()):
        print("  %-*s %5d x %s%s" % (width, prog, n, c, "   [host]" if prog in host else ""))
    print("# sha256 of every core loaded")
    for c in cores:
        print("  %s  %s" % (sha(c), c))
    fails = 0
    for fd in forbid:
        bad = [(p, c, n) for (p, c), n in seen.items() if os.path.dirname(c) == fd and p not in host]
        for p, c, n in bad:
            print(">>> FAIL: %s loaded %s (%d processes): a forbidden core directory" % (p, c, n))
        fails += len(bad)
    for rd in require:
        n = sum(v for (p, c), v in seen.items() if os.path.dirname(c) == rd)
        if n == 0:
            print(">>> FAIL: no process loaded a core from %s" % rd)
            fails += 1
        else:
            print(">>> ok: %d processes loaded a core from %s" % (n, rd))
    if forbid:
        print(">>> %s: no C++ process loaded a core from %d forbidden directories"
              % ("ok" if fails == 0 else "FAIL", len(forbid)))
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
