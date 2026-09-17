"""Deterministic field values.

No RNG anywhere: a value is a pure function of (field path, element index), so
five slices in five languages produce identical bytes by implementing the same
function rather than by sharing a data file. The manifest pins the outcome.
"""
import hashlib
import struct

VOCAB = ["alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel",
         "india", "juliet", "kilo", "lima", "mike", "november", "oscar", "papa"]


def h64(path, idx):
    d = hashlib.sha256(("%s#%d" % (path, idx)).encode()).digest()
    return int.from_bytes(d[:8], "little")


def guid(path, idx):
    """36 ASCII characters, the shape of every id in the real schema."""
    d = hashlib.sha256(("%s#%d" % (path, idx)).encode()).hexdigest()
    return "%s-%s-%s-%s-%s" % (d[0:8], d[8:12], d[12:16], d[16:20], d[20:32])


def word(path, idx):
    h = h64(path, idx)
    return "%s%d" % (VOCAB[h % len(VOCAB)], h % 1000)


def sentence(path, idx):
    h = h64(path, idx)
    return " ".join(VOCAB[(h >> (4 * i)) % len(VOCAB)] for i in range(5))


def blob(path, idx, n=16):
    out = b""
    i = 0
    while len(out) < n:
        out += hashlib.sha256(("%s#%d#%d" % (path, idx, i)).encode()).digest()
        i += 1
    return out[:n]


def bulk(n):
    """A multi-megabyte body, cheap to build and not compressible into a pattern
    a codec could accidentally special-case."""
    out = bytearray()
    i = 0
    while len(out) < n:
        out += hashlib.sha256(b"bulk#%d" % i).digest()
        i += 1
    return bytes(out[:n])


def scalar(kind, path, idx):
    h = h64(path, idx)
    if kind == "int32":
        return h % 100000
    if kind == "int64":
        return h % 1000000000000
    if kind == "bool":
        return bool(h & 1)
    if kind == "double":
        return struct.unpack("<d", struct.pack("<q", (h % 1000000) * 1000))[0] * 0 + (h % 1000000) / 1000.0
    raise KeyError(kind)


def enum_value(values, idx):
    """Cycles through the declared values so that every payload with enough
    elements reaches the large one: ResultStatus 127 is a two-byte varint."""
    vals = list(values.values())
    return vals[idx % len(vals)]


def timestamp(path, idx):
    return {"seconds": 1700000000 + idx * 37, "nanos": (idx * 7919) % 1000000000}


def duration(path, idx):
    return {"seconds": idx % 3600, "nanos": (idx * 104729) % 1000000000}
