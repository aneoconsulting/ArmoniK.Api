"""A protobuf writer, small enough to read in one sitting.

It exists because this container has neither protoc nor a protobuf runtime, and
the manifest needs canonical bytes to hash. It is NOT a reference implementation:
the first slice to encode these payloads with prost is what validates it, and
until then every hash in the manifest is provisional. That check is the Rust
slice's first task.

Canonical form, which every slice must reproduce:
  - fields ascending by tag, the one documented exception being DualResponse;
  - an implicit-presence leaf holding the proto zero is omitted;
  - an explicit-presence field is written when set, zero or not;
  - a message field is written when present, empty or not;
  - repeated scalars are always packed;
  - map entries are sorted by key.
"""
import struct

VARINT, I64, LEN, I32 = 0, 1, 2, 5


def varint(n):
    if n < 0:
        n += 1 << 64
    out = bytearray()
    while True:
        b = n & 0x7F
        n >>= 7
        if n:
            out.append(b | 0x80)
        else:
            out.append(b)
            return bytes(out)


def key(tag, wire):
    return varint((tag << 3) | wire)


def ld(tag, body):
    return key(tag, LEN) + varint(len(body)) + body


def s(tag, text):
    return ld(tag, text.encode("utf-8"))


def i(tag, n):
    return key(tag, VARINT) + varint(n)


def f64(tag, v):
    return key(tag, I64) + struct.pack("<d", v)


def packed_varint(tag, values):
    body = b"".join(varint(int(v)) for v in values)
    return ld(tag, body)


def packed_f64(tag, values):
    return ld(tag, b"".join(struct.pack("<d", v) for v in values))
