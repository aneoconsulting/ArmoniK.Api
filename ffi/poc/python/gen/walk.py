"""The ONE field walker every python-slice backend goes through.

README R1's second half: a backend that has no case for a shape RAISES rather than
skipping it.  The rule exists because the rust slice shipped a walker that excluded
oneof members, and every backend over it emitted a complete-looking codec that
ignored a message's oneof and reported nothing wrong.

`SCOPE` is the same statement one level up: the python slice covers the shapes listed
below and a message outside it raises, so the scope is enforced by the build rather than
described in a comment.

**A oneof member is yielded with cardinality `"oneof"`, not hidden.**  It would have been
less work to yield only `m.plain` the way the IR does and hand the oneof to the backends
through a second accessor -- and a backend that forgot to call the second accessor would
then emit a complete-looking codec that dropped the oneof, which is the exact defect this
walker exists because of.  Yielding it with a cardinality nothing has a case for makes
every backend fail at GENERATION time until it grows one.  `oneof_groups` is beside it for
the discriminant, which is a property of the group rather than of any one field.
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
# M3 brings the two shapes a by-value group is least obviously able to carry: a oneof,
# whose members share a slot on the wire and do not share one in the group (ABI v1's
# `<name>_case` discriminant plus every member inlined beside it), and explicit presence,
# where absent and present-and-zero are different messages and the value cannot say which.
# `Empty` is in scope because the oneof's payload-free member is typed by it.
SCOPE = [
    "ListResultsResponse", "ResultRaw", "Timestamp",          # M1
    "ListTasksDetailedResponse", "TaskDetailed", "TaskOptions",  # M2
    "Duration", "TaskOutput",
    "ListProbeResponse", "Probe", "Empty",                    # M3
    "ListTaskSummaryResponse", "TaskSummary",                 # M4
    "UploadResultDataMessage", "UploadResultData",            # M5
    "ListMetricsResponse", "MetricsBatch",                    # M6
    "DualResponse", "Pair",                                   # M7
]
# M4 adds no field shape at all: `TaskSummary` is `TaskOptions` and a `Timestamp` over
# shapes M1 and M2 already carry. What it adds is the PLAIN adapter site -- a string field
# where two of the adapter's three states both flatten to "" -- and that is a property of
# the payload's values, not of the walker, so nothing here has to grow a case for it.
# M7's root has TWO repeated fields of one element type, interleaved on the wire. No
# canonical writer can produce those bytes -- a writer that emits a repeated field
# contiguously cannot interleave two of them -- so it is validated by DECODING, and the
# re-encode is checked against a permutation of the same (tag, wire type, body) triples
# rather than against the manifest's hash (design/SHAPES.md, P7.1).
ROOTS = ["ListResultsResponse", "ListTasksDetailedResponse", "ListProbeResponse",
         "ListTaskSummaryResponse", "UploadResultDataMessage", "ListMetricsResponse",
         "DualResponse"]
DECODE_ONLY = ["DualResponse"]
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
    for f in S.fields(m):
        c = S.card(f)
        if f.get("oneof"):
            c = "oneof"
        elif f.get("presence") == "explicit" and c == "singular":
            # Same reasoning as the oneof: a backend that treats an `optional` scalar as an
            # ordinary one emits a codec that cannot tell absent from zero, and every
            # payload but the absent one passes. A cardinality nothing has a case for makes
            # that a generation-time failure instead.
            c = "optional"
        k = f["kind"]
        if c not in ("singular", "repeated", "packed", "map", "oneof", "optional"):
            raise Unsupported("%s.%s: cardinality %r" % (msg_name, f["name"], c))
        if c == "repeated" and k not in ("message", "string"):
            raise Unsupported("%s.%s: repeated %s" % (msg_name, f["name"], k))
        if c == "map" and (f.get("key"), f.get("value_kind")) != ("string", "string"):
            raise Unsupported("%s.%s: map<%s, %s>"
                              % (msg_name, f["name"], f.get("key"), f.get("value_kind")))
        if c == "packed" and k not in ("int32", "int64", "bool", "enum", "double"):
            raise Unsupported("%s.%s: packed %s" % (msg_name, f["name"], k))
        if k == "double" and c != "packed":
            # The description has exactly one `double`, and it is packed. A singular one
            # would need a fixed64 group field and a `PyFloat` read that nothing here has,
            # so it raises rather than taking the varint path by accident.
            raise Unsupported("%s.%s: a singular double" % (msg_name, f["name"]))
        if k not in ("string", "bytes", "int32", "int64", "bool", "enum", "message",
                     "map", "double"):
            raise Unsupported("%s.%s: kind %r" % (msg_name, f["name"], k))
        if c in ("oneof", "optional") and S.card(f) != "singular":
            raise Unsupported("%s.%s: a repeated %s member" % (msg_name, f["name"], c))
        if c == "optional" and k == "message":
            # An `optional` message child is the same thing as a plain one in proto3 and
            # the facade already says absent with None. Nothing in the payload set has one;
            # if one appears, decide deliberately rather than inherit this branch.
            raise Unsupported("%s.%s: explicit presence on a message child"
                              % (msg_name, f["name"]))
        yield f, k, c


ZERO = {"string": '""', "bytes": 'b""', "int32": "0", "int64": "0",
        "bool": "False", "enum": "0", "message": "None", "double": "0.0"}


def pydefault(k, c):
    if c in ("repeated", "packed", "map"):
        return "None"
    if c == "optional":
        # Explicit presence: `None` is absent and it is a DIFFERENT message from the one
        # holding the proto zero. The facade cannot say that with the value alone, which
        # is the whole reason this cardinality exists.
        return "None"
    # A oneof member that is not selected reads as its proto default, the way protobuf's
    # own Python API behaves; `<oneof>_case` is what says which one is selected.
    return ZERO[k]


def oneof_groups(schema, msg_name):
    """{oneof name: [field, ...]} in tag order, for a message in scope.

    The ABI gives a oneof a `<name>_case` discriminant carrying **the active member's tag**,
    0 for unset, and inlines every member beside it rather than in a union (ABI v1 section 6
    and `poc/codec/gen/rust_abi.py:group_fields`). A host that invents its own numbering
    gets `AK_ERR_ABI`, so nothing here renumbers: the facade stores the tag.
    """
    import sys
    S = sys.modules["shapes"]
    m = schema["messages"][msg_name]
    return {o: list(fs) for o, fs in S.oneofs(m).items()}


def explicit_fields(schema, msg_name):
    """The fields whose presence their value cannot carry, as (field, PRESENT macro suffix).

    Explicit presence only. A singular message child is also absent-able, and the facade
    says so with `None`; an `optional` scalar cannot, because 0 is a value.
    """
    import sys
    S = sys.modules["shapes"]
    m = schema["messages"][msg_name]
    return [(f, f["name"].upper()) for f in S.fields(m)
            if f.get("presence") == "explicit"]
