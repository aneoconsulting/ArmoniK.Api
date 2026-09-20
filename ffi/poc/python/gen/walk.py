"""The ONE field walker every python-slice backend goes through.

README R1's second half: a backend that has no case for a shape RAISES rather than
skipping it.  The rule exists because the rust slice shipped a walker that excluded
oneof members, and every backend over it emitted a complete-looking codec that
ignored a message's oneof and reported nothing wrong.

`SCOPE` is the same statement one level up: the python slice covers the M1 subtree
and a message outside it raises, so the scope is enforced by the build rather than
described in a comment.
"""

# M1 and M2. Widening this is what brings a shape in, and the raises below are what
# make the widening deliberate: a message outside SCOPE raises, and so does a field
# shape no backend has a case for.
#
# M2 is the one that decides whether the verdict is a verdict. `TaskDetailed` is NOT a
# leaf, so the batching predicate refuses it and the crossing count stops being constant
# in the element count -- which is the property every M1 figure rests on. It also brings
# the map, four repeated string fields, a 4-level chain (root -> TaskDetailed ->
# TaskOptions -> Duration) and the nested adapter site.
SCOPE = [
    "ListResultsResponse", "ResultRaw", "Timestamp",          # M1
    "ListTasksDetailedResponse", "TaskDetailed", "TaskOptions",  # M2
    "Duration", "TaskOutput",
]
ROOTS = ["ListResultsResponse", "ListTasksDetailedResponse"]
ROOT = "ListResultsResponse"   # M1, kept for the work unit 1 and 2 entry points

# Work unit 1's no-core C shim is a FROZEN artifact: its column was measured with the
# text it emits, and README R4's last paragraph is why that text does not move when the
# scope widens. It keeps the scope it was measured at.
SCOPE_M1 = ["ListResultsResponse", "ResultRaw", "Timestamp"]


class Unsupported(Exception):
    """A shape no backend here has a case for. R1: raise, never skip."""


def walk(schema, msg_name, scope=None):
    """Every field of a message, as (field, kind, cardinality)."""
    import sys, os
    scope = scope or SCOPE
    if msg_name not in scope:
        raise Unsupported(
            "message %r is outside the python slice's scope %r. A backend with no "
            "case for a shape raises (README R1); widen SCOPE deliberately."
            % (msg_name, scope))
    S = sys.modules["shapes"]
    m = schema["messages"][msg_name]
    if S.oneofs(m):
        raise Unsupported("%s has a oneof; no backend here has a case for it" % msg_name)
    for f in S.fields(m):
        c = S.card(f)
        k = f["kind"]
        if c not in ("singular", "repeated", "map"):
            raise Unsupported("%s.%s: cardinality %r" % (msg_name, f["name"], c))
        if c == "repeated" and k not in ("message", "string"):
            raise Unsupported("%s.%s: repeated %s" % (msg_name, f["name"], k))
        if c == "map" and (f.get("key"), f.get("value_kind")) != ("string", "string"):
            raise Unsupported("%s.%s: map<%s, %s>"
                              % (msg_name, f["name"], f.get("key"), f.get("value_kind")))
        if k not in ("string", "bytes", "int32", "int64", "bool", "enum", "message",
                     "map"):
            raise Unsupported("%s.%s: kind %r" % (msg_name, f["name"], k))
        if f.get("presence") == "explicit":
            raise Unsupported("%s.%s: explicit presence" % (msg_name, f["name"]))
        yield f, k, c


def pydefault(k, c):
    if c == "repeated":
        return "None"
    if c == "map":
        return "None"
    return {"string": '""', "bytes": 'b""', "int32": "0", "int64": "0",
            "bool": "False", "enum": "0", "message": "None"}[k]
