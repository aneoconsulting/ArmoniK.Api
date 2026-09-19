"""Java spellings shared by every backend in this slice, so two backends cannot disagree
on a name. The C names are fixed by ABI-v1.md and come from the cpp slice's `cppnames`;
the Java ones are this slice's."""

PKG = "ak.shapes"

# The idiomatic Java type of a facade scalar. `int` for an enum: see `facade_type`.
FSCALAR = {"int32": "int", "int64": "long", "bool": "boolean", "double": "double"}
BOXED = {"int32": "Integer", "int64": "Long", "bool": "Boolean", "double": "Double"}
ARRAY = {"int32": "int[]", "int64": "long[]", "bool": "boolean[]", "double": "double[]",
         "enum": "int[]"}
# The Unsafe accessor suffix for a group slot of this kind, matching rust_abi.CSCALAR
# (i32/i64/u8/f64/i32) exactly. A `bool` is ONE byte in the group and four in nothing.
GSCALAR = {"int32": ("Int", 4), "int64": ("Long", 8), "bool": ("Byte", 1),
           "double": ("Double", 8), "enum": ("Int", 4)}

ZERO = {"int32": "0", "int64": "0L", "bool": "false", "double": "0.0",
        "enum": "0"}


def snake(camel_name):
    out = []
    for i, c in enumerate(camel_name):
        if c.isupper() and i:
            out.append("_")
        out.append(c.lower())
    return "".join(out)


def camel(snake_name):
    return "".join(p.capitalize() for p in snake_name.split("_"))


def lcamel(snake_name):
    c = camel(snake_name)
    return c[0].lower() + c[1:]


def screaming(camel_name):
    out = []
    for i, c in enumerate(camel_name):
        if c.isupper() and i:
            out.append("_")
        out.append(c.upper())
    return "".join(out)


def const(enum_name, value_name):
    """RESULT_STATUS_CREATED, in enum ResultStatus, stays RESULT_STATUS_CREATED: a Java
    `public static final int` reads better SCREAMING than camel, and the wire value is the
    only thing that matters here."""
    return value_name


# The borrowed-facade arm (`ffi-borrow`) re-emits the facade and the binding with a view
# type in place of `String`. ABI v1 needs no change: `ak_span` is an OFFSET into the buffer
# the host handed in (section 4) and 7.4 already tells the host to resolve it against a
# base pointer it holds. Open decision 13 is about what the FACADE promises, not the ABI.
STRING_TYPE = ["String"]


def set_string_type(t):
    STRING_TYPE[0] = t


def string_type():
    return STRING_TYPE[0]


def is_borrow():
    return STRING_TYPE[0] != "String"


def facade_type(f):
    """The idiomatic Java type of one field.

    Four choices are decisions rather than transliterations and each is recorded in
    STATE.md, because each one moves a number:

    * an ENUM field is `int`, not a Java `enum`. design/SHAPES.md requires an open enum
      with an unknown value (`status = 999`) to round-trip losslessly, and a Java enum
      cannot hold one without a sentinel that is not the value. protobuf-java has the same
      problem and answers it the same way, with `getStatusValue()` beside `getStatus()`.
    * a PACKED field is a primitive array, not `List<Long>`. ABI v1 section 6 says the
      host hands over "its own array" and that a host which does not store the wire form
      pays a materialising copy; protobuf-java stores `List<Long>` and therefore does.
      Two different storage decisions, so P6.1 compares codecs over different data models
      and is labelled a control twice over.
    * a MAP is a `TreeMap`, because the canonical form sorts entries by key and a facade
      that iterates in hash order cannot produce the manifest's bytes at all.
    * EXPLICIT PRESENCE on a scalar is a primitive plus a `has` flag rather than a boxed
      type, which is what protobuf-java does, so neither arm is handed an allocation the
      other avoids. On a `string` it is plain `null`, which needs no flag: `null` is
      absent and `""` is present-and-empty, and those are different bytes.
    """
    if f.card == "map":
        return "java.util.TreeMap<%s, %s>" % (STRING_TYPE[0], STRING_TYPE[0])
    if f.card == "packed":
        return ARRAY[f.kind]
    base = (STRING_TYPE[0] if f.kind == "string"
            else "byte[]" if f.kind == "bytes"
            else "int" if f.kind == "enum"
            else f.of if f.kind == "message"
            else FSCALAR[f.kind])
    if f.card == "repeated":
        return "java.util.List<%s>" % base
    return base


def facade_init(f):
    """The initialiser a facade field is declared with. Never null for a container: a
    facade whose empty list is null makes every arm test for it, and the absent path is
    already the thing P1.3 exists to exercise."""
    if f.card == "map":
        return " = new java.util.TreeMap<%s, %s>()" % (STRING_TYPE[0], STRING_TYPE[0])
    if f.card == "packed":
        return " = %s" % {"int32": "EMPTY_I32", "int64": "EMPTY_I64", "bool": "EMPTY_BOOL",
                          "double": "EMPTY_F64", "enum": "EMPTY_I32"}[f.kind]
    if f.card == "repeated":
        base = (STRING_TYPE[0] if f.kind == "string" else "byte[]" if f.kind == "bytes"
                else f.of)
        return " = new java.util.ArrayList<%s>()" % base
    if f.kind == "bytes":
        return " = EMPTY_BYTES"
    if f.kind == "string":
        # `null` is ABSENT and `""` is present-and-empty, and they are different bytes.
        # An implicit-presence string has no absent state on the wire, so it starts empty.
        return " = null" if f.explicit else " = \"\"" if not is_borrow() else " = %s.EMPTY" % STRING_TYPE[0]
    if f.kind == "message":
        return " = null"
    if f.explicit:
        return " = %s" % ZERO[f.kind]
    return " = %s" % ZERO[f.kind]


def has_flag(f):
    """Explicit presence on a SCALAR needs a flag; on a string or a message `null` is the
    flag and a second field would be a second thing to get wrong."""
    return f.explicit and f.kind not in ("string", "bytes", "message")


def oneof_case_const(oname, fname):
    return "%s_%s" % (screaming(camel(oname)), screaming(camel(fname)))


def abi_order_topo(ir):
    """`ir.abi_order`, re-sorted so a group's layout is computed after every group it
    inlines. Same order as the cpp slice's, because the run-time layout table the core
    exports is in that order and this slice compares against the same table."""
    out = []
    seen = set()

    def visit(name):
        if name in seen:
            return
        seen.add(name)
        for f in ir.msg(name).fields:
            if f.kind == "message":
                visit(f.of)
            elif f.kind == "map":
                visit(f.entry)
        out.append(name)

    for name in ir.abi_order:
        visit(name)
    return out
