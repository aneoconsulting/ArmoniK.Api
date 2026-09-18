"""shapes.json -> the intermediate form every Rust backend in this directory reads.

Imports `ffi/schema/emit/shapes.py` rather than re-parsing the description (README R1),
so a shape can only be answered one way. Nothing here knows about Rust; the backends do.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
EMIT = os.path.abspath(os.path.join(HERE, "..", "..", "..", "schema", "emit"))
if EMIT not in sys.path:
    sys.path.insert(0, EMIT)

import shapes as S   # noqa: E402

VARINT, I64, LEN, I32 = 0, 1, 2, 5

# Wire type per schema kind, for the singular case.
WIRE = {"int32": VARINT, "int64": VARINT, "bool": VARINT, "enum": VARINT,
        "double": I64, "string": LEN, "bytes": LEN, "message": LEN}


class Field:
    def __init__(self, schema, owner, f):
        self.raw = f
        self.owner = owner
        self.name = f["name"]
        self.tag = f["tag"]
        self.kind = f["kind"]
        self.card = S.card(f)                       # singular | repeated | packed | map
        self.of = f.get("of")
        self.explicit = f.get("presence") == "explicit"
        self.oneof = f.get("oneof")
        self.value_rule = f.get("value", "word")
        self.adapter_site = f.get("adapter_site")
        self.wire = WIRE[self.kind] if self.kind != "map" else LEN

    @property
    def is_scalar_leaf(self):
        return self.kind in ("int32", "int64", "bool", "double", "enum")

    @property
    def is_blob(self):
        return self.kind in ("string", "bytes")

    def __repr__(self):
        return "<%s.%s tag=%d %s %s>" % (self.owner, self.name, self.tag, self.kind, self.card)


class Message:
    def __init__(self, schema, name):
        self.name = name
        self.raw = schema["messages"][name]
        self.fields = sorted((Field(schema, name, f) for f in S.fields(self.raw)),
                             key=lambda f: f.tag)
        self.oneofs = {}
        for f in self.fields:
            if f.oneof:
                self.oneofs.setdefault(f.oneof, []).append(f)
        self.plain = [f for f in self.fields if not f.oneof]
        # ABI v1 sections 6 and 7.2: an element type free of repeated and map fields,
        # transitively, is what may ride in a group and be handed over as a run.
        self.leaf = S.is_leaf(schema, name)

    @property
    def singular_children(self):
        """Message-typed singular fields, which the encode group inlines whatever their size."""
        return [f for f in self.plain if f.kind == "message" and f.card == "singular"]

    def __repr__(self):
        return "<msg %s leaf=%s>" % (self.name, self.leaf)


class Ir:
    def __init__(self, schema, roots):
        self.schema = schema
        self.roots = list(roots)
        self.messages = {}
        self.enums = {}
        for r in roots:
            self._close(r)
        self.order = [n for n in schema["messages"] if n in self.messages]

    def _close(self, name):
        if name in self.messages:
            return
        m = Message(self.schema, name)
        self.messages[name] = m
        for f in m.fields:
            if f.kind == "message":
                self._close(f.of)
            elif f.kind == "enum":
                self.enums[f.of] = self.schema["enums"][f.of]

    def msg(self, name):
        return self.messages[name]

    @property
    def enum_order(self):
        return [n for n in self.schema["enums"] if n in self.enums]


def load(roots):
    return Ir(S.load(), roots)
