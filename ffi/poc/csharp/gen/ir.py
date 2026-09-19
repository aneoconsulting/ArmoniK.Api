"""The IR every C# backend walks.

Imports `ffi/schema/emit/shapes.py` rather than re-parsing `shapes.json` (R1),
so a shape this slice does not handle is a raise in one place and not a silent
omission in four.

The rule that costs a session if it is got wrong, and that the Rust slice did
get wrong (its D12): **`walk()` enumerates every field, oneof members
included.** A walker that iterates only the non-oneof fields emits a
complete-looking codec that ignores a message's oneof and reports nothing.
Every backend calls `walk()`; none of them iterates `msg["fields"]` itself.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
SCHEMA = os.path.normpath(os.path.join(HERE, "..", "..", "..", "schema"))
sys.path.insert(0, os.path.join(SCHEMA, "emit"))

import shapes as S          # noqa: E402
import csnames as N         # noqa: E402

WIRE_VARINT, WIRE_I64, WIRE_LEN, WIRE_I32 = 0, 1, 2, 5

# Every (kind, cardinality) pair this generator has a case for. A pair outside
# it raises: R1's second half, and the reason it is a table and not a chain of
# `if`s is that a chain has a fall-through and a table does not.
KNOWN_KINDS = {"int32", "int64", "bool", "double", "string", "bytes", "enum", "message", "map"}
KNOWN_CARDS = {"singular", "repeated", "packed", "map"}


class Unsupported(Exception):
    """A shape no backend has a case for. Raised, never skipped."""


class Field:
    def __init__(self, schema, owner, f):
        self.raw = f
        self.owner = owner
        self.name = f["name"]
        self.cs = N.field(f["name"])
        self.tag = f["tag"]
        self.kind = f["kind"]
        self.card = S.card(f)
        self.of = f.get("of")
        self.oneof = f.get("oneof")
        self.presence = f.get("presence")          # "explicit" or None
        self.adapter_site = f.get("adapter_site")  # "nested", "plain" or None
        self.value_rule = f.get("value")           # guid / word / sentence / bulk
        self.entries = f.get("entries", 4)
        self.count = f.get("count", 30)
        self.map_key = f.get("key")
        self.map_value_kind = f.get("value_kind")

        if self.kind not in KNOWN_KINDS:
            raise Unsupported("%s.%s: kind %r" % (owner, self.name, self.kind))
        if self.card not in KNOWN_CARDS:
            raise Unsupported("%s.%s: card %r" % (owner, self.name, self.card))
        if self.card == "map" and (self.map_key, self.map_value_kind) != ("string", "string"):
            raise Unsupported("%s.%s: only map<string,string> has a case" % (owner, self.name))
        if self.card == "packed" and self.kind not in ("int32", "int64", "bool", "double", "enum"):
            raise Unsupported("%s.%s: packed %s" % (owner, self.name, self.kind))
        if self.card == "repeated" and self.kind not in ("string", "message"):
            raise Unsupported("%s.%s: repeated %s" % (owner, self.name, self.kind))
        if self.presence == "explicit" and self.card != "singular":
            raise Unsupported("%s.%s: explicit presence on a %s field" % (owner, self.name, self.card))
        if self.oneof and self.card != "singular":
            raise Unsupported("%s.%s: a %s oneof member" % (owner, self.name, self.card))

    @property
    def wire(self):
        if self.card in ("packed", "map"):
            return WIRE_LEN
        if self.kind in ("int32", "int64", "bool", "enum"):
            return WIRE_VARINT
        if self.kind == "double":
            return WIRE_I64
        return WIRE_LEN

    @property
    def key(self):
        return (self.tag << 3) | self.wire

    @property
    def is_scalar(self):
        return self.kind in ("int32", "int64", "bool", "double")

    def __repr__(self):
        return "<%s.%s %s %s>" % (self.owner, self.name, self.card, self.kind)


class Message:
    def __init__(self, schema, name):
        self.name = name
        self.cs = name
        self.raw = schema["messages"][name]
        self.fields = [Field(schema, name, f) for f in S.fields(self.raw)]
        self.oneofs = {}
        for f in self.fields:
            if f.oneof:
                self.oneofs.setdefault(f.oneof, []).append(f)
        self.leaf = S.is_leaf(schema, name)
        self.doc = self.raw.get("source") or self.raw.get("why") or ""

    def walk(self):
        """Every field, oneof members included. See the module docstring."""
        return list(self.fields)

    def plain(self):
        return [f for f in self.fields if not f.oneof]

    def sorted_wire(self):
        """Fields in the order the canonical form writes them: ascending by tag."""
        return sorted(self.fields, key=lambda f: f.tag)


class Ir:
    def __init__(self, schema, roots):
        self.schema = schema
        self.roots = roots
        self.enums = schema["enums"]
        self.messages = {}
        for r in roots:
            self._close(r)
        # Deterministic order: declaration order in shapes.json, so a diff of the
        # emitted file is a diff of the description and not of a set iteration.
        order = [n for n in schema["messages"] if n in self.messages]
        self.messages = {n: self.messages[n] for n in order}
        self.order = order

    def _close(self, name):
        if name in self.messages:
            return
        m = Message(self.schema, name)
        self.messages[name] = m
        for f in m.walk():
            if f.kind == "message":
                self._close(f.of)
            if f.kind == "enum" and f.of not in self.enums:
                raise Unsupported("%s.%s: enum %s is not declared" % (f.owner, f.name, f.of))

    def msg(self, name):
        return self.messages[name]

    def enum_values(self, name):
        return list(self.enums[name]["values"].items())

    def root_element(self, root):
        """(field, element message) for a list-shaped root, or None."""
        m = self.msg(root)
        for f in m.walk():
            if f.card == "repeated" and f.kind == "message":
                return f, self.msg(f.of)
        return None


def check_walker(ir):
    """R1's second half, exercised rather than asserted.

    A guard with no failing test is a guard nobody has seen work, so this runs
    `walk()` against a message that HAS a oneof and fails if the count does not
    exceed `plain()`. If a later edit reintroduces the Rust slice's D12 -- a
    walker that silently drops oneof members -- this is what says so.
    """
    checked = 0
    for m in ir.messages.values():
        if not m.oneofs:
            continue
        checked += 1
        members = sum(len(v) for v in m.oneofs.values())
        if len(m.walk()) != len(m.plain()) + members:
            raise Unsupported("walk() drops oneof members on %s" % m.name)
    if not checked:
        raise Unsupported(
            "no message in the closure has a oneof, so the walker guard proved nothing. "
            "design/SHAPES.md M3 exists precisely so that this cannot happen.")
    return checked


def load(roots):
    ir = Ir(S.load(), roots)
    check_walker(ir)
    return ir
