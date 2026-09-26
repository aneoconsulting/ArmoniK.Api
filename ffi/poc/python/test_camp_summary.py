"""R-H1's test: two builds whose incumbents differ must each be divided by their own.

  python3.12 test_camp_summary.py        (exit 0 on pass; also run by gate.sh)

Two synthetic launches of each build are written into a temporary directory, the no-unknown
file sorting AFTER the full one (the order in which the defect let it overwrite the full
build's reference). Full: incumbent 100 ns, core-ffi 200 ns -> ratio 2.0. No-unknown:
incumbent 400 ns, core-ffi 200 ns -> ratio 0.5. The RPC cell A is checked the same way. The
twin: the same rows with `build` removed must NOT give 2.0 for the full build's core-ffi
(pooling, as the old summary did), which shows the test can see the defect.
"""
import json
import os
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import camp_summary as S  # noqa: E402


def codec_rows(build, inc_ns, ffi_ns, launches=2, rounds=3):
    out = []
    for launch in range(1, launches + 1):
        for rnd in range(1, rounds + 1):
            for arm, ns, mode in (("incumbent-prod", inc_ns, "incumbent-default"),
                                  ("core-ffi", ffi_ns, "drop" if build == "full" else "no-unknown")):
                out.append({"suite": "codec", "build": build, "arm": arm, "payload": "P1.1", "content": "ascii",
                            "dir": "encode", "unknown_mode": mode, "launch": launch, "round": rnd,
                            "phase": "value", "cpu_ns": ns * 10, "wall_ns": ns * 10, "iters": 10})
    return out


def rpc_rows(build, a_ns, c_ns):
    out = []
    for launch in (1, 2):
        for cell, ns in (("A", a_ns), ("C-drop" if build == "full" else "C-nounk", c_ns)):
            out.append({"suite": "rpc", "build": build, "cell": cell, "payload": "P2.2", "dir": "a",
                        "transport": "shipped", "inflight": 1, "launch": launch, "round": 1,
                        "cpu_ns": ns * 4, "wall_ns": ns * 4, "iters": 4})
    return out


def ratios(res, suite, build, who):
    return [rat for k, _c, _w, rat in res[suite] if k[0] == build and k[1] == who]


def main():
    bad = []
    with tempfile.TemporaryDirectory() as d:
        with open(os.path.join(d, "codec-shapes-launch1.jsonl"), "w") as f:
            for r in codec_rows("full", 100, 200) + rpc_rows("full", 1000, 3000):
                f.write(json.dumps(r) + "\n")
        with open(os.path.join(d, "codec-shapes-nounk-launch1.jsonl"), "w") as f:
            for r in codec_rows("nounk", 400, 200) + rpc_rows("nounk", 6000, 3000):
                f.write(json.dumps(r) + "\n")
        rows, _ = S.load(d)
        res = S.summarise(rows)
        want = [("codec", "full", "core-ffi", [2.0, 2.0]), ("codec", "nounk", "core-ffi", [0.5, 0.5]),
                ("rpc", "full", "C-drop", [3.0, 3.0]), ("rpc", "nounk", "C-nounk", [0.5, 0.5])]
        for suite, build, who, w in want:
            got = ratios(res, suite, build, who)
            ok = got == [w]
            print("   %-5s %-5s %-8s ratios per launch %s (want %s): %s" % (suite, build, who, got, [w], "ok" if ok else "FAIL"))
            if not ok:
                bad.append((suite, build, who))
        # groups are per build: the two incumbents are two rows, never one pooled row
        inc = [k for k, *_ in res["codec"] if k[1] == "incumbent-prod"]
        print("   incumbent-prod groups: %d (want 2, one per build)" % len(inc))
        if len(inc) != 2:
            bad.append("pooled incumbent")
        # the twin: without `build`, the old pooling reappears and the check must see it
        for r in rows:
            r.pop("build", None)
        twin = ratios(S.summarise(rows), "codec", "full", "core-ffi")
        blind = twin == [[2.0, 2.0]]
        print("   must-fail twin (build removed, rows pooled): full core-ffi ratios %s -> %s"
              % (twin, "PASSED: the test is blind" if blind else "differs from 2.0, as required"))
        if blind:
            bad.append("blind twin")
    print("CAMP_SUMMARY TEST %s" % ("PASS" if not bad else "FAIL %r" % bad))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
