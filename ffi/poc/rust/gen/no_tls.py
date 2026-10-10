"""FIX-PLAN D26 (owner, 2026-10-10, as amended): no thread-local storage on any timed path of
the rust slice. Fails on a `thread_local!` block (or `#[thread_local]`, or a `LocalKey`) in
the Rust sources of this slice and of the shared core -- generated bindings, harness, campaign
(benches and bins), corpus harness, facade, core crates -- except the three allowed by the
owner's scope:

  HOST_RESETS, HOST_OTHER   the counting build's tallies, behind #[cfg(feature = "count")]
  FSM_BUMP                  the FSM differential's test arena (fsm_diff_<root> only)
  CS                        shapes-values' content-set selection, read by payload construction
                            only (before any timed region)

Comments are ignored. `--plant` adds a thread-local to a copy of one scanned file and requires
the scan to fail on it (the check's control).

  python3 gen/no_tls.py [--plant]
"""
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
RUST = os.path.dirname(HERE)
CODEC = os.path.join(os.path.dirname(RUST), "codec")
ROOTS = [os.path.join(RUST, "crates"), os.path.join(RUST, "corpus", "crates"), os.path.join(CODEC, "crates")]
ALLOWED = {"HOST_RESETS": "count", "HOST_OTHER": "count", "FSM_BUMP": "test", "CS": "setup"}


def strip_comments(text):
    """Line comments and block comments out; strings are left (no thread-local is in one)."""
    text = re.sub(r"/\*.*?\*/", lambda m: "\n" * m.group(0).count("\n"), text, flags=re.S)
    return "\n".join(re.sub(r"//.*$", "", ln) for ln in text.split("\n"))


def scan_text(text, name):
    bad = []
    t = strip_comments(text)
    lines = t.split("\n")
    for m in re.finditer(r"thread_local!\s*\{", t):
        # the block to its matching brace
        i, depth = m.end(), 1
        while i < len(t) and depth:
            depth += {"{": 1, "}": -1}.get(t[i], 0)
            i += 1
        block = t[m.start():i]
        names = re.findall(r"static\s+([A-Za-z_][A-Za-z0-9_]*)\s*:", block)
        ln = t.count("\n", 0, m.start())
        prev = next((lines[k].strip() for k in range(ln - 1, -1, -1) if lines[k].strip()), "")
        for n in names or ["<unnamed>"]:
            why = ALLOWED.get(n)
            if why == "count" and prev != '#[cfg(feature = "count")]':
                bad.append("%s:%d: %s is a counting tally but not behind #[cfg(feature = \"count\")]" % (name, ln + 1, n))
            elif why is None:
                bad.append("%s:%d: thread-local %s" % (name, ln + 1, n))
    for m in re.finditer(r"#\[thread_local\]|\bLocalKey\b", t):
        bad.append("%s:%d: %s" % (name, t.count("\n", 0, m.start()) + 1, m.group(0)))
    return bad


def files():
    for r in ROOTS:
        for d, dirs, fs in os.walk(r):
            dirs[:] = [x for x in dirs if not x.startswith("target")]
            for f in fs:
                if f.endswith(".rs"):
                    yield os.path.join(d, f)


def main(argv):
    allf = sorted(files())
    bad, seen = [], {}
    for p in allf:
        src = open(p, encoding="utf-8").read()
        rel = os.path.relpath(p, os.path.dirname(RUST))
        bad += scan_text(src, rel)
        for n in ALLOWED:
            if re.search(r"static\s+%s\s*:" % n, strip_comments(src)) and "thread_local!" in strip_comments(src):
                seen.setdefault(n, []).append(rel)
    print("no_tls: %d Rust files scanned (poc/rust crates, corpus, poc/codec crates)" % len(allf))
    for n, why in ALLOWED.items():
        print("  allowed %-12s (%s): %s" % (n, why, ", ".join(sorted(set(seen.get(n, [])))) or "none"))
    if "--plant" in argv:
        victim = next(p for p in allf if p.endswith(os.path.join("harness", "src", "generated", "binding.rs")))
        planted = open(victim, encoding="utf-8").read() + "\nthread_local! {\n    static PLANTED: ::core::cell::Cell<u32> = const { ::core::cell::Cell::new(0) };\n}\n"
        got = scan_text(planted, os.path.relpath(victim, os.path.dirname(RUST)) + " (PLANTED)")
        print("  control: a thread-local planted in a copy of %s -> %d finding(s): %s"
              % (os.path.relpath(victim, os.path.dirname(RUST)), len(got), got[-1] if got else "NONE"))
        if not got:
            print("NO_TLS CONTROL FAILED: the planted thread-local was not found")
            return 1
    for b in bad:
        print("  FAIL " + b)
    if bad:
        print("NO_TLS FAILED: %d thread-local(s) outside the allowed set" % len(bad))
        return 1
    print("NO_TLS PASSED")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
