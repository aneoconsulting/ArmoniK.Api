"""The ABI's shape, derived from the IR instead of transcribed from the Rust.

**Why this replaced a hand-written table.** `cs_abi.py` used to carry a `FIELDS`
dict spelling out every `ak_efix_*` and `ak_dfix_*` by hand, and
`abi/src/main.rs` carried the same list a second time for the layout probe. Two
hand-maintained transcriptions of a generated Rust file, and M2 alone produced
two defects in them: a vtable slot invented by analogy, and a presence bit
inferred from a field's position. Neither would have been possible if the
declaration were derived.

So this derives it, from the same description, following
`ffi/poc/codec/gen/rust_abi.py`'s rules exactly:

  * `group_fields` -- the by-value group is every SINGULAR, non-oneof field in
    tag order, then per oneof a `<name>_case` discriminant carrying the active
    member's TAG, then every member inlined beside it. Not a union: ABI v1
    section 6 wants a fixed shape, and a union would make the layout depend on
    which member is largest.
  * `presence_bits` -- one bit, in `plain` order, per field whose presence its
    value cannot carry: a singular message child, or an explicit-presence
    scalar or string. A oneof member gets none; `<name>_case` carries it.
  * `loop_slots` -- every repeated, packed or map field of the message OR of
    any singular message it inlines, under an underscore-joined path. ABI v1
    section 6: "a repeated or map field inside an inlined child keeps its loop
    slot and is reached through the parent."

The layout probe is emitted from this too, so "a struct the probe does not
cover" stops being a thing that can happen.
"""

# The Rust spelling of each scalar, and the C# spelling of each Rust one.
RUST = {"int32": "i32", "int64": "i64", "bool": "u8", "double": "f64",
        "enum": "i32", "fixed32": "u32"}
CS = {"i32": "int", "i64": "long", "u8": "byte", "f64": "double", "u32": "uint",
      "usize": "nuint", "ak_str": "ak_str", "ak_span": "ak_span"}


def camel(snake):
    return "".join(p.capitalize() for p in snake.split("_"))


def entry_type(f):
    """The synthetic message a map entry is on the wire: `<Owner><Field>Entry`."""
    return "%s%sEntry" % (f.owner, camel(f.name))


def group_fields(m, enc):
    """The by-value group's fields, in the order the Rust struct declares them."""
    pre = "ak_efix_" if enc else "ak_dfix_"
    blob = "ak_str" if enc else "ak_span"
    out = []
    for f in m.plain():
        if f.card != "singular":
            continue
        if f.kind in ("string", "bytes"):
            out.append((f.name, blob))
        elif f.kind == "message":
            out.append((f.name, pre + f.of))
        else:
            out.append((f.name, RUST[f.kind]))
    for oname, members in m.oneofs.items():
        out.append(("%s_case" % oname, "u32"))
        for g in members:
            n = "%s_%s" % (oname, g.name)
            if g.kind in ("string", "bytes"):
                out.append((n, blob))
            elif g.kind == "message":
                out.append((n, pre + g.of))
            else:
                out.append((n, RUST[g.kind]))
    out.append(("presence", "u32"))
    return out


def presence_bits(m):
    """{field name: bit index}, in `plain` order. Empty is normal and common."""
    bits = {}
    for f in m.plain():
        if (f.kind == "message" and f.card == "singular") or f.presence == "explicit":
            bits[f.name] = len(bits)
    return bits


def presence_const(m, fname, enc):
    return "AK_%sFIX_%s_PRESENT_%s" % ("E" if enc else "D", m.name.upper(), fname.upper())


def loop_slots(ir, name, prefix=()):
    """[(path tuple, Field)] for every slot of `name`'s vtable, in the codec's order."""
    out = []
    for f in ir.msg(name).plain():
        if f.card in ("repeated", "packed", "map"):
            out.append((prefix + (f.name,), f))
        elif f.kind == "message" and f.card == "singular":
            out.extend(loop_slots(ir, f.of, prefix + (f.name,)))
    return out


def slot_name(path):
    return "_".join(path)


def slot_elem(f, enc):
    """One element of a loop slot's run, as a Rust type name."""
    if f.card == "map":
        return ("ak_efix_" if enc else "ak_dfix_") + entry_type(f)
    if f.kind in ("string", "bytes"):
        return "ak_str" if enc else "ak_span"
    if f.card == "packed":
        return RUST[f.kind]
    if f.kind == "message":
        return ("ak_efix_" if enc else "ak_dfix_") + f.of
    raise NotImplementedError("slot %s %s.%s" % (f.card, f.owner, f.name))


def is_leaf(ir, name):
    """ABI v1's batching predicate: no repeated and no map field, transitively.

    A map ENTRY is a message on the wire and not in the description, so it is
    not in `ir.messages`. `map<string, string>` makes it two singular string
    fields, which is a leaf by construction.
    """
    if name not in ir.messages:
        return True
    return ir.msg(name).leaf


def elem_types(ir, roots):
    """Every message that appears as the element of a loop slot, plus map entries.

    These are the ones that get an `ak_elem_*` or `ak_elemu_*` entry point and,
    when they have slots of their own, a vtable.
    """
    out = []
    seen = set()
    for r in roots:
        for name in closure(ir, r):
            for _, f in loop_slots(ir, name):
                t = entry_type(f) if f.card == "map" else (f.of if f.kind == "message" else None)
                if t and t not in seen:
                    seen.add(t)
                    out.append((t, f))
    return out


def closure(ir, name, seen=None):
    """`name` and every message reachable from it."""
    seen = seen if seen is not None else set()
    if name in seen:
        return []
    seen.add(name)
    out = [name]
    for f in ir.msg(name).walk():
        if f.kind == "message":
            out.extend(closure(ir, f.of, seen))
    return out


def structs_for(ir, roots):
    """Every message needing an `ak_efix_`/`ak_dfix_` declaration, in a stable order.

    A map entry is a message on the wire but not in the description, so it is
    synthesised here rather than looked up.
    """
    names, seen = [], set()
    for r in roots:
        for n in closure(ir, r):
            if n not in seen:
                seen.add(n)
                names.append(n)
    return names


def map_entries(ir, roots):
    """[(entry type name, key field, value field)] for every map in the closure."""
    out, seen = [], set()
    for r in roots:
        for n in closure(ir, r):
            for f in ir.msg(n).walk():
                if f.card == "map":
                    t = entry_type(f)
                    if t not in seen:
                        seen.add(t)
                        out.append((t, f))
    return out


def vtable_messages(ir, roots):
    """Messages that get a vtable: every root, and every element type of a
    repeated-message or map slot. **A message that only ever appears as an
    INLINED child does not** -- `TaskOptions` is reached through
    `TaskDetailed`'s vtable and has none of its own. Mirrors
    `ffi/poc/codec/gen/rust_abi.py:vtable_messages`.
    """
    out = list(roots)
    for name in structs_for(ir, roots):
        for _, f in loop_slots(ir, name):
            t = entry_type(f) if f.card == "map" else (f.of if f.kind == "message" else None)
            if t and t not in out:
                out.append(t)
    return out
