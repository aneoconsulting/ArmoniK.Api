"""Every vector in the corpus, built from the description rather than listed.

Six classes, and each one is a numbered item of README section 10 or a defect
this branch actually found:

  U  unknown fields            item 1, and the C++ slice's first attempt
  E  absent and empty          item 2, R6, and the Java slice's offset defect
  S  every field shape         item 3, mechanically, with a coverage check
  T  the transcode pair        item 4, and Java/.NET not agreeing
  C  distinct tags + chunks    item 5, and the Rust slice's element-run defect
  X  malformed, must fail      the rejections nothing had ever been seen doing
  B  the schema's payloads     by reference; the corpus is a superset of them

A vector is a `Vec`. It carries the bytes, what it tests, whether a slice is
expected to PRODUCE it, only to CONSUME it, or both, and the set of encodings a
conformant implementation may write when it re-encodes what it read. That last
field is not decoration: an empty map value is an implicit-presence leaf holding
the proto zero, prost omits it and protobuf C++, upb and protobuf-java write it,
both parse to the same message and neither encoder is wrong.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import encode as E
import rawwire as W
import spec
from encode import Opts
from spec import ShapeNotCovered

ALL = ("cpp", "csharp", "java", "python", "rust")
NONE = ()


class Vec(object):
    def __init__(self, vid, cls, root, data, tests, why="",
                 expect="accept", produce=NONE, consume=ALL, canonical=False,
                 forms=(), notes=None, meta=None, reject=None, view="reader",
                 superset_root=None, trace=None):
        self.id = vid
        self.cls = cls
        self.root = root
        self.superset_root = superset_root or root
        self.data = bytes(data)
        self.tests = tests
        self.why = why
        self.expect = expect
        self.produce = tuple(produce)
        self.consume = tuple(consume)
        self.canonical = canonical
        # [(label, bytes)] -- encodings other than the committed one that a
        # conformant implementation may write for the same message.
        self.forms = list(forms)
        self.notes = dict(notes or {})
        self.meta = dict(meta or {})
        self.reject = reject
        self.view = view
        self.trace = set(trace or ())
        if expect == "reject" and produce:
            raise ShapeNotCovered("%s: a vector that must be rejected cannot be produced" % vid)
        if produce and not canonical:
            raise ShapeNotCovered("%s: only a canonical encoding can be asked of a producer" % vid)


def build_all(reader, superset, corpus):
    out = []
    out += unknown(reader, superset, corpus)
    out += empty(reader, superset, corpus)
    out += shapes(reader, superset, corpus)
    out += transcode(reader, superset, corpus)
    out += chunking(reader, superset, corpus)
    out += malformed(reader, superset, corpus)
    out += wp4(reader, superset, corpus)
    out += wp5s6(reader, superset, corpus)
    seen = set()
    for v in out:
        if v.id in seen:
            raise ShapeNotCovered("duplicate vector id %r" % v.id)
        seen.add(v.id)
    return out


# --------------------------------------------------------------------------
# helpers: a site is a message plus the root that carries it to the wire.

SITES = {
    # site message      root                          field  n  prose
    "root":       ("ListResultsResponse",        "ListResultsResponse",       None, 1,
                   "the top level of the message that crosses the wire"),
    "nested":     ("ResultRaw",                  "ListResultsResponse",       "results", 1,
                   "inside a nested message: the element of a repeated field"),
    "deep":       ("TaskOptions",                "ListTasksDetailedResponse", "tasks", 1,
                   "three levels down, on the message that carries the map"),
    "leaf":       ("Timestamp",                  "ListResultsResponse",       "results", 1,
                   "on a two-field leaf, where a decoder is most likely to have hand-rolled it"),
    "oneof":      ("Probe",                      "ListProbeResponse",         "probes", 1,
                   "on a message that ACTUALLY HAS a oneof"),
    "element":    ("TaskDetailed",               "ListTasksDetailedResponse", "tasks", 1,
                   "on the element of the payload the control plane really moves"),
    "chunkelem":  ("ChunkElement",               "ChunkedResponse",           "items", 2,
                   "on the corpus's own non-leaf element, under a root tag of its own"),
}


def wrap(reader, corpus, site, o, idx=0):
    """Encode the root that carries `site`, with `o` applied throughout."""
    smsg, root, field, n, _ = SITES[site]
    if field is None:
        return E.enc_message(reader, corpus, root, root, idx, o)
    f = next(g for g in reader["messages"][root]["fields"] if g["name"] == field)
    body = bytearray()
    for j in range(n):
        body += W.ld(f["tag"], E.enc_message(reader, corpus, f["of"], f["of"], j, o))
    return bytes(body)


def alt(reader, corpus, site, base_kw, **changes):
    kw = dict(base_kw)
    kw.update(changes)
    return wrap(reader, corpus, site, Opts(**kw))


def has_map(reader, corpus, site):
    """Whether this site's encoding contains a map, transitively."""
    tr = set()
    wrap(reader, corpus, site, Opts(trace=tr))
    return any(k.startswith("map/") for k in tr)


# --------------------------------------------------------------------------
# U: the fields the reader does not know.

def unknown(reader, superset, corpus):
    out = []
    uf = corpus["unknown_fields"]

    # The cross product, generated rather than chosen: every unknown wire shape
    # at every site. The C++ slice's first attempt put all five at the root of a
    # message with no oneof and labelled one of them oneof-shaped, which tests
    # neither the nesting nor the oneof.
    for site in SITES:
        smsg = SITES[site][0]
        for f in uf["fields"]:
            kw = dict(unknown_sites={smsg}, unknown_only=[f["name"]])
            data = wrap(superset, corpus, site, Opts(**kw))
            forms = [("unknown-dropped", alt(superset, corpus, site, kw, unknown_sites=set()))]
            if has_map(superset, corpus, site):
                forms.append(("unknown-retained, map values always written",
                              alt(superset, corpus, site, kw, map_value="always")))
                forms.append(("unknown-dropped, map values always written",
                              alt(superset, corpus, site, kw, unknown_sites=set(), map_value="always")))
            out.append(Vec(
                "U-%s-%s" % (site, f["name"].replace("_", "-")), "unknown",
                SITES[site][1], data,
                "an unknown field of wire type %d (%s), %s" % (f["wire"], f["name"], SITES[site][4]),
                why=f.get("why", ""),
                forms=forms,
                meta={"site": smsg, "unknown_tag": f["tag"], "wire_type": f["wire"]},
            ))
        # all seven at once, which is the forward-compatibility case as it
        # actually arrives: a peer built against a later schema.
        kw = dict(unknown_sites={smsg})
        data = wrap(superset, corpus, site, Opts(**kw))
        forms = [("unknown-dropped", alt(superset, corpus, site, kw, unknown_sites=set()))]
        if has_map(superset, corpus, site):
            forms.append(("unknown-retained, map values always written",
                          alt(superset, corpus, site, kw, map_value="always")))
            forms.append(("unknown-dropped, map values always written",
                          alt(superset, corpus, site, kw, unknown_sites=set(), map_value="always")))
        out.append(Vec("U-%s-all" % site, "unknown", SITES[site][1], data,
                       "every unknown wire shape at once, %s" % SITES[site][4],
                       forms=forms, meta={"site": smsg, "unknown_tags": sorted(f["tag"] for f in uf["fields"])}))

    # Placement. Section 10 says "after the known fields"; a reader that assumes
    # ascending tag order, or that stops at the first tag it does not know, needs
    # the other two.
    for site in ("root", "nested", "oneof"):
        smsg = SITES[site][0]
        for place in ("before", "interleaved"):
            kw = dict(unknown_sites={smsg}, unknown_placement=place)
            data = wrap(superset, corpus, site, Opts(**kw))
            out.append(Vec("U-%s-%s" % (site, place), "unknown", SITES[site][1], data,
                           "unknown fields %s the known ones, %s" % (place, SITES[site][4]),
                           why="protobuf does not require a writer to emit fields in tag order, "
                               "and a reader that assumes it drops or misreads what follows.",
                           forms=[("unknown-retained, appended in tag order",
                                   alt(superset, corpus, site, kw, unknown_placement="after")),
                                  ("unknown-dropped",
                                   alt(superset, corpus, site, kw, unknown_sites=set()))],
                           meta={"site": smsg, "placement": place}))

    # The deprecated group pair. proto3 has no syntax for it, so it is built by
    # hand and has no counterpart in corpus_superset.proto; a skipper that
    # assumes every unknown field carries its own length walks off the end.
    gbody = W.s(1, "in-a-group") + W.i(2, 7) + W.group(9, W.s(1, "nested-group"))
    for site in ("root", "nested", "oneof"):
        smsg = SITES[site][0]
        inject = {("%s.__group__" % smsg): None}
        base = wrap(superset, corpus, site, Opts(unknown_sites=set()))
        data = inject_after(reader, corpus, site, W.group(uf_group_tag(corpus), gbody))
        out.append(Vec("U-%s-group" % site, "unknown", SITES[site][1], data,
                       "an unknown field of the deprecated GROUP form (wire types 3 and 4), %s"
                       % SITES[site][4],
                       why="A group carries no length: a skipper has to recurse to find its end. "
                           "proto3 cannot express one, so no schema-generated corpus contains one, "
                           "and every conformant parser still has to skip it.",
                       forms=[("unknown-dropped", base)],
                       meta={"site": smsg, "group_tag": uf_group_tag(corpus),
                             "expressible_in_proto3": False, "declared_in_superset": False},
                       notes={"all": "corpus_superset.proto cannot declare this field; the vector is "
                                     "validated under the reader view only."}))

    # The largest legal field number, as an unknown field.
    big = 536870911
    data = inject_after(reader, corpus, "root", W.i(big, 1))
    out.append(Vec("U-root-max-tag", "unknown", "ListResultsResponse", data,
                   "an unknown field at the largest legal field number (536870911), a five-byte key",
                   why="a reader that packs a tag into fewer bits than protobuf allows fails here "
                       "and nowhere else.",
                   forms=[("unknown-dropped", wrap(reader, corpus, "root", Opts()))],
                   meta={"unknown_tag": big, "declared_in_superset": False},
                   notes={"all": "corpus.proto declares 536870911 only on WireZoo, so this tag is "
                                 "unknown under BOTH views and there is no superset projection. "
                                 "S-bigtag is the same field number as a KNOWN field."}))

    # An unknown field inside a MAP ENTRY. A map entry is a message on the wire
    # and has an unknown-field skip of its own, which a decoder that hand-rolls
    # entries rather than generating them usually does not.
    kw = dict(map_entry_unknown=True)
    data = wrap(reader, corpus, "deep", Opts(**kw))
    out.append(Vec("U-map-entry", "unknown", "ListTasksDetailedResponse", data,
                   "an unknown field inside every map entry",
                   why=corpus["unknown_fields"]["map_entry"]["why"],
                   forms=[("unknown-dropped", alt(reader, corpus, "deep", kw, map_entry_unknown=False)),
                          ("unknown-retained, map values always written",
                           alt(reader, corpus, "deep", kw, map_value="always")),
                          ("unknown-dropped, map values always written",
                           alt(reader, corpus, "deep", kw, map_entry_unknown=False, map_value="always"))],
                   meta={"entry_unknown_tag": corpus["unknown_fields"]["map_entry"]["tag"]}))

    # The oneof. A parser cannot tell an unrecognised oneof tag from any other
    # unknown field, since the grouping lives only in the descriptor.
    om = uf["oneof_member"]
    cases = [
        ("alone", ["u_as_extra"],
         "the reader sees the oneof UNSET: the unknown member is just an unknown field"),
        ("after-known", ["as_int", "u_as_extra"],
         "the reader keeps as_int; the superset reader sees u_as_extra win, because the last "
         "member on the wire wins and the reader cannot know the two are in one oneof"),
        ("before-known", ["u_as_extra", "as_int"],
         "both readers see as_int; this is the case that looks like it works"),
    ]
    for name, members, prose in cases:
        kw = dict(oneof_members=members)
        data = wrap(superset, corpus, "oneof", Opts(**kw))
        out.append(Vec("U-oneof-member-%s" % name, "unknown", "ListProbeResponse", data,
                       "an unrecognised oneof member, %s" % name.replace("-", " "),
                       why=prose + ". " + om["why"],
                       forms=[("unknown-dropped",
                               wrap(reader, corpus, "oneof",
                                    Opts(oneof_members=[m for m in members if m != "u_as_extra"] or None)))]
                             if "as_int" in members else
                             [("unknown-dropped", wrap(reader, corpus, "oneof", Opts(oneof_members=[])))],
                       meta={"members_on_wire": members, "unknown_tag": om["field"]["tag"]}))

    # An unknown VALUE on a KNOWN field. The wire says exactly where to put it and
    # it DOES round-trip losslessly, which is the row the one above is easy to
    # read as covering.
    declared = set(reader["enums"]["ResultStatus"]["values"].values())
    for ev in corpus["unknown_enum_values"]["values"]:
        data = W.ld(1, W.s(1, "enum-value") + W.i(4, ev))
        prose = ("a DECLARED enum value at the two-byte varint boundary (%d)" % ev
                 if ev in declared else
                 "an enum value the reader's descriptor does not declare (%d)" % ev)
        out.append(Vec("U-enum-value-%d" % ev, "unknown", "ListResultsResponse", data,
                       prose + " on a KNOWN field",
                       why=corpus["unknown_enum_values"]["_why"],
                       canonical=True, produce=NONE,
                       meta={"enum_value": ev, "round_trips_losslessly": True},
                       notes={"all": "produce is empty only because a host cannot name a value its "
                                     "enum type does not have; a host that models an open enum "
                                     "(prost's Unknown(n), a raw int) SHOULD be able to, and if it "
                                     "can it must reproduce these bytes exactly."}))
    # and inside a packed run, where an unknown value is NOT diverted anywhere
    packed = W.ld(1, W.s(1, "packed-enum") + W.packed_varint(6, [4, 999, 1, 2147483647]))
    out.append(Vec("U-enum-value-packed", "unknown", "ListMetricsResponse",
                   W.ld(1, W.s(1, "g") + W.packed_varint(6, [4, 999, 1, 2147483647])),
                   "a packed enum run carrying values the reader's descriptor does not declare",
                   why="all three packed fields in the real schema are enums, and a packed run is "
                       "where an implementation is most likely to reject or drop an unknown value "
                       "rather than carry it.",
                   canonical=True, produce=NONE,
                   meta={"enum_values": [4, 999, 1, 2147483647]}))
    return out


def uf_group_tag(corpus):
    return 120


def inject_after(reader, corpus, site, raw):
    """Append raw bytes to the body of `site`'s message, inside its root."""
    smsg, root, field, n, _ = SITES[site]
    if field is None:
        return E.enc_message(reader, corpus, root, root, 0, Opts()) + raw
    f = next(g for g in reader["messages"][root]["fields"] if g["name"] == field)
    body = bytearray()
    for j in range(n):
        body += W.ld(f["tag"], E.enc_message(reader, corpus, f["of"], f["of"], j, Opts()) + raw)
    return bytes(body)


# --------------------------------------------------------------------------
# E: absent and empty. R6, and the shape a generator that fills every field
# cannot reach. One such defect passed all seven standard payloads in the Java
# slice; SHAPES.md's P1.3 and P2.5 exist for it and are not optional.

def empty(reader, superset, corpus):
    out = []
    LRR, LTD, LPR = "ListResultsResponse", "ListTasksDetailedResponse", "ListProbeResponse"

    out.append(Vec("E-root-empty", "empty", LRR, b"",
                   "a root message that encodes to NOTHING",
                   why="zero bytes is a legal encoding of every proto3 message. A decoder with a "
                       "minimum-length assumption, or one that reads a header before checking the "
                       "length, fails here and on no other vector.",
                   canonical=True, produce=ALL))

    out.append(Vec("E-elem-empty", "empty", LRR, W.ld(1, b""),
                   "one repeated element whose body is zero bytes",
                   why="absent and present-but-empty are different states and a by-value group "
                       "reports them identically unless it is designed not to.",
                   canonical=True, produce=ALL, meta={"elements": 1}))

    out.append(Vec("E-elems-empty-3", "empty", LRR, W.ld(1, b"") * 3,
                   "three repeated elements, each zero bytes",
                   why="an offset defect that survives one empty element does not survive three.",
                   canonical=True, produce=ALL, meta={"elements": 3}))

    kw = dict(mode="all_absent")
    out.append(Vec("E-all-absent", "empty", LRR, wrap(reader, corpus, "nested", Opts(**kw)),
                   "every string empty, every child absent, every scalar zero: SHAPES.md's P1.3 shape",
                   why="a payload generator that fills every field cannot reach any path conditioned "
                       "on emptiness, which is exactly where an offset defect hides.",
                   canonical=True, produce=ALL, meta={"mode": "all_absent"}))

    kwh = dict(mode="half_absent")
    out.append(Vec("E-half-absent", "empty", LTD,
                   wrap(reader, corpus, "element", Opts(**kwh)),
                   "half the map values emptied and half the timestamps absent: SHAPES.md's P2.5 shape",
                   why="the absent path NESTED, and the vector whose two valid encodings are the "
                       "reason this corpus carries a set of accepted forms rather than one hash.",
                   canonical=True, produce=ALL,
                   forms=[("map values always written (protobuf C++, upb, protobuf-java)",
                           alt(reader, corpus, "element", kwh, map_value="always"))],
                   meta={"mode": "half_absent"}))

    # A leaf holding the proto zero, written anyway. Legal, not canonical: the
    # accident that caught the schema directory's only defect, made deliberate.
    out.append(Vec("E-leaf-zero-written", "empty", LRR,
                   W.ld(1, W.s(1, "present-and-zero") + W.ld(5, W.i(1, 0) + W.i(2, 0))),
                   "a Timestamp whose two implicit-presence leaves are both written as zero",
                   why="present and zero is the third case. `enc_field` wrote a Timestamp's leaves "
                       "unconditionally in schema/, moved 8 of 16 payloads, and P1.3 was "
                       "byte-identical throughout because everything in it is absent. A reader must "
                       "accept this and re-encode it without the zeros.",
                   forms=[("canonical: the zero leaves omitted",
                           W.ld(1, W.s(1, "present-and-zero") + W.ld(5, b"")))]))

    out.append(Vec("E-msg-empty-present", "empty", LRR,
                   W.ld(1, W.s(1, "child-present-and-empty") + W.ld(5, b"")),
                   "a singular message field PRESENT with an empty body",
                   why="a message field is written when present, empty or not: the canonical rule "
                       "that distinguishes it from a scalar, and the one a by-value group loses.",
                   canonical=True, produce=ALL))

    out.append(Vec("E-string-empty-written", "empty", LRR,
                   W.ld(1, W.s(1, "")),
                   "an implicit-presence string written as empty",
                   why="legal, and not canonical: an implicit-presence leaf holding the proto zero "
                       "is omitted. A reader must parse it and re-encode it as nothing.",
                   forms=[("canonical: omitted", W.ld(1, b""))]))

    out.append(Vec("E-bytes-empty-written", "empty", LRR,
                   W.ld(1, W.ld(11, b"")),
                   "an implicit-presence bytes field written as empty",
                   forms=[("canonical: omitted", W.ld(1, b""))]))

    # Maps. Every one of these came out of the probe against upb rather than out
    # of a reading of the spec.
    md = "ListTasksDetailedResponse"

    def in_options(entry_bodies):
        """A TaskDetailed carrying only `options`, carrying only these entries."""
        opts = b"".join(W.ld(1, b) for b in entry_bodies)
        return W.ld(1, W.ld(10, opts))

    out.append(Vec("E-map-entry-empty", "empty", md, in_options([b""]),
                   "a map entry whose body is zero bytes: key \"\" and value \"\"",
                   why="both fields of the entry are implicit-presence leaves holding the proto "
                       "zero, so the canonical form of an empty-key empty-value entry is an empty "
                       "entry. A decoder that requires a key present rejects a legal map.",
                   canonical=True, produce=ALL,
                   forms=[("key and value always written (protobuf C++, upb, protobuf-java)",
                           in_options([W.s(1, "") + W.s(2, "")]))],
                   meta={"map_entries": 1}))

    out.append(Vec("E-map-key-only", "empty", md, in_options([W.s(1, "k00")]),
                   "a map entry with a key and NO value: the canonical form of an empty value",
                   why="SHAPES.md's P2.5 result in one entry. prost omits the empty value; "
                       "protobuf C++, upb and protobuf-java write it at +2 B. Both parse to the "
                       "same map and neither encoder is wrong, so a reader must accept both "
                       "whichever it writes.",
                   canonical=True, produce=ALL,
                   forms=[("key and value always written (protobuf C++, upb, protobuf-java)",
                           in_options([W.s(1, "k00") + W.s(2, "")]))]))

    out.append(Vec("E-map-empty-value-written", "empty", md,
                   in_options([W.s(1, "k00") + W.s(2, "")]),
                   "the same entry with the empty value WRITTEN: the other accepted form",
                   why="committed as its own vector so a slice that writes this form has something "
                       "to match byte for byte rather than a rule to apply.",
                   forms=[("canonical: the empty value omitted", in_options([W.s(1, "k00")]))]))

    out.append(Vec("E-map-value-only", "empty", md, in_options([W.s(2, "v")]),
                   "a map entry with a value and NO key: the key is \"\"",
                   why="the mirror of the row above, and the one a hand-rolled entry decoder that "
                       "reads key-then-value positionally gets wrong.",
                   canonical=True, produce=ALL,
                   forms=[("key and value always written", in_options([W.s(1, "") + W.s(2, "v")]))]))

    out.append(Vec("E-map-entry-reversed", "empty", md,
                   in_options([W.s(2, "v") + W.s(1, "k")]),
                   "a map entry with the VALUE written before the key",
                   why="a map entry is an ordinary message and protobuf does not require tag order. "
                       "A decoder that reads the two positionally silently swaps them.",
                   forms=[("canonical: ascending by tag", in_options([W.s(1, "k") + W.s(2, "v")])),
                          ("key and value always written", in_options([W.s(1, "k") + W.s(2, "v")]))]))

    out.append(Vec("E-map-dup-key", "empty", md,
                   in_options([W.s(1, "k") + W.s(2, "first"), W.s(1, "k") + W.s(2, "last")]),
                   "two map entries with the same key: the last one wins",
                   why="protobuf's rule, and the only vector where a correct reader emits FEWER "
                       "entries than it read.",
                   forms=[("canonical: one entry, the last value",
                           in_options([W.s(1, "k") + W.s(2, "last")]))],
                   meta={"map_entries_on_wire": 2, "map_entries_after_parse": 1}))

    # Packed.
    mb = "ListMetricsResponse"
    out.append(Vec("E-packed-empty", "empty", mb, W.ld(1, W.s(1, "g") + W.ld(2, b"")),
                   "a packed field present with a zero-length body",
                   why="legal, and it parses to an empty list, which is indistinguishable from "
                       "absent. A canonical re-encode drops it.",
                   forms=[("canonical: omitted", W.ld(1, W.s(1, "g")))]))

    out.append(Vec("E-packed-as-unpacked", "empty", mb,
                   W.ld(1, W.s(1, "g") + W.i(2, 11) + W.i(2, 22) + W.i(2, 33)),
                   "a packed field written as individual varints",
                   why="proto3 requires a parser to accept both forms of a packed field whatever "
                       "the descriptor says, and every generated decoder that special-cases packed "
                       "has to have the unpacked branch too. No canonical writer emits this.",
                   forms=[("canonical: packed", W.ld(1, W.s(1, "g") + W.packed_varint(2, [11, 22, 33])))],
                   meta={"elements": 3}))

    out.append(Vec("E-packed-split", "empty", mb,
                   W.ld(1, W.s(1, "g") + W.packed_varint(2, [11, 22]) + W.packed_varint(2, [33])),
                   "one packed field split across two occurrences, which CONCATENATE",
                   why="a reader that treats the second occurrence as a replacement loses the first "
                       "two elements and reports no error.",
                   forms=[("canonical: one run", W.ld(1, W.s(1, "g") + W.packed_varint(2, [11, 22, 33])))],
                   meta={"elements": 3}))

    # Oneof and explicit presence.
    out.append(Vec("E-oneof-payload-free", "empty", LPR,
                   W.ld(1, W.s(1, "probe") + W.ld(14, b"")),
                   "the payload-free oneof member: a variant selected by the presence of an empty body",
                   why="the member a by-value group does not reach, and the one a walker that keys "
                       "a variant on its payload cannot represent at all.",
                   canonical=True, produce=ALL))

    out.append(Vec("E-oneof-empty-string", "empty", LPR,
                   W.ld(1, W.s(1, "probe") + W.s(11, "")),
                   "a oneof string member set to the EMPTY string",
                   why="inside a oneof the empty string is a set member, not an absent field, so "
                       "the omit-when-zero rule must not apply. A generator that shares its scalar "
                       "path with implicit presence drops the member and changes which variant the "
                       "message carries.",
                   canonical=True, produce=ALL))

    out.append(Vec("E-oneof-last-wins", "empty", LPR,
                   W.ld(1, W.s(1, "probe") + W.i(10, 5) + W.s(11, "second")),
                   "two oneof members on the wire: the last one wins",
                   forms=[("canonical: one member", W.ld(1, W.s(1, "probe") + W.s(11, "second")))],
                   meta={"members_on_wire": 2}))

    out.append(Vec("E-explicit-absent", "empty", LPR,
                   W.ld(1, W.s(1, "probe") + W.ld(14, b"")),
                   "all three explicit-presence fields ABSENT",
                   why="paired with the two below, this is the triple a by-value group flattens: "
                       "absent, present-and-zero and present-and-empty report identically unless "
                       "the group is designed not to. Unmeasured on .NET today.",
                   canonical=True, produce=ALL))

    out.append(Vec("E-explicit-zero", "empty", LPR,
                   W.ld(1, W.s(1, "probe") + W.i(2, 0) + W.i(4, 0) + W.ld(14, b"")),
                   "explicit-presence scalars PRESENT and zero",
                   why="an explicit-presence field is written when set, zero or not. Dropping it "
                       "because it is zero is the single most common defect in a hand-written "
                       "presence path, and it is invisible to a payload that never sets one to zero.",
                   canonical=True, produce=ALL))

    out.append(Vec("E-explicit-empty-string", "empty", LPR,
                   W.ld(1, W.s(1, "probe") + W.s(3, "") + W.ld(14, b"")),
                   "an explicit-presence string PRESENT and empty",
                   canonical=True, produce=ALL))

    # The adapter site: one facade type, two wire forms, and a map that is not
    # injective. The C# finding was an adapter wrong on the failure path only.
    ts = "ListTaskSummaryResponse"
    td = "ListTasksDetailedResponse"
    for state, nested_body, plain in (
            ("ok", W.i(1, 1), None),
            ("error", W.s(2, "it failed"), "it failed"),
            ("invalid", None, None)):
        n = (W.ld(1, W.s(1, "t") + (W.ld(16, nested_body) if nested_body is not None else b"")))
        out.append(Vec("E-adapter-nested-%s" % state, "empty", td, n,
                       "the Output adapter at its NESTED site, state %s" % state,
                       why="one facade type over two wire forms. The nested form round-trips all "
                           "three states; the plain form below flattens two of them, so one must "
                           "come back wrong whatever the adapter author picks.",
                       canonical=True, produce=ALL, meta={"adapter_state": state, "site": "nested"}))
        p = W.ld(1, W.s(1, "t") + (W.s(8, plain) if plain else b""))
        out.append(Vec("E-adapter-plain-%s" % state, "empty", ts, p,
                       "the Output adapter at its PLAIN site, state %s" % state,
                       why="Ok and Invalid BOTH flatten to the empty string here. These two vectors "
                           "are byte-identical on purpose: that is the collision, on the wire.",
                       canonical=True, produce=ALL, meta={"adapter_state": state, "site": "plain"}))
    return out


# --------------------------------------------------------------------------
# S: every field shape, mechanically. Driven off the description, not off a list
# somebody wrote. The coverage table in the manifest is computed from the same
# walk and emit/build.py FAILS if a shape in the description has no vector.

def shapes(reader, superset, corpus):
    out = []

    # Every message in the reader's description, as its own root. A protobuf
    # message is a legal top-level message, so this needs no carrier and no
    # choice about which ones matter.
    for name in sorted(reader["messages"]):
        for label, kw in (("full", dict()),
                          ("alt", dict()),
                          ("min", dict(mode="all_absent"))):
            idx = {"full": 0, "alt": 5, "min": 0}[label]
            tr = set()
            o = Opts(trace=tr, **kw)
            data = E.enc_message(reader, corpus, name, name, idx, o)
            forms = []
            if any(k.startswith("map/") for k in tr):
                forms.append(("map values always written (protobuf C++, upb, protobuf-java)",
                              E.enc_message(reader, corpus, name, name, idx,
                                            Opts(map_value="always", **kw))))
            out.append(Vec("S-%s-%s" % (name, label), "shape", name, data,
                           {"full": "every field of %s set",
                            "alt": "every field of %s at a second element index, so the cycling "
                                   "rules (enum value, explicit presence, oneof member, adapter "
                                   "state) land elsewhere",
                            "min": "every field of %s absent or zero"}[label] % name,
                           why="generated by walking the description. A curated corpus covers the "
                               "shapes somebody thought of; the ones nobody thought of are the "
                               "ones a new backend gets wrong.",
                           canonical=True, produce=ALL, forms=forms,
                           trace=tr, meta={"element_index": idx}))

    # Every oneof MEMBER, one vector each. R1's second half is a rule the Rust
    # slice paid for: a field walker that excluded oneof members emitted a
    # complete-looking codec for a message whose oneof it ignored entirely, and
    # reported nothing wrong. A per-message sweep does not reach this -- it picks
    # one member and calls the oneof covered -- which is why the coverage check
    # in emit/build.py failed until these existed.
    for name in sorted(reader["messages"]):
        m = reader["messages"][name]
        for oname, members in sorted(__import__("shapes").oneofs(m).items()):
            for g in members:
                tr = set()
                data = E.enc_message(reader, corpus, name, name, 0,
                                     Opts(trace=tr, oneof_members=[g["name"]]))
                out.append(Vec("S-%s-oneof-%s" % (name, g["name"].replace("_", "-")), "shape",
                               name, data,
                               "%s with oneof %s carrying member %s (%s)"
                               % (name, oname, g["name"], g["kind"]),
                               why="one vector per MEMBER, generated by walking the oneof rather "
                                   "than by picking one. A backend that has no case for a member "
                                   "emits a codec that silently ignores it.",
                               canonical=True, produce=ALL, trace=tr,
                               meta={"oneof": oname, "member": g["name"], "member_tag": g["tag"]}))

    # Value boundaries, per wire type. These are where an encoder's width
    # arithmetic and a decoder's accumulator are wrong, and no shape-driven sweep
    # reaches them because they are values and not shapes.
    Z = "WireZoo"

    def zoo(body):
        return body

    varints = [
        ("1", 1), ("127", 127), ("128", 128),
        ("16383", 16383), ("16384", 16384),
        ("2097151", 2097151), ("2097152", 2097152),
        ("268435455", 268435455), ("268435456", 268435456),
        ("int32-max", 2147483647), ("int64-max", (1 << 63) - 1),
        ("minus-1", -1), ("int64-min", -(1 << 63)),
    ]
    for label, v in varints:
        out.append(Vec("S-varint-i64-%s" % label, "shape", Z, W.i(2, v),
                       "an int64 carrying %s" % label.replace("-", " "),
                       why="a varint's byte width steps at every seven bits, and a negative proto3 "
                           "int is ten bytes. An accumulator that shifts into 32 bits, or a width "
                           "table that stops at five, is wrong here and nowhere else.",
                       canonical=True, produce=ALL,
                       meta={"value": str(v), "varint_bytes": len(W.varint(v))}))
    for label, v in (("1", 1), ("minus-1", -1), ("max", 2147483647), ("min", -2147483648)):
        out.append(Vec("S-varint-i32-%s" % label, "shape", Z, W.i(1, v),
                       "an int32 carrying %s" % label.replace("-", " "),
                       why="a NEGATIVE proto3 int32 is sign-extended to ten bytes on the wire. An "
                           "encoder that writes five, or a decoder that reads five, disagrees with "
                           "every conformant implementation and with itself on the round trip.",
                       canonical=True, produce=ALL,
                       meta={"value": str(v), "varint_bytes": len(W.varint(v))}))

    out.append(Vec("S-bool-true", "shape", Z, W.i(3, 1), "a bool set to true",
                   canonical=True, produce=ALL))

    doubles = [("one", 1.0), ("minus-zero", -0.0), ("inf", float("inf")),
               ("minus-inf", float("-inf")), ("nan", float("nan")),
               ("denormal-min", 5e-324), ("max", 1.7976931348623157e308),
               ("epsilon", 2.220446049250313e-16)]
    for label, v in doubles:
        out.append(Vec("S-double-%s" % label, "shape", Z, W.f64(4, v),
                       "a double carrying %s" % label.replace("-", " "),
                       why="minus zero is not zero, so the omit-when-zero rule must compare bits "
                           "and not value; NaN is not equal to itself, so a comparison-based "
                           "round-trip check on it is vacuous.",
                       canonical=True, produce=ALL, meta={"value": repr(v)}))

    for label, v in (("one", 1), ("max", 0xFFFFFFFF), ("high-bit", 0x80000000)):
        out.append(Vec("S-fixed32-%s" % label, "shape", Z, W.f32(5, v),
                       "a wire-type-5 field carrying %s" % label.replace("-", " "),
                       why="shapes.json has no field of wire type 5 at all, so without these the "
                           "mechanical sweep covers four of the five wire types and calls it every "
                           "shape.",
                       canonical=True, produce=ALL, meta={"value": v}))

    # Strings: length boundaries, and the content classes that decide whether a
    # transcoder has real work to do.
    for n in (1, 127, 128, 16383, 16384):
        out.append(Vec("S-string-len-%d" % n, "shape", Z, W.s(6, "a" * n),
                       "an ASCII string of %d bytes" % n,
                       why="the length prefix steps from one varint byte to two at 128 and to "
                           "three at 16384. ABI v1 section 6 reserves a LEARNED width and moves on "
                           "a miss, so these are the lengths that make it move.",
                       canonical=True, produce=ALL,
                       meta={"chars": n, "utf8_bytes": n, "prefix_bytes": len(W.varint(n))}))

    contents = [
        ("ascii", "abcXYZ019", "every id in the real schema is an ASCII GUID"),
        ("latin1", "".join(chr(c) for c in range(0xA0, 0xB0)),
         "where a narrowing transcoder has real work to do, and where the JDK 17 "
         "Latin-1 deoptimisation hazard of R9 lives"),
        ("wide", "".join(chr(c) for c in (0x100, 0x3B1, 0x4E2D, 0xFF21)),
         "above U+00FF: where a narrowing transcoder cannot represent its input"),
        ("astral", "\U0001F600\U0001F4A9\U00010000",
         "four-byte UTF-8, and a surrogate PAIR in a UTF-16 host"),
        ("boundary-0x7f-0x80", "\u007f\u0080", "the one-to-two byte UTF-8 step"),
        ("boundary-0x7ff-0x800", "߿ࠀ", "the two-to-three byte step"),
        ("boundary-0xffff-0x10000", "￿\U00010000", "the three-to-four byte step"),
        ("max-codepoint", "\U0010FFFF", "the largest code point UTF-8 can carry"),
        ("embedded-nul", "a\x00b", "a NUL INSIDE a string: legal UTF-8 and legal proto3, and "
                                   "the byte a host that hands the codec a C string truncates at"),
    ]
    for label, text, why in contents:
        out.append(Vec("S-string-%s" % label, "shape", Z, W.s(6, text),
                       "a string of content class %s" % label,
                       why=why,
                       canonical=True, produce=ALL,
                       meta={"chars": len(text), "utf8_bytes": len(text.encode("utf-8")),
                             "utf16_units": len(text.encode("utf-16-le")) // 2,
                             "codepoints": [ord(ch) for ch in text]}))

    out.append(Vec("S-bytes-all-256", "shape", Z, W.ld(7, bytes(range(256))),
                   "a bytes field carrying every byte value 0 through 255",
                   why="bytes is not string: no validation, no transcoding, and the same content "
                       "in a string field is the reject vectors of class T.",
                   canonical=True, produce=ALL, meta={"length": 256}))

    out.append(Vec("S-bigtag", "shape", Z, W.i(536870911, 42),
                   "a KNOWN field at the largest legal field number, a five-byte key",
                   why="the reader's counterpart to U-root-max-tag: there the tag is unknown and "
                       "skipped, here it is known and must be read.",
                   canonical=True, produce=ALL, meta={"tag": 536870911}))

    # Nesting depth. SHAPES.md reaches 4 and the real schema's 6-level chains are
    # all on filter and request messages this payload set does not carry, so
    # levels 5 and 6 are unmeasured and ABI v1 open decision 7 is unexercised.
    for d in (1, 4, 6, 20):
        out.append(Vec("S-depth-%d" % d, "shape", "Nest",
                       E.enc_message(reader, corpus, "Nest", "Nest", 0, Opts(nest_depth=d)),
                       "a message nested %d levels deep" % d,
                       why="SHAPES.md's payload set reaches depth 4 and says so; the schema's real "
                           "six-level chains are all on filter and request messages it does not "
                           "carry, which leaves ABI v1 open decision 7 (the decode recursion limit) "
                           "unexercised. These vectors are the legal side of it; X-depth-* is the "
                           "other side.",
                       canonical=True, produce=ALL, meta={"depth": d}))

    # Interleaved repeated fields: legal wire that no group buffer keyed by type
    # can decode, and that no canonical writer can produce.
    inter = b"".join(W.ld(1, E.enc_message(reader, corpus, "Pair", "Pair", j, Opts()))
                     + W.ld(2, E.enc_message(reader, corpus, "Pair", "Pair", j + 100, Opts()))
                     for j in range(3))
    contig = (b"".join(W.ld(1, E.enc_message(reader, corpus, "Pair", "Pair", j, Opts())) for j in range(3))
              + b"".join(W.ld(2, E.enc_message(reader, corpus, "Pair", "Pair", j + 100, Opts())) for j in range(3)))
    out.append(Vec("S-interleaved", "shape", "DualResponse", inter,
                   "two repeated fields of the same message type, interleaved on the wire",
                   why="SHAPES.md's M7/P7.1. No canonical writer can produce it, prost included, so "
                       "no slice is asked to reproduce its bytes: it is validated by DECODING it "
                       "and re-encoding contiguously.",
                   forms=[("canonical: each repeated field contiguous", contig)],
                   meta={"elements_per_field": 3}))
    return out


# --------------------------------------------------------------------------
# T: the transcode pair. Encode transcodes in the CORE and decode transcodes in
# the HOST (ABI v1 sections 4 and 6), so the two have to agree on malformed
# input and on the unpaired-surrogate substitution across every facade. Java and
# .NET do not agree today.

def utf16_to_utf8(units, replacement=u"�"):
    """UTF-16 code units to UTF-8, substituting for every UNPAIRED surrogate.

    An unpaired surrogate is not representable in UTF-8 at all, so this is a
    conversion question rather than a validation one (ABI v1 open decision 3),
    and it is the half of the pair the core owns.
    """
    out = bytearray()
    i = 0
    while i < len(units):
        u = units[i]
        if 0xD800 <= u <= 0xDBFF and i + 1 < len(units) and 0xDC00 <= units[i + 1] <= 0xDFFF:
            out += chr(0x10000 + ((u - 0xD800) << 10) + (units[i + 1] - 0xDC00)).encode("utf-8")
            i += 2
        elif 0xD800 <= u <= 0xDFFF:
            out += replacement.encode("utf-8")
            i += 1
        else:
            out += chr(u).encode("utf-8")
            i += 1
    return bytes(out)


BAD_UTF8 = [
    ("lone-continuation", b"\x80", "a continuation byte with no lead"),
    ("lead-no-continuation", b"\xC2", "a two-byte lead at the end of the string"),
    ("truncated-3", b"\xE2\x82", "a three-byte sequence one byte short"),
    ("truncated-4", b"\xF0\x9F\x98", "a four-byte sequence one byte short"),
    ("overlong-nul", b"\xC0\x80", "U+0000 in two bytes: the encoding that lets a NUL through a "
                                  "validator that only looks at the first byte"),
    ("overlong-2", b"\xC1\xBF", "U+007F in two bytes"),
    ("overlong-3", b"\xE0\x80\x80", "U+0000 in three bytes"),
    ("overlong-4", b"\xF0\x80\x80\x80", "U+0000 in four bytes"),
    ("surrogate-d800", b"\xED\xA0\x80", "U+D800 encoded as UTF-8: the exact input the encode half "
                                        "of the pair had to substitute for"),
    ("surrogate-dfff", b"\xED\xBF\xBF", "U+DFFF encoded as UTF-8"),
    ("cesu8-pair", b"\xED\xA0\xBD\xED\xB8\x80", "a surrogate PAIR encoded as two three-byte "
                                                "sequences: CESU-8, which a JVM that round-trips "
                                                "through its own char array can emit"),
    ("above-10ffff", b"\xF4\x90\x80\x80", "U+110000, one past the largest legal code point"),
    ("f5-lead", b"\xF5\x80\x80\x80", "a lead byte no legal sequence starts with"),
    ("five-byte", b"\xFB\xBF\xBF\xBF\xBF", "a five-byte sequence: legal in the original UTF-8, "
                                           "illegal since RFC 3629"),
    ("fe-ff", b"\xFE\xFF", "the two bytes that never appear in UTF-8"),
]


def transcode(reader, superset, corpus):
    out = []
    SUR = "Surrogate"

    # --- the ENCODE half. The host holds UTF-16; the core transcodes.
    inputs = [
        ("lone-high", [0x41, 0xD800, 0x42], "a lone HIGH surrogate between two ASCII characters"),
        ("lone-low", [0x41, 0xDC00, 0x42], "a lone LOW surrogate between two ASCII characters"),
        ("reversed-pair", [0xDC00, 0xD800], "a low surrogate followed by a high one: two unpaired "
                                            "surrogates, not a pair"),
        ("high-at-end", [0x41, 0xD800], "a high surrogate at the end of the string, where a "
                                        "transcoder that looks ahead reads past its input"),
        ("two-highs", [0xD800, 0xD801, 0x41], "two high surrogates in a row"),
        ("valid-pair", [0xD83D, 0xDE00], "a VALID surrogate pair: the control. Nothing may be "
                                         "substituted here, and a transcoder that substitutes per "
                                         "code unit fails this vector and passes every other one"),
        ("pair-then-lone", [0xD83D, 0xDE00, 0xD800], "a valid pair followed by a lone surrogate"),
    ]
    for label, units, prose in inputs:
        good = utf16_to_utf8(units)
        java_today = utf16_to_utf8(units, u"?")
        sites = (W.s(1, good.decode("utf-8"))
                 + W.ld(2, W.s(1, good.decode("utf-8")))
                 + W.ld(3, W.s(1, "k") + W.s(2, good.decode("utf-8")))
                 + W.s(4, good.decode("utf-8"))
                 + W.ld(5, bytes(b for u in units for b in (u & 0xFF, u >> 8))))
        alt_sites = (W.s(1, java_today.decode("utf-8"))
                     + W.ld(2, W.s(1, java_today.decode("utf-8")))
                     + W.ld(3, W.s(1, "k") + W.s(2, java_today.decode("utf-8")))
                     + W.s(4, java_today.decode("utf-8"))
                     + W.ld(5, bytes(b for u in units for b in (u & 0xFF, u >> 8))))
        out.append(Vec("T-enc-%s" % label, "transcode", SUR, sites,
                       "ENCODE half: %s, at four string sites at once" % prose,
                       why="ABI v1 section 6 puts the converting transcoders in the core and "
                           "section 12.2 makes the pair an obligation. The input is given as UTF-16 "
                           "code units in this vector's metadata because a Rust String cannot hold "
                           "an unpaired surrogate, so no content set constructed in Rust produces "
                           "it; these bytes were produced by CPython, whose str can. Four sites at "
                           "once because a transcoder wired at only some of its call sites passes a "
                           "root-only vector.",
                       canonical=True, produce=("csharp", "java"),
                       meta={
                           "input_utf16le_hex": bytes(b for u in units for b in (u & 0xFF, u >> 8)).hex(),
                           "input_code_units": ["%04X" % u for u in units],
                           "substitution": "U+FFFD per unpaired surrogate",
                           "unpaired_surrogates": sum(
                               1 for i, u in enumerate(units)
                               if 0xD800 <= u <= 0xDFFF and utf16_to_utf8([u]) == b"\xef\xbf\xbd"),
                           "sites": ["text", "nested.text", "attrs value", "texts[0]"],
                           "bytes_field_5": "the same code units as raw UTF-16LE, which is NOT a "
                                            "string field and must pass through untouched",
                           "observed_today_protobuf_java_hex": alt_sites.hex() if alt_sites != sites else None,
                       },
                       notes={
                           "rust": "cannot PRODUCE this: a Rust String cannot hold an unpaired "
                                   "surrogate, so the input does not exist in the host's type. It "
                                   "must still CONSUME the bytes, which are ordinary UTF-8.",
                           "python": "CPython's str CAN hold a lone surrogate, so a python host "
                                     "could reach the encode half through ak_tc_ucs4. README "
                                     "section 10 item 4 names only C# and Java, so this corpus does "
                                     "not require it; see corpus/STATE.md.",
                           "cpp": "a C++ host holding std::string is already UTF-8 and has no "
                                  "unpaired surrogate to transcode; one holding std::u16string "
                                  "does, and no slice does today.",
                           "java": "protobuf-java is recorded in design/ABI-v1.md section 6 as "
                                   "writing '?' rather than U+FFFD. That divergence is a migration "
                                   "note, not a defect, and observed_today_protobuf_java_hex is "
                                   "what the INCUMBENT arm is expected to produce. The corpus does "
                                   "not verify that claim; the Java slice is the only thing that "
                                   "can.",
                       }))

    # --- the DECODE half. The wire holds bytes; the host transcodes, and proto3
    # requires a parser to validate. Every one of these MUST be rejected.
    def at_root(bad):
        return W.s(1, "") [:0] + W.key(1, W.LEN) + W.varint(len(bad)) + bad

    def at_nested(bad):
        return W.ld(2, W.key(1, W.LEN) + W.varint(len(bad)) + bad)

    def at_map_key(bad):
        return W.ld(3, W.key(1, W.LEN) + W.varint(len(bad)) + bad + W.s(2, "v"))

    def at_map_value(bad):
        return W.ld(3, W.s(1, "k") + W.key(2, W.LEN) + W.varint(len(bad)) + bad)

    def at_repeated(bad):
        return W.s(4, "ok") + W.key(4, W.LEN) + W.varint(len(bad)) + bad

    def in_bytes(bad):
        return W.ld(5, bad)

    for label, bad, prose in BAD_UTF8:
        out.append(Vec("T-dec-root-%s" % label, "transcode", SUR, at_root(bad),
                       "DECODE half: %s, in a string field at the root" % prose,
                       why="proto3 requires a parser to validate a string field. Google.Protobuf "
                           "and protobuf-java both throw, prost returns an error, and ABI v1 open "
                           "decision 3 measured rejecting as CHEAPER than substituting, because a "
                           "lossy conversion already scans to decide what to replace. So the "
                           "guarantee is free and the weaker one was never cheaper.",
                       expect="reject",
                       reject={"reason": "invalid UTF-8 in a string field",
                               "bytes_hex": bad.hex()},
                       meta={"site": "root", "bad_utf8_hex": bad.hex()}))

    for site, fn, where in (("nested", at_nested, "in a string field inside a nested message"),
                            ("map-key", at_map_key, "in a map KEY"),
                            ("map-value", at_map_value, "in a map VALUE"),
                            ("repeated", at_repeated, "in the SECOND element of a repeated string "
                                                      "field, after a valid one")):
        for label in ("lone-continuation", "surrogate-d800", "overlong-nul", "above-10ffff"):
            bad = dict((l, b) for l, b, _ in BAD_UTF8)[label]
            out.append(Vec("T-dec-%s-%s" % (site, label), "transcode", SUR, fn(bad),
                           "DECODE half: invalid UTF-8 (%s) %s" % (label, where),
                           why="a validator wired at the root only passes T-dec-root-* and fails "
                               "here, which is the same defect shape as a transcoder wired at some "
                               "of its call sites.",
                           expect="reject",
                           reject={"reason": "invalid UTF-8 in a string field",
                                   "bytes_hex": bad.hex()},
                           meta={"site": site, "bad_utf8_hex": bad.hex()}))

    # The control: the same bytes in a BYTES field, which must NOT be rejected.
    for label, bad, _ in BAD_UTF8:
        out.append(Vec("T-bytes-%s" % label, "transcode", SUR, in_bytes(bad),
                       "the same invalid UTF-8 (%s) in a BYTES field, which must be ACCEPTED" % label,
                       why="a rejection test that rejects everything is a rejection test nobody has "
                           "watched work. These are the vectors that fail if a validator is wired "
                           "to the wrong field kind.",
                       canonical=True, produce=ALL,
                       meta={"bad_utf8_hex": bad.hex()}))
    return out


# --------------------------------------------------------------------------
# C: distinct tags across a nesting level, and enough elements to force more
# than one chunk. README section 10 item 5, and the defect byte identity could
# not catch: an element run read the open field's tag from the context at entry,
# so a chunking host wrote every chunk after the first under the INNER field's
# tag. Byte identity passed, because the outer repeated field and the inner map
# field were both tag 1.

def chunking(reader, superset, corpus):
    out = []
    chunk_bytes = corpus["element_counts"]["chunk_bytes"]

    def group_note(n):
        """The host-side group size at which `n` elements cross one 32 KB chunk.

        ABI v1 section 6 chunks element runs at 32 KB of host-side element
        GROUPS, not of wire bytes, so the chunk count is a function of the host's
        group and cannot be computed here. What can be stated is the threshold.
        """
        return {
            "elements": n,
            "chunk_bytes": chunk_bytes,
            "min_group_bytes_for_two_chunks": chunk_bytes // n + 1 if n else None,
            "chunks_if_group_is": dict((str(g), -(-n * g // chunk_bytes)) for g in (32, 64, 128, 256)),
        }

    def resp(root, field_tag, elem, n, o=None):
        o = o or Opts()
        return b"".join(W.ld(field_tag, E.enc_message(reader, corpus, elem, elem, j, o))
                        for j in range(n))

    cases = [
        ("elemu", "ChunkedResponse", 7, "ChunkElement", (4, 64, 512),
         "NON-LEAF elements, so a batching host uses the unrestricted element-run entry point "
         "(ak_elemu_X) -- the one the defect was in"),
        ("leaf", "LeafResponse", 9, "LeafElement", (8, 2048),
         "LEAF elements, so a batching host uses ak_elem_X, the default form, which is a different "
         "entry point with the same hazard"),
    ]
    for label, root, tag, elem, counts, prose in cases:
        for n in counts:
            tr = set()
            data = b"".join(W.ld(tag, E.enc_message(reader, corpus, elem, elem, j, Opts(trace=tr)))
                            for j in range(n))
            forms = []
            if any(k.startswith("map/") for k in tr):
                forms.append(("map values always written",
                              b"".join(W.ld(tag, E.enc_message(reader, corpus, elem, elem, j,
                                                               Opts(map_value="always")))
                                       for j in range(n))))
            meta = group_note(n)
            meta.update({"root_field_tag": tag, "element": elem,
                         "inner_repeated_and_map_tags": inner_tags(reader, elem),
                         "inner_tags_any_kind": inner_tags(reader, elem, every=True),
                         "root_key_bytes": len(W.key(tag, W.LEN))})
            out.append(Vec("C-%s-%d" % (label, n), "chunking", root, data,
                           "%d elements under a root repeated field at tag %d; %s" % (n, tag, prose),
                           why="The root's repeated tag is %d and no repeated or map field inside "
                               "the element is, transitively, so a chunk written under the inner "
                               "field's tag is a DIFFERENT byte sequence. In SHAPES.md the outer "
                               "repeated field and the inner map field are both tag 1 and the wrong "
                               "tag was the right tag, which is why no slice built against that "
                               "payload set can catch this class of defect in any language."
                               % tag,
                           canonical=True, produce=ALL, forms=forms, trace=tr, meta=meta))

    # The same shape with a three-byte field key, so a wrong tag also changes the
    # LENGTH of every chunk after the first.
    tag = 70000
    for n in (4, 64):
        data = b"".join(W.ld(tag, E.enc_message(reader, corpus, "ChunkElement", "ChunkElement", j, Opts()))
                        for j in range(n))
        meta = group_note(n)
        meta.update({"root_field_tag": tag, "element": "ChunkElement",
                     "inner_repeated_and_map_tags": inner_tags(reader, "ChunkElement"),
                     "inner_tags_any_kind": inner_tags(reader, "ChunkElement", every=True),
                     "root_key_bytes": len(W.key(tag, W.LEN))})
        out.append(Vec("C-wide-%d" % n, "chunking", "ChunkedResponseWide", data,
                       "%d elements under a root repeated field at tag %d, a three-byte key" % (n, tag),
                       why="a wrong tag that happens to have the same key WIDTH keeps every "
                           "downstream length prefix correct, so only the key bytes differ. Here it "
                           "also changes the length of every chunk after the first, which is a "
                           "second and coarser way for the same defect to show.",
                       canonical=True, produce=ALL,
                       forms=[("map values always written",
                               b"".join(W.ld(tag, E.enc_message(reader, corpus, "ChunkElement",
                                                                "ChunkElement", j,
                                                                Opts(map_value="always")))
                                        for j in range(n)))],
                       meta=meta))

    # One element in a hundred that is EMPTY, so a run cannot amortise its state
    # over uniform elements. The learned length-prefix width of ABI v1 section 6
    # is per site and warm; this is the run that makes it cold in the middle.
    n = 100
    body = []
    for j in range(n):
        o = Opts(mode="all_absent") if j % 10 == 4 else Opts()
        body.append(W.ld(7, E.enc_message(reader, corpus, "ChunkElement", "ChunkElement", j, o)))
    meta = group_note(n)
    meta.update({"root_field_tag": 7, "element": "ChunkElement", "empty_elements": n // 10,
                 "inner_repeated_and_map_tags": inner_tags(reader, "ChunkElement")})
    out.append(Vec("C-mixed-100", "chunking", "ChunkedResponse", b"".join(body),
                   "100 elements under tag 7, every tenth one encoding to NOTHING",
                   why="a run whose elements are not uniform. The length-prefix width ABI v1 "
                       "section 6 learns per site is warm on a uniform payload and misses here, and "
                       "an element run that carries state across elements has to survive an element "
                       "that writes no bytes at all.",
                   canonical=True, produce=ALL, meta=meta))
    return out


def inner_tags(reader, elem, every=False):
    """Every repeated, packed or map tag inside `elem`, transitively.

    With `every`, every tag of any kind: the tag a corrupted run writes under is
    whichever field the element body last opened, which on a LEAF element is a
    singular one and not a repeated one.
    """
    seen, tags = set(), set()

    def walk(name):
        if name in seen:
            return
        seen.add(name)
        for f in reader["messages"][name]["fields"]:
            if every or spec.card(f) in ("repeated", "packed", "map"):
                tags.add(f["tag"])
            if f["kind"] == "message":
                walk(f["of"])
    walk(elem)
    return sorted(tags)


# --------------------------------------------------------------------------
# X: wire that a conformant parser MUST reject. Every one of these is SEEN
# failing by emit/build.py against an implementation that shares no code with the
# writer -- a rejection test that nothing rejects is a test nobody has watched
# work, and that lesson cost this branch twice.

def malformed(reader, superset, corpus):
    out = []
    Z, LRR = "WireZoo", "ListResultsResponse"

    def X(vid, root, data, tests, reason, why="", meta=None):
        out.append(Vec(vid, "malformed", root, data, tests, why=why, expect="reject",
                       reject={"reason": reason}, meta=meta or {}))

    X("X-tag-zero", Z, W.key(0, W.VARINT) + W.varint(1),
      "field number 0, which protobuf does not allow",
      "field number 0 is not a legal tag",
      "zero is the value a reader gets from an empty buffer it forgot to bounds-check, so accepting "
      "it turns a truncation into a silently empty message.")

    for wt in (6, 7):
        X("X-wire-type-%d" % wt, Z, W.key(9, wt),
          "wire type %d, which does not exist" % wt,
          "wire type %d is not defined" % wt,
          "the wire type is three bits and only six of the eight values are defined. A dispatch "
          "table sized to eight entries accepts these; one sized to six raises.")

    X("X-varint-key-truncated", Z, b"\x80",
      "a field key that is a varint with the continuation bit set and no next byte",
      "the key varint runs off the end of the buffer")

    X("X-varint-value-truncated", Z, W.key(2, W.VARINT) + b"\x80\x80",
      "a varint field value that runs off the end of the buffer",
      "the value varint runs off the end of the buffer")

    X("X-varint-11-bytes", Z, W.key(2, W.VARINT) + b"\x80" * 10 + b"\x01",
      "a varint eleven bytes long: one more than a 64-bit value can need",
      "a varint longer than ten bytes",
      "a decoder with no length bound here loops on adversarial input, and one that silently takes "
      "the low 64 bits disagrees with every conformant parser about the value.")

    X("X-len-truncated", Z, W.key(6, W.LEN) + W.varint(10) + b"abc",
      "a length-delimited field whose declared length overruns the buffer",
      "the declared length overruns the buffer",
      "the single most common way a framing defect shows, and the one emit/check.py in schema/ was "
      "written to catch on the canonical payloads.")

    X("X-len-prefix-truncated", Z, W.key(6, W.LEN) + b"\x80",
      "a length prefix that is itself truncated",
      "the length varint runs off the end of the buffer")

    X("X-len-huge", Z, W.key(6, W.LEN) + W.varint((1 << 31) - 1),
      "a length prefix of 2147483647 with no body",
      "the declared length overruns the buffer",
      "a decoder that allocates from the declared length before checking it against the buffer "
      "turns a four-byte message into a two-gigabyte allocation.")

    X("X-nested-len-overrun", LRR,
      W.key(1, W.LEN) + W.varint(12) + (W.key(1, W.LEN) + W.varint(40) + b"short"),
      "a nested message whose inner length overruns its OUTER body, while the outer length is right",
      "an inner length overruns its enclosing message",
      "the outer frame is well-formed, so a decoder that bounds-checks against the whole buffer "
      "instead of against the enclosing message accepts this and reads a neighbouring field's bytes.")

    X("X-fixed64-truncated", Z, W.key(4, W.I64) + b"\x01\x02\x03",
      "a wire-type-1 field with three bytes instead of eight",
      "a fixed64 field runs off the end of the buffer")

    X("X-fixed32-truncated", Z, W.key(5, W.I32) + b"\x01\x02",
      "a wire-type-5 field with two bytes instead of four",
      "a fixed32 field runs off the end of the buffer")

    X("X-packed-truncated", "ListMetricsResponse",
      W.ld(1, W.key(2, W.LEN) + W.varint(4) + b"\x01\x80\x80\x80"),
      "a packed varint run whose last element is truncated inside the run's own length",
      "a packed element runs off the end of its run",
      "the run's length is correct and the buffer is not overrun, so only a decoder that bounds "
      "each ELEMENT against the run rejects this. One that trusts the run length reads the next "
      "field's key as a continuation byte.")

    X("X-group-unterminated", Z, W.key(120, W.SGROUP) + W.s(1, "inside"),
      "a group that is never closed",
      "an unterminated group",
      "a group carries no length, so a skipper has to recurse to find its end. One that scans for "
      "the next end tag without checking the buffer walks off it.")

    X("X-group-mismatched-end", Z, W.key(120, W.SGROUP) + W.s(1, "inside") + W.key(121, W.EGROUP),
      "a group closed with a different field number than it opened with",
      "a group end tag whose field number does not match its start",
      "a skipper that counts depth rather than matching field numbers accepts this, and then "
      "mis-nests every group after it.")

    X("X-trailing-byte", Z, W.i(1, 1) + b"\x00",
      "a message with one trailing byte that is not a field key",
      "field number 0 is not a legal tag",
      "trailing garbage after a well-formed message is what a length that is one byte too long "
      "produces, and 0x00 is the byte a zeroed buffer supplies.")

    # The recursion limit. ABI v1 open decision 7 -- Rust holds the reader and
    # recurses in Rust -- and nothing in SHAPES.md exercises it, because the
    # payload set reaches depth 4.
    for d in (101, 300):
        body = b""
        for _ in range(d):
            body = W.key(1, W.LEN) + W.varint(len(body)) + body
        X("X-depth-%d" % d, "Nest", body,
          "a message nested %d levels deep" % d,
          "the decode recursion limit",
          "protobuf's own implementations cap recursion at 100 levels and REJECT beyond it. ABI v1 "
          "open decision 7 has no answer and no slice exercises it, because SHAPES.md's payload set "
          "reaches depth 4. A decoder that recurses without a limit does not fail this vector: it "
          "exhausts its stack, which on a native core is a crash in the host's process.",
          meta={"depth": d, "protobuf_default_limit": 100})
    return out


# --------------------------------------------------------------------------
# FIX-PLAN WP4 item 2: vectors that would have caught defects a review found and
# the corpus did not cover. Appended after every earlier class so that no
# earlier vector's bytes can move (generated/vectors.sha256 would refuse it);
# each one lands in the class whose obligation it tests.
#
#   X-lenwrap-*    a length that wraps 2^64 from its own position   (R-D1)
#   U-wire-*       a known field number at a foreign wire type       (R-E2)
#   X-tag-zero-*   field number 0, on every message the corpus roots (R-E4, R-E5)
#   S-mzero-*      -0.0 in the repeated double the schema has        (R-E3)
#   S-neg-*        negative int32/int64, projected, on SHAPES roots  (R-E5)

TWO64 = 1 << 64


def wp4(reader, superset, corpus):
    out = []
    out += lenwrap(reader, corpus)
    out += wrongwire(reader, superset, corpus)
    out += tagzero(reader, corpus)
    out += minuszero(reader, corpus)
    out += negints(reader, corpus)
    return out


def wrap_arith(data, meta):
    """Check a length-wrap vector against its own claim, from the bytes.

    Reads the length varint that ends at `body_offset_abs`, and asserts it is
    the declared length and that it wraps (or, for mode `under`, sits exactly
    one below wrapping) from the position the vector says it is counted from.
    Used by the build on every X-lenwrap row and by emit/selftest.py on a row
    whose claim is false, so the guard is watched refusing something.
    """
    end = meta["body_offset_abs"]
    start = end - 10
    n, j = 0, start
    for sh in range(0, 70, 7):
        c = data[j]
        n |= (c & 0x7F) << sh
        j += 1
        if not c & 0x80:
            break
    if j != end:
        raise ShapeNotCovered("length varint does not end at body_offset_abs %d" % end)
    if str(n) != meta["declared_length"]:
        raise ShapeNotCovered("declared_length %s but the bytes say %d" % (meta["declared_length"], n))
    base = meta["counted_from_offset"]
    total = base + n
    if meta["mode"] == "under":
        if total != TWO64 - 1:
            raise ShapeNotCovered("mode under: %d + %d is not 2^64 - 1" % (base, n))
    elif total < TWO64:
        raise ShapeNotCovered("%d + %d does not wrap 2^64" % (base, n))
    if str(total % TWO64) != meta["sum_mod_2_64"]:
        raise ShapeNotCovered("sum_mod_2_64 %s but the arithmetic says %d"
                              % (meta["sum_mod_2_64"], total % TWO64))
    return True


def lenwrap(reader, corpus):
    out = []
    modes = corpus["length_wrap"]["modes"]
    LRR, Z = "ListResultsResponse", "WireZoo"
    rr_full = E.enc_message(reader, corpus, "ResultRaw", "ResultRaw", 0, Opts())

    # (site id, root, prose, field tag, what the field is, prefix in its frame,
    #  outer frame tag or None)
    sites = [
        ("lrr-unknown", LRR, "an UNKNOWN field (tag 15) at the root, first on the wire",
         15, "unknown", b"", None),
        ("lrr-unknown-after", LRR, "an UNKNOWN field (tag 15) at the root, after page and total",
         15, "unknown", W.i(2, 7) + W.i(3, 42), None),
        ("lrr-results", LRR, "the KNOWN repeated message field results (tag 1), first on the wire",
         1, "message", b"", None),
        ("lrr-results-second", LRR, "the KNOWN message field results (tag 1), after one valid element",
         1, "message", W.ld(1, rr_full), None),
        ("zoo-string", Z, "the KNOWN string field v_string (tag 6), first on the wire",
         6, "string", b"", None),
        ("zoo-string-after", Z, "the KNOWN string field v_string (tag 6), after a one-byte and a "
                                "ten-byte varint field",
         6, "string", W.i(1, 1) + W.i(2, -1), None),
        ("zoo-msg", Z, "the KNOWN message field v_msg (tag 9), first on the wire",
         9, "message", b"", None),
        ("sur-string", "Surrogate", "the KNOWN string field text (tag 1) of the transcode carrier",
         1, "string", b"", None),
        ("wide-items", "ChunkedResponseWide", "the KNOWN message field items at tag 70000, a "
                                              "three-byte key",
         70000, "message", b"", None),
        ("lrr-nested-string", LRR, "the KNOWN string field session_id (tag 1) inside results[0]",
         1, "string", b"", 1),
        ("lrr-nested-msg", LRR, "the KNOWN message field created_at (tag 5) inside results[0]",
         5, "message", b"", 1),
        ("lrr-nested-unknown", LRR, "an UNKNOWN field (tag 100) inside results[0]",
         100, "unknown", b"", 1),
    ]

    for sid, root, prose, tag, what, prefix, outer in sites:
        key = W.key(tag, W.LEN)
        pos_rel = len(prefix) + len(key) + 10
        frame_abs = 0
        if outer is not None:
            inner_len = pos_rel
            frame_abs = len(W.key(outer, W.LEN)) + len(W.varint(inner_len))
        pos_abs = frame_abs + pos_rel
        counted = [("", pos_rel, len(prefix))] if outer is None else \
            [("-rel", pos_rel, len(prefix)), ("-abs", pos_abs, frame_abs + len(prefix))]
        for suffix, base, keyoff in counted:
            for mode in ("under", "zero", "one", "max", "start"):
                if mode == "start" and keyoff == 0:
                    continue                          # identical to `zero`
                if mode == "max" and suffix == "-abs":
                    continue                          # 2^64 - 1 is the same bytes either way
                n = {"under": TWO64 - base - 1, "zero": TWO64 - base, "one": TWO64 - base + 1,
                     "max": TWO64 - 1, "start": TWO64 - base + keyoff}[mode]
                lenv = W.varint(n)
                if len(lenv) != 10:
                    raise ShapeNotCovered("%s: a wrapping length must be a ten-byte varint" % sid)
                inner = prefix + key + lenv
                data = inner if outer is None else W.ld(outer, inner)
                if outer is not None and len(W.varint(len(inner))) != frame_abs - len(W.key(outer, W.LEN)):
                    raise ShapeNotCovered("%s: outer length width moved" % sid)
                if mode == "max":
                    frame = "its own frame and the buffer alike"
                elif suffix == "-abs":
                    frame = "the start of the BUFFER (one reader over the whole input)"
                elif suffix == "-rel":
                    frame = "the start of the ENCLOSING MESSAGE (a sub-reader per message)"
                else:
                    frame = "the start of the buffer, which is also the frame"
                meta = {
                    "site": sid, "field_tag": tag, "field_is": what, "mode": mode,
                    "declared_length": str(n),
                    "body_offset_abs": pos_abs, "body_offset_in_frame": pos_rel,
                    "counted_from": ("buffer" if suffix != "-rel" else "enclosing message"),
                    "counted_from_offset": base,
                    "sum_mod_2_64": str((base + n) % TWO64),
                    "mode_means": modes[mode],
                    "register": "R-D1",
                }
                wrap_arith(data, meta)
                vid = "X-lenwrap-%s%s-%s" % (sid, suffix, mode)
                out.append(Vec(
                    vid, "malformed", root, data,
                    "%s, whose declared length is %s: counted from %s, pos + length %s"
                    % (prose, {"under": "2^64 - pos - 1", "zero": "2^64 - pos",
                               "one": "2^64 - pos + 1", "max": "2^64 - 1",
                               "start": "2^64 - pos + (offset of its own key)"}[mode], frame,
                       "= 2^64 - 1 and does not wrap" if mode == "under"
                       else "wraps to %d" % ((base + n) % TWO64)),
                    expect="reject",
                    reject={"reason": "the declared length overruns the buffer (pos + length "
                                      "reaches or wraps 2^64)"},
                    why="R-D1: `pos + n > len` in unsigned 64-bit arithmetic passes when the sum "
                        "wraps, and the decoder then jumps backwards (a skipper loops forever), "
                        "hands the host a span that ends before it starts, or panics slicing it. "
                        "The corpus had only X-len-huge (2^31 - 1), which wraps nothing on a "
                        "64-bit usize. The check that cannot wrap is `n > len - pos`.",
                    meta=meta))
    return out


def wrongwire(reader, superset, corpus):
    """A known field number at every wire type its kind does not use.

    Mechanical, like the shape sweep: every message the corpus roots, the first
    field of each SHAPE it has (E.shape_key: kind, cardinality, presence,
    oneof), and every wire type among 0, 1, 2, 5 that the field's kind cannot
    arrive as. A packed repeated scalar legally arrives as its element's wire
    type too (unpacked), so that one is not foreign.
    """
    out = []
    ww = corpus["wrong_wire_type"]
    payload = {0: W.varint(150), 1: bytes(range(1, 9)), 2: W.varint(3) + b"\x08\x96\x01",
               5: bytes(range(1, 5))}
    for name in sorted(reader["messages"]):
        m = reader["messages"][name]
        picked = {}
        for f in __import__("shapes").fields(m):
            picked.setdefault(E.shape_key(f), f)
        if not picked:
            continue
        tr = set()
        full = E.enc_message(reader, corpus, name, name, 0, Opts(trace=tr))
        maps = any(k.startswith("map/") for k in tr)
        for skey in sorted(picked, key=lambda k: picked[k]["tag"]):
            f = picked[skey]
            legal = {spec.wire_of(f)}
            if spec.card(f) == "packed":
                legal.add(spec.KINDS[f["kind"]])
            for wt in sorted({0, 1, 2, 5} - legal):
                foreign = W.key(f["tag"], wt) + payload[wt]
                data = full + foreign
                forms = [("unknown-dropped: the known field as it was, the foreign-typed one gone",
                          full)]
                if maps:
                    always = E.enc_message(reader, corpus, name, name, 0, Opts(map_value="always"))
                    forms.append(("unknown-retained, map values always written", always + foreign))
                    forms.append(("unknown-dropped, map values always written", always))
                out.append(Vec(
                    "U-wire-%s-%s-as-wt%d" % (name, f["name"].replace("_", "-"), wt),
                    "unknown", name, data,
                    "%s.%s (%s, wire type %s) arriving with wire type %d after the full message"
                    % (name, f["name"], skey, "/".join(str(x) for x in sorted(legal)), wt),
                    why=ww["_why"],
                    forms=forms,
                    meta={"wrong_wire_type": {"field": f["name"], "tag": f["tag"],
                                              "shape": skey, "declared_wire": sorted(legal),
                                              "sent_wire": wt,
                                              "payload": ww["payloads"][str(wt)]},
                          "declared_in_superset": False,
                          "register": "R-E2"},
                    notes={"all": "the known field keeps the value the full message gave it; the "
                                  "foreign-typed occurrence is an unknown field. It stays unknown "
                                  "under the superset view too, so there is no superset "
                                  "projection."}))
    return out


def tagzero(reader, corpus):
    out = []
    for name in sorted(reader["messages"]):
        full = E.enc_message(reader, corpus, name, name, 0, Opts())
        out.append(Vec(
            "X-tag-zero-%s" % name, "malformed", name, full + W.key(0, W.VARINT) + W.varint(1),
            "field number 0 after the full %s" % name,
            expect="reject",
            reject={"reason": "field number 0 is not a legal tag"},
            why="X-tag-zero covered WireZoo only, and a generated decoder is one function per "
                "message, so each one has its own chance to treat key 0 as the end of the message "
                "or to dispatch it as a field. Placed after a well-formed message so that a "
                "decoder which stops at key 0 returns a complete-looking message instead of "
                "failing (R-E4, R-E5).",
            meta={"register": "R-E4, R-E5", "prefix_bytes": len(full)}))
    # The same inside a field-less message reached through a carrier. Added
    # after the first build showed upb accepting X-tag-zero-Empty: upb skips
    # field number 0 as an unknown field on a message with no fields (wire types
    # 0 and 5), where it refuses it on every other message. The pure-python
    # backend and protobuf C++ refuse both. This row puts that code path where a
    # real payload would reach it: the payload-free oneof member.
    out.append(Vec(
        "X-tag-zero-nested-Empty", "malformed", "ListProbeResponse",
        W.ld(1, W.s(1, "p") + W.ld(14, W.key(0, W.VARINT) + W.varint(1))),
        "field number 0 inside the payload-free oneof member Probe.as_nothing (an Empty), "
        "inside ListProbeResponse.probes[0]",
        expect="reject",
        reject={"reason": "field number 0 is not a legal tag"},
        why="X-tag-zero-Empty's nested twin. A decoder for a message with no fields has nothing "
            "to dispatch and is the one most likely to skip every key as unknown without "
            "looking at it; upb does exactly that.",
        meta={"register": "R-E4, R-E5", "empty_message_path": "probes[0].as_nothing"}))
    return out


def minuszero(reader, corpus):
    out = []
    MZ = -0.0
    why = corpus["minus_zero"]["_why"]
    cases = [
        ("MetricsBatch-values", "MetricsBatch", W.packed_f64(3, [MZ]),
         "a packed repeated double holding one -0.0"),
        ("MetricsBatch-values-mixed", "MetricsBatch", W.packed_f64(3, [0.0, MZ, 1.0, MZ]),
         "a packed repeated double holding 0.0, -0.0, 1.0, -0.0"),
        ("ListMetricsResponse-values", "ListMetricsResponse",
         W.ld(1, W.s(1, "g") + W.packed_f64(3, [MZ, MZ])),
         "-0.0 twice in the packed doubles of a nested batch"),
    ]
    for label, root, data, prose in cases:
        out.append(Vec("S-mzero-%s" % label, "shape", root, data, prose,
                       why=why + " The projection of -0.0 is \"-0\", so a reader that loses the "
                                 "sign fails C2 as well as C3.",
                       canonical=True, produce=ALL, meta={"register": "R-E3"}))
    unpacked = W.f64(3, MZ)
    out.append(Vec("S-mzero-MetricsBatch-values-unpacked", "shape", "MetricsBatch", unpacked,
                   "one -0.0 in a repeated double written UNPACKED (wire type 1)",
                   why=why + " Consume-only: the canonical re-encoding is packed.",
                   forms=[("canonical: packed", W.packed_f64(3, [MZ]))],
                   meta={"register": "R-E3"}))
    return out


def negints(reader, corpus):
    out = []
    why = corpus["negative_ints"]["_why"]
    I32MIN, I64MIN = -(1 << 31), -(1 << 63)

    def ts(sec, nanos):
        return (W.i(1, sec) if sec else b"") + (W.i(2, nanos) if nanos else b"")

    canon = [
        ("Timestamp", "Timestamp", ts(-1, -1), "seconds = -1 (int64), nanos = -1 (int32)"),
        ("Timestamp-min", "Timestamp", ts(I64MIN, I32MIN),
         "seconds = INT64_MIN, nanos = INT32_MIN"),
        ("Duration", "Duration", ts(-2, -999999999), "seconds = -2, nanos = -999999999"),
        ("ListResultsResponse", "ListResultsResponse",
         W.ld(1, W.ld(5, ts(-1, -1)) + W.i(9, -1)) + W.i(2, -1) + W.i(3, I32MIN),
         "page = -1 and total = INT32_MIN at the root, results[0].size = -1 (int64) and "
         "results[0].created_at = {-1, -1}"),
        ("ListTasksDetailedResponse", "ListTasksDetailedResponse",
         W.ld(1, W.ld(10, W.i(3, -3) + W.i(4, I32MIN))) + W.i(2, -1),
         "page = -1, tasks[0].options.max_retries = -3 and priority = INT32_MIN (three levels "
         "down)"),
        ("ListTaskSummaryResponse", "ListTaskSummaryResponse",
         W.ld(1, W.ld(3, W.i(3, -1)) + W.i(11, -1)),
         "tasks[0].options.max_retries = -1 (int32), tasks[0].count_data_dependencies = -1 "
         "(int64)"),
        ("ListMetricsResponse-packed", "ListMetricsResponse",
         W.ld(1, W.s(1, "g") + W.packed_varint(2, [-1, I64MIN, 1, -(1 << 32)])
              + W.packed_varint(4, [-1, I32MIN, 7, -128])),
         "packed int64 ticks [-1, INT64_MIN, 1, -2^32] and packed int32 codes "
         "[-1, INT32_MIN, 7, -128]"),
        ("ListProbeResponse", "ListProbeResponse",
         W.ld(1, W.s(1, "p") + W.i(2, -1) + W.i(10, I64MIN)),
         "an explicit-presence int32 opt_count = -1 and the oneof member as_int = INT64_MIN"),
        ("ChunkedResponse", "ChunkedResponse",
         W.ld(7, W.ld(4, W.packed_varint(1, [-1]) + W.ld(2, W.s(1, "a") + W.i(2, -1)))) + W.i(8, -7),
         "items[0].inner.marks = [-1] (packed int64), items[0].inner.leaves[0].v = -1 (int32), "
         "page = -7"),
        ("LeafResponse", "LeafResponse", W.ld(9, W.s(1, "x") + W.i(2, I64MIN)),
         "items[0].n = INT64_MIN"),
        ("DualResponse", "DualResponse",
         W.ld(1, W.s(1, "a") + W.i(2, I32MIN)) + W.ld(2, W.s(1, "b") + W.i(2, -1)),
         "left[0].value = INT32_MIN, right[0].value = -1"),
    ]
    for label, root, data, prose in canon:
        out.append(Vec("S-neg-%s" % label, "shape", root, data, "negative integers: " + prose,
                       why=why, canonical=True, produce=ALL, meta={"register": "R-E5"}))

    # Two legal wire forms of a negative int32 that no conformant writer emits.
    # protobuf's language guide: a number parsed from the wire that does not fit
    # the field's type gets the effect of a C++ cast to that type, so an int32
    # reads the low 32 bits of whatever varint arrives.
    five = [
        ("i32-five-byte-Timestamp", "Timestamp", W.key(2, W.VARINT) + b"\xff\xff\xff\xff\x0f",
         ts(0, -1), "nanos = -1 written as the five-byte varint 0xFFFFFFFF"),
        ("i32-five-byte-ListResultsResponse", "ListResultsResponse",
         W.key(2, W.VARINT) + b"\x80\x80\x80\x80\x08", W.i(2, I32MIN),
         "page = INT32_MIN written as the five-byte varint 0x80000000"),
        ("i32-truncated-Pair", "Pair", W.s(1, "k") + W.i(2, (1 << 40) + (1 << 32) - 1),
         W.s(1, "k") + W.i(2, -1),
         "value carrying the varint 2^40 + 2^32 - 1, whose low 32 bits are -1"),
    ]
    for label, root, data, canonical_form, prose in five:
        out.append(Vec("S-neg-%s" % label, "shape", root, data, "negative int32 decode: " + prose,
                       why=why + " Consume-only: an int32 reads the low 32 bits of the varint and "
                                 "sign-extends from bit 31, so a decoder that keeps the 64-bit "
                                 "value reads a large positive number here. The canonical "
                                 "re-encoding is the ten-byte sign-extended form.",
                       forms=[("canonical: ten-byte sign-extended", canonical_form)],
                       meta={"register": "R-E5"}))
    return out


# --------------------------------------------------------------------------
# FIX-PLAN WP5 step 6: rules the shared plan (poc/codec/gen/plan.py) now states,
# proposed by the rust slice (poc/rust/gen/probe_corpus.py, rows P-*) and built
# here through the corpus's own generator and oracles. Appended after wp4 so no
# earlier vector's bytes can move.
#
#   X-field-*        field numbers above 2^29 - 1, refused
#   U-group-field-max  2^29 - 1 inside a skipped group, accepted (the control)
#   S-varint10-*     a tenth varint byte carrying bits beyond 64, discarded
#   S-map-order-*    map entries in ascending UTF-8 key order

FIELD_MAX = (1 << 29) - 1


def wp5s6(reader, superset, corpus):
    out = []
    Z = "WireZoo"
    why_f = corpus["field_number_limit"]["_why"]
    over = [
        ("X-field-over-max", W.varint(((FIELD_MAX + 1) << 3) | W.VARINT) + W.varint(1),
         "field number 2^29, one above the largest legal field number, at the root"),
        ("X-field-2p32-plus-2", W.varint((((1 << 32) + 2) << 3) | W.VARINT) + W.varint(5),
         "field number 2^32 + 2, which a decoder keeping the field number in 32 bits reads as "
         "field 2 (v_int64 = 5)"),
        ("X-field-over-max-in-group",
         W.key(100, W.SGROUP) + W.varint(((FIELD_MAX + 1) << 3) | W.VARINT) + W.varint(1)
         + W.key(100, W.EGROUP),
         "field number 2^29 inside an unknown group the reader is only skipping"),
    ]
    for vid, data, prose in over:
        out.append(Vec(vid, "malformed", Z, data, prose, expect="reject",
                       reject={"reason": "field number above 2^29 - 1"},
                       why=why_f, meta={"source": "poc/rust/gen/probe_corpus.py", "plan": "WP5 step 6"}))

    gbody = W.i(FIELD_MAX, 1)
    out.append(Vec("U-group-field-max", "unknown", Z,
                   W.key(100, W.SGROUP) + gbody + W.key(100, W.EGROUP),
                   "field number 2^29 - 1 inside an unknown group the reader is only skipping",
                   why=why_f + " The control for X-field-over-max-in-group: the same group, one "
                               "field number lower, must be skipped without complaint.",
                   forms=[("unknown-dropped", b"")],
                   meta={"group_tag": 100, "declared_in_superset": False,
                         "source": "poc/rust/gen/probe_corpus.py", "plan": "WP5 step 6"},
                   notes={"all": "WireZoo is not an unknown-field site, so tag 100 is unknown "
                                 "under both views and there is no superset projection."}))

    why_v = corpus["varint_tenth_byte"]["_why"]
    FF = b"\xff"
    ten = [
        ("bit64", Z, W.key(2, W.VARINT) + FF * 9 + b"\x02", W.i(2, (1 << 63) - 1),
         "v_int64 whose tenth byte is 0x02: bit 64 set and discarded, bit 63 clear, so 2^63 - 1"),
        ("7f", Z, W.key(2, W.VARINT) + FF * 9 + b"\x7f", W.i(2, -1),
         "v_int64 whose tenth byte is 0x7f: bits 64 to 69 discarded, so -1"),
        ("over-zero", Z, W.key(2, W.VARINT) + b"\x80" * 9 + b"\x7e", b"",
         "v_int64 whose only set bits are beyond 64: the value is 0, which an implicit-presence "
         "field does not keep"),
        ("int32", Z, W.key(1, W.VARINT) + FF * 9 + b"\x02", W.i(1, -1),
         "v_int32 carrying the ten-byte varint with bit 64 set: the low 32 bits are -1"),
        ("bit64-Timestamp", "Timestamp", W.key(1, W.VARINT) + FF * 9 + b"\x02", W.i(1, (1 << 63) - 1),
         "Timestamp.seconds with the same tenth byte 0x02, on a root every slice implements"),
    ]
    for label, root, data, canon, prose in ten:
        out.append(Vec("S-varint10-%s" % label, "shape", root, data, prose,
                       why=why_v + " Consume-only; the canonical re-encoding is declared.",
                       forms=[("canonical", canon)],
                       meta={"source": "poc/rust/gen/probe_corpus.py", "plan": "WP5 step 6"}))

    why_m = corpus["map_entry_order"]["_why"]
    k1, k2 = u"\ue000", u"\U00010000"

    def entry(k, v):
        return W.ld(1, W.s(1, k) + W.s(2, v))
    srt = entry(k1, "a") + entry(k2, "b")
    rev = entry(k2, "b") + entry(k1, "a")
    out.append(Vec("S-map-order-utf8", "shape", "TaskOptions", srt,
                   "TaskOptions.options with keys U+E000 and U+10000, in UTF-8 byte order",
                   why=why_m, canonical=True, produce=ALL,
                   meta={"keys_utf8_hex": [k1.encode("utf-8").hex(), k2.encode("utf-8").hex()],
                         "keys_utf16be_hex": [k1.encode("utf-16-be").hex(), k2.encode("utf-16-be").hex()],
                         "plan": "WP5 step 6"}))
    out.append(Vec("S-map-order-reversed", "shape", "TaskOptions", rev,
                   "the same two entries in the reverse (UTF-16) order on the wire",
                   why=why_m + " Consume-only: a reader must take either order, and re-encode in "
                               "UTF-8 key order.",
                   forms=[("canonical: entries ascending by UTF-8 key", srt)],
                   meta={"plan": "WP5 step 6"}))
    return out
