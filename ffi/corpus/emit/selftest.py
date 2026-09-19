"""The guards, each one watched working.

A guard with no failing test is a guard nobody has seen work. That is R1's
second half and this branch paid for it twice: the first build of ABI v1 section
8's generator-time refusal walked singular message children only, so it found
nothing, refused nothing, and read as working; and a field walker that excluded
oneof members emitted a complete-looking codec and reported nothing wrong.

Every check here does the same thing to this directory's own guards: it hands
each one input it is supposed to refuse, and fails if the guard does not refuse
it. Run by `run.sh`, not by hand.
"""
import copy
import json
import os
import shutil
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import build as B          # noqa: E402
import encode as E         # noqa: E402
import spec                # noqa: E402
from encode import Opts    # noqa: E402
from spec import ShapeNotCovered  # noqa: E402

RESULTS = []


def must_raise(name, fn, exc=ShapeNotCovered):
    try:
        fn()
    except exc as e:
        RESULTS.append((True, name, "%s: %s" % (type(e).__name__, str(e)[:90])))
        return
    except Exception as e:                       # noqa: BLE001
        RESULTS.append((False, name, "raised %s, wanted %s" % (type(e).__name__, exc.__name__)))
        return
    RESULTS.append((False, name, "DID NOT RAISE"))


def ok(name, cond, detail=""):
    RESULTS.append((bool(cond), name, detail))


def main():
    reader, superset, corpus = spec.load()

    # 1. R1: a walker with no case for a shape RAISES, it never skips.
    for label, mutate in (
        ("an unknown field KIND", lambda m: m["fields"].append(
            {"name": "x", "tag": 900, "kind": "sfixed64"})),
        ("an unknown CARDINALITY", lambda m: m["fields"].append(
            {"name": "x", "tag": 901, "kind": "int32", "card": "optional-group"})),
        ("a map that is not map<string, string>", lambda m: m["fields"].append(
            {"name": "x", "tag": 902, "kind": "map", "key": "int32", "value_kind": "string"})),
        ("an unknown VALUE rule", lambda m: m["fields"].append(
            {"name": "x", "tag": 903, "kind": "string", "value": "haiku"})),
        ("a message type that does not exist", lambda m: m["fields"].append(
            {"name": "x", "tag": 904, "kind": "message", "of": "NoSuchMessage"})),
    ):
        s2 = copy.deepcopy(reader)
        mutate(s2["messages"]["ChunkLeaf"])
        must_raise("the walker raises on %s" % label,
                   lambda s2=s2: E.enc_message(s2, corpus, "ChunkLeaf", "ChunkLeaf", 0, Opts()))

    must_raise("the walker raises on a second oneof in one message",
               lambda: E.enc_message(two_oneofs(reader), corpus, "Probe", "Probe", 0, Opts()))
    must_raise("the walker raises on an encoder option it does not have",
               lambda: Opts(no_such_option=True))
    must_raise("load() raises on a duplicate tag",
               lambda: spec.check_tags(dup_tag(reader), "selftest"))
    must_raise("load() raises on a tag in protobuf's reserved range",
               lambda: spec.check_tags(reserved_tag(reader), "selftest"))

    # 2. A vector declared must-fail that nothing rejects FAILS THE BUILD. Proved
    #    by handing the reject path a vector that is perfectly valid.
    import vectors as VEC
    pools = compiled(reader, superset)
    good = [v for v in VEC.build_all(reader, superset, corpus)
            if v.expect == "accept" and v.data][0]
    cls = B.msg_class(pools, "reader", good.root)
    m = cls()
    try:
        m.ParseFromString(good.data)
        ok("a valid vector is NOT rejected by upb, so the reject tests are not vacuous",
           True, good.id)
    except Exception as e:                       # noqa: BLE001
        ok("a valid vector is NOT rejected by upb", False, str(e))

    rejected = 0
    for v in VEC.build_all(reader, superset, corpus):
        if v.expect != "reject":
            continue
        try:
            B.msg_class(pools, "reader", v.root)().ParseFromString(v.data)
        except Exception:                        # noqa: BLE001
            rejected += 1
    total = sum(1 for v in VEC.build_all(reader, superset, corpus) if v.expect == "reject")
    ok("every must-fail vector is seen failing (%d of %d)" % (rejected, total), rejected == total)

    # 3. The coverage check reports a gap rather than passing over it.
    universe = B.shape_universe(reader)
    ok("the shape universe is computed from the description (%d shapes)" % len(universe),
       len(universe) > 15 and "message/singular/oneof" in universe)
    ok("the gap reporter is silent when every shape is covered",
       B.shape_gaps(universe, set(universe)) == [])
    one_missing = set(universe) - {"message/singular/oneof"}
    ok("the gap reporter NAMES a shape that has no vector",
       B.shape_gaps(universe, one_missing) == ["message/singular/oneof"])

    # The two descriptor views really do differ, and in the direction claimed:
    # per SITE, not by grepping the text. Tag 15 is an unknown oneof member on
    # Probe and TaskDetailed.pod_ttl, which is why a textual check passes the
    # wrong thing.
    sites = sorted(corpus["unknown_fields"]["sites"])
    utags = sorted(spec.unknown_tags(corpus))

    def tags_at(schema, name):
        return set(f["tag"] for f in schema["messages"][name]["fields"])

    leak = [(s_, sorted(tags_at(reader, s_) & spec.unknown_tags(corpus, s_))) for s_ in sites
            if tags_at(reader, s_) & spec.unknown_tags(corpus, s_)]
    ok("no unknown tag is declared at its site in the READER view", not leak, str(leak))
    gap = [(s_, sorted(set(f["tag"] for f in corpus["unknown_fields"]["fields"])
                       - tags_at(superset, s_))) for s_ in sites
           if set(f["tag"] for f in corpus["unknown_fields"]["fields"]) - tags_at(superset, s_)]
    ok("every unknown tag IS declared at its site in the SUPERSET view", not gap, str(gap))
    om = corpus["unknown_fields"]["oneof_member"]
    ok("the unknown ONEOF member is declared on %s in the superset only" % om["message"],
       om["field"]["tag"] not in tags_at(reader, om["message"])
       and om["field"]["tag"] in tags_at(superset, om["message"]))

    # 4. --check notices drift, and notices it in BOTH directions.
    out = os.path.join(spec.ROOT, "generated")
    tmp = tempfile.mkdtemp(prefix="ffi-corpus-selftest-")
    try:
        shutil.copytree(out, os.path.join(tmp, "g"))
        g = os.path.join(tmp, "g")
        vec = os.path.join(g, "vectors", "S-bool-true.bin")
        with open(vec, "ab") as fh:
            fh.write(b"\x00")
        ok("--check notices a changed vector", B.check(g) == 1)
        shutil.rmtree(g)
        shutil.copytree(out, g)
        os.remove(os.path.join(g, "vectors", "S-bool-true.bin"))
        ok("--check notices a missing vector", B.check(g) == 1)
        shutil.rmtree(g)
        shutil.copytree(out, g)
        with open(os.path.join(g, "vectors", "S-not-a-vector.bin"), "wb") as fh:
            fh.write(b"x")
        ok("--check notices a file that is no longer generated", B.check(g) == 1)
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    # 5. Path independence. The python slice's D4 was an absolute path baked into
    #    a generated module, so a clone at any other path could not rebuild its
    #    own tree. It was found by cloning the pushed branch and building it, not
    #    by reading the code, so this is the cheap half of that check and
    #    run.sh's clean-clone step is the other half.
    leaked = []
    for base, _, files in os.walk(out):
        for f in files:
            p = os.path.join(base, f)
            with open(p, "rb") as fh:
                blob = fh.read()
            for needle in (spec.ROOT.encode(), spec.FFI.encode(), b"/home/", b"/tmp/"):
                if needle in blob:
                    leaked.append((os.path.relpath(p, out), needle.decode()))
    ok("no generated file contains an absolute path from this machine", not leaked,
       "; ".join("%s carries %s" % x for x in leaked[:3]))

    # 6. The manifest says what a consumer needs, for every vector.
    man = json.load(open(os.path.join(out, "manifest.json")))
    missing = [k for k, v in man["vectors"].items()
               if not v.get("tests") or "produce" not in v or "consume" not in v
               or "expect" not in v or "sha256" not in v or "file" not in v]
    ok("every manifest row carries tests/produce/consume/expect/sha256/file", not missing,
       ", ".join(missing[:5]))
    unhashed = []
    for k, v in man["vectors"].items():
        p = os.path.normpath(os.path.join(out, v["file"]))
        import hashlib
        with open(p, "rb") as fh:
            if hashlib.sha256(fh.read()).hexdigest() != v["sha256"]:
                unhashed.append(k)
    ok("every committed vector matches its own hash", not unhashed, ", ".join(unhashed[:5]))
    badproduce = [k for k, v in man["vectors"].items() if v["produce"] and not v["canonical"]]
    ok("no vector asks a producer for a non-canonical encoding", not badproduce,
       ", ".join(badproduce[:5]))
    rejects = [v for v in man["vectors"].values() if v["expect"] == "reject"]
    ok("every reject row names what was seen refusing it",
       all("seen_failing" in v.get("reject", {}) for v in rejects), "%d rows" % len(rejects))

    bad = [r for r in RESULTS if not r[0]]
    for good_, name, detail in RESULTS:
        print("%-4s %-62s %s" % ("ok" if good_ else "FAIL", name, detail))
    print("\n%d checks, %d failed" % (len(RESULTS), len(bad)))
    return 1 if bad else 0


def two_oneofs(reader):
    s2 = copy.deepcopy(reader)
    s2["messages"]["Probe"]["fields"].append(
        {"name": "other", "tag": 905, "kind": "int32", "oneof": "second"})
    return s2


def dup_tag(reader):
    s2 = copy.deepcopy(reader)
    s2["messages"]["ChunkLeaf"]["fields"].append({"name": "again", "tag": 1, "kind": "int32"})
    return s2


def reserved_tag(reader):
    s2 = copy.deepcopy(reader)
    s2["messages"]["ChunkLeaf"]["fields"].append({"name": "res", "tag": 19500, "kind": "int32"})
    return s2


def compiled(reader, superset):
    tmp = tempfile.mkdtemp(prefix="ffi-corpus-desc-")
    with open(os.path.join(tmp, "corpus.proto"), "w") as fh:
        fh.write(spec.emit_proto(reader, spec.READER_BANNER))
    with open(os.path.join(tmp, "corpus_superset.proto"), "w") as fh:
        fh.write(spec.emit_proto(superset, spec.SUPERSET_BANNER))
    try:
        return B.compile_protos(tmp)
    finally:
        pass


if __name__ == "__main__":
    sys.exit(main())
