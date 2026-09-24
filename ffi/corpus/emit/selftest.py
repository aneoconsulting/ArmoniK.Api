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

    # upb is one of three oracles, not the judge. Every must-fail vector upb
    # does NOT refuse must be one the committed manifest records as disputed
    # with upb among its acceptors -- so a upb acceptance is either a known,
    # published disagreement or a failure here.
    with open(os.path.join(spec.ROOT, "generated", "manifest.json")) as fh:
        man0 = json.load(fh)
    rejected, upb_accepts = 0, []
    for v in VEC.build_all(reader, superset, corpus):
        if v.expect != "reject":
            continue
        try:
            B.msg_class(pools, "reader", v.root)().ParseFromString(v.data)
        except Exception:                        # noqa: BLE001
            rejected += 1
        else:
            upb_accepts.append(v.id)
    total = sum(1 for v in VEC.build_all(reader, superset, corpus) if v.expect == "reject")
    unpublished = [k for k in upb_accepts
                   if man0["vectors"].get(k, {}).get("verdict") != "disputed"
                   or B.UPB not in man0["vectors"][k]["reject"]["seen_failing"]["accepted_by"]]
    ok("every must-fail vector upb accepts is published as disputed (%d of %d refused by upb; "
       "accepted: %s)" % (rejected, total, ", ".join(upb_accepts) or "none"),
       not unpublished, ", ".join(unpublished))

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
    with open(os.path.join(out, "manifest.json")) as fh:
        man = json.load(fh)
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

    # 7. The seal. The committed vector bytes are pinned by every consumer that
    #    has run against them, so the build must refuse to move one.
    sealed = B.read_seal(out)
    ok("the vector bytes are sealed (%d of them)" % (len(sealed or ())),
       sealed is not None and len(sealed) == len(os.listdir(os.path.join(out, "vectors"))))
    import hashlib as _h
    live = {}
    for f in os.listdir(os.path.join(out, "vectors")):
        with open(os.path.join(out, "vectors", f), "rb") as fh:
            live[f] = _h.sha256(fh.read()).hexdigest()
    ok("every committed vector still matches the seal", live == sealed)
    must_raise("the seal refuses a vector whose BYTES changed",
               lambda: B.check_seal(out, dict(live, **{sorted(live)[0]: "0" * 64})), SystemExit)
    must_raise("the seal refuses a vector that disappeared",
               lambda: B.check_seal(out, dict((k, v) for k, v in list(live.items())[1:])),
               SystemExit)
    must_raise("the seal refuses a vector that was added",
               lambda: B.check_seal(out, dict(live, **{"Z-new.bin": "0" * 64})), SystemExit)

    # 8. The dispute machinery, watched working. A corpus that can only agree
    #    with itself is a corpus with one oracle, which is where this started.
    man2 = man
    disputed = [k for k, v in man2["vectors"].items() if v.get("verdict") == "disputed"]
    ok("at least one row is DISPUTED, so the machinery is not vacuous",
       bool(disputed), ", ".join(disputed))
    ok("every disputed row has no projection of its own and is excluded from pass/fail",
       all(man2["vectors"][k].get("projection") is None
           and man2["vectors"][k]["dispute"].get("excluded_from_pass_fail") is True
           for k in disputed))
    ok("every disputed row carries more than one reading, each naming its runtime",
       all(len(man2["vectors"][k]["dispute"].get("readings", [])) > 1
           and all(r.get("read_by") and r.get("projection")
                   for r in man2["vectors"][k]["dispute"]["readings"])
           for k in disputed if "readings" in man2["vectors"][k]["dispute"]))
    # A dispute is about a READING (two projections) or about the VERDICT (some
    # oracles refuse the vector, some accept it). Only the first kind has
    # readings to compare; the second is checked in section 10.
    for k in disputed:
        if "readings" not in man2["vectors"][k]["dispute"]:
            ok("%s is a verdict dispute naming who refused and who accepted" % k,
               man2["vectors"][k]["dispute"].get("refused_by")
               and (man2["vectors"][k]["dispute"].get("accepted_by")
                    or man2["vectors"][k]["dispute"].get("parsed_by")))
            continue
        rs = man2["vectors"][k]["dispute"].get("readings", [])
        blobs = set()
        for r in rs:
            with open(os.path.join(out, r["projection"])) as fh:
                blobs.add(fh.read())
        ok("%s's readings really do differ" % k, len(blobs) == len(rs) > 1)
    ok("a disputed row names WHERE the readings differ",
       all(man2["vectors"][k]["dispute"].get("differs_at") for k in disputed
           if "readings" in man2["vectors"][k]["dispute"]))
    ok("the diff reporter is silent on two identical readings",
       B.reading_diff({json.dumps({"a": 1}): ["x"], }) == set())
    ok("the diff reporter names a nested path",
       B.reading_diff({json.dumps({"a": {"b": [{"c": 1}]}}): ["x"],
                       json.dumps({"a": {"b": [{"c": 2}]}}): ["y"]}) == {"a.b.[0].c"})

    # 9. Every accepted encoding says WHO was seen writing it. "upb writes this
    #    form" and "every conformant encoder writes this form" are different
    #    claims and the manifest used to make only the first.
    noprov = [k for k, v in man2["vectors"].items()
              for a in v.get("accepted_encodings", [])
              if not a.get("written_by") or "observed_in_a_protobuf_runtime" not in a]
    ok("every accepted encoding names who was seen writing it", not noprov,
       ", ".join(sorted(set(noprov))[:5]))
    declared_only = sorted(set(
        k for k, v in man2["vectors"].items()
        for a in v.get("accepted_encodings", []) if not a["observed_in_a_protobuf_runtime"]))
    ok("forms the corpus DECLARES but no runtime was seen writing are marked as such",
       bool(declared_only), "%d rows, e.g. %s" % (len(declared_only), ", ".join(declared_only[:3])))
    nocons = [k for k, v in man2["vectors"].items()
              if v["expect"] == "accept" and v.get("verdict") == "agreed"
              and not v.get("accepted_encodings")]
    ok("every agreed accept row has at least one accepted encoding", not nocons,
       ", ".join(nocons[:5]))
    perm = sorted(k for k, v in man2["vectors"].items() if v.get("permutation_accepted"))
    ok("the interleaved rows accept a permutation, so a conformant encoder passes C3",
       "B-P7_1" in perm and "S-interleaved" in perm, ", ".join(perm))
    ok("the permutation rule is not applied to rows that are not permutations",
       not man2["vectors"]["B-P2_5"].get("permutation_accepted")
       and not man2["vectors"]["E-half-absent"].get("permutation_accepted"))

    # 10. Three oracles, and all three were asked.
    used = [o["name"] for o in man2["oracles"]["used"]]
    ok("three oracles are named in the manifest (%d)" % len(used), len(used) == 3,
       "; ".join(used))
    rej_rows = dict((k, v) for k, v in man2["vectors"].items() if v["expect"] == "reject")
    partial = sorted(k for k, v in rej_rows.items()
                     if len(v["reject"]["seen_failing"]["refused_by"]) != 3)
    ok("every must-fail vector was refused by all three, or is DISPUTED naming who accepted it "
       "(%d of %d by all three)" % (len(rej_rows) - len(partial), len(rej_rows)),
       all(rej_rows[k].get("verdict") == "disputed"
           and rej_rows[k]["reject"]["seen_failing"]["accepted_by"] for k in partial)
       and all(len(v["reject"]["seen_failing"]["refused_by"]) >= 1 for v in rej_rows.values()),
       "; ".join("%s accepted by %s" % (k, ", ".join(rej_rows[k]["reject"]["seen_failing"]
                                                    ["accepted_by"])) for k in partial)
       + " | " + json.dumps(man2["oracles"]["refusals_confirmed_by"]))

    # 11. FIX-PLAN WP4 item 2. Each category is checked against the claim it
    #     makes, from the committed bytes and the description, not by counting.
    wp4_checks(reader, man2, out)

    bad = [r for r in RESULTS if not r[0]]
    for good_, name, detail in RESULTS:
        print("%-4s %-62s %s" % ("ok" if good_ else "FAIL", name, detail))
    print("\n%d checks, %d failed" % (len(RESULTS), len(bad)))
    return 1 if bad else 0


def wp4_checks(reader, man, out):
    import vectors as VEC
    import shapes as SH
    rows = man["vectors"]

    # (a) A length that wraps 2^64. The arithmetic guard, watched refusing a
    #     false claim, then run over every committed row.
    lw = sorted(k for k in rows if k.startswith("X-lenwrap-"))
    probe = rows["X-lenwrap-lrr-unknown-zero"]
    with open(os.path.join(out, probe["file"]), "rb") as fh:
        pdata = fh.read()
    ok("X-lenwrap-lrr-unknown-zero is FIX-PLAN WP4 item 1's reproducer, byte for byte",
       pdata == bytes.fromhex("7af5ffffffffffffffff01"), pdata.hex())
    nowrap = dict(probe["meta"], counted_from_offset=probe["meta"]["counted_from_offset"] - 2)
    must_raise("the wrap guard refuses a length that does NOT wrap from its position",
               lambda: VEC.wrap_arith(pdata, nowrap))
    must_raise("the wrap guard refuses a declared length the bytes do not carry",
               lambda: VEC.wrap_arith(pdata, dict(
                   probe["meta"], declared_length=str(int(probe["meta"]["declared_length"]) - 2))))
    must_raise("the wrap guard refuses a vector whose stated sum is wrong",
               lambda: VEC.wrap_arith(pdata, dict(probe["meta"], sum_mod_2_64="1")))
    bad = []
    for k in lw:
        with open(os.path.join(out, rows[k]["file"]), "rb") as fh:
            d = fh.read()
        try:
            VEC.wrap_arith(d, rows[k]["meta"])
        except ShapeNotCovered as e:
            bad.append("%s: %s" % (k, e))
    ok("every X-lenwrap row wraps 2^64 (or sits one below) exactly as it says (%d rows)" % len(lw),
       lw and not bad, "; ".join(bad[:3]))
    kinds = set(rows[k]["meta"]["field_is"] for k in lw)
    ok("the wrap reaches an unknown, a string and a message field", kinds == {"unknown", "string",
                                                                              "message"},
       ", ".join(sorted(kinds)))
    both = set(rows[k]["meta"]["counted_from"] for k in lw)
    ok("nested wraps are counted from the buffer AND from the enclosing message",
       both == {"buffer", "enclosing message"}, ", ".join(sorted(both)))
    ok("every X-lenwrap row is must-fail and agreed",
       all(rows[k]["expect"] == "reject" and rows[k]["verdict"] == "agreed" for k in lw))

    # (b) A known field at a foreign wire type: every (message, shape, foreign
    #     wire type) the description implies has its row, and every oracle read
    #     it as an unknown field rather than refusing or misreading it.
    want = set()
    for name, m in reader["messages"].items():
        picked = {}
        for f in SH.fields(m):
            picked.setdefault(E.shape_key(f), f)
        for skey, f in picked.items():
            legal = {spec.wire_of(f)}
            if spec.card(f) == "packed":
                legal.add(spec.KINDS[f["kind"]])
            for wt in {0, 1, 2, 5} - legal:
                want.add("U-wire-%s-%s-as-wt%d" % (name, f["name"].replace("_", "-"), wt))
    have = set(k for k in rows if k.startswith("U-wire-"))
    ok("every message x shape x foreign wire type has a U-wire row (%d)" % len(want),
       want == have, "missing %s; extra %s" % (sorted(want - have)[:3], sorted(have - want)[:3]))
    wrong = [k for k in have
             if rows[k]["verdict"] != "agreed"
             or rows[k].get("unknown_tags_seen_by_reader") != [rows[k]["meta"]["wrong_wire_type"]["tag"]]]
    ok("every U-wire row is agreed and reads the foreign-typed field as UNKNOWN", not wrong,
       ", ".join(sorted(wrong)[:5]))

    # (c) Tag 0 on every message the corpus roots.
    roots = set(r["root"] for r in rows.values())
    tz = set(k[len("X-tag-zero-"):] for k in rows
             if k.startswith("X-tag-zero-") and k != "X-tag-zero-nested-Empty")
    ok("field number 0 has a must-fail row on every root (%d)" % len(roots), roots <= tz,
       ", ".join(sorted(roots - tz)))

    # (d) -0.0 keeps its sign, and (e) negative integers are PROJECTED.
    def pj(k):
        with open(os.path.join(out, rows[k]["projection"])) as fh:
            return fh.read()
    mz = sorted(k for k in rows if k.startswith("S-mzero-"))
    ok("every S-mzero row projects \"-0\" (%d rows)" % len(mz),
       mz and all(rows[k].get("projection") and '"-0"' in pj(k) for k in mz))
    ng = sorted(k for k in rows if k.startswith("S-neg-"))
    ok("every S-neg row projects a negative integer (%d rows)" % len(ng),
       ng and all(rows[k].get("projection") and '"-' in pj(k) for k in ng))
    noncanon = [k for k in ng + mz if rows[k]["produce"] and rows[k].get("upb_agreement") != "identical"]
    ok("every S-neg / S-mzero row a slice must produce is the form upb writes too", not noncanon,
       ", ".join(noncanon))


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
