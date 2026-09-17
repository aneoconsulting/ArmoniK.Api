"""Emit shapes.proto from shapes.json.

The .proto is an OUTPUT, not a source. Editing generated/shapes.proto is a defect:
change shapes.json and re-run this.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import shapes as S

PROTO_KIND = {"int32": "int32", "int64": "int64", "bool": "bool",
              "double": "double", "string": "string", "bytes": "bytes"}


def field_line(schema, f, indent="  "):
    if f["kind"] == "map":
        t = "map<%s, %s>" % (f["key"], f["value_kind"])
        return "%s%s %s = %d;" % (indent, t, f["name"], f["tag"])
    t = PROTO_KIND.get(f["kind"]) or f["of"]
    prefix = ""
    if S.card(f) in ("repeated", "packed"):
        prefix = "repeated "
    elif f.get("presence") == "explicit":
        prefix = "optional "
    return "%s%s%s %s = %d;" % (indent, prefix, t, f["name"], f["tag"])


def emit(schema):
    out = ['syntax = "proto3";', "", "package %s;" % schema["package"], ""]
    out += ["// Generated from shapes.json by emit/proto.py. Do not edit.",
            "// The well-known types are copied rather than imported, so this file has no",
            "// include path to get right in five toolchains.", ""]

    for name, e in schema["enums"].items():
        out.append("enum %s {" % name)
        for k, v in e["values"].items():
            out.append("  %s = %d;" % (k, v))
        out += ["}", ""]

    for name, m in schema["messages"].items():
        if m.get("source"):
            out.append("// %s" % m["source"])
        out.append("message %s {" % name)
        seen_oneof = set()
        ofs = S.oneofs(m)
        for f in S.fields(m):
            o = f.get("oneof")
            if o:
                if o in seen_oneof:
                    continue
                seen_oneof.add(o)
                out.append("  oneof %s {" % o)
                for g in ofs[o]:
                    out.append(field_line(schema, g, "    "))
                out.append("  }")
            else:
                out.append(field_line(schema, f))
        out += ["}", ""]
    return "\n".join(out)


if __name__ == "__main__":
    schema = S.load()
    dst = os.path.join(S.ROOT, "generated", "shapes.proto")
    with open(dst, "w") as f:
        f.write(emit(schema))
    print("wrote", dst)
