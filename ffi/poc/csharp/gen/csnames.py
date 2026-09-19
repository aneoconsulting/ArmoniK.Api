"""C# naming, in one place.

Every backend asks the same questions about a name, and three of the four
defects the Rust slice logged in its generator were a rule spelled differently
at two call sites. So the rule lives here and nowhere else.
"""

# A word the C# compiler will not accept as an identifier. The list is short on
# purpose: these are the ones this schema can actually reach.
KEYWORDS = {
    "abstract", "as", "base", "bool", "break", "byte", "case", "catch", "char",
    "checked", "class", "const", "continue", "decimal", "default", "delegate",
    "do", "double", "else", "enum", "event", "explicit", "extern", "false",
    "finally", "fixed", "float", "for", "foreach", "goto", "if", "implicit",
    "in", "int", "interface", "internal", "is", "lock", "long", "namespace",
    "new", "null", "object", "operator", "out", "override", "params",
    "private", "protected", "public", "readonly", "ref", "return", "sbyte",
    "sealed", "short", "sizeof", "stackalloc", "static", "string", "struct",
    "switch", "this", "throw", "true", "try", "typeof", "uint", "ulong",
    "unchecked", "unsafe", "ushort", "using", "virtual", "void", "volatile",
    "while",
}


def pascal(snake):
    return "".join(p[:1].upper() + p[1:] for p in snake.split("_") if p)


def camel(snake):
    p = pascal(snake)
    return p[:1].lower() + p[1:]


def field(name):
    """A facade field name. Escaped rather than renamed: a renamed field stops
    matching the description, and then two backends disagree about it."""
    n = pascal(name)
    return "@" + n if n in KEYWORDS else n


def enum_member(enum_name, value_name):
    """`RESULT_STATUS_NOTFOUND` in enum `ResultStatus` -> `Notfound`.

    The prefix is stripped the way protoc's C# backend strips it, so the facade
    reads like the incumbent and a reviewer comparing the two is not comparing
    spellings. Where stripping would leave nothing or a leading digit, the full
    name is kept and that is a case the generator must not silently mangle.
    """
    prefix = ""
    for ch in enum_name:
        prefix += ("_" if ch.isupper() and prefix else "") + ch
    prefix = (prefix.upper() + "_")
    v = value_name[len(prefix):] if value_name.startswith(prefix) else value_name
    out = pascal(v.lower())
    if not out or out[0].isdigit():
        out = pascal(value_name.lower())
    return "@" + out if out in KEYWORDS else out
