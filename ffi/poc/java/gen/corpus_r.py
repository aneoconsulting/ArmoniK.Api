#!/usr/bin/env python3
"""R-E4: run ffi/corpus against arm R, the generated pure-Java codec, and report every row.

CONTRACT.md is the obligation. This is a CONFIRMATION run: FIX-PLAN WP5 ports arm R onto
the shared generator, so nothing found here is fixed in `gen/java_codec.py`; the rows that
fail are listed by id with the reason, which is what WP5's Java step has to turn green.

What runs: C1 (parse), C2 (project, against the manifest's projection), C3 (re-encode to
an accepted form, or a permutation where `permutation_accepted`), C4 (refuse, with the
error recorded per row). C5 (produce) is not run: arm R's facade is built from
`ffi/schema`'s payload builders, not from the corpus's produce set, and that is stated in
the output rather than counted.

Scope: every row whose root is a message arm R's codec has (`ak.RunCorpusR --roots`, read
from the compiled codec, not restated) AND whose declaration in `corpus.proto` equals this
slice's `shapes.proto` (CONTRACT.md rule 0, checked per root). Every other row is listed by
root as out of scope.

Disputed rows are excluded from the count and the reading arm R produced is reported.
`_unknown` is stripped from projections (optional per CONTRACT.md section 3; arm R drops
unknown fields, which is its answer to open decision 11).

  gen/corpus_r.py <java-bin-dir> <classes-dir>        (run from poc/java)
"""
import json
import os
import re
import struct
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
SLICE = os.path.dirname(HERE)
FFI = os.path.dirname(os.path.dirname(SLICE))
CORPUS = os.path.join(FFI, "corpus", "generated")
SCHEMA = os.path.join(FFI, "schema", "generated")
OUT = os.path.join(SLICE, "build", "corpus_r_out")


def msgs(path):
    t = open(path).read()
    out = {}
    for m in re.finditer(r"^message (\w+) \{(.*?)^\}", t, re.S | re.M):
        out[m.group(1)] = [l.strip() for l in m.group(2).splitlines()
                           if l.strip() and not l.strip().startswith("//")]
    return out


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
    """The Java side prints a double as its raw bits; render it as CONTRACT.md says."""
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
            return "%s keys: extra %s, missing %s" % (path or "<root>",
                                                     sorted(gk - wk)[:4],
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


def main():
    jbin, classes = sys.argv[1], sys.argv[2]
    cp = classes + ":" + open(os.path.join(SLICE, "deps", "cp.txt")).read().strip()
    java = os.path.join(jbin, "java")
    timeout_ms = os.environ.get("AK_CORPUS_TIMEOUT_MS", "5000")
    man = json.load(open(os.path.join(CORPUS, "manifest.json")))
    rows = man["vectors"]
    env = dict(os.environ)
    env.pop("JAVA_TOOL_OPTIONS", None)

    roots = subprocess.run([java, "-cp", cp, "ak.RunCorpusR", "--roots"], env=env,
                           stdout=subprocess.PIPE, check=True).stdout.decode().split()
    print("# java slice, arm R (generated pure-Java codec) against ffi/corpus -- R-E4")
    print("# corpus:  %s, %d vectors" % (man.get("corpus_version", "?"), len(rows)))
    print("# classes: %s" % classes)
    print("# arm R roots (from the compiled codec): %s" % ", ".join(sorted(roots)))
    print()

    print("## CONTRACT.md rule 0: arm R's schema against corpus.proto, per root")
    mine = msgs(os.path.join(SCHEMA, "shapes.proto"))
    reader = msgs(os.path.join(CORPUS, "corpus.proto"))
    sup = msgs(os.path.join(CORPUS, "corpus_superset.proto"))
    scope = []
    for n in sorted(roots):
        if n not in reader:
            print("   [out] %s: not in corpus.proto" % n)
        elif mine.get(n) != reader.get(n):
            print("   FAIL  %s: shapes.proto differs from corpus.proto -- out of scope" % n)
        else:
            extra = [x for x in sup.get(n, []) if x not in mine.get(n, [])]
            print("   [ok]  %s: identical to corpus.proto; superset adds %d field(s)"
                  % (n, len(extra)))
            scope.append(n)

    inscope = {k: r for k, r in rows.items() if r.get("root") in scope}
    print("\n## scope: %d of %d rows" % (len(inscope), len(rows)))
    byroot = {}
    for r in rows.values():
        if r.get("root") not in scope:
            byroot[r.get("root")] = byroot.get(r.get("root"), 0) + 1
    print("   out of scope, by root (messages arm R does not have):")
    for k, v in sorted(byroot.items(), key=lambda x: -x[1]):
        print("      %-28s %d" % (k, v))

    tfile = os.path.join(SLICE, "build", "corpus_r_tasks.tsv")
    os.makedirs(os.path.dirname(tfile), exist_ok=True)
    with open(tfile, "w") as f:
        for vid, r in sorted(inscope.items()):
            f.write("%s\t%s\t%s\n" % (vid, r["root"],
                                      os.path.normpath(os.path.join(CORPUS, r["file"]))))
    os.makedirs(OUT, exist_ok=True)
    p = subprocess.run([java, "-Xss4m", "-cp", cp, "-Dak.corpus.timeoutms=" + timeout_ms,
                        "-Dak.corpus.out=" + OUT,
                        "ak.RunCorpusR", tfile], env=env, stdout=subprocess.PIPE,
                       stderr=subprocess.STDOUT)
    res = {}
    for line in p.stdout.decode("utf-8", "replace").splitlines():
        if line.startswith("#"):
            print(line)
            continue
        f = line.split("\t")
        if f[0] == "R" and len(f) >= 7:
            res[f[1]] = f[2:]
    print("# harness exit %d, %d result lines" % (p.returncode, len(res)))

    fails, disputed, rejects, forms, timeouts = [], [], [], {}, []
    c = dict(c1=0, c2=0, c3=0, c4=0, perm=0, noproj=0)
    for vid, r in sorted(inscope.items()):
        got = res.get(vid)
        if got is None:
            fails.append((vid, "harness", "no result line"))
            continue
        status, err, sha, nbytes, proj = got[0], got[1], got[2], int(got[3]), got[4]
        if status == "timeout":
            timeouts.append(vid)
        if r.get("verdict") == "disputed":
            reading = "?"
            mineproj = None
            if status == "accept":
                try:
                    mineproj = strip_unknown(fix_doubles(json.loads(proj)))
                except ValueError:
                    pass
            for rd in (r.get("dispute") or {}).get("readings", []):
                pf = rd.get("projection")
                if pf and mineproj is not None:
                    try:
                        want = strip_unknown(json.load(open(os.path.join(CORPUS, pf))))
                    except (OSError, ValueError):
                        continue
                    if want == mineproj:
                        reading = "matches reading of %s" % ", ".join(rd.get("read_by", []))
            if reading == "?" and status != "accept":
                d = r.get("dispute") or {}
                reading = "refused, as %s" % ", ".join(d.get("refused_by", [])) \
                    if d.get("refused_by") else "refused"
            elif reading == "?" and (r.get("dispute") or {}).get("accepted_by"):
                reading = "accepted, as %s" % ", ".join(r["dispute"]["accepted_by"])
            disputed.append((vid, r["expect"], status, err, reading))
            continue
        if r["expect"] == "reject":
            if status in ("reject",):
                c["c4"] += 1
                rejects.append((vid, err))
            elif status == "timeout":
                fails.append((vid, "C4", "decoder did not return within %s ms" % timeout_ms))
            else:
                fails.append((vid, "C4", "ACCEPTED a reject vector (%s)" % status))
            continue
        if status != "accept":
            fails.append((vid, "C1", "%s %s" % (status, err)))
            continue
        c["c1"] += 1
        want = r.get("projection")
        if want:
            try:
                wantj = strip_unknown(json.load(open(os.path.join(CORPUS, want))))
                gotj = strip_unknown(fix_doubles(json.loads(proj)))
            except ValueError as e:
                fails.append((vid, "C2", "projection unreadable: %s" % e))
                gotj = wantj = None
            if gotj is not None:
                if gotj == wantj:
                    c["c2"] += 1
                else:
                    fails.append((vid, "C2", pdiff(gotj, wantj)))
        else:
            c["noproj"] += 1
        acc = r.get("accepted_encodings", [])
        hit = [a for a in acc if a["sha256"] == sha]
        if hit:
            c["c3"] += 1
            fm = "/".join(hit[0].get("forms", ["?"]))
            forms[fm] = forms.get(fm, 0) + 1
        elif r.get("permutation_accepted") and \
                triples(open(os.path.join(OUT, vid + ".bin"), "rb").read()) == \
                triples(open(os.path.join(CORPUS, r["file"]), "rb").read()):
            # a re-ordering of the committed bytes, which are an accepted form
            c["c3"] += 1
            c["perm"] += 1
        else:
            fails.append((vid, "C3", "re-encode %d B sha %s matches no accepted form (%s)"
                          % (nbytes, sha[:12], ", ".join("%d B" % a["bytes"] for a in acc))))

    n_acc = sum(1 for r in inscope.values() if r["expect"] == "accept"
                and r.get("verdict") != "disputed")
    n_rej = sum(1 for r in inscope.values() if r["expect"] == "reject"
                and r.get("verdict") != "disputed")
    print("\n## result")
    print("   accept rows %d: C1 parsed %d, C2 projected equal %d (of those with a "
          "projection; %d have none), C3 accepted form %d (%d as a permutation of the "
          "committed triples)" % (n_acc, c["c1"], c["c2"], c["noproj"], c["c3"],
                                         c["perm"]))
    print("   reject rows %d: C4 refused %d" % (n_rej, c["c4"]))
    print("   disputed rows %d (excluded)" % len(disputed))
    print("   timeouts: %d%s" % (len(timeouts), (" (" + ", ".join(timeouts) + ")")
                                 if timeouts else ""))
    print("   C3 forms written: %s" % ", ".join("%s=%d" % kv for kv in sorted(forms.items())))
    print("   C5 (produce): not run; arm R's facade is built from ffi/schema's payload "
          "builders, not the corpus's produce set")

    print("\n## disputed rows, and the reading arm R produced")
    for d in disputed:
        print("   %-34s expect=%-6s armR=%-7s %s %s" % d)

    print("\n## FAILING rows: %d" % len(fails))
    byk = {}
    for vid, k, why in fails:
        byk.setdefault(k, []).append((vid, why))
    for k in sorted(byk):
        print("   -- %s: %d" % (k, len(byk[k])))
        for vid, why in byk[k]:
            print("      %-40s %s" % (vid, why))

    print("\n## C4: the error arm R raised, per refused row")
    for vid, err in rejects:
        print("   %-40s %s" % (vid, err[:100]))

    print("\nRESULT: %d failing of %d counted rows (%d disputed excluded)"
          % (len(fails), n_acc + n_rej, len(disputed)))
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
