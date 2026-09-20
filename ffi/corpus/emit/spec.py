"""The merged description the corpus generates from, and the two .proto views of it.

`../schema/shapes.json` is the base and is read, never written. `../corpus.json`
adds the shapes SHAPES.md deliberately does not have and the fields a reader is
not built against. Two views come out of the merge:

  reader    -- what a conformant reader is built against. corpus.proto.
  superset  -- reader plus the unknown fields. corpus_superset.proto.

A vector is bytes written against the superset and read against the reader. That
is the only way a corpus can execute the unknown-field skip, and it is why this
directory exists rather than another payload in `schema/`.

Paths are resolved from this file's own location and never baked into anything
generated. The python slice's D4 was an absolute path in a generated module, so a
clone at a different path could not rebuild its own tree; a generator is exactly
the shape of thing that repeats it.
"""
import copy
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)                       # ffi/corpus
FFI = os.path.dirname(ROOT)                        # ffi
SCHEMA = os.path.join(FFI, "schema")

sys.path.insert(0, os.path.join(SCHEMA, "emit"))
import shapes as S                                 # noqa: E402  the schema's own accessors
import values as V                                 # noqa: E402  the schema's own value rules

# Wire types. 3 and 4 are the deprecated group pair: proto3 has no syntax for
# them, so the group vectors are built by hand and have no superset counterpart.
VARINT, I64, LEN, SGROUP, EGROUP, I32 = 0, 1, 2, 3, 4, 5

# Every field kind this corpus knows. A walker with no case for one RAISES (R1):
# the Rust slice's field walker silently excluded oneof members and emitted a
# complete-looking codec for a message whose oneof it ignored entirely.
KINDS = {
    "int32": VARINT, "int64": VARINT, "bool": VARINT, "enum": VARINT,
    "double": I64, "fixed32": I32,
    "string": LEN, "bytes": LEN, "message": LEN, "map": LEN,
}

PROTO_KIND = {"int32": "int32", "int64": "int64", "bool": "bool",
              "double": "double", "fixed32": "fixed32",
              "string": "string", "bytes": "bytes"}


class ShapeNotCovered(Exception):
    """Raised where a walker has no case. Never caught inside the generator."""


def wire_of(f):
    c = card(f)
    if c in ("packed", "map"):
        return LEN
    k = f["kind"]
    if k not in KINDS:
        raise ShapeNotCovered("no wire type for kind %r (field %r)" % (k, f.get("name")))
    return KINDS[k]


def card(f):
    if f["kind"] == "map":
        return "map"
    return f.get("card", "singular")


def load():
    """(reader, superset, corpus) -- two merged schemas and the corpus delta."""
    base = S.load(os.path.join(SCHEMA, "shapes.json"))
    with open(os.path.join(ROOT, "corpus.json")) as fh:
        c = json.load(fh)

    reader = copy.deepcopy(base)
    reader["package"] = c["package"]
    for name, m in c["messages"].items():
        if name in reader["messages"]:
            raise ShapeNotCovered("corpus.json redefines %r, which shapes.json owns" % name)
        reader["messages"][name] = copy.deepcopy(m)

    superset = copy.deepcopy(reader)
    uf = c["unknown_fields"]
    for target in uf["sites"]:
        if target not in superset["messages"]:
            raise ShapeNotCovered("unknown-field site %r is not a message" % target)
        for f in uf["fields"]:
            superset["messages"][target]["fields"].append(copy.deepcopy(f))
    om = uf["oneof_member"]
    g = copy.deepcopy(om["field"])
    g["oneof"] = om["oneof"]
    superset["messages"][om["message"]]["fields"].append(g)

    check_tags(reader, "reader")
    check_tags(superset, "superset")
    return reader, superset, c


def check_tags(schema, label):
    for name, m in schema["messages"].items():
        seen = {}
        for f in S.fields(m):
            t = f["tag"]
            if not 1 <= t <= 536870911:
                raise ShapeNotCovered("%s.%s tag %d is out of range" % (name, f["name"], t))
            if 19000 <= t <= 19999:
                raise ShapeNotCovered("%s.%s tag %d is in protobuf's reserved range" % (name, f["name"], t))
            if t in seen:
                raise ShapeNotCovered("%s: tag %d on both %s and %s (%s)"
                                      % (name, t, seen[t], f["name"], label))
            seen[t] = f["name"]
            wire_of(f)          # raises on a kind with no case, at load time


def unknown_tags(corpus, msg=None):
    """The tags that are unknown to the reader -- PER MESSAGE when `msg` is given.

    A flat, schema-wide set is wrong and was wrong here: the unknown oneof member
    is tag 15 and `TaskDetailed.pod_ttl` is also tag 15, so a global filter
    dropped a REAL field out of every TaskDetailed vector in the corpus. Caught
    by emit/selftest.py asking whether any unknown tag is declared at its own
    site in the reader view, which is the question a textual grep of the .proto
    cannot answer.
    """
    uf = corpus["unknown_fields"]
    om = uf["oneof_member"]
    if msg is None:
        return {f["tag"] for f in uf["fields"]} | {om["field"]["tag"]}
    out = set()
    if msg in uf["sites"]:
        out |= {f["tag"] for f in uf["fields"]}
    if msg == om["message"]:
        out.add(om["field"]["tag"])
    return out


def known_fields(corpus, name, msg):
    """The fields of `msg` as the READER sees them, msg taken from the superset."""
    u = unknown_tags(corpus, name)
    return [f for f in S.fields(msg) if f["tag"] not in u]


# ---------------------------------------------------------------- .proto views

def field_line(f, indent="  "):
    if f["kind"] == "map":
        return "%smap<%s, %s> %s = %d;" % (indent, f["key"], f["value_kind"], f["name"], f["tag"])
    t = PROTO_KIND.get(f["kind"])
    if t is None:
        if f["kind"] not in ("message", "enum"):
            raise ShapeNotCovered("no proto type for kind %r" % f["kind"])
        t = f["of"]
    prefix = ""
    if card(f) in ("repeated", "packed"):
        prefix = "repeated "
    elif f.get("presence") == "explicit":
        prefix = "optional "
    return "%s%s%s %s = %d;" % (indent, prefix, t, f["name"], f["tag"])


def emit_proto(schema, banner):
    out = ['syntax = "proto3";', "", "package %s;" % schema["package"], ""]
    out += ["// GENERATED by ffi/corpus/emit/spec.py. Do not edit.", "//"]
    out += ["// " + line for line in banner.strip().splitlines()]
    out += [""]
    for name, e in schema["enums"].items():
        out.append("enum %s {" % name)
        for k, v in e["values"].items():
            out.append("  %s = %d;" % (k, v))
        out += ["}", ""]
    for name, m in schema["messages"].items():
        if m.get("source"):
            out.append("// %s" % m["source"])
        out.append("message %s {" % name)
        ofs = S.oneofs(m)
        done = set()
        for f in S.fields(m):
            o = f.get("oneof")
            if o:
                if o in done:
                    continue
                done.add(o)
                out.append("  oneof %s {" % o)
                for gf in ofs[o]:
                    out.append(field_line(gf, "    "))
                out.append("  }")
            else:
                out.append(field_line(f))
        out += ["}", ""]
    return "\n".join(out)


READER_BANNER = """
The READER view: the schema a conformant reader of this corpus is built against.
Every vector is READ against this file. Tags 100 and up do not appear here on
purpose -- they are the fields the reader does not know, and a vector carrying
one exercises the unknown-field skip, which is the whole of protobuf's forward
compatibility and which a corpus generated from the schema that reads it never
executes.
"""

SUPERSET_BANNER = """
The SUPERSET view: the reader's schema plus the fields it was not built against.
Every vector is WRITTEN against this file. The difference between this file and
corpus.proto is the corpus's whole unknown-field claim, and it is checked
mechanically: emit/build.py parses every vector under both and asserts that the
extra fields are unknown under one and known under the other.

The deprecated group wire types (3 and 4) have no proto3 syntax, so the group
vectors are built by hand and validated under the reader view only.
"""
