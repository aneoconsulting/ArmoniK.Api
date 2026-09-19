"""C and C++ spellings shared by every backend in this directory, so two backends cannot
disagree on a name. The C names are fixed by ABI-v1.md; the C++ ones are this slice's."""

# The C spelling of a schema scalar inside a group. Must match rust_abi.CSCALAR exactly:
# i32/i64/u8/f64/i32 for int32/int64/bool/double/enum.
CSCALAR = {"int32": "int32_t", "int64": "int64_t", "bool": "uint8_t",
           "double": "double", "enum": "int32_t"}
RUST_TO_C = {"i32": "int32_t", "i64": "int64_t", "u8": "uint8_t", "f64": "double",
             "u32": "uint32_t", "usize": "size_t", "isize": "intptr_t"}

# The idiomatic C++ type of a facade field.
FSCALAR = {"int32": "int32_t", "int64": "int64_t", "bool": "bool", "double": "double"}


def snake(camel):
    out = []
    for i, c in enumerate(camel):
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


def variant(enum_name, value_name):
    """RESULT_STATUS_CREATED, in enum ResultStatus, is Created."""
    prefix = screaming(enum_name) + "_"
    tail = value_name[len(prefix):] if value_name.startswith(prefix) else value_name
    return "".join(p.capitalize() for p in tail.split("_"))


def oneof_type(msg_name, oneof_name):
    return "%s%s" % (msg_name, camel(oneof_name))


# The borrowed-facade arm (bench's `ffi-borrow`) re-emits the facade and the binding with
# `ak::StringView` in place of `std::string`. ABI v1 already supports it: `ak_span` is an
# OFFSET into the buffer the host handed in (section 4), section 7 says the span points
# into that buffer, and 7.4 tells the host to resolve it against the base pointer it
# already holds. So no ABI change, and it is exactly upb's aliasing contract.
STRING_TYPE = ["std::string"]


def set_string_type(t):
    STRING_TYPE[0] = t


def string_type():
    return STRING_TYPE[0]


def facade_type(f):
    """The idiomatic C++ type of one field.

    `std::string` for both string and bytes, which is what protobuf C++ itself does, so the
    incumbent and the facade hold the same representation and no arm is handed a different
    data model. A post-C++11 vocabulary type would be OURS (README 5.1.1); none is needed
    here, because every type below exists at C++11 and means the same thing at every level.
    """
    if f.card == "map":
        return "std::map<%s, %s>" % (STRING_TYPE[0], STRING_TYPE[0])
    base = (STRING_TYPE[0] if f.kind in ("string", "bytes")
            else f.of if f.kind in ("enum", "message")
            else FSCALAR[f.kind])
    if f.card in ("repeated", "packed"):
        return "std::vector<%s>" % base
    if f.kind == "message":
        return "ak::Optional<%s>" % base
    if f.explicit:
        return "ak::Optional<%s>" % base
    return base


# `abi_order_topo` used to live here. It is an ABI-level ordering, not a C++ one -- the
# run-time layout export in `poc/codec/gen/cpp_layout.py` needs the same order the C header
# uses, and there a disagreement is silent rather than a compile error -- so R0 moved the
# one definition into the shared `ir.py` and this line is the re-export every caller in
# this directory already imports.
from ir import abi_order_topo  # noqa: F401,E402
