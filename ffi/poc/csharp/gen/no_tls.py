#!/usr/bin/env python3
"""gen/no_tls.py DIR...: D26 (owner, 2026-10-10): the C# slice holds NO thread-local storage.
Fails (exit 1) on any [ThreadStatic], ThreadLocal<T>, AsyncLocal<T> or ConcurrentBag<T> (whose
per-thread lists are thread-local storage) in the C# sources and generated code under DIR
(every *.cs, bin/ and obj/ directories skipped), comments excluded (// and /* */; string
literals are kept, so a token inside one also fails). A gate step (gen/gate.sh, step 1b),
with a planted file as its control."""
import os
import re
import sys

TOKENS = re.compile(r"\b(ThreadStatic(?:Attribute)?|ThreadLocal|AsyncLocal|ConcurrentBag)\b")
SKIP = {"bin", "obj", "bin-nounk", "obj-nounk", "bin-count", "obj-count", "bin-count-nounk", "obj-count-nounk"}


def strip_comments(src):
    """C# source without its comments; string and char literals kept (verbatim and
    interpolated strings treated as plain ones: enough for a token scan)."""
    out, i, n = [], 0, len(src)
    while i < n:
        c = src[i]
        if c == "/" and i + 1 < n and src[i + 1] == "/":
            j = src.find("\n", i)
            i = n if j < 0 else j
        elif c == "/" and i + 1 < n and src[i + 1] == "*":
            j = src.find("*/", i + 2)
            j = n if j < 0 else j + 2
            out.append("\n" * src.count("\n", i, j))   # keep the line numbers
            i = j
        elif c in "\"'":
            j = i + 1
            while j < n and src[j] != c and src[j] != "\n":
                j += 2 if src[j] == "\\" else 1
            out.append(src[i:j + 1])
            i = j + 1
        else:
            out.append(c)
            i += 1
    return "".join(out)


def main(dirs):
    hits, files = [], 0
    for d in dirs:
        for root, sub, names in os.walk(d):
            sub[:] = [s for s in sub if s not in SKIP]
            for nm in names:
                if not nm.endswith(".cs"):
                    continue
                files += 1
                p = os.path.join(root, nm)
                code = strip_comments(open(p, encoding="utf-8").read())
                for ln, line in enumerate(code.split("\n"), 1):
                    for m in TOKENS.finditer(line):
                        hits.append("%s:%d: %s" % (p, ln, m.group(1)))
    for h in hits:
        print("THREAD-LOCAL " + h)
    print("no_tls: %d C# files scanned under %s; %d thread-local site(s)" % (files, " ".join(dirs), len(hits)))
    return 1 if hits else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:] or ["src"]))
