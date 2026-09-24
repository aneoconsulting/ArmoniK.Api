"""The facade's field facts, for the harness glue (FIX-PLAN WP5 step 5).

`gen/walk.py` is retired: it re-derived the field list from the description, and every
harness here walked messages through it. The facts now come from the generated facade's
`MESSAGES` table, which `poc/codec/gen/py_pure.py` renders from the PLAN, so the harness
reads what the codec was generated from rather than a second reading of the schema.

This module states no rule. It re-shapes those facts into the (field, kind, cardinality)
triples the harnesses were written against, where a oneof member has cardinality "oneof"
and an explicit-presence singular field "optional" -- two cardinalities a consumer with no
case for them fails on, which was walk.py's R1 property and is kept.
"""


def walk(fac, msg):
    """Every field of `msg`, as (field dict, kind, cardinality)."""
    if msg not in fac.MESSAGES:
        raise KeyError("message %r has no facade in %s (a synthetic map pair, or not "
                       "reachable from the plan's roots)" % (msg, fac.__name__))
    for name, tag, kind, card, of, presence, oneof, entry in fac.MESSAGES[msg]["fields"]:
        if oneof:
            c = "oneof"
        elif presence == "explicit" and card == "singular":
            c = "optional"
        else:
            c = card
        f = {"name": name, "tag": tag, "kind": kind, "of": of, "oneof": oneof,
             "presence": presence, "entry": entry}
        yield f, kind, c


def oneof_groups(fac, msg):
    """{oneof: [member field dict, ...]} in tag order."""
    out = {}
    for f, k, c in walk(fac, msg):
        if c == "oneof":
            out.setdefault(f["oneof"], []).append(f)
    return out
