"""Rust spellings shared by every backend, so two backends cannot disagree on a name."""

SCALAR = {"int32": "i32", "int64": "i64", "bool": "bool", "double": "f64", "fixed32": "u32"}


def screaming(camel):
    out = []
    for i, c in enumerate(camel):
        if c.isupper() and i:
            out.append("_")
        out.append(c.upper())
    return "".join(out)


def variant(enum_name, value_name):
    """RESULT_STATUS_CREATED, in enum ResultStatus, is Created."""
    prefix = screaming(enum_name) + "_"
    tail = value_name[len(prefix):] if value_name.startswith(prefix) else value_name
    return "".join(p.capitalize() for p in tail.split("_"))


def camel(snake_name):
    return "".join(p.capitalize() for p in snake_name.split("_"))


def oneof_type(msg_name, oneof_name):
    """The Rust enum a oneof becomes: `Probe`'s `body` is `ProbeBody`."""
    return "%s%s" % (msg_name, camel(oneof_name))


def facade_type(f):
    """The idiomatic Rust type of one field, in the style packages/rust already uses:
    String, bytes::Bytes, a real enum with a lossless Unknown, Option for a message."""
    if f.card == "map":
        return "::std::collections::BTreeMap<String, String>"
    base = ("String" if f.kind == "string"
            else "::bytes::Bytes" if f.kind == "bytes"
            else f.of if f.kind in ("enum", "message")
            else SCALAR[f.kind])
    if f.kind == "message" and f.card == "singular" and getattr(f, "recursive", False):
        # A message that can contain itself (the corpus's `Nest`) needs an indirection to
        # have a size. `plan.FieldPlan.recursive` says which field.
        return "Option<Box<%s>>" % base
    if f.card in ("repeated", "packed"):
        return "Vec<%s>" % base
    if f.kind == "message":
        return "Option<%s>" % base
    if f.explicit:
        return "Option<%s>" % base
    return base
