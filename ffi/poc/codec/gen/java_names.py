"""Java backend: the Java spellings every Java-rendering module shares (FIX-PLAN WP5 step 3).

Naming and facade typing only. No wire rule, no layout member and no traversal lives here:
those are `plan.py`'s. Moved from `poc/java/gen/javanames.py`, which retires; the one
change is that it reads the plan's `FieldPlan` / `MessagePlan` (the descriptor view the
plan carries) and never the IR.

The string type is a parameter of a rendering, not of the description: decision 13's
borrowed facade renders the SAME plan with `ak.Utf8View` in place of `String`. It is held
here for the duration of one rendering (`string_type_as`), so the facade, the codec and the
binding of one package cannot disagree on it.
"""
import contextlib

PKG = "ak.shapes"

# The idiomatic Java type of a facade scalar. `int` for an enum (an open enum must hold a
# value the reader was not built against) and for fixed32 (protobuf-java's choice: the
# value is a u32 carried in an int's 32 bits; the projection prints it unsigned).
FSCALAR = {"int32": "int", "int64": "long", "bool": "boolean", "double": "double",
           "enum": "int", "fixed32": "int"}
ARRAY = {"int32": "int[]", "int64": "long[]", "bool": "boolean[]", "double": "double[]",
         "enum": "int[]", "fixed32": "int[]"}
ZERO = {"int32": "0", "int64": "0L", "bool": "false", "double": "0.0", "enum": "0",
        "fixed32": "0"}
EMPTY_ARRAY = {"int32": "EMPTY_I32", "int64": "EMPTY_I64", "bool": "EMPTY_BOOL",
               "double": "EMPTY_F64", "enum": "EMPTY_I32", "fixed32": "EMPTY_I32"}

# The plan's ABI scalar vocabulary (plan.ABI_SCALAR values) -> the Unsafe accessor that
# reads or writes one group member of that type, and its width. A backend maps each abi
# type to its own spelling (plan contract, ABI LAYOUT); this is the Java spelling.
UNSAFE = {"i32": ("Int", 4), "i64": ("Long", 8), "u8": ("Byte", 1), "f64": ("Double", 8),
          "u32": ("Int", 4)}

_STRING = ["String"]


@contextlib.contextmanager
def string_type_as(t):
    old = _STRING[0]
    _STRING[0] = t
    try:
        yield
    finally:
        _STRING[0] = old


def string_type():
    return _STRING[0]


def is_borrow():
    return _STRING[0] != "String"


def snake(camel_name):
    out = []
    for i, c in enumerate(camel_name):
        if c.isupper() and i:
            out.append("_")
        out.append(c.lower())
    return "".join(out)


def camel(snake_name):
    return "".join(p.capitalize() for p in snake_name.split("_"))


def screaming(camel_name):
    out = []
    for i, c in enumerate(camel_name):
        if c.isupper() and i:
            out.append("_")
        out.append(c.upper())
    return "".join(out)


def const(enum_name, value_name):
    return value_name


def case_const(msg, oname, fname):
    """`Probe.BODY_CASE_AS_INT`: the facade's name for a oneof member's case value, which
    is the member's TAG (the group's `<oneof>_case` carries the active member's tag)."""
    return "%s.%s_CASE_%s" % (msg, screaming(oname), screaming(fname))


def check_map(p, f):
    """Every map in every description is map<string, string>; a facade `TreeMap<S, S>`
    is rendered for exactly that, and anything else RAISES (plan contract: a shape a
    backend cannot render must raise, never be skipped)."""
    e = p.msg(f.entry)
    kinds = sorted((g.name, g.kind) for g in e.fields)
    if kinds != [("key", "string"), ("value", "string")]:
        raise NotImplementedError("map %s.%s: the Java backend renders map<string, string>"
                                  " only, not %r" % (f.owner, f.name, kinds))


def facade_type(f):
    """The idiomatic Java type of one field (see STATE.md for why each choice is one)."""
    if f.card == "map":
        return "java.util.TreeMap<%s, %s>" % (_STRING[0], _STRING[0])
    if f.card == "packed":
        return ARRAY[f.kind]
    base = elem_type_java(f)
    if f.card == "repeated":
        return "java.util.List<%s>" % base
    return base


def elem_type_java(f):
    if f.kind == "string":
        return _STRING[0]
    if f.kind == "bytes":
        return "byte[]"
    if f.kind == "message":
        return f.of
    return FSCALAR[f.kind]


def facade_init(f):
    """Never null for a container; `null` is ABSENT for an explicit string and a message."""
    if f.card == "map":
        return " = new java.util.TreeMap<%s, %s>()" % (_STRING[0], _STRING[0])
    if f.card == "packed":
        return " = %s" % EMPTY_ARRAY[f.kind]
    if f.card == "repeated":
        return " = new java.util.ArrayList<%s>()" % elem_type_java(f)
    if f.kind == "bytes":
        return " = EMPTY_BYTES"
    if f.kind == "string":
        if f.explicit:
            return " = null"
        return " = \"\"" if not is_borrow() else " = %s.EMPTY" % _STRING[0]
    if f.kind == "message":
        return " = null"
    return " = %s" % ZERO[f.kind]


def has_flag(f):
    """Explicit presence on a SCALAR needs a flag; on a string or a message `null` is it."""
    return f.explicit and f.kind not in ("string", "bytes", "message")


def member_type_java(g):
    """A oneof member's facade slot type."""
    if g.kind == "bytes":
        return "byte[]"
    if g.kind == "string":
        return _STRING[0]
    if g.kind == "message":
        return g.of
    return FSCALAR[g.kind]


def member_init(g):
    if g.kind == "bytes":
        return " = EMPTY_BYTES"
    if g.kind in ("string", "message"):
        return " = null"
    return " = %s" % ZERO[g.kind]


# The field the facade carries a message's captured unknown fields in (plan: unknown
# fields retained and re-emitted after the known ones). camelCase, so it can never collide
# with a description's snake_case field name.
UNKNOWN = "unknownFields"


def java_head(p, who):
    return ("// @generated by ffi/poc/codec/gen/%s (plan from %s). Do not edit.\n"
            % (who, getattr(p, "source", "ffi/schema/shapes.json")))


def c_head(p, who):
    return ("/* @generated by ffi/poc/codec/gen/%s (plan from %s). Do not edit. */\n"
            % (who, getattr(p, "source", "ffi/schema/shapes.json")))
