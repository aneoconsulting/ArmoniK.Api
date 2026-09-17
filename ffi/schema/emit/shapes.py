"""Read shapes.json and answer questions about it.

Shared by every emitter in this directory. A slice's own generator backend is
welcome to import this rather than re-parse the description.
"""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)

SCALARS = {"int32", "int64", "bool", "double", "string", "bytes"}


def load(path=None):
    with open(path or os.path.join(ROOT, "shapes.json")) as f:
        return json.load(f)


def fields(msg):
    return msg.get("fields", [])


def card(f):
    """singular, repeated, packed or map."""
    if f["kind"] == "map":
        return "map"
    return f.get("card", "singular")


def is_leaf(schema, name, seen=None):
    """A message is a leaf if it has no repeated and no map field, transitively.

    This is the batching predicate of ABI v1 sections 6 and 7.2, and it is
    computed here so that no slice has to re-derive it.
    """
    seen = seen or set()
    if name in seen:
        return False
    seen = seen | {name}
    for f in fields(schema["messages"][name]):
        if card(f) in ("repeated", "packed", "map"):
            return False
        if f["kind"] == "message" and not is_leaf(schema, f["of"], seen):
            return False
    return True


def oneofs(msg):
    """{oneof name: [field, ...]} in declaration order."""
    out = {}
    for f in fields(msg):
        if "oneof" in f:
            out.setdefault(f["oneof"], []).append(f)
    return out


def depth(schema, name, seen=None):
    seen = seen or set()
    if name in seen:
        return 0
    seen = seen | {name}
    best = 0
    for f in fields(schema["messages"][name]):
        if f["kind"] == "message":
            best = max(best, 1 + depth(schema, f["of"], seen))
    return best
