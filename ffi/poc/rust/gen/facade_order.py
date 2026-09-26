"""FIX-PLAN R-H12: the `armonik` arm's prost impl writes fields in TAG order, oneof members
included, because it renders the plan's encode steps (poc/codec/gen/rust_facade.py).

shapes.json cannot show it: every oneof tag there is above every plain tag, so "oneofs
after plain fields" and "tag order" write the same bytes. This plants, in memory, a
description where they differ -- a plain field (tag 20) above Probe's oneof (tags 10-14) --
and reads the order of the rendered `encode_raw`. The old renderer (oneofs last) is shown
beside it when its source is given as the first argument.

Nothing under ffi/schema/ is written.
"""
import importlib.util
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.abspath(os.path.join(HERE, "..", "..", "codec", "gen")))

import plan as P          # noqa: E402
import rust_facade        # noqa: E402

TAG = re.compile(r"::(?:\w+)::encode(?:_packed|_repeated)?\((\d+),")


def order(module, p, name):
    text = module.emit_prost_impl(p)
    body = text.split("impl ::prost::Message for %s {" % name, 1)[1]
    body = body.split("fn encode_raw", 1)[1].split("\n    fn ", 1)[0]
    return [int(t) for t in TAG.findall(body)]


def main(argv):
    sc = P.shapes_schema()
    sc["messages"]["Probe"]["fields"].append({"name": "late", "tag": 20, "kind": "int32"})
    p = P.load_schema(sc, ["ListProbeResponse"])
    got = order(rust_facade, p, "Probe")
    want = sorted(got)
    print("# R-H12: Probe with a plain field (20) above its oneof (10-14)")
    print("plan-rendered encode_raw tag order: %s" % got)
    if len(argv) > 1:
        spec = importlib.util.spec_from_file_location("old_facade", argv[1])
        old = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(old)
        print("pre-R-H12 renderer's order:          %s" % order(old, p, "Probe"))
    shipped = order(rust_facade, P.load(["ListProbeResponse"]), "Probe")
    print("shapes.json Probe (unchanged bytes):  %s" % shipped)
    ok = got == want and 20 in got and shipped == sorted(shipped)
    print("tag order: %s" % ("yes" if ok else "NO"))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
