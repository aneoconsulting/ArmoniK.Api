#!/usr/bin/env python3
"""WP12: which libak_core.so every process of a gate run loaded, from the dynamic linker's own
record (LD_DEBUG=libs, one file per process, named <step>.<pid> by gen/gate.sh's step()).

  wp12_loads.py LDDEBUG_DIR RUST_DIR VARIANT CORES_DIR > loads.txt

For each step: every executable that loaded a libak_core.so (at start-up, `needed by`, or by
dlopen), the path the loader opened, its real path and sha256, and what it is (the variant's
core of a feature set, a target directory's own core, a temporary copy). Then, per executable,
the static-link check: RUNPATH (searched after LD_LIBRARY_PATH) and no DT_RPATH (searched
before it), no `ak_*` C symbol defined in the executable itself (so every core entry point
resolves into the loaded library), and the h2 source compiled into the executable (the host's
own tonic) and into each core. Verdict: in a variant run every process that loaded a core
loaded exactly one, and it is a core of that variant (c_variant.sh's temporary copies are
named by their sha256, the serve.sh server is the host side and is listed apart).
"""
import hashlib, os, re, subprocess, sys
from collections import defaultdict

ldd_dir, rust, variant, cores = sys.argv[1:5]
rust = os.path.realpath(rust); cores = os.path.realpath(cores)
sha_cache = {}
def sha(p):
    if p not in sha_cache:
        try:
            with open(p, "rb") as f: sha_cache[p] = hashlib.sha256(f.read()).hexdigest()
        except OSError: sha_cache[p] = None
    return sha_cache[p]
def rel(p): return p[len(rust) + 1:] if p.startswith(rust + "/") else p

# the variant's cores, by sha256
core_of_sha = {}
for v in sorted(os.listdir(cores)) if os.path.isdir(cores) else []:
    for k in sorted(os.listdir(os.path.join(cores, v))):
        so = os.path.join(cores, v, k, "libak_core.so")
        if os.path.isfile(so): core_of_sha.setdefault(sha(so), []).append("%s/%s" % (v, k))

def step_key(s):
    m = re.match(r"s(\d+)([a-z]*)$", s)
    return (0, int(m.group(1)), m.group(2)) if m else (1, 0, s)

rows = defaultdict(int)   # (step, exe, how, opened) -> processes
bad = []; exes = set(); procs = 0; listed = 0
# LD_DEBUG=libs, per process (one file per pid; an exec keeps the pid and the file): libraries are
# searched and initialised ("calling init: PATH") BEFORE "initialize program: EXE" of the image
# that needs them; one initialised after "transferring control: EXE" was dlopen'ed by EXE (an
# exec in the same pid starts a new image with "calling init: .../ld-linux-*.so").
for name in sorted(os.listdir(ldd_dir)):
    step, _, pid = name.rpartition(".")
    try: lines = open(os.path.join(ldd_dir, name), errors="replace").read().splitlines()
    except OSError: continue
    if not any("libak_core" in l for l in lines): continue
    pending = []; last = "?"; after = False; got = []
    for l in lines:
        # a new image (an exec in the same pid) starts by initialising the dynamic linker itself
        if re.search(r"calling init: \S*/ld-linux[^/]*\.so", l): after = False; continue
        m = re.search(r"calling init: (\S*libak_core\.so)\s*$", l)
        if m:
            if after: got.append((last, "dlopen", m.group(1)))
            else: pending.append(m.group(1))
            continue
        m = re.search(r"initialize program: (\S+)", l)
        if m:
            last = m.group(1); after = False
            got += [(last, "needed", o) for o in pending]; pending = []
            continue
        if "transferring control:" in l: after = True
    got += [(last, "needed", o) for o in pending]
    if not got:
        listed += 1   # ldd (LD_TRACE_LOADED_OBJECTS): resolved, never run; ldd's own output is in the log
        continue
    procs += 1
    images = defaultdict(list)
    for exe, how, o in got: images[exe].append(o)
    for exe, os_ in images.items():
        if len(os_) != 1:
            bad.append("%s pid %s (%s): %d cores initialised: %s" % (step, pid, exe, len(os_), os_))
    for exe, how, o in got:
        exes.add(exe if os.path.isabs(exe) else os.path.normpath(os.path.join(rust, exe)))
        rows[(step, exe, how, o)] += 1

def what(opened):
    real = os.path.realpath(opened); h = sha(real)
    if real.startswith(cores + "/"):
        return "variant core " + rel(os.path.dirname(real)).split("target-wp12-cores/", 1)[-1], h
    if h and h in core_of_sha:
        return "copy of variant core " + "/".join(core_of_sha[h]), h
    if "/target" in real and "/release/deps/" in real:
        return "target's own core (" + rel(real).split("/release/")[0] + ")", h
    return "other" if h else "removed after the run (temporary)", h

print("# WP12 core loads: variant %s, %d processes loaded a libak_core.so, %d more listed by ldd only" % (variant, procs, listed))
print("# columns: step | processes | executable (how) | path opened by the loader -> what it is (sha256)")
cur = None
for (step, exe, how, opened), n in sorted(rows.items(), key=lambda kv: (step_key(kv[0][0]), kv[0][1], kv[0][3])):
    if step != cur:
        print("\n## step %s" % step[1:] if step.startswith("s") else "\n## %s" % step); cur = step
    w, h = what(opened)
    print("%4d  %s (%s)\n      %s -> %s (sha256 %s)" % (n, rel(exe), how, rel(opened), w, (h or "?")[:16]))
    server = exe.endswith("/target-server/release/rpc_server")
    if variant in ("stock", "h2-batch") and not server:
        ok = w.startswith("variant core %s/" % variant) or w.startswith("copy of variant core %s/" % variant)
        # c_variant.sh's header/core pairs: a copy in its mktemp directory, removed on exit; the
        # source path and sha256 of each copy are printed in the gate log (step 12, "cores:")
        ok = ok or (w.startswith("removed") and re.match(r"h-(full|nounk)-(full|nounk)$", os.path.basename(exe)))
        if not ok:
            bad.append("%s: %s loaded %s (%s), not a %s core" % (step, rel(exe), rel(opened), w, variant))

print("\n# static-link check, every executable above")
print("# columns: executable | DT_RUNPATH | DT_RPATH | ak_* C symbols defined in the executable | h2 compiled into the executable")
def h2_of(path):
    try: s = subprocess.run(["strings", path], capture_output=True, text=True).stdout
    except OSError: return "?"
    hs = sorted(set(re.findall(r"([^/\s]*)/src/codec/framed_write\.rs", s)))
    return ",".join(hs) if hs else "none"
for exe in sorted(exes):
    if not os.path.isfile(exe):
        print("%s | (gone)" % rel(exe)); continue
    d = subprocess.run(["readelf", "-d", exe], capture_output=True, text=True).stdout
    runpath = "yes" if "(RUNPATH)" in d else "no"; rpath = "yes" if "(RPATH)" in d else "no"
    nm = subprocess.run(["nm", "--defined-only", exe], capture_output=True, text=True).stdout
    defs = [l.split()[-1] for l in nm.splitlines() if re.search(r" [TtWwDdBbRr] ak_", l)]
    print("%s | %s | %s | %d%s | %s" % (rel(exe), runpath, rpath, len(defs), (" " + ",".join(defs[:5])) if defs else "", h2_of(exe)))
    if rpath == "yes": bad.append("%s has a DT_RPATH (searched before LD_LIBRARY_PATH)" % rel(exe))
    if defs: bad.append("%s defines ak_* symbols itself: %s" % (rel(exe), defs[:5]))

print("\n# the cores of this run (h2 compiled in)")
seen = set()
for (step, exe, how, opened) in rows:
    real = os.path.realpath(opened)
    if real in seen or not os.path.isfile(real): continue
    seen.add(real)
    print("%s  sha256 %s  h2: %s" % (rel(real), sha(real), h2_of(real)))

print()
if bad:
    print("LOADS CHECK FAILED"); [print("  " + b) for b in bad]; sys.exit(1)
print("LOADS CHECK PASSED: every process that loaded a core loaded exactly one%s; no executable carries a DT_RPATH or defines an ak_* symbol"
      % ("" if variant == "plain" else ", a %s core of its own feature set (rpc_server, the host-side server, apart)" % variant))
