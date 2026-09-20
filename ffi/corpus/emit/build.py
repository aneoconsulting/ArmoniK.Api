"""Generate the corpus, and validate every vector against runtimes that share no
code with the writer -- more than one of them, on purpose.

    python3 emit/build.py            regenerate generated/
    python3 emit/build.py --check    fail if what is committed is not what this
                                     script writes today
    python3 emit/build.py --reseal   re-freeze generated/vectors.sha256. A
                                     decision, not a step: see that file.

`--check` regenerates into a temporary directory at a DIFFERENT path and diffs,
so a generator that bakes its own location into its output fails here: that is
the python slice's D4, found by cloning the pushed branch and building it rather
than by reading the code.

## Three oracles, and why one was not enough

The first version of this file generated every vector, every accepted encoding
and every projection with **upb**, and validated every accept and reject verdict
by parsing with upb. **One runtime deciding what the right answer is makes that
runtime the specification**, and the corpus's first two consumers found a row
where it is the minority. On `U-map-entry` -- an unknown field inside every map
entry -- upb drops the entry from the map and keeps its bytes as an unknown
field of the parent, while protobuf-python's pure backend and protobuf C++ both
put the entry in the map. A map field is shorthand for a repeated `MapEntry`
message and an unknown field inside a submessage is skipped while the submessage
still parses, so upb is alone. The corpus published upb's reading as *the*
projection, and a conformant consumer failed that row.

So:

| Oracle | How | What it decides |
|---|---|---|
| **upb** | in process | the structural checks, and one reading |
| **protobuf-python, pure backend** | a subprocess with `PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION=python`, running `emit/oracle.py` | a second reading, full projection, machine-comparable |
| **protobuf C++** | `protoc --decode` per vector | the accept/reject verdict on every row, and its text reading as evidence |

The pure backend is a different parser reached through the same API, which makes
it less independent than protobuf C++ and far easier to diff: it answers in the
same projection format, so a disagreement is a JSON difference rather than an
argument. protobuf C++ is fully independent and is used for what it can answer
cleanly. **It is deliberately NOT used as the projection oracle**: `protoc
--decode` renders a map field as its wire-level repeated `MapEntry` list -- on
`E-map-dup-key` it prints two entries with the same key, which a map cannot hold
-- so it cannot answer a map-semantics question. The cpp slice's reflection arm
can, and that is where protobuf C++'s reading of `U-map-entry` comes from.

## Where they disagree, the row is DISPUTED

A disputed row carries every reading, names which runtime produced each, has no
`projection` of its own, and is **excluded from a consumer's pass or fail count**
rather than failing it. If two conformant runtimes read the same bytes
differently, the corpus's job is to say so, not to pick.

## What still fails the build

1. an accept-vector **no** oracle will parse;
2. a reject-vector **every** oracle accepts -- a rejection test that nothing
   rejects is a test nobody has watched work, and that lesson cost this branch
   twice;
3. a field shape present in the description that no vector exercises (R1: a
   walker with no case for a shape raises, it never skips);
4. a vector whose bytes differ from `generated/vectors.sha256`. Consumers have
   pinned those bytes; the manifest's claims about them are what this file is
   allowed to change.
"""
import base64
import filecmp
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import encode as E                                        # noqa: E402
import oracle as O                                        # noqa: E402
import spec                                               # noqa: E402
import vectors as VEC                                     # noqa: E402
from oracle import compile_protos, msg_class, project, runtime_id, unknown_tags_seen  # noqa: E402,F401
from spec import ShapeNotCovered                          # noqa: E402

sys.path.insert(0, os.path.join(spec.SCHEMA, "emit"))
import shapes as S                                        # noqa: E402

PKG = "armonik.ffi.corpus.v1"

# A projection larger than this is omitted; see the comment where it is used.
PROJECTION_LIMIT = 32 * 1024

import google.protobuf as _pb                                  # noqa: E402
PBVER = _pb.__version__


def fields_of(buf):
    """(tag, wire, body) for every field of one message body, or None if `buf`
    does not parse as a message. A structural read: no descriptor."""
    out, i = [], 0
    try:
        while i < len(buf):
            k, i = _varint(buf, i)
            tag, w = k >> 3, k & 7
            if tag == 0:
                return None
            if w == 0:
                j = i
                _, i = _varint(buf, i)
                out.append((tag, w, buf[j:i]))
            elif w == 1:
                out.append((tag, w, buf[i:i + 8]))
                i += 8
            elif w == 5:
                out.append((tag, w, buf[i:i + 4]))
                i += 4
            elif w == 2:
                n, i = _varint(buf, i)
                out.append((tag, w, buf[i:i + n]))
                i += n
            else:
                return None
            if i > len(buf):
                return None
        return out
    except (IndexError, ValueError):
        return None


def _varint(b, i):
    n = sh = 0
    while True:
        c = b[i]
        n |= (c & 0x7F) << sh
        i += 1
        if not c & 0x80:
            return n, i
        sh += 7
        if sh > 63:
            raise ValueError("varint too long")


def permutation(a, b):
    """Whether two message bodies carry the same fields in a different ORDER.

    Protobuf does not fix field order on the wire; the corpus's canonical form
    does, because without one there is no byte identity to check. This says so
    when the two differ only by that, so the manifest can label upb's form
    rather than record an unexplained second hash.
    """
    fa, fb = fields_of(a), fields_of(b)
    if fa is None or fb is None:
        return False
    return sorted(fa) == sorted(fb)


def shape_gaps(universe, covered):
    """The shapes in the description that no vector exercises. Kept as its own
    function so emit/selftest.py can watch it report one."""
    return sorted(k for k in universe if k not in covered)


def shape_universe(reader):
    """Every field shape the description contains, as the keys enc_field traces.

    R1 one level up: the corpus claims to cover every field shape mechanically,
    and the mechanical form of that claim is this set. A shape in here with no
    vector fails the build.
    """
    out = {}
    for name, m in reader["messages"].items():
        for f in S.fields(m):
            out.setdefault(E.shape_key(f), []).append("%s.%s" % (name, f["name"]))
    return out


# ----------------------------------------------------------------------- build

# --------------------------------------------------------------- the seal

SEAL = "vectors.sha256"


def read_seal(outdir):
    path = os.path.join(outdir, SEAL)
    if not os.path.exists(path):
        return None
    out = {}
    with open(path) as fh:
        for line in fh:
            if line.startswith("#") or not line.strip():
                continue
            digest, name = line.split()
            out[name] = digest
    return out


def write_seal(outdir, rows, header):
    with open(os.path.join(outdir, SEAL), "w") as fh:
        fh.write(header)
        for name in sorted(rows):
            fh.write("%s  %s\n" % (rows[name], name))


def seal_header(outdir):
    """Keep the prose at the top of the seal file, so re-sealing does not lose it."""
    path = os.path.join(outdir, SEAL)
    if not os.path.exists(path):
        return "# sha256  name\n"
    out = []
    with open(path) as fh:
        for line in fh:
            if line.startswith("#") or not line.strip():
                out.append(line)
            else:
                break
    return "".join(out)


def check_seal(outdir, produced):
    """The committed vector bytes are pinned by every consumer that has run.

    The manifest's claims about those bytes are what this file exists to revise;
    the bytes are not. A row that moves here is almost always a generator change
    nobody meant to make, so it stops the build and names the vector.
    """
    sealed = read_seal(outdir)
    if sealed is None:
        return
    bad = []
    for name, digest in sorted(produced.items()):
        if name not in sealed:
            bad.append("added, not in the seal: " + name)
        elif sealed[name] != digest:
            bad.append("BYTES CHANGED: %s\n    sealed %s\n    now    %s"
                       % (name, sealed[name], digest))
    for name in sorted(sealed):
        if name not in produced:
            bad.append("no longer generated, but sealed: " + name)
    if bad:
        raise SystemExit(
            "the committed vector bytes have moved, and consumers have pinned them:\n  "
            + "\n  ".join(bad[:20])
            + ("\n  ... and %d more" % (len(bad) - 20) if len(bad) > 20 else "")
            + "\nIf the change is intended, re-seal deliberately: emit/build.py --reseal")


# ------------------------------------------------------- the other oracles

def alt_oracle(outdir, jobs, backend="python"):
    """protobuf-python's PURE backend, in a subprocess.

    `PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION` is read once when google.protobuf is
    first imported, so the only way to ask two backends the same question is to
    ask them in two processes. A different parser reached through the same API:
    less independent than protobuf C++ and far easier to diff, because it answers
    in the same projection format.
    """
    env = dict(os.environ, PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION=backend)
    r = subprocess.run([sys.executable, os.path.join(HERE, "oracle.py"), outdir],
                       input=json.dumps(jobs).encode(), capture_output=True, env=env)
    if r.returncode:
        raise SystemExit("the %s-backend oracle failed:\n%s" % (backend, r.stderr.decode()[:2000]))
    return json.loads(r.stdout)


def cpp_oracle(outdir, jobs):
    """protobuf C++, through `protoc --decode`, one process per vector.

    Used for the accept/reject VERDICT on every row -- a fully independent third
    opinion on all 49 refusals -- and for its text reading, kept as evidence on
    the rows where the two Python backends disagree.

    **Not** used as the projection oracle. `protoc --decode` renders a map field
    as its wire-level repeated `MapEntry` list: on `E-map-dup-key` it prints two
    entries with the same key, which a map cannot hold, so it cannot answer a
    map-semantics question. The cpp slice's reflection arm can, and that is where
    protobuf C++'s reading of `U-map-entry` comes from.
    """
    out = {}
    for job in jobs:
        with open(job["path"], "rb") as fh:
            data = fh.read()
        r = subprocess.run([sys.executable, "-m", "grpc_tools.protoc", "-I", ".",
                            "--decode=%s.%s" % (PKG, job["root"]), "corpus.proto"],
                           input=data, capture_output=True, cwd=outdir)
        row = {"parse": "error" if r.returncode else "ok"}
        if r.returncode:
            row["error"] = r.stderr.decode("utf-8", "replace").strip().splitlines()[-1][:120]
        else:
            row["text"] = r.stdout.decode("utf-8", "replace")
        out[job["id"]] = row
    return out


def vector_path(outdir, row):
    """Where a vector's bytes actually live.

    The manifest's `file` is relative to the manifest's own directory, which is
    what a consumer wants; a BASELINE's is "../../schema/generated/...", which
    only resolves when that directory is two levels deep. `--check` regenerates
    into a temporary directory at a different path on purpose, so the generator
    resolves baselines against the schema directory rather than by traversal.
    """
    f = row["file"]
    if f.startswith("../../schema/"):
        return os.path.join(spec.SCHEMA, f[len("../../schema/"):])
    return os.path.normpath(os.path.join(outdir, f))


def cpp_runtime_id():
    r = subprocess.run([sys.executable, "-m", "grpc_tools.protoc", "--version"],
                       capture_output=True)
    return "protobuf C++ via %s (protoc --decode)" % r.stdout.decode().strip()


def sha(b):
    return hashlib.sha256(b).hexdigest()


def generate(outdir, quiet=False):
    reader, superset, corpus = spec.load()
    os.makedirs(outdir, exist_ok=True)
    for sub in ("vectors", "projections"):
        d = os.path.join(outdir, sub)
        if os.path.isdir(d):
            shutil.rmtree(d)
        os.makedirs(d)

    with open(os.path.join(outdir, "corpus.proto"), "w") as fh:
        fh.write(spec.emit_proto(reader, spec.READER_BANNER))
    with open(os.path.join(outdir, "corpus_superset.proto"), "w") as fh:
        fh.write(spec.emit_proto(superset, spec.SUPERSET_BANNER))

    pools = compile_protos(outdir)
    vs = VEC.build_all(reader, superset, corpus)
    covered = set()
    rows = {}
    upb_verdict, upb_reading, acc_by_id, raw_forms = {}, {}, {}, {}
    produced_sha = {}

    for v in vs:
        covered |= v.trace
        row = {
            "class": v.cls,
            "tests": v.tests,
            "root": v.root,
            "view": v.view,
            "expect": v.expect,
            "bytes": len(v.data),
            "sha256": sha(v.data),
            "canonical": v.canonical,
            "produce": list(v.produce),
            "consume": list(v.consume),
        }
        if v.why:
            row["why"] = v.why
        if v.notes:
            row["notes"] = v.notes
        if v.meta:
            row["meta"] = dict((k, x) for k, x in v.meta.items() if x is not None)

        path = os.path.join("vectors", v.id + ".bin")
        with open(os.path.join(outdir, path), "wb") as fh:
            fh.write(v.data)
        row["file"] = path
        produced_sha[v.id + ".bin"] = sha(v.data)

        cls = msg_class(pools, "reader", v.root)
        if v.expect == "reject":
            # One runtime's refusal is one runtime's opinion. Recorded here and
            # reconciled against the other two below: the build dies only if
            # NOTHING refuses the vector.
            m = cls()
            row["reject"] = dict(v.reject)
            try:
                m.ParseFromString(v.data)
            except Exception as exc:                              # noqa: BLE001
                upb_verdict[v.id] = ("reject", type(exc).__name__)
            else:
                upb_verdict[v.id] = ("accept", None)
            rows[v.id] = row
            continue

        m = cls()
        try:
            m.ParseFromString(v.data)
        except Exception as exc:                                  # noqa: BLE001
            upb_verdict[v.id] = ("reject", type(exc).__name__)
            rows[v.id] = row
            continue
        upb_verdict[v.id] = ("accept", None)

        pj = project(m)

        # Every encoding a conformant implementation may write for this message.
        # A vector may have MORE THAN ONE, and P2.5 is the proof: an empty map
        # value is an implicit-presence leaf holding the proto zero, prost omits
        # it and protobuf C++, upb and protobuf-java write it, both parse to the
        # same message and neither encoder is wrong.
        forms = [("as committed", v.data)] + list(v.forms)
        accepted = {}
        for label, b in forms:
            # Every DECLARED form was produced by the corpus's own writer -- that
            # is where its bytes come from. Whether any protobuf runtime was
            # also seen writing it is a separate and weaker claim, and keeping
            # the two apart is the point of `written_by`: "upb writes this form"
            # and "every conformant encoder writes this form" are not the same
            # sentence, and the manifest used to make only the first.
            accepted.setdefault(sha(b), {"sha256": sha(b), "bytes": len(b), "forms": [],
                                         "written_by": [CANON], "_observed": []})
            accepted[sha(b)]["forms"].append(label)
        re = m.SerializeToString(deterministic=True)
        if sha(re) in accepted:
            agreement = "identical" if re == v.data else "a form the vector declares"
        else:
            # Not one of the declared forms. It is only acceptable if it encodes
            # the SAME MESSAGE: protobuf does not fix field order on the wire,
            # and this branch is where that shows. Anything that does not
            # re-parse to the same projection fails the build.
            back = cls()
            back.ParseFromString(re)
            if project(back) != pj:
                raise SystemExit(
                    "%s: upb re-encodes to bytes that do NOT parse back to the same message.\n"
                    "  committed %s\n  upb       %s\n"
                    "That is a defect in one of the two implementations, not a form to declare."
                    % (v.id, v.data.hex()[:160], re.hex()[:160]))
            agreement = ("the same message in a different field ORDER"
                         if permutation(v.data, re) else
                         "the same message, encoded differently")
            accepted[sha(re)] = {"sha256": sha(re), "bytes": len(re),
                                 "forms": ["as upb writes it: " + agreement]}
        accepted[sha(re)].setdefault("written_by", []).append(UPB)
        accepted[sha(re)].setdefault("_observed", []).append(UPB)
        raw_forms[v.id] = dict((sha(b), b) for _, b in forms)
        raw_forms[v.id][sha(re)] = re
        acc_by_id[v.id] = accepted
        upb_reading[v.id] = {"projection": pj, "reencode": re.hex(), "agreement": agreement}

        # The unknown-field claim, checked rather than asserted: the tags the
        # vector says are unknown must BE unknown under the reader, and known
        # under the superset.
        utags = spec.unknown_tags(corpus) | {VEC.uf_group_tag(corpus), 536870911}
        seen = unknown_tags_seen(m) & utags
        declared = set()
        for k in ("unknown_tag", "group_tag"):
            if v.meta.get(k):
                declared.add(v.meta[k])
        for k in ("unknown_tags",):
            declared |= set(v.meta.get(k, ()))
        if v.cls == "unknown" and not seen and not v.meta.get("enum_value") \
                and "enum_values" not in v.meta and not v.meta.get("entry_unknown_tag"):
            raise SystemExit("%s is in class `unknown` and upb found no unknown field in it" % v.id)
        if declared and not declared <= seen:
            raise SystemExit("%s declares unknown tag(s) %s; upb found %s"
                             % (v.id, sorted(declared), sorted(seen)))
        if seen:
            row["unknown_tags_seen_by_reader"] = sorted(seen)
            if v.meta.get("declared_in_superset", True):
                sup = msg_class(pools, "superset", v.superset_root)()
                sup.ParseFromString(v.data)
                still = unknown_tags_seen(sup) & seen
                if still:
                    raise SystemExit("%s: tag(s) %s are still unknown under the SUPERSET view, so "
                                     "the vector is not testing what it claims"
                                     % (v.id, sorted(still)))
                write_projection(outdir, row, v.id, project(sup), "superset")
        rows[v.id] = row

    # The bytes every consumer has already pinned. Read from the COMMITTED tree
    # whatever tree is being written, so `--check` -- which regenerates into a
    # temporary directory at a different path -- compares against the same seal.
    committed = os.path.join(spec.ROOT, "generated")
    check_seal(committed, produced_sha)
    write_seal(outdir, produced_sha, seal_header(committed))

    # The schema's payloads, through the SAME reconciliation as everything else.
    vs = list(vs) + baselines(pools, outdir, rows, upb_verdict, upb_reading,
                              acc_by_id, raw_forms)

    # The other two oracles, and the reconciliation. One runtime deciding what
    # the right answer is makes that runtime the specification.
    jobs = [{"id": v.id, "root": v.root, "file": rows[v.id]["file"],
             "path": vector_path(outdir, rows[v.id])} for v in vs]
    alt = alt_oracle(outdir, jobs)
    cpp = cpp_oracle(outdir, jobs)
    names = {UPB: UPB, PURE: PURE, CPP: cpp_runtime_id(), CANON: CANON}
    if alt["runtime"]["backend"] != "python":
        raise SystemExit(
            "the second oracle came back on the %r backend, not the pure-python one. Two "
            "readings from one parser are one reading."
            % alt["runtime"]["backend"])
    disputed, seen_failing, rejected_by = reconcile(
        outdir, rows, vs, upb_verdict, upb_reading, acc_by_id, raw_forms, alt, cpp, names)

    # Coverage. A shape in the description that no vector exercises fails here.
    universe = shape_universe(reader)
    gaps = shape_gaps(universe, covered)
    if gaps:
        raise SystemExit(
            "no vector exercises these field shapes, which the description contains:\n"
            + "\n".join("  %-28s e.g. %s" % (k, universe[k][0]) for k in gaps))

    manifest = {
        "corpus_version": corpus["corpus_version"],
        "generated_by": "ffi/corpus/emit/build.py from corpus.json and ../schema/shapes.json",
        "status": ("VALIDATED BY THREE RUNTIMES. Every accept-vector was parsed, re-encoded and "
                   "projected by two protobuf implementations that share no parser with the "
                   "writer or with each other, and every reject-vector was SEEN failing by all "
                   "three. A row where the readings disagree is marked `disputed`, carries every "
                   "reading, and is excluded from a consumer's pass or fail count rather than "
                   "failing it -- if two conformant runtimes read the same bytes differently, "
                   "the corpus's job is to say so, not to pick. The vector BYTES are frozen in "
                   "vectors.sha256 and the build refuses to move them; everything else in this "
                   "file is a claim about them and is expected to sharpen as more runtimes are "
                   "asked. `python3 emit/build.py --check` fails if what is committed is not "
                   "what the generator writes today."),
        "validated_against": runtime_id(),
        "oracles": {
            "why_more_than_one": (
                "The first version of this corpus generated every vector, every accepted "
                "encoding and every projection with upb, and validated every verdict by "
                "parsing with upb. One runtime deciding what the right answer is makes that "
                "runtime the specification, and on U-map-entry it is the minority: upb drops "
                "a map entry carrying an unknown field out of the map and keeps its bytes as "
                "an unknown field of the parent, while protobuf-python's pure backend and "
                "protobuf C++ both put the entry in the map."),
            "used": [
                {"name": names[UPB], "asked": "the structural checks, one reading, one re-encoding"},
                {"name": names[PURE],
                 "asked": "a second reading and re-encoding, in the same projection format",
                 "independence": "a different parser reached through the same API: less "
                                 "independent than protobuf C++, and far easier to diff"},
                {"name": names[CPP],
                 "asked": "the accept/reject verdict on every row, and its text reading as "
                          "evidence on disputed rows",
                 "not_asked": "the projection. `protoc --decode` renders a map field as its "
                              "wire-level repeated MapEntry list -- on E-map-dup-key it prints "
                              "two entries with the same key, which a map cannot hold -- so it "
                              "cannot answer a map-semantics question. The cpp slice's "
                              "reflection arm can, and does."},
            ],
            "refusals_confirmed_by": rejected_by,
            "disputed_rows": sorted(disputed),
        },
        "descriptors": {
            "reader": "corpus.proto",
            "superset": "corpus_superset.proto",
            "difference": ("the fields a conformant reader is NOT built against. A vector is "
                           "WRITTEN against the superset and READ against the reader."),
        },
        "reading_this_file": {
            "verdict": "`agreed` means every oracle asked read the vector the same way. "
                       "`disputed` means they did not: the row carries every reading, names "
                       "which runtime produced each, has no projection of its own, and is "
                       "EXCLUDED from a consumer's pass or fail count rather than failing it.",
            "accepted_encodings.written_by": "who was SEEN writing that form. "
                       "'%s' means the corpus's own canonical writer; a runtime name means "
                       "that runtime re-encoded its own reading this way. The two are "
                       "different claims and this field keeps them apart." % CANON,
            "permutation_accepted": "true where two accepted encodings are the same fields in "
                       "a different ORDER. Protobuf does not fix field order on the wire, so a "
                       "consumer's re-encoding passes C3 if it is a permutation of an accepted "
                       "form. SHAPES.md already validates M7/P7.1 that way and the manifest "
                       "now says so too.",
            "produce": "the slices expected to emit these exact bytes from their own facade.",
            "consume": "the slices expected to parse them. Empty produce is never a gap by "
                       "default: an unknown field cannot be emitted by a codec that does not "
                       "know it, and a must-fail vector cannot be emitted at all.",
            "accepted_encodings": "a vector may have MORE THAN ONE valid encoding. A reader must "
                                  "parse every one of them whatever it writes; a producer must "
                                  "write one of them and say which.",
            "projection": "what a reader must SEE. Byte identity against a manifest generated "
                          "from the same schema that reads it is a weaker oracle than it looks.",
            "expect": "`reject` means a conformant implementation must refuse the vector. The "
                      "`seen_failing` row names what was watched refusing it.",
        },
        "classes": {
            "unknown": "README section 10 item 1: fields the reader does not know.",
            "empty": "README section 10 item 2 and R6: absent and empty, where offset defects hide.",
            "shape": "README section 10 item 3: every field shape, mechanically.",
            "transcode": "README section 10 item 4: the transcode pair, both halves.",
            "chunking": "README section 10 item 5: distinct tags across a nesting level, and "
                        "enough elements to force more than one chunk.",
            "malformed": "wire a conformant parser must reject. Not one of the five items; the "
                         "five items are about what must be READ, and a corpus with no vector "
                         "that must fail has never watched a rejection work.",
            "baseline": "the payloads of ffi/schema/generated, by reference. The corpus is a "
                        "superset of them and does not copy them.",
        },
        "shape_coverage": dict((k, {"fields": sorted(universe[k])}) for k in sorted(universe)),
        "counts": {},
        "vectors": rows,
    }
    by_class = {}
    for r in manifest["vectors"].values():
        by_class[r["class"]] = by_class.get(r["class"], 0) + 1
    manifest["counts"] = dict(sorted(by_class.items()))
    manifest["counts"]["total"] = len(manifest["vectors"])
    manifest["counts"]["must_fail_seen_failing"] = seen_failing
    manifest["counts"]["disputed"] = len(disputed)
    manifest["counts"]["agreed"] = sum(1 for r in manifest["vectors"].values()
                                       if r.get("verdict") == "agreed")

    with open(os.path.join(outdir, "manifest.json"), "w") as fh:
        json.dump(manifest, fh, indent=2, sort_keys=False)
        fh.write("\n")

    if not quiet:
        print("%-12s %s" % ("runtime", manifest["validated_against"]["runtime"]))
        print("%-12s %s" % ("protoc", manifest["validated_against"]["protoc"]))
        for k, n in manifest["counts"].items():
            print("%-12s %d" % (k, n))
        print("%-12s %d shapes, no gaps" % ("coverage", len(universe)))
        for o in manifest["oracles"]["used"]:
            print("%-12s %s" % ("oracle", o["name"]))
        if disputed:
            print("%-12s %s" % ("disputed", ", ".join(sorted(disputed))))
    return manifest


UPB = "protobuf %s (upb backend)" % PBVER
PURE = "protobuf %s (pure-python backend)" % PBVER
CPP = "protobuf C++ (protoc --decode)"
CANON = "ffi/corpus/emit, the canonical form of ../schema/README.md"

# short, file-safe tags, so a disputed row's per-runtime projection has a name a
# human can read and a filesystem can hold
TAG = {UPB: "upb", PURE: "pure-python", CPP: "cpp"}


def reading_diff(readings):
    """The dotted paths on which two readings of one vector differ.

    A disputed row should say what the argument is about without a reader having
    to diff two JSON files by eye. Recursive, and it stops at the first level
    where the two sides are no longer both objects, so the answer is the deepest
    path that is still shared rather than every leaf under it.
    """
    MISSING = object()

    def walk(a, b, path, out):
        if a == b:
            return
        if isinstance(a, dict) and isinstance(b, dict):
            for k in sorted(set(a) | set(b)):
                walk(a.get(k, MISSING), b.get(k, MISSING), path + [str(k)], out)
            return
        if isinstance(a, list) and isinstance(b, list) and len(a) == len(b):
            for i, (x, y) in enumerate(zip(a, b)):
                walk(x, y, path + ["[%d]" % i], out)
            return
        out.add(".".join(path) or "(the whole message)")

    parsed = [json.loads(b) for b in readings]
    out = set()
    for i, a in enumerate(parsed):
        for b in parsed[i + 1:]:
            walk(a, b, [], out)
    return out


def reconcile(outdir, rows, vs, upb_verdict, upb_reading, acc_by_id, raw_forms, alt, cpp,
              names):
    """Turn three runtimes' readings of each vector into one manifest row.

    Where they agree the row says so and names who agreed. Where they disagree
    the row becomes DISPUTED: it carries every reading, names which runtime
    produced each, has no `projection` of its own, and is excluded from a
    consumer's pass or fail count. If two conformant runtimes read the same bytes
    differently, the corpus's job is to say so, not to pick.
    """
    disputed, seen_failing, rejected_by = [], {}, {}
    for v in vs:
        row = rows[v.id]
        a = alt["rows"].get(v.id, {})
        c = cpp.get(v.id, {})
        uv, uerr = upb_verdict[v.id]
        verdicts = {names[UPB]: uv, names[PURE]: a.get("parse", "?"), names[CPP]: c.get("parse", "?")}

        # --- the accept/reject verdict, three ways ---------------------------
        refusers = sorted(k for k, x in verdicts.items() if x == "error" or x == "reject")
        acceptors = sorted(k for k, x in verdicts.items() if x == "ok" or x == "accept")
        if v.expect == "reject":
            row["reject"]["seen_failing"] = {
                "refused_by": refusers,
                "accepted_by": acceptors,
                "raised": {names[UPB]: uerr, names[PURE]: a.get("error"), names[CPP]: c.get("error")},
            }
            row["reject"]["seen_failing"]["raised"] = dict(
                (k, x) for k, x in row["reject"]["seen_failing"]["raised"].items() if x)
            if not refusers:
                raise SystemExit(
                    "%s is declared must-fail and EVERY oracle accepted it (%s). A rejection "
                    "test that nothing rejects is a test nobody has watched work; fix the "
                    "vector or withdraw the claim." % (v.id, ", ".join(acceptors)))
            for r in refusers:
                rejected_by[r] = rejected_by.get(r, 0) + 1
            seen_failing[v.id] = True
            if acceptors:
                row["verdict"] = "disputed"
                row["dispute"] = {
                    "what": "whether the vector is refused at all",
                    "refused_by": refusers, "accepted_by": acceptors,
                    "excluded_from_pass_fail": True,
                }
                disputed.append(v.id)
            else:
                row["verdict"] = "agreed"
                row["verdict_agreed_by"] = refusers
            continue

        if not acceptors:
            raise SystemExit("%s is declared acceptable and NO oracle parsed it: %s"
                             % (v.id, verdicts))
        if refusers:
            row["verdict"] = "disputed"
            row["dispute"] = {
                "what": "whether the vector parses at all",
                "parsed_by": acceptors, "refused_by": refusers,
                "excluded_from_pass_fail": True,
            }
            disputed.append(v.id)
            continue

        # --- the reading. protobuf C++ is not asked this question; see cpp_oracle
        ur = upb_reading.get(v.id)
        if ur is None:
            continue
        readings = {}
        readings.setdefault(json.dumps(ur["projection"], sort_keys=True), []).append(names[UPB])
        if "projection" in a:
            readings.setdefault(json.dumps(a["projection"], sort_keys=True), []).append(names[PURE])

        accepted = acc_by_id[v.id]
        raw = raw_forms[v.id]
        if a.get("reencode"):
            b = bytes.fromhex(a["reencode"])
            accepted.setdefault(sha(b), {"sha256": sha(b), "bytes": len(b), "forms": []})
            if not accepted[sha(b)]["forms"]:
                accepted[sha(b)]["forms"].append("as the pure-python backend writes it")
            accepted[sha(b)].setdefault("written_by", []).append(names[PURE])
            accepted[sha(b)].setdefault("_observed", []).append(names[PURE])
            raw[sha(b)] = b

        # A form that is a re-ORDERING of another is not a third encoding of
        # interest: protobuf does not fix field order on the wire, and SHAPES.md
        # already validates M7/P7.1 that way -- by re-encoding contiguously to a
        # permutation of the same (tag, wire type, body) triples.
        keys = sorted(raw)
        perm = any(raw[x] != raw[y] and permutation(raw[x], raw[y])
                   for i, x in enumerate(keys) for y in keys[i + 1:])
        row["permutation_accepted"] = perm
        for enc in accepted.values():
            enc["written_by"] = sorted(set(enc["written_by"]))
            enc["observed_in_a_protobuf_runtime"] = bool(enc.pop("_observed", None))
        row["accepted_encodings"] = sorted(accepted.values(), key=lambda r: r["sha256"])
        row["upb_agreement"] = ur["agreement"]

        if len(readings) == 1:
            row["verdict"] = "agreed"
            row["verdict_agreed_by"] = sorted(list(readings.values())[0])
            write_projection(outdir, row, v.id, ur["projection"])
            if "superset_projection" in a and a["superset_projection"] is not None:
                pass
        else:
            row["verdict"] = "disputed"
            row["projection"] = None
            out = []
            for blob, who in sorted(readings.items(), key=lambda kv: sorted(kv[1])):
                tag = "-".join(sorted(TAG.get(w, w) for w in who))
                sub = {}
                write_projection(outdir, sub, v.id, json.loads(blob), "reading-" + tag)
                entry = {"read_by": sorted(who),
                         "projection": sub.get("projection_reading-" + tag)}
                for who_one in who:
                    src = ur if who_one == names[UPB] else a
                    if src.get("reencode"):
                        entry["reencode_sha256"] = sha(bytes.fromhex(src["reencode"]))
                        entry["reencode_bytes"] = len(src["reencode"]) // 2
                out.append(entry)
            ev = None
            if c.get("text"):
                ev = os.path.join("projections", v.id + ".reading-cpp.txt")
                with open(os.path.join(outdir, ev), "w") as fh:
                    fh.write(c["text"])
            row["dispute"] = {
                "what": "the reading: the oracles' ListFields disagree about this message",
                "differs_at": sorted(reading_diff(readings)),
                "readings": out,
                "protobuf_cpp_text_reading": ev,
                "protobuf_cpp_caveat": (
                    "text format only, and `protoc --decode` renders a map field as its "
                    "wire-level repeated MapEntry list, so it is evidence and not a vote. "
                    "The cpp slice's reflection arm is the one that can vote."),
                "excluded_from_pass_fail": True,
                "superset_projection_is": ("upb's reading; on a disputed row it inherits the "
                                           "dispute and is evidence rather than an expectation"),
            }
            disputed.append(v.id)
    return disputed, len(seen_failing), rejected_by


def write_projection(outdir, row, vid, pj, suffix=""):
    """Commit what a reader must SEE, unless it is very large.

    The only things the limit reaches are the deliberately large runs, whose
    oracle is byte identity plus the element count, and each has a small twin of
    the same shape that carries the projection. A projection of a 2048-element
    run is a third of a megabyte saying the same thing 2048 times.
    """
    blob = json.dumps(pj, indent=1, sort_keys=True) + "\n"
    key = "projection" + ("_" + suffix if suffix else "")
    if len(blob) <= PROJECTION_LIMIT:
        path = os.path.join("projections", vid + (("." + suffix) if suffix else "") + ".json")
        with open(os.path.join(outdir, path), "w") as fh:
            fh.write(blob)
        row[key] = path
    else:
        row[key] = None
        row[key + "_omitted"] = {"json_bytes": len(blob), "limit": PROJECTION_LIMIT,
                                 "semantic_oracle_is": small_twin(vid)}


def small_twin(vid):
    """The vector of the same shape whose projection IS committed."""
    return {"C-elemu-512": "C-elemu-4", "C-elemu-64": "C-elemu-4",
            "C-leaf-2048": "C-leaf-8", "C-wide-64": "C-wide-4",
            "C-mixed-100": "C-elemu-4",
            "B-P2_5": "E-half-absent", "B-P4_1": "S-TaskSummary-full",
            "B-P3_1": "S-Probe-full", "B-P2_1": "S-TaskDetailed-full",
            "B-P1_3": "E-all-absent"}.get(vid, "none: this vector has no twin")


def baselines(pools, outdir, rows, upb_verdict, upb_reading, acc_by_id, raw_forms):
    """The schema's own payloads, by REFERENCE and re-validated here.

    The corpus is a superset of `ffi/schema/generated`, and R0's rule one level
    down: it does not carry a second copy of them. What it adds is a second and
    third opinion -- the schema's manifest was validated against prost, and these
    rows say where upb, the pure backend and protobuf C++ agree with it.

    They go through the same reconciliation as every other row, which they did
    not before, and that was a defect: `B-P7_1` interleaves two repeated fields
    on purpose, so its only accepted encoding was one **no conformant encoder
    produces** -- every one of them writes each repeated field contiguously -- and
    a consumer that re-encoded correctly failed CONTRACT.md's C3. SHAPES.md
    already handles P7.1 by permutation; the manifest does now too.
    """
    gen = os.path.join(spec.SCHEMA, "generated")
    with open(os.path.join(gen, "manifest.json")) as fh:
        sm = json.load(fh)
    roots = {"P1.1": "ListResultsResponse", "P1.3": "ListResultsResponse",
             "P2.1": "ListTasksDetailedResponse", "P2.5": "ListTasksDetailedResponse",
             "P3.1": "ListProbeResponse", "P4.1": "ListTaskSummaryResponse",
             "P5.1": "UploadResultDataMessage", "P7.1": "DualResponse"}
    refs = []
    for pid, prow in sm["payloads"].items():
        if "vector" not in prow:
            continue
        with open(os.path.join(gen, prow["vector"]), "rb") as fh:
            data = fh.read()
        if sha(data) != prow["sha256"]:
            raise SystemExit("%s does not match its own hash in schema/generated/manifest.json" % pid)
        vid = "B-" + pid.replace(".", "_")
        root = roots[pid]
        m = msg_class(pools, "reader", root)()
        m.ParseFromString(data)
        re = m.SerializeToString(deterministic=True)
        rows[vid] = {
            "class": "baseline",
            "tests": "SHAPES.md payload %s, by reference" % pid,
            "why": "the corpus is a superset of schema/generated and does not copy it. The row is "
                   "here so a slice has one manifest to read, and so that every oracle's opinion "
                   "of each payload is recorded beside prost's.",
            "root": root,
            "view": "reader",
            "expect": "accept",
            "bytes": prow["bytes"],
            "sha256": prow["sha256"],
            "canonical": True,
            "produce": list(VEC.ALL),
            "consume": list(VEC.ALL),
            # every `file` in this manifest is relative to the manifest's own
            # directory, this one included: the corpus references the schema's
            # payloads rather than carrying a second copy of them.
            "file": "../../schema/generated/" + prow["vector"],
            "upb_reencodes_identically": re == data,
            "upb_reencoded_bytes": len(re),
            "meta": {"delta_bytes": len(re) - len(data)},
        }
        upb_verdict[vid] = ("accept", None)
        upb_reading[vid] = {
            "projection": project(m), "reencode": re.hex(),
            "agreement": "identical" if re == data else (
                "the same message in a different field ORDER" if permutation(data, re)
                else "the same message, encoded differently"),
        }
        acc = {sha(data): {"sha256": sha(data), "bytes": len(data),
                           "forms": ["as committed in ffi/schema/generated"],
                           "written_by": [CANON + ", validated against prost 0.14.4"],
                           "_observed": []}}
        acc.setdefault(sha(re), {"sha256": sha(re), "bytes": len(re),
                                 "forms": ["as upb writes it: " + upb_reading[vid]["agreement"]],
                                 "written_by": [], "_observed": []})
        acc[sha(re)]["written_by"].append(UPB)
        acc[sha(re)]["_observed"].append(UPB)
        acc_by_id[vid] = acc
        raw_forms[vid] = {sha(data): data, sha(re): re}
        refs.append(Ref(vid, root, "accept", VEC.ALL))
    return refs


class Ref(object):
    """What reconcile() needs of a vector. A baseline has no builder behind it."""

    def __init__(self, vid, root, expect, produce):
        self.id = vid
        self.root = root
        self.expect = expect
        self.produce = tuple(produce)


def check(outdir):
    """Regenerate into a temporary directory AT A DIFFERENT PATH and diff."""
    tmp = tempfile.mkdtemp(prefix="ffi-corpus-check-")
    try:
        generate(tmp, quiet=True)
        bad = []
        for base, _, files in os.walk(tmp):
            for f in files:
                rel = os.path.relpath(os.path.join(base, f), tmp)
                here = os.path.join(outdir, rel)
                if not os.path.exists(here):
                    bad.append("missing from the committed tree: " + rel)
                elif not filecmp.cmp(os.path.join(tmp, rel), here, shallow=False):
                    bad.append("differs: " + rel)
        for base, _, files in os.walk(outdir):
            for f in files:
                rel = os.path.relpath(os.path.join(base, f), outdir)
                if not os.path.exists(os.path.join(tmp, rel)):
                    bad.append("committed but no longer generated: " + rel)
        if bad:
            print("generated/ is not what emit/build.py writes today:")
            for b in sorted(bad)[:40]:
                print("  " + b)
            if len(bad) > 40:
                print("  ... and %d more" % (len(bad) - 40))
            return 1
        print("generated/ is exactly what emit/build.py writes today (%d files)"
              % sum(len(f) for _, _, f in os.walk(outdir)))
        return 0
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


if __name__ == "__main__":
    out = os.path.join(spec.ROOT, "generated")
    if "--check" in sys.argv:
        sys.exit(check(out))
    if "--reseal" in sys.argv:
        hdr = seal_header(out)
        rows = {}
        for f in sorted(os.listdir(os.path.join(out, "vectors"))):
            with open(os.path.join(out, "vectors", f), "rb") as fh:
                rows[f] = sha(fh.read())
        write_seal(out, rows, hdr)
        print("re-sealed %d vectors. That is a decision; say why in JOURNAL.md." % len(rows))
        sys.exit(0)
    generate(out)
