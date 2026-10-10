"""FIX-PLAN D26 (owner, 2026-10-10, as amended): no thread-local storage on any timed path of
the cpp slice. Fails on `thread_local`, `__thread`, `pthread_key_create` / `pthread_getspecific`
or `tss_create` in the slice's C and C++ sources -- the generated bindings (binding*.{h,cpp},
both facades, both unknown-field builds, the corpus's), the facades and native codecs, the
runtime headers, the harness and RPC code under src/ and corpus/src/ -- except the counting
build's tallies, which the owner's scope allows:

  t_host_calls, t_pull_records   behind `#ifdef AK_COUNTING` (a timed binary has no counting code)

Comments and string literals are ignored. The preprocessor nesting is tracked, so an allowed
tally outside an `#ifdef AK_COUNTING` branch is a failure too. `--plant` adds a thread-local to
a copy of src/generated/binding.cpp's text and requires the scan to find it (the control).

  python3 gen/no_tls.py [--plant]
"""
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
DIRS = ["src", "include", "nounk", "corpus/src", "corpus/include", "corpus/nounk"]
EXT = (".h", ".hpp", ".cpp", ".cc", ".c", ".inc")
ALLOWED = {"t_host_calls": "AK_COUNTING", "t_pull_records": "AK_COUNTING"}
PAT = re.compile(r"\bthread_local\b|\b__thread\b|\bpthread_key_create\b|\bpthread_getspecific\b|\btss_create\b")


def strip(text):
    """Comments and string/char literals blanked, line structure kept."""
    out, i, n = [], 0, len(text)
    while i < n:
        c = text[i]
        if text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j < 0 else j
            i = j
        elif text.startswith("/*", i):
            j = text.find("*/", i + 2)
            j = n if j < 0 else j + 2
            out.append("\n" * text.count("\n", i, j))
            i = j
        elif c in "\"'":
            j = i + 1
            while j < n and text[j] != c:
                j += 2 if text[j] == "\\" else 1
            out.append(c + " " * 0 + c)
            out.append("\n" * text.count("\n", i, j))
            i = j + 1
        else:
            out.append(c)
            i += 1
    return "".join(out)


def scan_text(text, name):
    bad = []
    stack = []  # one entry per open #if: the macro it tests positively, or None
    for ln, line in enumerate(strip(text).split("\n"), 1):
        t = line.strip()
        m = re.match(r"#\s*(ifdef|ifndef|if|elif|else|endif)\b\s*(.*)", t)
        if m:
            d, arg = m.group(1), m.group(2).strip()
            if d == "ifdef":
                stack.append(arg.split()[0] if arg else None)
            elif d in ("ifndef", "if"):
                mm = re.match(r"defined\s*\(?\s*(\w+)\s*\)?\s*$", arg) if d == "if" else None
                stack.append(mm.group(1) if mm else None)
            elif d in ("else", "elif"):
                if stack:
                    stack[-1] = None
            elif d == "endif":
                if stack:
                    stack.pop()
            continue
        for mt in PAT.finditer(line):
            nm = re.search(r"\b(t_\w+|\w+)\s*(=|;|\[|\{)", line[mt.end():])
            var = nm.group(1) if nm else "<?>"
            need = ALLOWED.get(var)
            if need is not None and need in stack:
                continue
            why = ("allowed only under #ifdef %s" % need) if need else "thread-local"
            bad.append("%s:%d: %s %s (%s)" % (name, ln, mt.group(0), var, why))
    return bad


def files():
    for d in DIRS:
        for base, dirs, fs in os.walk(os.path.join(ROOT, d)):
            dirs[:] = [x for x in dirs if not x.startswith("build")]
            for f in fs:
                if f.endswith(EXT):
                    yield os.path.join(base, f)


def main(argv):
    allf = sorted(files())
    bad, seen = [], {}
    for p in allf:
        rel = os.path.relpath(p, ROOT)
        src = open(p, encoding="utf-8", errors="replace").read()
        bad += scan_text(src, rel)
        for v in ALLOWED:
            if re.search(r"\bthread_local\b[^;\n]*\b%s\b" % v, strip(src)):
                seen.setdefault(v, []).append(rel)
    print("no_tls: %d C/C++ files scanned (%s)" % (len(allf), ", ".join(DIRS)))
    for v, why in ALLOWED.items():
        print("  allowed %-15s (under #ifdef %s): %d file(s)" % (v, why, len(seen.get(v, []))))
    if "--plant" in argv:
        victim = os.path.join(ROOT, "src", "generated", "binding.cpp")
        txt = open(victim, encoding="utf-8").read()
        # outside any #if, and an allowed name outside its #ifdef: both must be found
        planted = txt + "\nstatic thread_local int t_planted = 0;\nstatic thread_local uint64_t t_host_calls = 0;\n"
        got = scan_text(planted, "src/generated/binding.cpp (PLANTED)")
        print("  control: two thread-locals planted in a copy of src/generated/binding.cpp (one new,"
              " one an allowed name outside #ifdef AK_COUNTING) -> %d finding(s)" % len(got))
        for g in got:
            print("    " + g)
        if len(got) != 2:
            print("NO_TLS CONTROL FAILED: expected 2 findings")
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
