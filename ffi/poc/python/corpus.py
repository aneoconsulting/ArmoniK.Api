"""W8: the python slice consumes the conformance corpus, as far as its scope reaches.

`ffi/corpus/CONTRACT.md` says the corpus's first consumer is its second opinion -- until
one exists, a systematic misreading shared by its generator and by upb would survive.
This is that consumer for the rows it can reach.

**Scope, stated before any result.**  A row is in scope when its root is a message this
slice's codec roots, which is now every root in `walk.ROOTS` rather than
`ListResultsResponse` alone.  Every other row is reported as out of scope BY ROOT, never
silently dropped.

**CONTRACT.md rule 0, and it is checked rather than asserted.**  A reader generated from
`corpus_superset.proto` knows every field, executes no unknown-field skip, and passes
class `unknown` while testing nothing.  This slice's codec is generated from
`ffi/schema/shapes.json`, and `check_rule_zero()` below diffs every message it covers
against BOTH corpus schemas.  Two conditions, and only the first is per message:

*   **every** scoped message is identical to `corpus.proto`'s, and
*   **at least one** of them is missing fields that `corpus_superset.proto` adds, so the
    unknown-field skip is actually executed somewhere.

The second used to be per message as well, which passed while the scope was M1 -- all
three of its messages happen to gain `u_*` fields -- and turned into eleven spurious
failures the moment the scope widened to messages the superset does not extend. A check
that only holds for the scope it was written against is not a check.

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
    ok, notes, extended = True, [], 0
    for n in W.SCOPE:
        if n not in reader:
            notes.append("%s: not in corpus.proto at all, so no row can reach it" % n)
            continue
        if mine.get(n) != reader.get(n):
            ok = False
            notes.append("%s: this slice's schema differs from corpus.proto" % n)
            continue
        extra = [x for x in sup.get(n, []) if x not in mine.get(n, [])]
        if extra:
            extended += 1
            notes.append("%s: identical to corpus.proto; the superset adds %d fields "
                         "this reader does not know (%s)"
                         % (n, len(extra),
                            ", ".join(x.split()[-3] for x in extra[:3]) + ", ..."))
        else:
            notes.append("%s: identical to corpus.proto; the superset extends it "
                         "nowhere, so the unknown-field skip is tested elsewhere" % n)
    if not extended:
        ok = False
        notes.append("NO scoped message is extended by the superset: every `unknown` row "
                     "would pass while executing no unknown-field skip at all")
    else:
        notes.append("%d of %d scoped messages are extended by the superset, which is "
                     "what makes the `unknown` class mean something here"
                     % (extended, len(W.SCOPE)))
    return ok, notes


# --------------------------------------------------------------------------------------
# CONTRACT.md section 3: the projection encoding.  Driven from the description through
# the same walker every backend uses, so a shape it has no case for raises (R1).
# --------------------------------------------------------------------------------------

def project(obj, name):
    out = {}
    for f, k, c in W.walk(SCHEMA, name):
        if c == "oneof":
            continue                  # emitted below, once per oneof, under its case
        v = getattr(obj, f["name"])
        if c == "optional":
            # CONTRACT.md section 3: an explicit-presence field is projected when PRESENT,
            # zero or not, and omitted when absent. That is the one shape where the
            # projection and the omit-when-zero rule disagree on purpose.
            if v is None:
                continue
            out[f["name"]] = (v if k == "string" else v.hex() if k == "bytes"
                              else bool(v) if k == "bool" else str(int(v)))
            continue
        if c == "packed":
            # CONTRACT.md section 3's type table, and a packed run follows it per VALUE:
            # a bool is JSON true/false, a double is a string at `%.17g`, everything else
            # is a decimal string. Note that `false` appears in the run -- the
            # omit-when-zero rule is about a leaf, and a run's members are not leaves.
            if v:
                out[f["name"]] = [bool(x) if k == "bool"
                                  else "%.17g" % x if k == "double"
                                  else str(int(x)) for x in v]
            continue
        if c == "map":
            if v:
                out[f["name"]] = dict(v)
            continue
        if c == "repeated":
            if v:
                out[f["name"]] = ([e for e in v] if k == "string"
                                  else [project(e, f["of"]) for e in v])
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
    for oname, members in W.oneof_groups(SCHEMA, name).items():
        tag = getattr(obj, "%s_case" % oname)
        g = next((x for x in members if x["tag"] == tag), None)
        if g is None:
            continue
        v = getattr(obj, g["name"])
        out[g["name"]] = (v if g["kind"] == "string"
                          else v.hex() if g["kind"] == "bytes"
                          else project(v, g["of"]) if g["kind"] == "message"
                          else str(int(v)))
    return out


DECODE_ONLY_ROOTS = set(W.DECODE_ONLY)


def _triples(b):
    """The (tag, wire type, body) multiset of a message's top-level fields.

    Length-delimited fields only, which is all P7.1 has at the root. A vector that needed
    more would need a real reader here, and this one does not.
    """
    out, i = [], 0
    while i < len(b):
        k = b[i]
        i += 1
        tag, wt = k >> 3, k & 7
        if wt != 2 or i >= len(b):
            return None
        n = b[i]
        i += 1
        out.append((tag, wt, bytes(b[i:i + n])))
        i += n
    return sorted(out)


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

# Each arm is (name, decode(bytes, root) -> facade, encode(facade, root) -> bytes). The
# ROOT is an argument because the corpus has rows at six of them; the previous table had
# `decode(backend, buf, types)` baked in from work unit 2's three-argument entry point and
# stopped running the moment the entry point grew a root. Nothing caught that, because
# nothing ran `corpus.py` in `run.sh`. It runs there now.
#
# Rows where this slice disagrees with the corpus ON PURPOSE are listed in DISAGREEMENTS
# below, each with the evidence that says which way the disagreement goes. They are
# reported, never suppressed: they print under a heading of their own and are counted
# apart from the failures, because a slice that folds a disagreement into its pass count
# has removed the only thing the corpus's first consumer is for.
# EMPTY, like UPSTREAM above, and for the same reason: a row left here after its cause is
# gone is a regression nobody would see.
#
# It held `U-map-entry` C2. This slice's reader parsed the map entries and skipped the
# unknown field inside each, the corpus's projection recorded the whole entries as
# `_unknown`, and upb dropped them. The corpus now runs three oracles and WITHDREW that
# projection rather than deciding it -- the vector has `projection: None` and is excluded
# from the C2 denominator, which is why C2 reads 123/123 and not 123/124.
#
# The finding itself has not gone anywhere and is not this table's to carry: **protobuf
# 7.36.2 on upb drops a whole map entry that carries any unknown field**, isolated on a
# two-field message, while the same version's pure-Python backend keeps it. STATE.md's
# defect table is where that lives.
DISAGREEMENTS = {}

# Rows failing on ANOTHER component's defect: {(vector id, obligation): defect id}, with
# UPSTREAM_WHY[defect id] saying what it is. EMPTY. Commit 7e0404a deleted this table when
# it emptied it and left the three places that read it, so the first C1 failure after that
# crashed the script with a NameError instead of being reported -- invisible for as long as
# every row passed, and found by the first corpus that made a row fail (D12).
UPSTREAM = {}
UPSTREAM_WHY = {}

ARMS = [
    ("core-ffi / C ext type",
     lambda b, r: arms._ffi.decode("cext", r, b, arms.TY_CEXT),
     lambda o, r: arms._ffi.encode("cext", r, o)),
    ("core-ffi / plain",
     lambda b, r: arms._ffi.decode("attr", r, b, arms.TY_PLAIN),
     lambda o, r: arms._ffi.encode("attr", r, o)),
    ("pycodec / plain",
     lambda b, r: getattr(arms.pycodec, "decode_root_" + r)(b, arms.CT_PLAIN),
     lambda o, r: bytes(getattr(arms.pycodec, "encode_root_" + r)(o))),
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
    roots = set(W.ROOTS)
    inscope = {k: r for k, r in rows.items() if r.get("root") in roots}
    print("\n## scope")
    print("   %d of %d rows root at one of this slice's %d roots and are in scope."
          % (len(inscope), len(rows), len(roots)))
    print("   Roots covered: %s" % ", ".join(sorted(roots)))
    byroot = {}
    for k, r in rows.items():
        byroot.setdefault(r.get("root"), 0)
        byroot[r.get("root")] += 1
    out_of_scope = {k: v for k, v in byroot.items() if k not in roots}
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
    disagreed = {}
    upstream = {}
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
                    dec(buf, r["root"])
                    bad.append("%s: C4 accepted a reject vector" % vid)
                except Exception as e:  # noqa: BLE001
                    nc4 += 1
                    rejects_seen.append((arm, vid, type(e).__name__, str(e)[:60]))
                continue
            try:
                obj = dec(buf, r["root"])
                nc1 += 1
            except Exception as e:  # noqa: BLE001
                if (vid, "C1") in UPSTREAM:
                    upstream.setdefault((vid, "C1"), []).append(arm)
                else:
                    bad.append("%s: C1 %s: %s" % (vid, type(e).__name__, str(e)[:70]))
                continue
            if r.get("projection"):
                want = strip_unknown(json.load(
                    open(os.path.normpath(os.path.join(CORPUS, r["projection"])))))
                got = project(obj, r["root"])
                if got == want:
                    nc2 += 1
                elif (vid, "C2") in DISAGREEMENTS:
                    disagreed.setdefault((vid, "C2"), []).append(arm)
                else:
                    bad.append("%s: C2 projection differs: %s"
                               % (vid, _pdiff(got, want)))
            try:
                back = enc(obj, r["root"])
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
            if (r["root"] in DECODE_ONLY_ROOTS
                    and h not in {a["sha256"] for a in enc_list}):
                # design/SHAPES.md, P7.1: the vector interleaves two repeated fields and
                # no writer that emits a repeated field contiguously can reproduce it. The
                # obligation stated in advance is a permutation of the same (tag, wire
                # type, body) triples, so that is what is checked, not the hash.
                form = "a permutation, contiguous (SHAPES.md P7.1)"
                if _triples(back) is not None and _triples(back) == _triples(buf):
                    nc3 += 1
                    forms_written.setdefault(arm, {}).setdefault(form, 0)
                    forms_written[arm][form] += 1
                else:
                    bad.append("%s: C3 is not even a permutation of the vector" % vid)
                continue
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
        # Every failure, not the first eight: the log is the evidence, and a truncated list
        # cannot be classified by whoever reads it (WP4 item 2's vectors made 42 at once).
        for b in bad:
            print("        FAIL %s" % b)
            fails += 1

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

    print("\n## failures owned by another component, named and not fixed here")
    if not upstream:
        print("   none")
    for (vid, ob), armlist in sorted(upstream.items()):
        d = UPSTREAM[(vid, ob)]
        print("   %-18s %s  %s, on %d of %d arms"
              % (vid, ob, d, len(armlist), len(ARMS)))
    for d in sorted({UPSTREAM[k] for k in upstream}):
        print("       %s: %s" % (d, UPSTREAM_WHY[d]))

    print("\n## disagreements reported upstream, not folded into the pass count")
    if not disagreed:
        print("   none")
    for (vid, ob), armlist in sorted(disagreed.items()):
        print("   %s %s, on %d of %d arms" % (vid, ob, len(armlist), len(ARMS)))
        print("       %s" % DISAGREEMENTS[(vid, ob)])

    extra = []
    if upstream:
        extra.append("%d row(s) failing on another component's defect" % len(upstream))
    if disagreed:
        extra.append("%d reported disagreement(s)" % len(disagreed))
    print("\n%s%s" % ("CORPUS SUBSET PASSES" if not fails else "%d FAILURE(S)" % fails,
                      ("  (with %s, above)" % " and ".join(extra)) if extra else ""))
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
