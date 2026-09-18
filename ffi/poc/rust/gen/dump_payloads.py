"""Write every payload in shapes.json to a directory, as bytes.

`ffi/schema/generated/payloads/` only commits the vectors at or under 64 KB, so eleven of
the sixteen payloads exist in the repository as a sha256 and nothing else. Stage 1
compares bytes rather than hashes, so it needs all sixteen on disk.

This imports the emitters rather than re-deriving them (README R1) and writes
OUTSIDE ffi/schema/, which a slice does not own. Nothing here is edited; the
payload bytes it writes are exactly what `emit/payloads.py` would commit if the
64 KB limit were lifted.

  python3 gen/dump_payloads.py <outdir>
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
EMIT = os.path.abspath(os.path.join(HERE, "..", "..", "..", "schema", "emit"))
sys.path.insert(0, EMIT)

import shapes as S       # noqa: E402
import payloads as E     # noqa: E402


def main(outdir):
    os.makedirs(outdir, exist_ok=True)
    schema = S.load()
    for pid, spec in schema["payloads"].items():
        E.stats["strings"] = E.stats["elements"] = 0
        data = E.build(schema, pid, spec)
        name = pid.replace(".", "_") + ".bin"
        with open(os.path.join(outdir, name), "wb") as f:
            f.write(data)
        print("%-6s %9d B  elements=%-5d strings=%d"
              % (pid, len(data), E.stats["elements"], E.stats["strings"]))


if __name__ == "__main__":
    main(sys.argv[1])
