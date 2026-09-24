"""RETIRED as the core-native emitter (FIX-PLAN WP5 step 1). A compatibility adapter only.

`emit_core_native` moved to `rust_native.py`, which renders the plans of `plan.py`; the
`NativeEnc`/`NativeDec` backends that lived here carried their own copy of every wire
decision (review finding R-E1) and are gone.

What is left is the traversal the cpp slice's `cpp_core.py` still imports --
`walk_encode`, `walk_decode` and `Sites` -- kept so that slice keeps generating while it is
ported (WP5 migration step 2). The walk no longer iterates the IR: the ORDER of the calls
comes from the plan's encode plan (tag order, R-E8), and each call hands the backend the
plan's `FieldPlan`, which carries the same descriptor attributes the IR field did. What the
cpp backend's methods do with a field is still that backend's own rendering, i.e. still a
per-language wire rule, until its port replaces them with a renderer of the plan's steps
and decode table. A shape the backend has no method for (fixed32) raises, as R1 requires.

Delete this file when `poc/cpp/gen/cpp_core.py` no longer imports it.
"""
from plan import as_plan

_CACHE = {}


def _plan_of(ir):
    key = id(ir)
    hit = _CACHE.get(key)
    if hit is None or hit[0] is not ir:
        hit = (ir, as_plan(ir))
        _CACHE[key] = hit
    return hit[1]


class Sites:
    """Length-prefix sites, one learned-width slot each (ABI v1 section 6)."""

    def __init__(self):
        self.ids = {}

    def id(self, key):
        return self.ids.setdefault(key, len(self.ids))

    def __len__(self):
        return len(self.ids)


def _dispatch(ir, m, b, o):
    pm = _plan_of(ir).msg(m.name)
    done = set()
    for st in pm.encode:
        f = st.field
        if st.op == "unknown_tail":
            continue
        if st.op == "child":
            b.child(m, f, o)
        elif st.op == "repeated_message":
            b.repeated_message(m, f, o)
        elif st.op == "blob":
            b.blob(m, f, o)
        elif st.op == "scalar" and f.kind == "double":
            b.double(m, f, o)
        elif st.op == "scalar" and f.kind in ("int32", "int64", "bool", "enum"):
            b.scalar(m, f, o)
        elif st.op == "repeated_blob":
            b.repeated_blob(m, f, o)
        elif st.op == "packed":
            b.packed(m, f, o)
        elif st.op == "map":
            b.map(m, f, o)
        elif st.op == "oneof_member":
            if st.oneof not in done:
                done.add(st.oneof)
                b.oneof(m, st.oneof, pm.oneofs[st.oneof], o)
        else:
            raise NotImplementedError("%s.%s (%s %s): no method in this legacy backend"
                                      % (m.name, f.name, f.card, f.kind))


def walk_encode(ir, m, b, o):
    """Legacy adapter: calls `b` in the plan's write order."""
    _dispatch(ir, m, b, o)


def walk_decode(ir, m, b, o):
    """Legacy adapter: calls `b` once per field, in the plan's order. The decode table the
    plan states (which wire types each field accepts) is not yet what the cpp backend
    renders; its port does that."""
    _dispatch(ir, m, b, o)
