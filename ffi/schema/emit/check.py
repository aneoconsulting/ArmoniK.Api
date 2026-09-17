"""Walk every generated payload as raw protobuf and prove the framing holds.

This is not a semantic check and it cannot be one: there is no protobuf
implementation in this container to check against. What it does catch is the
class of defect emit/wire.py could plausibly have, which is a length that does not
match its body. It descends into every length-delimited field that parses as a
message, so a bad length anywhere fails here rather than in a slice.
"""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import shapes as S

VARINT, I64, LEN, I32 = 0, 1, 2, 5


def read_varint(b, i):
    n = shift = 0
    while True:
        if i >= len(b):
            raise ValueError("varint runs off the end")
        c = b[i]
        n |= (c & 0x7F) << shift
        i += 1
        if not c & 0x80:
            return n, i
        shift += 7
        if shift > 63:
            raise ValueError("varint too long")


def walk(b, depth=0, counts=None):
    """Returns the number of fields seen. Raises on any framing error."""
    counts = counts if counts is not None else {"fields": 0, "nested": 0, "maxdepth": 0}
    counts["maxdepth"] = max(counts["maxdepth"], depth)
    i = 0
    while i < len(b):
        k, i = read_varint(b, i)
        tag, wire = k >> 3, k & 7
        if tag == 0:
            raise ValueError("tag 0 is not legal")
        counts["fields"] += 1
        if wire == VARINT:
            _, i = read_varint(b, i)
        elif wire == I64:
            i += 8
        elif wire == I32:
            i += 4
        elif wire == LEN:
            n, i = read_varint(b, i)
            if i + n > len(b):
                raise ValueError("length %d overruns the buffer at %d" % (n, i))
            body = b[i:i + n]
            i += n
            if n and looks_like_message(body):
                counts["nested"] += 1
                walk(body, depth + 1, counts)
        else:
            raise ValueError("wire type %d is not legal" % wire)
        if i > len(b):
            raise ValueError("field body overruns the buffer")
    if i != len(b):
        raise ValueError("trailing bytes")
    return counts


def looks_like_message(b):
    try:
        walk(b)
        return True
    except ValueError:
        return False


def main():
    out = os.path.join(S.ROOT, "generated")
    manifest = json.load(open(os.path.join(out, "manifest.json")))
    import hashlib
    bad = 0
    import payloads as E
    schema = S.load()
    for pid, row in manifest["payloads"].items():
        if "vector" in row:
            data = open(os.path.join(out, row["vector"]), "rb").read()
        else:
            # no committed vector: rebuild it in memory so that every payload is
            # framing-checked, not just the small ones
            E.stats["strings"] = E.stats["elements"] = 0
            data = E.build(schema, pid, schema["payloads"][pid])
        digest = hashlib.sha256(data).hexdigest()
        try:
            c = walk(data)
        except ValueError as e:
            print("%-6s FRAMING: %s" % (pid, e))
            bad += 1
            continue
        ok = (digest == row["sha256"] and len(data) == row["bytes"])
        print("%-6s %-5s %-8s fields=%-7d nested=%-7d depth=%d" %
              (pid, "ok" if ok else "HASH", "" if "vector" in row else "(rebuilt)",
               c["fields"], c["nested"], c["maxdepth"]))
        bad += 0 if ok else 1
    print("\n%d payload(s) with a problem" % bad)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
