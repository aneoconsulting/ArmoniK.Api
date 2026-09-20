"""The ONE field walker every python-slice backend goes through.

README R1's second half: a backend that has no case for a shape RAISES rather than
skipping it.  The rule exists because the rust slice shipped a walker that excluded
oneof members, and every backend over it emitted a complete-looking codec that
ignored a message's oneof and reported nothing wrong.

`SCOPE` is the same statement one level up: the python slice covers the M1 subtree
and a message outside it raises, so the scope is enforced by the build rather than
described in a comment.
"""

# The M1 subtree, and nothing else. Widening this is the work item that brings M2
# and P2.2 in; the raise below is what makes the widening deliberate.
SCOPE = ["ListResultsResponse", "ResultRaw", "Timestamp"]
ROOT = "ListResultsResponse"


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
        if c not in ("singular", "repeated"):
            raise Unsupported("%s.%s: cardinality %r" % (msg_name, f["name"], c))
        if c == "repeated" and k != "message":
            raise Unsupported("%s.%s: repeated %s" % (msg_name, f["name"], k))
        if k not in ("string", "bytes", "int32", "int64", "bool", "enum", "message"):
            raise Unsupported("%s.%s: kind %r" % (msg_name, f["name"], k))
        if f.get("presence") == "explicit":
            raise Unsupported("%s.%s: explicit presence" % (msg_name, f["name"]))
        yield f, k, c


def pydefault(k, c):
    if c == "repeated":
        return "None"
    return {"string": '""', "bytes": 'b""', "int32": "0", "int64": "0",
            "bool": "False", "enum": "0", "message": "None"}[k]
