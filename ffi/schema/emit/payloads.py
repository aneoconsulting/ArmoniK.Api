"""Build every payload from shapes.json and write generated/manifest.json.

Each payload gets its byte length and a sha256 of the canonical encoding. Two
slices disagreeing on either is a defect under R1, not a difference.

Payloads at or under 64 KB are also written to generated/payloads/ as bytes, so a slice
has something to diff against without running this. The larger ones are hashes
only: a 4 MB vector in git buys nothing that a hash does not.
"""
import hashlib
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import shapes as S
import values as V
import wire as W

COMMIT_LIMIT = 64 * 1024

stats = {"strings": 0, "elements": 0}


def enc_message(schema, name, path, idx, mode=None, repeats=3, bulk=None):
    """Return the encoded body of one message. `path` makes every value unique
    to its field, and `idx` to its element."""
    m = schema["messages"][name]
    out = bytearray()
    for f in S.fields(m):
        fpath = "%s.%s" % (path, f["name"])
        piece = enc_field(schema, f, fpath, idx, mode, repeats, bulk)
        if piece:
            out += piece
    return bytes(out)


def explicit_present(f, idx):
    """An explicit-presence field that is always written never exercises the
    thing it exists for. Presence cycles per field, and one element in seven
    carries a present-but-zero value, which is the case a by-value group cannot
    distinguish from absent unless it is designed to."""
    return (idx % (f["tag"] + 1)) != 0


def absent(f, mode, idx):
    """Which fields the absent-path modes remove. The point of these payloads is
    that a generator filling every field never reaches the code that handles an
    empty one."""
    if mode == "all_absent":
        return True
    if mode == "half_absent":
        if f.get("adapter_site") == "nested":
            return True
        if f["kind"] == "message" and f["of"] == "Timestamp" and f["tag"] % 2 == 0:
            return True
    return False


def enc_field(schema, f, path, idx, mode, repeats, bulk):
    kind, c = f["kind"], S.card(f)
    if absent(f, mode, idx):
        return b""

    if c == "map":
        n = f.get("entries", 4)
        entries = []
        for k in range(n):
            mk = "k%02d-%s" % (k, V.word(path + ".key", idx * 31 + k))
            mv = "" if (mode == "half_absent" and k % 2 == 0) else V.word(path + ".value", idx * 31 + k)
            entries.append((mk, mv))
        entries.sort(key=lambda kv: kv[0])
        out = bytearray()
        for mk, mv in entries:
            body = W.s(1, mk) + (W.s(2, mv) if mv else b"")
            out += W.ld(f["tag"], body)
            stats["strings"] += 2
        return bytes(out)

    if c == "packed":
        n = f.get("count", 30)
        vals = [V.scalar(kind, path, idx * 97 + j) for j in range(n)]
        if kind == "double":
            return W.packed_f64(f["tag"], vals)
        return W.packed_varint(f["tag"], vals)

    if c == "repeated":
        n = repeats
        if kind == "string":
            out = bytearray()
            for j in range(n):
                out += W.s(f["tag"], V.guid(path, idx * 211 + j))
                stats["strings"] += 1
            return bytes(out)
        if kind == "message":
            out = bytearray()
            for j in range(n):
                out += W.ld(f["tag"], enc_message(schema, f["of"], path, j, mode, repeats, bulk))
            return bytes(out)
        raise NotImplementedError(c + kind)

    # singular
    if kind == "string":
        if mode == "all_absent":
            return b""
        if f.get("presence") == "explicit":
            if not explicit_present(f, idx):
                return b""
            if idx % 7 == 0:
                stats["strings"] += 1
                return W.s(f["tag"], "")      # present AND empty, which is not absent
        text = {"guid": V.guid, "word": V.word, "sentence": V.sentence}.get(f.get("value", "word"))(path, idx)
        stats["strings"] += 1
        return W.s(f["tag"], text)
    if kind == "bytes":
        if mode == "all_absent":
            return b""
        body = V.bulk(bulk) if f.get("value") == "bulk" and bulk is not None else V.blob(path, idx)
        return W.ld(f["tag"], body)
    if kind == "enum":
        v = V.enum_value(schema["enums"][f["of"]]["values"], idx)
        return W.i(f["tag"], v) if v else b""
    if kind in ("int32", "int64", "bool"):
        v = V.scalar(kind, path, idx)
        if f.get("presence") == "explicit":
            if not explicit_present(f, idx):
                return b""
            return W.i(f["tag"], 0 if idx % 7 == 0 else int(v))
        return W.i(f["tag"], int(v)) if v else b""
    if kind == "double":
        v = V.scalar(kind, path, idx)
        return W.f64(f["tag"], v) if v else b""
    if kind == "message":
        if f["of"] == "Timestamp":
            t = V.timestamp(path, idx)
            return W.ld(f["tag"], W.i(1, t["seconds"]) + W.i(2, t["nanos"]))
        if f["of"] == "Duration":
            t = V.duration(path, idx)
            return W.ld(f["tag"], W.i(1, t["seconds"]) + W.i(2, t["nanos"]))
        return W.ld(f["tag"], enc_message(schema, f["of"], path, idx, mode, repeats, bulk))
    raise NotImplementedError(kind)


def oneof_member(schema, msg_name, idx):
    """A oneof carries exactly one member, cycled by element index so that every
    variant including the payload-free one is reached."""
    m = schema["messages"][msg_name]
    ofs = S.oneofs(m)
    if not ofs:
        return b""
    members = list(ofs.values())[0]
    f = members[idx % len(members)]
    path = "%s.%s" % (msg_name, f["name"])
    if f["kind"] == "string":
        return W.s(f["tag"], V.word(path, idx))
    if f["kind"] == "bytes":
        return W.ld(f["tag"], V.blob(path, idx))
    if f["kind"] == "int64":
        return W.i(f["tag"], V.scalar("int64", path, idx))
    if f["kind"] == "message" and f["of"] == "Empty":
        return W.ld(f["tag"], b"")        # the payload-free member: present, empty
    if f["kind"] == "message":
        t = V.timestamp(path, idx)
        return W.ld(f["tag"], W.i(1, t["seconds"]) + W.i(2, t["nanos"]))
    raise NotImplementedError(f["kind"])


def enc_element(schema, name, idx, mode, repeats):
    body = bytearray()
    m = schema["messages"][name]
    for f in S.fields(m):
        if "oneof" in f:
            continue
        body += enc_field(schema, f, "%s.%s" % (name, f["name"]), idx, mode, repeats, None)
    body += oneof_member(schema, name, idx)
    return bytes(body)


def build(schema, pid, spec):
    root = schema["messages"][spec["root"]]
    out = bytearray()

    if "bulk" in spec:                                   # P5.x
        f = S.fields(root)[0]
        inner = enc_message(schema, f["of"], f["of"], 0, None, 3, spec["bulk"])
        out += W.ld(f["tag"], inner)
        stats["elements"] += 1
        return bytes(out)

    if spec.get("interleaved"):                          # P7.1
        left, right = S.fields(root)[0], S.fields(root)[1]
        for j in range(spec["count"]):
            for f in (left, right):
                out += W.ld(f["tag"], enc_element(schema, f["of"], j, None, 3))
                stats["elements"] += 1
        return bytes(out)

    field = next(f for f in S.fields(root) if f["name"] == spec["field"])
    repeats = spec.get("repeats", 3)
    for j in range(spec["count"]):
        r = repeats[j % len(repeats)] if isinstance(repeats, list) else repeats
        out += W.ld(field["tag"], enc_element(schema, field["of"], j, spec.get("mode"), r))
        stats["elements"] += 1
    for f in S.fields(root):                             # page, total
        if f["name"] in ("page", "total"):
            out += W.i(f["tag"], 1 if f["name"] == "page" else spec["count"])
    return bytes(out)


def main():
    schema = S.load()
    outdir = os.path.join(S.ROOT, "generated")
    pdir = os.path.join(outdir, "payloads")
    os.makedirs(pdir, exist_ok=True)

    manifest = {
        "generated_by": "emit/payloads.py from shapes.json",
        "status": "PROVISIONAL: produced by emit/wire.py, which no protobuf "
                  "implementation has yet checked. The Rust slice validates it "
                  "with prost before any other slice trusts a hash.",
        "canonical_form": [
            "fields ascending by tag, DualResponse excepted (it interleaves on purpose)",
            "an implicit-presence leaf holding the proto zero is omitted",
            "an explicit-presence field is written when set, zero or not",
            "a message field is written when present, empty or not",
            "repeated scalars are always packed",
            "map entries are sorted by key",
        ],
        "payloads": {},
    }

    for pid, spec in schema["payloads"].items():
        stats["strings"] = stats["elements"] = 0
        data = build(schema, pid, spec)
        row = {
            "root": spec["root"],
            "elements": stats["elements"],
            "strings": stats["strings"],
            "bytes": len(data),
            "sha256": hashlib.sha256(data).hexdigest(),
        }
        if spec.get("mode"):
            row["mode"] = spec["mode"]
        if len(data) <= COMMIT_LIMIT:
            name = pid.replace(".", "_") + ".bin"
            with open(os.path.join(pdir, name), "wb") as f:
                f.write(data)
            row["vector"] = "payloads/" + name
        manifest["payloads"][pid] = row

    with open(os.path.join(outdir, "manifest.json"), "w") as f:
        json.dump(manifest, f, indent=2)
        f.write("\n")

    w = max(len(p) for p in manifest["payloads"])
    print("%-*s %10s %9s %8s  %s" % (w, "payload", "bytes", "elements", "strings", "sha256"))
    for pid, row in manifest["payloads"].items():
        print("%-*s %10d %9d %8d  %s" % (w, pid, row["bytes"], row["elements"],
                                         row["strings"], row["sha256"][:16]))


if __name__ == "__main__":
    main()
