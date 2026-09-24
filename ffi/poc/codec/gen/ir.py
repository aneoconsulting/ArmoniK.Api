"""The front end: a description -> the one IR. FIX-PLAN WP5 item 1.

Two descriptions, one IR:

  load(roots)          `ffi/schema/shapes.json`, through `ffi/schema/emit/shapes.py`
                       (README R1), so a shape can only be answered one way;
  load_corpus(roots)   the corpus's READER view (`ffi/corpus/corpus.json` merged over
                       shapes.json by `ffi/corpus/emit/spec.py`, the same merge that writes
                       `corpus/generated/corpus.proto`). CONTRACT.md rule 0: a reader is
                       generated from the reader view, never from the superset.

**Only `plan.py` imports this module.** Backends take plans (`plan.py`'s contract), and
`generate.py --check` fails if a backend imports the IR or the schema. The IR states what
the description SAYS (names, tags, kinds, cardinality, presence); every rule about what
that means on the wire is in `plan.py`.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
EMIT = os.path.abspath(os.path.join(HERE, "..", "..", "..", "schema", "emit"))
if EMIT not in sys.path:
    sys.path.insert(0, EMIT)

import shapes as S   # noqa: E402

VARINT, I64, LEN, I32 = 0, 1, 2, 5

# Wire type per schema kind, for the singular case. Every kind either description uses
# (R-E3: the corpus's `fixed32` included); a kind with no entry raises at load time.
WIRE = {"int32": VARINT, "int64": VARINT, "bool": VARINT, "enum": VARINT,
        "double": I64, "fixed32": I32, "string": LEN, "bytes": LEN, "message": LEN}


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
        # Set by Ir._synthesise_entry for a map field: the pair message it becomes.
        self.entry = None
        # ABI v1 section 8: a bulk `bytes` field crosses as a DIRECT ARGUMENT of the call
        # rather than as a pointer into staging, so a JNI host can hold a critical section
        # across it. The descriptor says which: `"value": "bulk"`.
        self.direct = f.get("value") == "bulk"
        self.value_rule = f.get("value", "word")
        self.adapter_site = f.get("adapter_site")
        self.wire = WIRE[self.kind] if self.kind != "map" else LEN

    @property
    def is_scalar_leaf(self):
        return self.kind in ("int32", "int64", "bool", "double", "enum", "fixed32")

    @property
    def is_blob(self):
        return self.kind in ("string", "bytes")

    def __repr__(self):
        return "<%s.%s tag=%d %s %s>" % (self.owner, self.name, self.tag, self.kind, self.card)


def map_entry_name(owner, field):
    """ABI v1 section 11: a map has no case of its own. It is a repeated field of a pair
    message, and the repeated-message path handles it both ways. So the description's map
    field becomes a synthetic pair message here, once, and every backend sees a repeated
    message from then on."""
    return "%s%sEntry" % (owner, "".join(p.capitalize() for p in field.split("_")))


class Message:
    def __init__(self, schema, name, raw=None, synthetic=False):
        self.name = name
        self.synthetic = synthetic
        self.raw = raw if raw is not None else schema["messages"][name]
        self.fields = sorted((Field(schema, name, f) for f in S.fields(self.raw)),
                             key=lambda f: f.tag)
        self.oneofs = {}
        for f in self.fields:
            if f.oneof:
                self.oneofs.setdefault(f.oneof, []).append(f)
        self.plain = [f for f in self.fields if not f.oneof]
        # ABI v1 sections 6 and 7.2: an element type free of repeated and map fields,
        # transitively, is what may ride in a group and be handed over as a run.
        self.leaf = S.is_leaf(schema, name) if not synthetic else True

    @property
    def singular_children(self):
        """Message-typed singular fields, which the encode group inlines whatever their size."""
        return [f for f in self.plain if f.kind == "message" and f.card == "singular"]

    @property
    def loop_fields(self):
        """Fields that could not ride in the encode group: one vtable slot each."""
        return [f for f in self.plain if f.card in ("repeated", "packed", "map")]

    def __repr__(self):
        return "<msg %s leaf=%s%s>" % (self.name, self.leaf, " synthetic" if self.synthetic else "")


class Ir:
    def __init__(self, schema, roots):
        self.schema = schema
        self.roots = list(roots)
        self.messages = {}
        self.enums = {}
        for r in roots:
            self._close(r)
        # Declaration order for the real messages; synthetic pair messages first, because a
        # group for a pair has to exist before the message that loops over it.
        self.order = [n for n in schema["messages"] if n in self.messages]
        self.abi_order = ([n for n, m in self.messages.items() if m.synthetic]
                          + [n for n in self.order])

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
            elif f.kind == "map":
                self._synthesise_entry(name, f)

    def _synthesise_entry(self, owner, f):
        """The pair message a map field is, so no backend needs a map case."""
        ename = map_entry_name(owner, f.name)
        f.entry = ename
        if ename in self.messages:
            return
        raw = {
            "source": "synthetic: the pair message of %s.%s, per ABI v1 section 11" % (owner, f.name),
            "fields": [
                {"name": "key", "tag": 1, "kind": f.raw["key"], "value": "word"},
                {"name": "value", "tag": 2, "kind": f.raw["value_kind"], "value": "word"},
            ],
        }
        self.messages[ename] = Message(self.schema, ename, raw=raw, synthetic=True)

    def msg(self, name):
        return self.messages[name]

    @property
    def enum_order(self):
        return [n for n in self.schema["enums"] if n in self.enums]


def load(roots):
    return Ir(S.load(), roots)


def corpus_schema():
    """The corpus's READER view, merged by the corpus's own loader so the IR reads exactly
    the schema `corpus.proto` was written from. Imported lazily: only the corpus build
    needs it, and the corpus tree is not a dependency of the shipped core."""
    corpus_emit = os.path.abspath(os.path.join(HERE, "..", "..", "..", "corpus", "emit"))
    if corpus_emit not in sys.path:
        sys.path.insert(0, corpus_emit)
    import spec  # noqa: E402  ffi/corpus/emit/spec.py
    reader, _superset, _delta = spec.load()
    return reader


def load_corpus(roots=None):
    """The IR over the corpus reader schema. `roots=None` means every message it declares,
    in declaration order: the corpus roots a vector at almost every message."""
    schema = corpus_schema()
    if roots is None:
        roots = list(schema["messages"])
    return Ir(schema, roots)


# ---- ABI layout helpers: DEFINED IN plan.py, re-exported here for the generators that
# still import them from the IR (cpp, java, csharp, python) until they are ported to plans
# (FIX-PLAN WP5 migration steps 2 to 5). One definition; these are forwarding names only.

def loop_slots(ir, name, prefix=()):
    import plan
    return plan.loop_slots(ir, name, prefix)


def slot_name(path):
    import plan
    return plan.slot_name(path)


def direct_fields(ir, name, prefix=(), seen=None):
    import plan
    return plan.direct_fields(ir, name, prefix, seen)


def check_direct(ir, root):
    import plan
    return plan.check_direct(ir, root)


def abi_order_topo(ir):
    import plan
    return plan.abi_order_topo(ir)
