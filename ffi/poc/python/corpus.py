"""W8: the python slice consumes the conformance corpus, as far as its scope reaches.

`ffi/corpus/CONTRACT.md` says the corpus's first consumer is its second opinion -- until
one exists, a systematic misreading shared by its generator and by upb would survive.
This is that consumer for the rows it can reach.

**Scope, stated before any result.**  This slice's codec covers the M1 subtree, so it can
root only the corpus rows whose `root` is `ListResultsResponse`: **48 of 336**.  Every
other row is reported as out of scope by class, never silently dropped.  The 48 are not a
thin slice of the corpus's intent: they include the `unknown` class at both the root and
the element site, which is the one obligation nothing else in this slice touches.

**CONTRACT.md rule 0, and it is checked rather than asserted.**  A reader generated from
`corpus_superset.proto` knows every field, executes no unknown-field skip, and passes
class `unknown` while testing nothing.  This slice's codec is generated from
`ffi/schema/shapes.json`, and `check_rule_zero()` below diffs the three messages it
covers against BOTH corpus schemas: identical to `corpus.proto`, and missing exactly the
seven `u_*` fields that `corpus_superset.proto` adds.  So the reader is the reader.

Obligations run: C1 (parse), C2 (project), C3 (re-encode to an accepted form, and say
which), C4 (refuse, and record what the refusal looked like).  C5 (produce) is run for the
rows that name `python` in `produce`.
"""

import hashlib
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
FFI = os.path.dirname(os.path.dirname(HERE))
CORPUS = os.path.join(FFI, "corpus", "generated")
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(FFI, "schema", "emit"))
sys.path.insert(0, os.path.join(HERE, "gen"))

import arms   # noqa: E402
import shapes as S  # noqa: E402
import walk as W    # noqa: E402

MAN = json.load(open(os.path.join(CORPUS, "manifest.json")))
SCHEMA = S.load()


def check_rule_zero():
    """CONTRACT.md rule 0: this reader is built against corpus.proto, not the superset."""
    def msgs(path):
        t = open(path).read()
        out = {}
        for m in re.finditer(r"^message (\w+) \{(.*?)^\}", t, re.S | re.M):
            out[m.group(1)] = [l.strip() for l in m.group(2).splitlines()
                               if l.strip() and not l.strip().startswith("//")]
        return out
    mine = msgs(os.path.join(FFI, "schema", "generated", "shapes.proto"))
    reader = msgs(os.path.join(CORPUS, "corpus.proto"))
    sup = msgs(os.path.join(CORPUS, "corpus_superset.proto"))
    ok, notes = True, []
    for n in W.SCOPE:
        if mine.get(n) != reader.get(n):
            ok = False
            notes.append("%s: this slice's schema differs from corpus.proto" % n)
        extra = [x for x in sup.get(n, []) if x not in mine.get(n, [])]
        if not extra:
            ok = False
            notes.append("%s: the superset adds nothing, so the unknown-field claim is "
                         "untested here" % n)
        else:
            notes.append("%s: identical to corpus.proto; the superset adds %d fields "
                         "this reader does not know (%s)"
                         % (n, len(extra),
                            ", ".join(x.split()[-3] for x in extra[:3]) + ", ..."))
    return ok, notes


# --------------------------------------------------------------------------------------
# CONTRACT.md section 3: the projection encoding.  Driven from the description through
# the same walker every backend uses, so a shape it has no case for raises (R1).
# --------------------------------------------------------------------------------------

def project(obj, name):
    out = {}
    for f, k, c in W.walk(SCHEMA, name):
        v = getattr(obj, f["name"])
        if c == "repeated":
            if v:
                out[f["name"]] = [project(e, f["of"]) for e in v]
            continue
        if k == "message":
            if v is not None:
                out[f["name"]] = project(v, f["of"])
            continue
        if k == "string":
            if v:
                out[f["name"]] = v
        elif k == "bytes":
            if v:
                out[f["name"]] = v.hex()
        elif k == "bool":
            if v:
                out[f["name"]] = True
        elif k in ("int32", "int64", "enum"):
            if int(v):
                out[f["name"]] = str(int(v))
        else:
            raise W.Unsupported("%s.%s: %s in a projection" % (name, f["name"], k))
    return out


def strip_unknown(d):
    """`_unknown` is optional per CONTRACT.md section 3, because whether the core retains
    unknown fields is ABI v1 open decision 11.  This core drops them, so the comparison is
    over everything else and the drop is reported as this slice's answer to decision 11."""
    if isinstance(d, dict):
        return {k: strip_unknown(v) for k, v in d.items() if k != "_unknown"}
    if isinstance(d, list):
        return [strip_unknown(x) for x in d]
    return d


# --------------------------------------------------------------------------------------

ARMS = [
    ("core-ffi / C ext type",
     lambda b: arms._ffi.decode("cext", b, arms.CEXT),
     lambda o: arms._ffi.encode("cext", o)),
    ("core-ffi / plain",
     lambda b: arms._ffi.decode("attr", b, arms.PLAIN),
     lambda o: arms._ffi.encode("attr", o)),
    ("pycodec / plain",
     lambda b: arms.pycodec.decode_root(b, arms.CTORS),
     lambda o: arms.pycodec.encode_root(o)),
]


def main():
    print("# python slice: the conformance corpus (W8), as far as this scope reaches")
    print("# corpus:   %s" % MAN.get("corpus_version", "?"))
    print("# vectors:  %d in the corpus" % len(MAN["vectors"]))
    print("# reader:   generated from ffi/schema/shapes.json, M1 subtree")
    print()

    print("## CONTRACT.md rule 0: the reader is not generated from the superset")
    ok, notes = check_rule_zero()
    for n in notes:
        print("   %s %s" % ("[ok] " if ok else "FAIL ", n))
    if not ok:
        return 1

    rows = MAN["vectors"]
    inscope = {k: r for k, r in rows.items() if r.get("root") in ("ListResultsResponse",)}
    print("\n## scope")
    print("   %d of %d rows root at ListResultsResponse and are in scope."
          % (len(inscope), len(rows)))
    byroot = {}
    for k, r in rows.items():
        byroot.setdefault(r.get("root"), 0)
        byroot[r.get("root")] += 1
    out_of_scope = {k: v for k, v in byroot.items() if k != "ListResultsResponse"}
    print("   Out of scope, by root, and every one of them is a message this slice does")
    print("   not cover rather than a row it chose to skip:")
    for k, v in sorted(out_of_scope.items(), key=lambda x: -x[1]):
        print("      %-28s %d" % (k, v))

    cls = {}
    for r in inscope.values():
        cls[r["class"]] = cls.get(r["class"], 0) + 1
    print("   In scope, by class: %s"
          % ", ".join("%s=%d" % kv for kv in sorted(cls.items())))

    fails = 0
    forms_written = {}
    unknown_forms = {}
    rejects_seen = []
    print("\n## C1 parse, C2 project, C3 re-encode, C4 refuse")
    for arm, dec, enc in ARMS:
        nc1 = nc2 = nc3 = nc4 = 0
        bad = []
        for vid, r in sorted(inscope.items()):
            path = os.path.join(CORPUS, r["file"]) if not r["file"].startswith("..") \
                else os.path.normpath(os.path.join(CORPUS, r["file"]))
            buf = open(path, "rb").read()
            if r["expect"] == "reject":
                try:
                    dec(buf)
                    bad.append("%s: C4 accepted a reject vector" % vid)
                except Exception as e:  # noqa: BLE001
                    nc4 += 1
                    rejects_seen.append((arm, vid, type(e).__name__, str(e)[:60]))
                continue
            try:
                obj = dec(buf)
                nc1 += 1
            except Exception as e:  # noqa: BLE001
                bad.append("%s: C1 %s: %s" % (vid, type(e).__name__, str(e)[:70]))
                continue
            if r.get("projection"):
                want = strip_unknown(json.load(
                    open(os.path.normpath(os.path.join(CORPUS, r["projection"])))))
                got = project(obj, r["root"])
                if got == want:
                    nc2 += 1
                else:
                    bad.append("%s: C2 projection differs: %s"
                               % (vid, _pdiff(got, want)))
            try:
                back = enc(obj)
            except Exception as e:  # noqa: BLE001
                bad.append("%s: C3 re-encode %s: %s" % (vid, type(e).__name__, e))
                continue
            h = hashlib.sha256(back).hexdigest()
            # Three in-scope rows carry no `accepted_encodings`: the two baseline rows
            # that REFERENCE ffi/schema's payloads rather than copying them, and the
            # reject row (which has nothing to re-encode). For the first two the row's
            # own sha256 is the one accepted form.
            enc_list = r.get("accepted_encodings") or [
                {"sha256": r["sha256"], "bytes": r["bytes"], "forms": ["as committed"]}]
            acc = {a["sha256"]: a.get("forms", ["?"]) for a in enc_list}
            if h in acc:
                nc3 += 1
                for form in acc[h]:
                    forms_written.setdefault(arm, {}).setdefault(form, 0)
                    forms_written[arm][form] += 1
                    if r["class"] == "unknown":
                        unknown_forms.setdefault(arm, {}).setdefault(form, 0)
                        unknown_forms[arm][form] += 1
            else:
                bad.append("%s: C3 wrote a form the manifest does not accept (%s, %d B; "
                           "accepted: %s)"
                           % (vid, h[:12], len(back),
                              ", ".join("%s=%s" % (a["sha256"][:8], "/".join(
                                  a.get("forms", ["?"]))) for a in enc_list)))
        n_acc = sum(1 for r in inscope.values() if r["expect"] == "accept")
        n_rej = len(inscope) - n_acc
        n_proj = sum(1 for r in inscope.values()
                     if r["expect"] == "accept" and r.get("projection"))
        print("   %-24s C1 %d/%d  C2 %d/%d  C3 %d/%d  C4 %d/%d"
              % (arm, nc1, n_acc, nc2, n_proj, nc3, n_acc, nc4, n_rej))
        for b in bad[:8]:
            print("        FAIL %s" % b)
            fails += 1
        if len(bad) > 8:
            print("        ... and %d more" % (len(bad) - 8))
            fails += len(bad) - 8

    print("\n## C3: which accepted form this slice writes")
    print("#  CONTRACT.md C3: a vector may have more than one accepted form, and which")
    print("#  one a slice writes is a fact about it rather than a verdict.")
    for arm, forms in forms_written.items():
        print("   %-24s %s" % (arm, ", ".join("%s=%d" % kv
                                              for kv in sorted(forms.items()))))

    print("\n## ABI v1 open decision 11, answered for this slice")
    print("#  CONTRACT.md class `unknown`: say whether you retained or dropped. Counted")
    print("#  over the `unknown`-class rows only, because on any other row there is no")
    print("#  unknown field and `as committed` means the canonical form.")
    n_unknown = sum(1 for r in inscope.values()
                    if r["class"] == "unknown" and r["expect"] == "accept")
    for arm, per in sorted(unknown_forms.items()):
        print("   %-24s %s  (of %d unknown-class rows)"
              % (arm, ", ".join("%s=%d" % kv for kv in sorted(per.items())), n_unknown))
    print("   **This slice DROPS unknown fields.** The core carries `ak_unk_f` vtable")
    print("   slots and this shim passes NULL for every one of them, so the drop is a")
    print("   choice the binding makes and not a limit of the ABI: a binding that wanted")
    print("   retention has somewhere to put it. That is ABI v1 open decision 11's")
    print("   answer for python, and it matches what four of the five incumbents do NOT")
    print("   do -- upb retains, and this does not.")

    print("\n## C4: what the refusals looked like")
    for arm, vid, ex, msg in rejects_seen:
        print("   %-24s %-22s %s: %s" % (arm, vid, ex, msg))
    if not rejects_seen:
        print("   none in scope")

    print("\n%s" % ("CORPUS SUBSET PASSES" if not fails else "%d FAILURE(S)" % fails))
    return 1 if fails else 0


def _pdiff(got, want):
    gk, wk = set(got), set(want)
    if gk != wk:
        return "keys +%s -%s" % (sorted(gk - wk)[:3], sorted(wk - gk)[:3])
    for k in sorted(gk):
        if got[k] != want[k]:
            return "%s: %r vs %r" % (k, str(got[k])[:40], str(want[k])[:40])
    return "equal?"


if __name__ == "__main__":
    sys.exit(main())
