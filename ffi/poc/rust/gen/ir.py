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
        return self.kind in ("int32", "int64", "bool", "double", "enum")

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


def loop_slots(ir, name, prefix=()):
    """Every field of `name` that needs a vtable slot, INCLUDING the ones inside a
    singular message child that the encode group inlines.

    ABI v1 section 6: "A repeated or map field inside an inlined child keeps its loop slot
    and is reached through the parent." So `TaskDetailed`'s vtable carries
    `loop_options_options` for the map that lives on `TaskOptions`, and `TaskOptions` never
    appears in the ABI as a message with a vtable of its own.

    Returns [(path tuple, Field)] in tag order, depth first.
    """
    out = []
    for f in ir.msg(name).plain:
        if f.oneof:
            continue
        if f.card in ("repeated", "packed", "map"):
            out.append((prefix + (f.name,), f))
        elif f.kind == "message" and f.card == "singular":
            out.extend(loop_slots(ir, f.of, prefix + (f.name,)))
    return out


def slot_name(path):
    return "_".join(path)


def direct_fields(ir, name, prefix=(), seen=None):
    """Every direct-argument field ANYWHERE in the tree rooted at `name`.

    Through every edge, not only singular children: a direct field declared on a repeated
    element type is still a direct field of the tree, and walking only singular children is
    how the first version of this check found nothing and refused nothing.
    """
    seen = seen or set()
    if name in seen:
        return []
    seen = seen | {name}
    out = []
    for f in ir.msg(name).fields:
        if f.direct:
            out.append((prefix + (f.name,), f))
        elif f.kind == "message":
            out.extend(direct_fields(ir, f.of, prefix + (f.name,), seen))
    return out


def check_direct(ir, root):
    """ABI v1 section 8: "a direct field declared on a message that does make a reverse call
    should be a generator-time refusal and currently is not."

    It is now. A direct argument exists so a host can pin its buffer across the whole call --
    `GetPrimitiveArrayCritical` on JNI, `critical(true)` on FFM -- and a critical section and
    an upcall are mutually exclusive. So a message tree that carries a direct field and also
    needs a reverse call is a contract no host can honour, and the honest place to say so is
    here rather than in a comment in the specification.

    Also refused: more than one direct field in one tree. The path is built for one field of
    one root message and nothing has tested it otherwise; silently generalising it is how an
    untested path ships.
    """
    ds = direct_fields(ir, root)
    if not ds:
        return
    # Every reverse call the tree would make, not only the root's own.
    slots = list(loop_slots(ir, root))
    for n in ir.messages:
        if n == root:
            continue
        if direct_fields(ir, n):
            slots.extend(loop_slots(ir, n))
    if slots:
        raise NotImplementedError(
            "REFUSED: %s declares a direct-argument field (%s) and also needs a reverse call "
            "for %s. ABI v1 section 8: a direct argument exists so the host can pin its "
            "buffer across the call, and a critical section and an upcall are mutually "
            "exclusive, so no host can honour both."
            % (root, slot_name(ds[0][0]), ", ".join(slot_name(p) for p, _ in slots)))
    if len(ds) > 1:
        raise NotImplementedError(
            "REFUSED: %s declares %d direct-argument fields (%s). ABI v1 section 8 builds the "
            "path for ONE field of one root message and nothing tests it otherwise."
            % (root, len(ds), ", ".join(slot_name(p) for p, _ in ds)))
