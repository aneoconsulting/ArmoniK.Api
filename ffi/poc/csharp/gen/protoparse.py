"""A proto3 front end for the IR, so this slice can read `ffi/corpus`.

**Why this exists, which is the only interesting thing about it.**
`corpus/CONTRACT.md`'s rule 0 is "generate your codec from
`generated/corpus.proto`, never from `generated/corpus_superset.proto`", and it
is the rule that makes the whole corpus mean anything: a slice that generates
from the superset knows every field, executes no unknown-field skip, and passes
class `unknown` while testing nothing.

This generator had no `.proto` front end at all. It drives off
`ffi/schema/emit/shapes.py`, which reads a JSON description. So conformance was
not a matter of pointing the harness at a directory; it needed a second front
end that produces the SAME schema dict `shapes.json` holds, so that every
backend already written -- the facade types, the structural comparer, the
managed codec -- emits against the corpus's reader view without being touched.
That is what this is. It parses one file and emits a dict; it is deliberately
not a general protobuf parser and it raises on anything it was not written for.

What it must NOT become: a parser lenient enough to swallow the superset. There
is no `--superset` switch and there should never be one.
"""
import re

SCALARS = {
    "int32": "int32", "int64": "int64", "bool": "bool", "double": "double",
    "string": "string", "bytes": "bytes", "fixed32": "fixed32",
}

# proto3 packs a repeated numeric field by default and cannot pack a
# length-delimited one. `packed` here is the IR's word for "repeated scalar or
# enum", which is what the codec's packed case is conditioned on.
PACKABLE = {"int32", "int64", "bool", "double", "fixed32"}


class ProtoError(Exception):
    """A construct this front end was not written for. Raised, never skipped."""


def _strip_comments(text):
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    return "\n".join(l.split("//", 1)[0] for l in text.splitlines())


def parse(path):
    """`corpus.proto` to the dict shape `shapes.json` holds."""
    src = _strip_comments(open(path).read())
    enums, messages = {}, {}

    # Pass 1: every enum, because a field's type tells message from enum only
    # once the enum names are known.
    for m in re.finditer(r"\benum\s+(\w+)\s*\{(.*?)\}", src, flags=re.S):
        name, body = m.group(1), m.group(2)
        values = {}
        for vm in re.finditer(r"(\w+)\s*=\s*(-?\d+)\s*;", body):
            values[vm.group(1)] = int(vm.group(2))
        enums[name] = {"source": "corpus.proto", "values": values}

    # Pass 2: every message. Bodies are extracted by brace matching rather than
    # by a regex, because `oneof` and `map<>` both nest.
    for start in [m.start() for m in re.finditer(r"\bmessage\s+\w+\s*\{", src)]:
        head = re.match(r"message\s+(\w+)\s*\{", src[start:])
        name = head.group(1)
        i = start + head.end() - 1
        depth, j = 0, i
        while j < len(src):
            if src[j] == "{":
                depth += 1
            elif src[j] == "}":
                depth -= 1
                if depth == 0:
                    break
            j += 1
        messages[name] = {"source": "corpus.proto",
                          "fields": _fields(src[i + 1:j], enums, name)}

    return {"version": 1, "package": "corpus", "note": "parsed from corpus.proto",
            "enums": enums, "messages": messages}


def _fields(body, enums, owner):
    out = []
    # The oneof blocks first, removed from the body so the plain scan does not
    # see their members twice. Tags stay in declaration order overall because
    # every backend sorts by tag where the wire cares.
    def take_oneof(m):
        oname = m.group(1)
        for f in _plain(m.group(2), enums, owner):
            f["oneof"] = oname
            if f.get("card") or f["kind"] == "map":
                raise ProtoError("%s.%s: a repeated or map oneof member" % (owner, f["name"]))
            out.append(f)
        return ""

    rest = re.sub(r"\boneof\s+(\w+)\s*\{(.*?)\}", take_oneof, body, flags=re.S)
    out.extend(_plain(rest, enums, owner))
    out.sort(key=lambda f: f["tag"])
    return out


FIELD = re.compile(
    r"^\s*(?:(repeated|optional)\s+)?"
    r"(map\s*<\s*(\w+)\s*,\s*(\w+)\s*>|[\w.]+)\s+"
    r"(\w+)\s*=\s*(\d+)\s*;",
    flags=re.M)


def _plain(body, enums, owner):
    out = []
    consumed = 0
    for m in FIELD.finditer(body):
        consumed += 1
        label, typ, mk, mv, fname, tag = m.groups()
        f = {"name": fname, "tag": int(tag)}
        if typ.startswith("map"):
            if label:
                raise ProtoError("%s.%s: a labelled map field" % (owner, fname))
            if (mk, mv) != ("string", "string"):
                raise ProtoError("%s.%s: only map<string, string> has a case" % (owner, fname))
            f.update(kind="map", key=mk, value_kind=mv)
            out.append(f)
            continue
        if typ in SCALARS:
            f["kind"] = SCALARS[typ]
        elif typ in enums:
            f.update(kind="enum", of=typ)
        else:
            f.update(kind="message", of=typ)
        if label == "repeated":
            # proto3's default: numerics and enums pack, everything else does not.
            f["card"] = "packed" if (f["kind"] in PACKABLE or f["kind"] == "enum") else "repeated"
        elif label == "optional":
            f["presence"] = "explicit"
        out.append(f)
    # A body line that looks like a declaration and did not parse is a construct
    # this front end does not know, and silently dropping a field is exactly the
    # failure mode rule 0 exists to prevent.
    for line in body.splitlines():
        s = line.strip()
        if not s or s.startswith(("reserved", "option", "extensions")):
            continue
        if "=" in s and s.endswith(";") and not FIELD.match(line):
            raise ProtoError("%s: unparsed declaration %r" % (owner, s))
    return out
