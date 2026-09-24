"""C and C++ spellings shared by every C++ backend, so two backends cannot disagree on a
name. FIX-PLAN WP5 step 2: moved here from `poc/cpp/gen/cppnames.py` with the C++ backend.

The C names are fixed by ABI-v1.md and by `plan.py`'s ABI vocabulary; the C++ ones are the
cpp host's. Nothing here decides a wire rule or a layout: `C_OF` maps the plan's abi type
vocabulary to its C spelling, one to one, and `facade_type` is the host's data model.
"""

# The plan's ABI vocabulary (plan.py, "ABI LAYOUT") -> C.
C_OF = {"i32": "int32_t", "i64": "int64_t", "u8": "uint8_t", "f64": "double",
        "u32": "uint32_t", "u64": "uint64_t", "usize": "size_t", "isize": "intptr_t",
        "void": "void"}

# The idiomatic C++ type of a facade scalar (the host's data model, not the ABI's).
FSCALAR = {"int32": "int32_t", "int64": "int64_t", "bool": "bool", "double": "double",
           "fixed32": "uint32_t"}


def cty(abi_ty):
    """The C spelling of one abi type of the plan's vocabulary (a group member, a slot
    element, an RPC struct member or parameter)."""
    t = abi_ty.strip()
    if t.startswith("*const "):
        return "const %s *" % cty(t[len("*const "):])
    if t.startswith("*mut "):
        return "%s *" % cty(t[len("*mut "):])
    if t in C_OF:
        return C_OF[t]
    if t in ("ak_str", "ak_span", "ak_blob", "ak_bytes", "ak_completion", "ak_client_opts",
             "ak_init_opts", "ak_err"):
        return "struct " + t
    if t.startswith(("ak_efix_", "ak_dfix_", "ak_ufix_")):
        return "struct " + t
    if t in ("ak_runtime", "ak_client", "ak_call", "ak_queue", "ak_completion_cb",
             "ak_log_fn"):
        return t
    raise NotImplementedError("REFUSED: no C spelling for abi type %r" % abi_ty)


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


def variant(enum_name, value_name):
    """RESULT_STATUS_CREATED, in enum ResultStatus, is Created."""
    prefix = screaming(enum_name) + "_"
    tail = value_name[len(prefix):] if value_name.startswith(prefix) else value_name
    return "".join(p.capitalize() for p in tail.split("_"))


def oneof_type(msg_name, oneof_name):
    return "%s%s" % (msg_name, camel(oneof_name))


# The borrowed-facade arm (bench's `ffi-borrow`) re-emits the facade and the binding with
# `ak::StringView` in place of `std::string`. ABI v1 already supports it: `ak_span` is an
# OFFSET into the buffer the host handed in (section 4). No ABI change.
STRING_TYPE = ["std::string"]


def set_string_type(t):
    STRING_TYPE[0] = t


def string_type():
    return STRING_TYPE[0]


def facade_type(f):
    """The idiomatic C++ type of one field (a plan.FieldPlan).

    `std::string` for both string and bytes, which is what protobuf C++ itself does. A
    post-C++11 vocabulary type would be OURS (README 5.1.1). A message field that can reach
    its owner again (`FieldPlan.recursive`, the corpus's `Nest`) is held through
    `ak::Box<T>`, because `ak::Optional<T>` needs a complete `T`.
    """
    if f.card == "map":
        return "std::map<%s, %s>" % (STRING_TYPE[0], STRING_TYPE[0])
    base = (STRING_TYPE[0] if f.kind in ("string", "bytes")
            else f.of if f.kind in ("enum", "message")
            else FSCALAR[f.kind])
    if f.card in ("repeated", "packed"):
        return "std::vector<%s>" % base
    if f.kind == "message":
        return ("ak::Box<%s>" if f.recursive else "ak::Optional<%s>") % base
    if f.explicit:
        return "ak::Optional<%s>" % base
    return base
