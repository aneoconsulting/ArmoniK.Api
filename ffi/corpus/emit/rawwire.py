"""The wire writer the corpus builds vectors with.

Named `rawwire` and not `wire` so that `import wire` inside it reaches the
schema's writer rather than itself.

It extends `../../schema/emit/wire.py` rather than replacing it -- the schema's
writer is imported and its primitives are used unchanged, so the two directories
cannot drift on varint or length-delimited framing. What is added here is what a
CORPUS needs and a canonical payload generator must never have:

  - a 32-bit fixed field, because shapes.json has no wire-type-5 field at all;
  - the deprecated group pair (wire types 3 and 4), which proto3 cannot express
    and which an unknown-field skipper must still handle;
  - raw injection, so a vector can carry bytes no canonical writer would emit:
    a non-minimal varint, a field out of tag order, a truncated body.

That last group is the point. A corpus made only of things a canonical writer
produces is a corpus that cannot test a reader.
"""
import os
import struct
import sys

sys.path.insert(0, os.path.join(
    os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))),
    "schema", "emit"))
from wire import VARINT, I64, LEN, I32, varint, key, ld, s, i, f64, packed_varint, packed_f64  # noqa: E402,F401

SGROUP, EGROUP = 3, 4


def f32(tag, v):
    """Wire type 5. shapes.json has no field of this wire type, so without it the
    mechanical sweep of README section 10 item 3 covers four of the five wire
    types and calls it every shape."""
    return key(tag, I32) + struct.pack("<I", v & 0xFFFFFFFF)


def group(tag, body):
    """An unknown field of the deprecated group form: SGROUP ... EGROUP, with the
    same field number on both ends and no length anywhere. A skipper that assumes
    every unknown field carries its own length walks off the end of the message."""
    return key(tag, SGROUP) + body + key(tag, EGROUP)


def nonminimal_varint(n, width):
    """A varint padded to `width` bytes with redundant continuation bits.

    Legal on the wire and every conformant parser accepts it; no conformant
    writer emits it. It is here because ABI v1 section 6 refuses to pad a length
    prefix to a learned width for exactly this reason, and a reader that assumes
    minimality is a reader that has never seen one."""
    out = bytearray()
    for _ in range(width - 1):
        out.append((n & 0x7F) | 0x80)
        n >>= 7
    out.append(n & 0x7F)
    if n >> 7:
        raise ValueError("%d does not fit in %d bytes" % (n, width))
    return bytes(out)


def ld_nonminimal(tag, body, width):
    return key(tag, LEN) + nonminimal_varint(len(body), width) + body
