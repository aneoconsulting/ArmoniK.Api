"""README R2: correctness before timing, and byte identity across every arm.

Nothing in `bench_codec.py` is timed until this passes.  Four checks, and the
first is the one that makes the rest mean something:

1.  **Against the validated manifest.**  `ffi/schema/generated/manifest.json`
    carries a sha256 per payload and it is validated against prost 0.14.4 and a
    second, independent encoder.  A disagreement is a defect in this slice, not
    a difference (W2).
2.  **Against `emit/payloads.py` itself**, byte for byte rather than by hash, so
    a failure says WHERE the bytes diverge instead of only that they do.
3.  **Across every arm**, including the incumbent.
4.  **The absent path.**  P1.3 is every string empty and every child absent, and
    README R6 is emphatic: a payload generator that fills every field cannot
    reach any path conditioned on emptiness, and a defect that lived exactly
    there passed every other payload in the Java slice.

It also checks the generated tree is current with `shapes.json` (R1), because a
measured artifact that does not match its description is a figure about nothing.
"""

import hashlib
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
FFI = os.path.dirname(os.path.dirname(os.path.dirname(HERE)))

sys.path.insert(0, HERE)
import arms  # noqa: E402

MANIFEST = json.load(open(os.path.join(FFI, "schema", "generated",
                                       "manifest.json")))


def first_diff(a, b):
    n = min(len(a), len(b))
    for i in range(n):
        if a[i] != b[i]:
            lo = max(0, i - 8)
            return ("byte %d: %02x vs %02x  (context %s | %s)"
                    % (i, a[i], b[i], a[lo:i].hex(), b[lo:i + 8].hex()))
    return "identical for %d bytes, then lengths %d vs %d" % (n, len(a), len(b))


def main():
    fails = 0
    print("# python slice, work unit 1: conformance (README R2)")
    print("# interpreter: %s" % sys.version.split()[0])
    print("# manifest:    %s" % MANIFEST["status"].split(".")[0])
    print()

    print("## the generated tree is current with shapes.json (R1)")
    r = subprocess.run([sys.executable, os.path.join(HERE, "gen", "generate.py"),
                        "--check"], capture_output=True, text=True)
    print("   " + (r.stdout or r.stderr).strip())
    if r.returncode:
        fails += 1

    for a in arms.absent:
        print("## ARM ABSENT: %s" % a)
        fails += 1

    print("\n## byte identity, every arm, against the validated manifest")
    for pid in arms.PAYLOADS:
        want = arms.gvalues.reference(pid)
        want_sha = hashlib.sha256(want).hexdigest()
        man = MANIFEST["payloads"][pid]
        tag = "P%s" % pid
        if want_sha != man["sha256"] or len(want) != man["bytes"]:
            print("   FAIL %s: emit/payloads.py disagrees with the manifest "
                  "(%s vs %s). The schema directory is broken, not this slice."
                  % (tag, want_sha[:16], man["sha256"][:16]))
            fails += 1
            continue
        fac = arms.build_facades(pid)
        upb = arms.build_upb(pid)
        print("   %-6s %7d bytes  sha %s  (%d elements%s)"
              % (pid, man["bytes"], man["sha256"][:16], man["elements"],
                 ", " + man["mode"] if "mode" in man else ""))
        for name, fn in arms.arms(fac, upb):
            got = fn()
            if got == want:
                print("        ok    %s" % name)
            else:
                print("        FAIL  %-34s %s" % (name, first_diff(got, want)))
                fails += 1

    print("\n## crossing counts, from the counting build (README R5)")
    if arms._codec_count is None or not arms._codec_count.counting():
        print("   ABSENT: no counting build. Counts would be inferred, which R5")
        print("   forbids.")
        fails += 1
    else:
        print("   %-6s %-30s %s" % ("payload", "backend", "crossings into CPython"))
        for pid in arms.PAYLOADS:
            fac = arms.build_facades(pid)
            n_elem = MANIFEST["payloads"][pid]["elements"]
            for backend, call in (
                    ("getattr / plain",
                     lambda f: arms._codec_count.encode_attr(f["plain"])),
                    ("member / C ext type",
                     lambda f: arms._codec_count.encode_cext(f["cext"])),
                    ("pyacc / plain",
                     lambda f: arms._codec_count.encode_pyacc(f["plain"],
                                                              arms.GETTERS_PY))):
                arms._codec_count.reset_counts()
                got = call(fac)
                c = arms._codec_count.counts()
                total = sum(c.values())
                if got != arms.gvalues.reference(pid):
                    print("        FAIL: the counting build does not agree on bytes")
                    fails += 1
                print("   %-6s %-30s %7d total, %6.2f per element  %s"
                      % (pid, backend, total, total / max(n_elem, 1),
                         ", ".join("%s=%d" % (k, v) for k, v in sorted(c.items())
                                   if v)))

    print("\n%s" % ("ALL CHECKS PASS" if not fails else "%d FAILURE(S)" % fails))
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
