"""The "read every field" reader of the decode+read rows, shared by arms.py (shapes) and
camp_codec.py (the corpus family, which cannot import arms: another libak_core).

`plan(fac, msg)` builds the walk from a facade module's plan-rendered MESSAGES table;
`read` walks a facade object, `read_pb` a protobuf message, doing the same number of
attribute reads so the two readers differ only where they must: presence.
"""
import facts as _F


def plan(fac, msg):
    """[(name, kind, card, child plan or None)] for one message (see arms.py, D9)."""
    out = []
    for f, k, c in _F.walk(fac, msg):
        child = plan(fac, f["of"]) if k == "message" else None
        if c == "oneof":
            continue
        out.append((f["name"], k, c, child))
    for oname, members in _F.oneof_groups(fac, msg).items():
        sel = {g["tag"]: (g["name"], g["kind"],
                          plan(fac, g["of"]) if g["kind"] == "message" else None)
               for g in members}
        out.append(("%s_case" % oname, "oneof_case", "oneof_case", (oname, sel)))
    return out


def read(o, plan):
    n = 0
    for name, k, c, child in plan:
        if c == "oneof_case":
            n += 1
            hit = child[1].get(getattr(o, name))
            if hit is not None:
                gn, gk, gchild = hit
                gv = getattr(o, gn)
                n += 1
                if gk == "message":
                    n += read(gv, gchild)
                elif gk in ("string", "bytes"):
                    n += len(gv)
            continue
        v = getattr(o, name)
        n += 1
        if c == "optional":
            # Absent is None and there is nothing under it. The facade's presence test IS
            # the attribute read; the incumbent needs a HasField and then a read, and that
            # asymmetry is protobuf's API rather than something to hide.
            if v is not None and k in ("string", "bytes"):
                n += len(v)
        elif c == "packed":
            for x in v:
                n += 1
        elif c == "map":
            for k2, v2 in v.items():
                n += len(k2) + len(v2)
        elif c == "repeated":
            for x in v:
                n += len(x) if k == "string" else read(x, child)
        elif k == "message":
            if v is not None:
                n += read(v, child)
        elif k in ("string", "bytes"):
            n += len(v)
    return n


def read_pb(o, plan, present=True):
    n = 0
    for name, k, c, child in plan:
        if c == "oneof_case":
            oname, sel = child
            n += 1
            which = o.WhichOneof(oname)
            if which is not None:
                gk, gchild = next((v[1], v[2]) for v in sel.values() if v[0] == which)
                gv = getattr(o, which)
                n += 1
                if gk == "message":
                    n += read_pb(gv, gchild, True)
                elif gk in ("string", "bytes"):
                    n += len(gv)
            continue
        if c == "optional":
            n += 1
            if o.HasField(name):
                v = getattr(o, name)
                if k in ("string", "bytes"):
                    n += len(v)
            continue
        if c == "packed":
            n += 1
            for x in getattr(o, name):
                n += 1
            continue
        if k == "message" and c == "singular":
            # The one place the two readers MUST differ: a protobuf message has no absent
            # representation an attribute read would show, so presence is a HasField call
            # where the facade's is `is None`. Same count of reads either way.
            n += 1
            if o.HasField(name):
                n += read_pb(getattr(o, name), child, True)
            continue
        v = getattr(o, name)
        n += 1
        if c == "map":
            for k2, v2 in v.items():
                n += len(k2) + len(v2)
        elif c == "repeated":
            for x in v:
                n += len(x) if k == "string" else read_pb(x, child, True)
        elif k in ("string", "bytes"):
            n += len(v)
    return n


