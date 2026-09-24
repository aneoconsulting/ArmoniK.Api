#!/usr/bin/env python3
"""FIX-PLAN WP5 step 3: ffi/corpus against every Java arm that decodes, and every row.

CONTRACT.md is the obligation; this file owns every judgement, `ak.RunCorpus` (Java) only
decodes, projects and re-encodes. The arms render the CORPUS description (`ak.corpus`),
generated from the corpus's reader schema by the Java backend in poc/codec/gen, so every
row is in scope: arm R over all 30 messages, the ffi arms over the 29 the C ABI can carry
(`Nest` is refused by plan.check_expressible; its rows are `noabi` for those arms and are
reported, never dropped).

Per row and arm: C1 (parse), C2 (project, against the manifest's projection; `_unknown` is
optional per CONTRACT.md section 3 and stripped), C3 (re-encode to an accepted form, or a
permutation where `permutation_accepted`), C4 (refuse). C5 (produce) is not run. Disputed
rows are excluded and the reading each arm produced is reported. A retain arm that wrote
the unknown-DROPPED form on an `unknown` row passes C3 (both forms are accepted) and is
listed as a retention gap, as the rust slice's harness does.

  gen/corpus.py <java-bin-dir> <classes-dir> <arms> [<shim.so>]      (run from poc/java)
  env: AK_CORPUS_DIR (a manifest directory other than ffi/corpus/generated, e.g. the
       rust slice's probe manifest, poc/rust/gen/probe_corpus.py), AK_CORPUS_TIMEOUT_MS (5000), AK_CORPUS_PLANT (proj|reenc|accept), AK_CORPUS_ONLY
       (comma-separated row id prefixes), AK_SKIP_INIT=1
"""
import json
import os
import struct
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
SLICE = os.path.dirname(HERE)
FFI = os.path.dirname(os.path.dirname(SLICE))
CORPUS = os.environ.get("AK_CORPUS_DIR") or os.path.join(FFI, "corpus", "generated")
OUT = os.path.join(SLICE, "build", "corpus_out")


def triples(buf):
    out, i = [], 0

    def var():
        nonlocal i
        k, s = 0, 0
        while True:
            c = buf[i]
            i += 1
            k |= (c & 0x7f) << s
            if not c & 0x80:
                return k
            s += 7
    try:
        while i < len(buf):
            k = var()
            t, w = k >> 3, k & 7
            if w == 0:
                st = i
                var()
                out.append((t, w, bytes(buf[st:i])))
            elif w == 1:
                out.append((t, w, bytes(buf[i:i + 8]))); i += 8
            elif w == 2:
                n = var()
                out.append((t, w, bytes(buf[i:i + n]))); i += n
            elif w == 5:
                out.append((t, w, bytes(buf[i:i + 4]))); i += 4
            else:
                return None
    except IndexError:
        return None
    return sorted(out)


def fix_doubles(x):
    if isinstance(x, dict):
        return {k: fix_doubles(v) for k, v in x.items()}
    if isinstance(x, list):
        return [fix_doubles(v) for v in x]
    if isinstance(x, str) and x.startswith("D:"):
        bits = int(x[2:], 16)
        return "%.17g" % struct.unpack("<d", struct.pack("<Q", bits))[0]
    return x


def strip_unknown(d):
    if isinstance(d, dict):
        return {k: strip_unknown(v) for k, v in d.items() if k != "_unknown"}
    if isinstance(d, list):
        return [strip_unknown(v) for v in d]
    return d


def pdiff(got, want, path=""):
    if isinstance(got, dict) and isinstance(want, dict):
        gk, wk = set(got), set(want)
        if gk != wk:
            return "%s keys: extra %s, missing %s" % (path or "<root>", sorted(gk - wk)[:4],
                                                     sorted(wk - gk)[:4])
        for k in sorted(gk):
            if got[k] != want[k]:
                return pdiff(got[k], want[k], path + "." + k)
    if isinstance(got, list) and isinstance(want, list):
        if len(got) != len(want):
            return "%s: %d elements vs %d" % (path, len(got), len(want))
        for i, (a, b) in enumerate(zip(got, want)):
            if a != b:
                return pdiff(a, b, "%s[%d]" % (path, i))
    return "%s: got %.60r want %.60r" % (path, got, want)


def judge(arm, vid, r, got, timeout_ms):
    """-> (kind, detail): kind pass | fail | disputed | noabi; detail per kind."""
    if got is None:
        return "fail", ("harness", "no result line")
    status, err, sha, nbytes, proj = got[0], got[1], got[2], int(got[3]), got[4]
    if status == "noabi":
        return "noabi", None
    if r.get("verdict") == "disputed":
        reading = status
        if status == "accept":
            try:
                mine = strip_unknown(fix_doubles(json.loads(proj)))
                for rd in (r.get("dispute") or {}).get("readings", []):
                    pf = rd.get("projection")
                    if pf and strip_unknown(json.load(open(os.path.join(CORPUS, pf)))) == mine:
                        reading = "accept, matches the reading of %s" % ", ".join(rd.get("read_by", []))
            except (OSError, ValueError):
                pass
        else:
            reading = "%s (%s)" % (status, err[:60])
        return "disputed", reading
    if r["expect"] == "reject":
        if status == "reject":
            return "pass", ("C4", err)
        if status == "timeout":
            return "fail", ("C4", "decoder did not return within %s ms" % timeout_ms)
        return "fail", ("C4", "ACCEPTED a reject vector (%s)" % status)
    if status != "accept":
        return "fail", ("C1", "%s %s" % (status, err))
    want = r.get("projection")
    if want:
        try:
            wantj = strip_unknown(json.load(open(os.path.join(CORPUS, want))))
            gotj = strip_unknown(fix_doubles(json.loads(proj)))
        except ValueError as e:
            return "fail", ("C2", "projection unreadable: %s" % e)
        if gotj != wantj:
            return "fail", ("C2", pdiff(gotj, wantj))
    acc = r.get("accepted_encodings", [])
    hit = [a for a in acc if a["sha256"] == sha]
    if hit:
        return "pass", ("C3", "/".join(hit[0].get("forms", ["?"])))
    if r.get("permutation_accepted"):
        mine = triples(open(os.path.join(OUT, "%s.%s.bin" % (arm, vid)), "rb").read())
        if mine is not None and mine == triples(open(os.path.join(CORPUS, r["file"]), "rb").read()):
            return "pass", ("C3", "a permutation of the committed triples")
    return "fail", ("C3", "re-encode %d B sha %s matches no accepted form (%s)"
                    % (nbytes, sha[:12], ", ".join("%d B" % a["bytes"] for a in acc)))


def main():
    jbin, classes, arms = sys.argv[1], sys.argv[2], sys.argv[3].split(",")
    lib = sys.argv[4] if len(sys.argv) > 4 else None
    cp = classes + ":" + open(os.path.join(SLICE, "deps", "cp.txt")).read().strip()
    timeout_ms = os.environ.get("AK_CORPUS_TIMEOUT_MS", "5000")
    only = [x for x in os.environ.get("AK_CORPUS_ONLY", "").split(",") if x]
    man = json.load(open(os.path.join(CORPUS, "manifest.json")))
    rows = {k: v for k, v in man["vectors"].items()
            if not only or any(k.startswith(o) for o in only)}
    env = dict(os.environ)
    env.pop("JAVA_TOOL_OPTIONS", None)
    print("# java slice: ffi/corpus against %s  (FIX-PLAN WP5 step 3)" % ", ".join(arms))
    print("# corpus %s, %d of %d vectors%s" % (man.get("corpus_version", "?"), len(rows),
                                             len(man["vectors"]),
                                             ("  (only " + ",".join(only) + ")") if only else ""))
    print("# classes %s   shim %s" % (classes, lib or "-"))
    tfile = os.path.join(SLICE, "build", "corpus_tasks.tsv")
    os.makedirs(OUT, exist_ok=True)
    with open(tfile, "w") as f:
        for vid, r in sorted(rows.items()):
            f.write("%s\t%s\t%s\n" % (vid, r["root"], os.path.normpath(os.path.join(CORPUS, r["file"]))))
    cmd = [os.path.join(jbin, "java"), "-Xss8m", "-cp", cp,
           "-Dak.corpus.timeoutms=" + timeout_ms, "-Dak.corpus.out=" + OUT,
           "-Dak.corpus.arms=" + ",".join(arms)]
    if lib:
        cmd.append("-Dak.lib=" + lib)
    if os.environ.get("AK_CORPUS_PLANT"):
        cmd.append("-Dak.corpus.plant=" + os.environ["AK_CORPUS_PLANT"])
    if os.environ.get("AK_SKIP_INIT"):
        cmd.append("-Dak.skipInit=1")
    cmd += ["ak.RunCorpus", tfile]
    p = subprocess.run(cmd, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    res = {}
    for line in p.stdout.decode("utf-8", "replace").splitlines():
        if line.startswith("#"):
            print(line)
            continue
        f = line.split("\t")
        if f[0] == "R" and len(f) >= 8:
            res[(f[1], f[2])] = f[3:]
        elif line.strip():
            print("# java: " + line[:200])
    print("# harness exit %d, %d result lines" % (p.returncode, len(res)))

    total_fail = 0
    for arm in arms:
        c = dict(passed=0, fail=0, disputed=0, noabi=0, c4=0, accept=0)
        fails, disputed, gaps, forms, refusals, timeouts = [], [], [], {}, [], []
        for vid, r in sorted(rows.items()):
            got = res.get((arm, vid))
            if got is not None and got[0] == "timeout":
                timeouts.append(vid)
            kind, d = judge(arm, vid, r, got, timeout_ms)
            if kind == "pass":
                c["passed"] += 1
                if d[0] == "C4":
                    c["c4"] += 1
                    refusals.append((vid, d[1]))
                else:
                    c["accept"] += 1
                    forms[d[1]] = forms.get(d[1], 0) + 1
                    if arm.endswith("retain") and r.get("class") == "unknown" and "dropped" in d[1]:
                        gaps.append(vid)
            elif kind == "fail":
                c["fail"] += 1
                fails.append((vid, d[0], d[1]))
            elif kind == "disputed":
                c["disputed"] += 1
                disputed.append((vid, r["expect"], d))
            else:
                c["noabi"] += 1
        total_fail += c["fail"]
        print("\n################ arm %s" % arm)
        print("   rows %d: pass %d (accept %d, refused %d), FAIL %d, disputed (excluded) %d,"
              " not in the C ABI %d, timeouts %d"
              % (len(rows), c["passed"], c["accept"], c["c4"], c["fail"], c["disputed"],
                 c["noabi"], len(timeouts)))
        print("   C3 forms written: %s" % ", ".join("%s=%d" % kv for kv in sorted(forms.items())))
        if gaps:
            print("   retain mode wrote the DROPPED form on %d unknown row(s) (accepted by the"
                  " contract; a retention gap): %s" % (len(gaps), ", ".join(gaps)))
        for d in disputed:
            print("   disputed %-34s expect=%-6s read: %s" % d)
        print("   FAILING rows: %d" % len(fails))
        for vid, k, why in fails:
            print("      %-4s %-40s %s" % (k, vid, why[:200]))
        if arm in ("R", "ffi") and not os.environ.get("AK_CORPUS_PLANT"):
            print("   refusals (error per refused row):")
            for vid, err in refusals:
                print("      %-40s %s" % (vid, err[:100]))
    print("\nRESULT: %d failing arm-rows over %d arms x %d rows" % (total_fail, len(arms), len(rows)))
    return 1 if total_fail else 0


if __name__ == "__main__":
    sys.exit(main())
