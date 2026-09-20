"""Generate the corpus, and validate every vector against an implementation that
shares no code with the writer.

    python3 emit/build.py            regenerate generated/
    python3 emit/build.py --check    fail if what is committed is not what this
                                     script writes today

The second form is the gate. `--check` regenerates into a temporary directory at
a DIFFERENT path and diffs, so a generator that bakes its own location into its
output fails here: that is the python slice's D4, found by cloning the pushed
branch and building it rather than by reading the code, and a generator is
exactly the shape of thing that repeats it.

The oracle is not this file. Every vector is parsed, re-encoded and projected by
**protobuf 7.36.2 with the upb backend**, over descriptors that `protoc` compiled
from the emitted .proto. Two independent producers agreeing is worth far more
than one producer and a hash, and this branch has the receipts: byte identity
against a manifest generated from the same schema that reads it passed a defect
that corrupted every chunk after the first.

Three things fail the build rather than being reported:

  1. an accept-vector upb will not parse, or whose re-encoding is not one of the
     forms the vector declares;
  2. a reject-vector upb ACCEPTS -- a rejection test that nothing rejects is a
     test nobody has watched work, and that lesson cost this branch twice;
  3. a field shape present in the description that no vector exercises (R1: a
     walker with no case for a shape raises, it never skips).
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
import spec                                               # noqa: E402
import vectors as VEC                                     # noqa: E402
from spec import ShapeNotCovered                          # noqa: E402

sys.path.insert(0, os.path.join(spec.SCHEMA, "emit"))
import shapes as S                                        # noqa: E402

PKG = "armonik.ffi.corpus.v1"

# A projection larger than this is omitted; see the comment where it is used.
PROJECTION_LIMIT = 32 * 1024


# ---------------------------------------------------------------- protobuf side

def compile_protos(outdir):
    """protoc -> a FileDescriptorSet for each view, then upb message classes."""
    from google.protobuf import descriptor_pb2, descriptor_pool, message_factory
    pools = {}
    for view, fname in (("reader", "corpus.proto"), ("superset", "corpus_superset.proto")):
        desc = os.path.join(outdir, view + ".desc")
        cmd = [sys.executable, "-m", "grpc_tools.protoc", "-I", outdir,
               "--descriptor_set_out=" + desc, fname]
        r = subprocess.run(cmd, capture_output=True)
        if r.returncode:
            raise SystemExit("protoc failed on %s:\n%s" % (fname, r.stderr.decode()))
        fds = descriptor_pb2.FileDescriptorSet()
        with open(desc, "rb") as fh:
            fds.ParseFromString(fh.read())
        pool = descriptor_pool.DescriptorPool()
        for f in fds.file:
            pool.Add(f)
        pools[view] = (pool, message_factory)
        os.remove(desc)
    return pools


def msg_class(pools, view, name):
    pool, factory = pools[view]
    return factory.GetMessageClass(pool.FindMessageTypeByName("%s.%s" % (PKG, name)))


def runtime_id():
    import google.protobuf
    from google.protobuf.internal import api_implementation
    out = subprocess.run([sys.executable, "-m", "grpc_tools.protoc", "--version"],
                         capture_output=True)
    return {"runtime": "protobuf %s (%s backend)" % (google.protobuf.__version__,
                                                     api_implementation.Type()),
            "protoc": out.stdout.decode().strip()}


# ------------------------------------------------------------------ projection

def project(m):
    """What a reader must SEE, in a form every language can compare against.

    Not protobuf JSON: proto3 JSON omits a default value, which erases the
    difference between absent and present-and-zero, and that difference is the
    whole of vector class E. This walks `ListFields`, whose semantics ("the
    fields that are set") are the same in every protobuf implementation.

    The encoding of each kind is fixed here and stated in CONTRACT.md:
    integers and enums are DECIMAL STRINGS (an enum value the descriptor does
    not declare has no name to use), bytes are lowercase hex, doubles are
    %.17g, and unknown fields appear under "_unknown" as a list of
    {tag, wire_type, hex}.
    """
    from google.protobuf.descriptor import FieldDescriptor as FD
    from google.protobuf.unknown_fields import UnknownFieldSet

    def val(fd, v):
        t = fd.type
        if t == FD.TYPE_MESSAGE:
            return project(v)
        if t == FD.TYPE_BYTES:
            return v.hex()
        if t == FD.TYPE_STRING:
            return v
        if t == FD.TYPE_BOOL:
            return bool(v)
        if t in (FD.TYPE_DOUBLE, FD.TYPE_FLOAT):
            return "%.17g" % v
        if t == FD.TYPE_ENUM or t in (FD.TYPE_INT32, FD.TYPE_INT64, FD.TYPE_UINT32,
                                      FD.TYPE_UINT64, FD.TYPE_SINT32, FD.TYPE_SINT64,
                                      FD.TYPE_FIXED32, FD.TYPE_FIXED64,
                                      FD.TYPE_SFIXED32, FD.TYPE_SFIXED64):
            return str(v)
        raise ShapeNotCovered("project() has no case for protobuf type %d (%s)" % (t, fd.name))

    out = {}
    for fd, v in m.ListFields():
        if fd.is_repeated and fd.message_type is not None and fd.message_type.GetOptions().map_entry:
            vfd = fd.message_type.fields_by_name["value"]
            out[fd.name] = dict((str(k), val(vfd, v[k])) for k in sorted(v))
        elif fd.is_repeated:
            out[fd.name] = [val(fd, x) for x in v]
        else:
            out[fd.name] = val(fd, v)
    u = unknown_list(UnknownFieldSet(m))
    if u:
        out["_unknown"] = u
    return out


def unknown_list(uset):
    out = []
    for f in uset:
        row = {"tag": f.field_number, "wire_type": f.wire_type}
        if f.wire_type == 3:
            row["group"] = unknown_list(f.data)
        elif f.wire_type == 2:
            row["hex"] = f.data.hex()
        else:
            row["value"] = str(f.data)
        out.append(row)
    return sorted(out, key=lambda r: (r["tag"], r["wire_type"]))


def unknown_tags_seen(m, acc=None):
    """Every unknown field number anywhere in the message, transitively."""
    from google.protobuf.unknown_fields import UnknownFieldSet
    from google.protobuf.descriptor import FieldDescriptor as FD
    acc = set() if acc is None else acc
    for f in UnknownFieldSet(m):
        acc.add(f.field_number)
    for fd, v in m.ListFields():
        if fd.type != FD.TYPE_MESSAGE:
            continue
        if fd.is_repeated and fd.message_type.GetOptions().map_entry:
            vfd = fd.message_type.fields_by_name["value"]
            if vfd.type == FD.TYPE_MESSAGE:
                for k in v:
                    unknown_tags_seen(v[k], acc)
        elif fd.is_repeated:
            for x in v:
                unknown_tags_seen(x, acc)
        else:
            unknown_tags_seen(v, acc)
    return acc


# -------------------------------------------------------------------- coverage

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
    seen_failing = 0

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

        cls = msg_class(pools, "reader", v.root)
        if v.expect == "reject":
            m = cls()
            try:
                m.ParseFromString(v.data)
            except Exception as exc:
                seen_failing += 1
                row["reject"] = dict(v.reject)
                row["reject"]["seen_failing"] = {
                    "by": "upb", "raised": type(exc).__name__}
            else:
                raise SystemExit(
                    "%s is declared must-fail and upb ACCEPTED it. A rejection test that "
                    "nothing rejects is a test nobody has watched work; fix the vector or "
                    "withdraw the claim." % v.id)
            rows[v.id] = row
            continue

        m = cls()
        try:
            m.ParseFromString(v.data)
        except Exception as exc:
            raise SystemExit("%s is declared acceptable and upb rejected it: %s: %s"
                             % (v.id, type(exc).__name__, exc))

        pj = project(m)

        # Every encoding a conformant implementation may write for this message.
        # A vector may have MORE THAN ONE, and P2.5 is the proof: an empty map
        # value is an implicit-presence leaf holding the proto zero, prost omits
        # it and protobuf C++, upb and protobuf-java write it, both parse to the
        # same message and neither encoder is wrong.
        forms = [("as committed", v.data)] + list(v.forms)
        accepted = {}
        for label, b in forms:
            accepted.setdefault(sha(b), {"sha256": sha(b), "bytes": len(b), "forms": []})
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
            if v.produce:
                raise SystemExit(
                    "%s is a PRODUCE vector, so its canonical form has to be unambiguous, and upb "
                    "writes a form the vector does not declare:\n  committed %s\n  upb       %s\n"
                    "Declare the form or withdraw the produce claim."
                    % (v.id, v.data.hex()[:160], re.hex()[:160]))
            agreement = ("the same message in a different field ORDER"
                         if permutation(v.data, re) else
                         "the same message, encoded differently")
            accepted[sha(re)] = {"sha256": sha(re), "bytes": len(re),
                                 "forms": ["as upb writes it: " + agreement]}
        accepted[sha(re)].setdefault("produced_by", []).append("upb")
        accepted[sha(v.data)].setdefault("produced_by", []).append("ffi/corpus/emit")
        row["accepted_encodings"] = sorted(accepted.values(), key=lambda r: r["sha256"])
        row["upb_agreement"] = agreement

        write_projection(outdir, row, v.id, pj)

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
        "status": ("VALIDATED. Every accept-vector was parsed, re-encoded and projected by an "
                   "implementation that shares no code with the writer, and every reject-vector "
                   "was SEEN failing by it. `python3 emit/build.py --check` fails if what is "
                   "committed is not what the generator writes today."),
        "validated_against": runtime_id(),
        "descriptors": {
            "reader": "corpus.proto",
            "superset": "corpus_superset.proto",
            "difference": ("the fields a conformant reader is NOT built against. A vector is "
                           "WRITTEN against the superset and READ against the reader."),
        },
        "reading_this_file": {
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
    manifest["vectors"].update(baselines(pools, outdir))
    by_class = {}
    for r in manifest["vectors"].values():
        by_class[r["class"]] = by_class.get(r["class"], 0) + 1
    manifest["counts"] = dict(sorted(by_class.items()))
    manifest["counts"]["total"] = len(manifest["vectors"])
    manifest["counts"]["must_fail_seen_failing"] = seen_failing

    with open(os.path.join(outdir, "manifest.json"), "w") as fh:
        json.dump(manifest, fh, indent=2, sort_keys=False)
        fh.write("\n")

    if not quiet:
        print("%-12s %s" % ("runtime", manifest["validated_against"]["runtime"]))
        print("%-12s %s" % ("protoc", manifest["validated_against"]["protoc"]))
        for k, n in manifest["counts"].items():
            print("%-12s %d" % (k, n))
        print("%-12s %d shapes, no gaps" % ("coverage", len(universe)))
    return manifest


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


def baselines(pools, outdir):
    """The schema's own payloads, by REFERENCE and re-validated here.

    The corpus is a superset of `ffi/schema/generated`, and R0's rule one level
    down: it does not carry a second copy of them. What it does add is a second
    opinion -- the schema's manifest was validated against prost, and these rows
    say upb agrees, or exactly where it does not.
    """
    gen = os.path.join(spec.SCHEMA, "generated")
    with open(os.path.join(gen, "manifest.json")) as fh:
        sm = json.load(fh)
    roots = {"P1.1": "ListResultsResponse", "P1.3": "ListResultsResponse",
             "P2.1": "ListTasksDetailedResponse", "P2.5": "ListTasksDetailedResponse",
             "P3.1": "ListProbeResponse", "P4.1": "ListTaskSummaryResponse",
             "P5.1": "UploadResultDataMessage", "P7.1": "DualResponse"}
    out = {}
    for pid, row in sm["payloads"].items():
        if "vector" not in row:
            continue
        with open(os.path.join(gen, row["vector"]), "rb") as fh:
            data = fh.read()
        if sha(data) != row["sha256"]:
            raise SystemExit("%s does not match its own hash in schema/generated/manifest.json" % pid)
        m = msg_class(pools, "reader", roots[pid])()
        m.ParseFromString(data)
        re = m.SerializeToString(deterministic=True)
        vid = "B-" + pid.replace(".", "_")
        row = {
            "class": "baseline",
            "tests": "SHAPES.md payload %s, by reference" % pid,
            "why": "the corpus is a superset of schema/generated and does not copy it. The row is "
                   "here so a slice has one manifest to read, and so that upb's opinion of each "
                   "payload is recorded beside prost's.",
            "root": roots[pid],
            "view": "reader",
            "expect": "accept",
            "bytes": row["bytes"],
            "sha256": row["sha256"],
            "canonical": True,
            "produce": list(VEC.ALL),
            "consume": list(VEC.ALL),
            # every `file` in this manifest is relative to the manifest's own
            # directory, this one included: the corpus references the schema's
            # payloads rather than carrying a second copy of them.
            "file": "../../schema/generated/" + row["vector"],
            "upb_reencodes_identically": re == data,
            "upb_reencoded_bytes": len(re),
            "meta": {"delta_bytes": len(re) - len(data)},
        }
        write_projection(outdir, row, vid, project(m))
        out[vid] = row
    return out


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
    generate(out)
