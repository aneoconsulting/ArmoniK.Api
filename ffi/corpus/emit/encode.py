"""The descriptor-driven encoder the vectors are generated from.

Driven off the merged description, not off a list somebody wrote: README section
10 item 3 asks for every field shape "mechanically", because a curated corpus
covers the shapes somebody thought of and the ones nobody thought of are the ones
a new backend gets wrong.

R1 applies to this file as hard as to a slice's generator: **a walker with no
case for a shape raises, it never skips.** The Rust slice's field walker silently
excluded oneof members and emitted a complete-looking codec for a message whose
oneof it ignored entirely, and the byte oracle is what caught it. Every `raise
ShapeNotCovered` below is that rule, and `emit/selftest.py` feeds this walker a
shape it has no case for and asserts it raises, because a guard with no failing
test is a guard nobody has seen work.

The canonical form is `../../schema/README.md`'s, unchanged:
  fields ascending by tag; an implicit-presence leaf holding the proto zero is
  omitted; an explicit-presence field is written when set, zero or not; a message
  field is written when present, empty or not; repeated scalars are packed; map
  entries are sorted by key.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import rawwire as W
import spec
from spec import ShapeNotCovered, card
sys.path.insert(0, os.path.join(spec.SCHEMA, "emit"))
import shapes as S
import values as V


class Opts(object):
    """What a vector wants that is not in the description.

    Everything here exists because some vector needs it; nothing is speculative.
    """

    def __init__(self, **kw):
        # which messages get the unknown fields written into them, by name
        self.unknown_sites = frozenset(kw.pop("unknown_sites", ()))
        # which unknown fields, by name. None means all of them.
        self.unknown_only = kw.pop("unknown_only", None)
        # 'after' (the normal forward-compatibility case), 'before' (a reader that
        # assumes ascending tag order), 'interleaved' (a reader that stops at the
        # first tag it does not know)
        self.unknown_placement = kw.pop("unknown_placement", "after")
        # an unknown tag INSIDE every map entry. A map entry is a message on the
        # wire and has an unknown-field skip of its own.
        self.map_entry_unknown = kw.pop("map_entry_unknown", False)
        # 'canonical' omits an empty map value (prost's rule, the schema's
        # manifest); 'always' writes key and value unconditionally, which is what
        # protobuf C++, upb and protobuf-java do. Both are valid proto3 and
        # neither encoder is wrong: SHAPES.md's P2.5 result, generalised.
        self.map_value = kw.pop("map_value", "canonical")
        # None | 'all_absent' | 'half_absent', as shapes.json's mode_note
        self.mode = kw.pop("mode", None)
        # which oneof members to write, in wire order. Several is legal and the
        # last one wins; that is how an unknown oneof member is tested.
        self.oneof_members = kw.pop("oneof_members", None)
        # override a field's value by "Message.field" path
        self.override = dict(kw.pop("override", {}))
        # how many elements a repeated field with no declared count carries
        self.repeats = kw.pop("repeats", 2)
        # a set the encoder drops a shape key into for every field it actually
        # WRITES. README section 10 item 3 asks for every field shape
        # mechanically, and the mechanical form of that claim is a coverage table
        # computed from the description and checked against this trace, so a
        # shape with no vector fails the build instead of being noticed later.
        self.trace = kw.pop("trace", None)
        # How deep a singular message field is followed. A VALUE rule, not a
        # shape rule: `Nest` is self-recursive so there is no natural bottom, and
        # 6 is SHAPES.md's maximum static nesting depth for the real schema.
        # Below the budget the walker still has a case for every shape; at the
        # budget it omits the child, which is a legal encoding of an absent
        # message and not a skipped shape.
        self.nest_depth = kw.pop("nest_depth", 6)
        if kw:
            raise ShapeNotCovered("unknown encoder option(s): %s" % sorted(kw))


DEFAULT = Opts()


# ------------------------------------------------------------------- values

def fixed32_value(path, idx):
    return V.h64(path, idx) % (1 << 32)


def text_value(f, path, idx):
    rule = f.get("value", "word")
    fn = {"guid": V.guid, "word": V.word, "sentence": V.sentence}.get(rule)
    if fn is None:
        raise ShapeNotCovered("no value rule %r for %s" % (rule, path))
    return fn(path, idx)


def scalar_value(kind, path, idx):
    if kind == "fixed32":
        return fixed32_value(path, idx)
    if kind in ("int32", "int64", "bool", "double"):
        return V.scalar(kind, path, idx)
    raise ShapeNotCovered("no value rule for scalar kind %r at %s" % (kind, path))


def stamp_body(t):
    """A Timestamp or Duration body. Both leaves are implicit presence, so a leaf
    holding the proto zero is omitted -- the rule the schema directory's only
    defect broke at all three of its call sites, which is why it is one helper
    there and one helper here."""
    return ((W.i(1, t["seconds"]) if t["seconds"] else b"")
            + (W.i(2, t["nanos"]) if t["nanos"] else b""))


# ------------------------------------------------------------------- fields

def absent(f, o, idx):
    if o.mode == "all_absent":
        return True
    if o.mode == "half_absent":
        if f["kind"] == "message" and f.get("of") in ("Timestamp", "Duration") and f["tag"] % 2 == 0:
            return True
    return False


def enc_scalar(kind, tag, value):
    if kind in ("int32", "int64", "bool", "enum"):
        return W.i(tag, int(value))
    if kind == "double":
        return W.f64(tag, value)
    if kind == "fixed32":
        return W.f32(tag, int(value))
    raise ShapeNotCovered("enc_scalar has no case for kind %r" % kind)


def shape_key(f):
    """What makes two fields the same SHAPE to a code generator."""
    return "%s/%s%s%s" % (f["kind"], card(f),
                          "/explicit" if f.get("presence") == "explicit" else "",
                          "/oneof" if "oneof" in f else "")


def enc_field(schema, corpus, f, path, idx, o, depth=0):
    out = _enc_field(schema, corpus, f, path, idx, o, depth)
    if out and o.trace is not None:
        o.trace.add(shape_key(f))
    return out


def _enc_field(schema, corpus, f, path, idx, o, depth=0):
    """One field of one message. Returns b"" where the canonical form omits it."""
    kind, c = f["kind"], card(f)
    if absent(f, o, idx):
        return b""
    if path in o.override:
        return o.override[path](f, path, idx)

    if c == "map":
        if f["key"] != "string" or f["value_kind"] != "string":
            raise ShapeNotCovered("only map<string, string> has a case (%s)" % path)
        n = f.get("entries", 2)
        entries = []
        for k in range(n):
            mk = "k%02d" % k
            empty = (o.mode == "half_absent" and k % 2 == 0) or o.mode == "all_absent"
            mv = "" if empty else V.word(path + ".value", idx * 31 + k)
            entries.append((mk, mv))
        entries.sort(key=lambda kv: kv[0])          # map entries are sorted by key
        out = bytearray()
        for mk, mv in entries:
            body = W.s(1, mk)
            if mv or o.map_value == "always":
                body += W.s(2, mv)
            if o.map_entry_unknown:
                body += W.i(corpus["unknown_fields"]["map_entry"]["tag"], 7)
            out += W.ld(f["tag"], body)
        return bytes(out)

    if c == "packed":
        n = f.get("count", 4)
        if o.mode == "all_absent":
            return b""
        if kind == "enum":
            vals = [V.enum_value(schema["enums"][f["of"]]["values"], idx * 97 + j) for j in range(n)]
            return W.packed_varint(f["tag"], vals)
        if kind in ("int32", "int64", "bool"):
            return W.packed_varint(f["tag"], [int(V.scalar(kind, path, idx * 97 + j)) for j in range(n)])
        if kind == "double":
            return W.packed_f64(f["tag"], [V.scalar(kind, path, idx * 97 + j) for j in range(n)])
        if kind == "fixed32":
            return b"".join([W.key(f["tag"], W.LEN),
                             W.varint(4 * n)] + [W.f32(0, fixed32_value(path, idx * 97 + j))[1:] for j in range(n)])
        raise ShapeNotCovered("packed has no case for kind %r at %s" % (kind, path))

    if c == "repeated":
        if f["kind"] == "message" and depth + 1 > o.nest_depth:
            return b""
        n = f.get("count", o.repeats)
        if o.mode == "all_absent":
            return b""
        out = bytearray()
        for j in range(n):
            if kind == "string":
                out += W.s(f["tag"], text_value(f, path, idx * 211 + j))
            elif kind == "bytes":
                out += W.ld(f["tag"], V.blob(path, idx * 211 + j))
            elif kind == "message":
                out += W.ld(f["tag"], enc_message(schema, corpus, f["of"], path, j, o, depth + 1))
            elif kind == "enum":
                out += W.i(f["tag"], V.enum_value(schema["enums"][f["of"]]["values"], idx * 211 + j))
            elif kind in ("int32", "int64", "bool", "double", "fixed32"):
                out += enc_scalar(kind, f["tag"], scalar_value(kind, path, idx * 211 + j))
            else:
                raise ShapeNotCovered("repeated has no case for kind %r at %s" % (kind, path))
        return bytes(out)

    if c != "singular":
        raise ShapeNotCovered("no case for cardinality %r at %s" % (c, path))

    explicit = f.get("presence") == "explicit"

    if kind == "string":
        if o.mode == "all_absent" and not explicit:
            return b""
        return W.s(f["tag"], "" if o.mode == "all_absent" else text_value(f, path, idx))
    if kind == "bytes":
        if o.mode == "all_absent":
            return b""
        return W.ld(f["tag"], V.blob(path, idx))
    if kind == "enum":
        v = 0 if o.mode == "all_absent" else V.enum_value(schema["enums"][f["of"]]["values"], idx)
        return W.i(f["tag"], v) if (v or explicit) else b""
    if kind in ("int32", "int64", "bool", "double", "fixed32"):
        v = 0 if o.mode == "all_absent" else scalar_value(kind, path, idx)
        if explicit:
            return enc_scalar(kind, f["tag"], v)
        return enc_scalar(kind, f["tag"], v) if v else b""
    if kind == "message":
        if o.mode == "all_absent":
            return b""
        if depth + 1 > o.nest_depth:
            return b""
        return W.ld(f["tag"], enc_message(schema, corpus, f["of"], path, idx, o, depth + 1))

    raise ShapeNotCovered("enc_field has no case for kind %r at %s" % (kind, path))


# ----------------------------------------------------------------- messages

def enc_unknown(schema, corpus, name, path, idx, o, depth=0):
    """The fields the reader was not built against, for one message.

    Written only where the vector asked for them, so that a vector can put an
    unknown field at the root ALONE, inside a nested message ALONE, or at a tag
    on a message that actually has a oneof -- which is the distinction the C++
    slice's first attempt collapsed by putting all five at the root of a message
    with no oneof.
    """
    if name not in o.unknown_sites:
        return b""
    out = bytearray()
    for f in corpus["unknown_fields"]["fields"]:
        if o.unknown_only is not None and f["name"] not in o.unknown_only:
            continue
        out += enc_field(schema, corpus, f, "%s.%s" % (path, f["name"]), idx, o, depth)
    return bytes(out)


def enc_message(schema, corpus, name, path, idx, o=DEFAULT, depth=0):
    """The encoded BODY of one message, canonical except where `o` says otherwise."""
    m = schema["messages"].get(name)
    if m is None:
        raise ShapeNotCovered("no message %r in the description" % name)
    utags = spec.unknown_tags(corpus, name)       # PER MESSAGE: see spec.unknown_tags
    known = [f for f in S.fields(m) if f["tag"] not in utags and "oneof" not in f]
    ofs = S.oneofs(m)

    body = bytearray()
    if name in ("Timestamp", "Duration"):
        # A VALUE rule, not a shortcut. The earlier spelling returned the body
        # from enc_field and never re-entered enc_message, so an unknown field
        # targeted at a two-field leaf was silently written nowhere and the
        # vector tested nothing -- the same shape of defect as a field walker
        # that excludes oneof members, in the generator that is supposed to
        # catch it. Found by the unknown-tag assertion in emit/build.py.
        body += stamp_body(V.timestamp(path, idx) if name == "Timestamp" else V.duration(path, idx))
        if o.trace is not None:
            for f in known:
                o.trace.add(shape_key(f))
    else:
        for f in known:
            body += enc_field(schema, corpus, f, "%s.%s" % (path, f["name"]), idx, o, depth)

    # The oneof. A walker that has no case for one emits a complete-looking codec
    # for a message whose oneof it ignores entirely; that is R1's second half and
    # it was found in this branch, so this file enumerates rather than filters.
    if ofs:
        if len(ofs) != 1:
            raise ShapeNotCovered("%s has %d oneofs and this walker handles one" % (name, len(ofs)))
        members = list(ofs.values())[0]
        by_name = dict((g["name"], g) for g in members)
        chosen = o.oneof_members
        if chosen is None:
            chosen = [members[idx % len(members)]["name"]]
        for mname in chosen:
            g = by_name.get(mname)
            if g is None:
                raise ShapeNotCovered("%s has no oneof member %r" % (name, mname))
            gpath = "%s.%s" % (path, g["name"])
            if o.trace is not None:
                o.trace.add(shape_key(g))
            if g["kind"] == "message" and g["of"] == "Empty":
                body += W.ld(g["tag"], b"")          # the payload-free member
            elif g["kind"] == "message":
                body += W.ld(g["tag"], enc_message(schema, corpus, g["of"], gpath, idx, o, depth + 1))
            elif g["kind"] == "string":
                body += W.s(g["tag"], text_value(g, gpath, idx))
            elif g["kind"] == "bytes":
                body += W.ld(g["tag"], V.blob(gpath, idx))
            elif g["kind"] in ("int32", "int64", "bool", "double", "fixed32", "enum"):
                body += enc_scalar(g["kind"], g["tag"],
                                   V.enum_value(schema["enums"][g["of"]]["values"], idx)
                                   if g["kind"] == "enum" else scalar_value(g["kind"], gpath, idx))
            else:
                raise ShapeNotCovered("oneof member %s.%s has kind %r with no case"
                                      % (name, g["name"], g["kind"]))

    unk = enc_unknown(schema, corpus, name, path, idx, o, depth)
    if not unk:
        return bytes(body)
    if o.unknown_placement == "after":
        return bytes(body) + unk
    if o.unknown_placement == "before":
        return unk + bytes(body)
    if o.unknown_placement == "interleaved":
        half = len(known) // 2
        head = bytearray()
        for f in known[:half]:
            head += enc_field(schema, corpus, f, "%s.%s" % (path, f["name"]), idx, o, depth)
        return bytes(head) + unk + bytes(body)[len(head):]
    raise ShapeNotCovered("no case for unknown_placement %r" % o.unknown_placement)
