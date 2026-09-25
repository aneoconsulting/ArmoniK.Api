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


def unknown_slots(fac, cext_mod=None, objs=()):
    """WP5 step 10: every place a facade could still carry `_unknown`. Returns a list of
    "<class>: <how>" findings, empty when there are none. Checked: each Plain* and Slots*
    class (a `__slots__` entry, a constructor parameter, the attribute on a fresh instance);
    each C extension type of `cext_mod` (a member descriptor, the attribute on a fresh
    instance); and the given decoded objects, walked through every message-valued field."""
    import inspect
    found = []
    for n in fac.MESSAGES:
        for pre in ("Plain", "Slots"):
            cls = getattr(fac, pre + n, None)
            if cls is None:
                continue
            if "_unknown" in getattr(cls, "__slots__", ()):
                found.append("%s%s: __slots__ entry" % (pre, n))
            if "_unknown" in inspect.signature(cls.__init__).parameters:
                found.append("%s%s: constructor parameter" % (pre, n))
            if hasattr(cls(), "_unknown"):
                found.append("%s%s: attribute on a fresh instance" % (pre, n))
    if cext_mod is not None:
        for tn in cext_mod.types():
            t = getattr(cext_mod, tn)
            if "_unknown" in dir(t):
                found.append("%s: member descriptor" % tn)
            if hasattr(t(), "_unknown"):
                found.append("%s: attribute on a fresh instance" % tn)

    def walk(o, msg, seen):
        if o is None or id(o) in seen:
            return
        seen.add(id(o))
        if hasattr(o, "_unknown"):
            found.append("decoded %s (%s): has _unknown" % (msg, type(o).__name__))
        for f in fac.MESSAGES[msg]["fields"]:
            name, _tag, kind, card, of = f[0], f[1], f[2], f[3], f[4]
            if kind != "message":
                continue
            v = getattr(o, name, None)
            for x in (v if card == "repeated" else [v]):
                walk(x, of, seen)
    for o, msg in objs:
        walk(o, msg, set())
    return found
