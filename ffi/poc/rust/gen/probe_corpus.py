#!/usr/bin/env python3
"""FIX-PLAN WP5 step 6: a scratch manifest, in the corpus's own format, of the inputs
`gen/oracle_probe.py` asks the three oracles about, with the verdict and reading of the
majority (upb and protobuf C++; logs/rust/wp5s6-oracles.log). NOT the corpus: the corpus is
the corpus agent's, and these rows are proposed to it through the aggregating session. It
exists so a slice's corpus driver can run them now (`corpus --manifest DIR/manifest.json`).

    gen/probe_corpus.py DIR
"""
import hashlib
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import oracle_probe as O  # noqa: E402

FF = b"\xff"
EXPECT = {
    "varint10-bit63": ("accept", {"v_int64": "-1"}, b"\x10" + FF * 9 + b"\x01"),
    "varint10-bit64": ("accept", {"v_int64": "9223372036854775807"}, b"\x10" + FF * 8 + b"\x7f"),
    "varint10-7f": ("accept", {"v_int64": "-1"}, b"\x10" + FF * 9 + b"\x01"),
    "varint10-over-zero": ("accept", {}, b""),
    "varint10-int32": ("accept", {"v_int32": "-1"}, b"\x08" + FF * 9 + b"\x01"),
    "field-max": ("accept", {"v_big_tag": "1"}, None),
    "field-max+1": ("reject", None, None),
    "field-2^32+2": ("reject", None, None),
    "field-max+1-in-group": ("reject", None, None),
}


def _entry(k, v):
    kb, vb = k.encode(), v.encode()
    body = b"\x0a" + bytes([len(kb)]) + kb + b"\x12" + bytes([len(vb)]) + vb
    return b"\x0a" + bytes([len(body)]) + body


# The map-entry ORDER rule (plan ENCODE RULES, WP5 step 6): U+E000 (EE 80 80) sorts BEFORE
# U+10000 (F0 90 80 80) in UTF-8 byte / code-point order, and AFTER it in UTF-16 code-unit
# order (D800 DC00), so a String map ordered by UTF-16 units writes them reversed. Root
# TaskOptions (map<string, string> options = 1). Both inputs must re-encode SORTED.
K1, K2 = "\ue000", "\U00010000"
MAP_SORTED = _entry(K1, "a") + _entry(K2, "b")
MAP_REVERSED = _entry(K2, "b") + _entry(K1, "a")
EXTRA = {
    "map-order-sorted": ("TaskOptions", MAP_SORTED, {"options": {K1: "a", K2: "b"}}, MAP_SORTED),
    "map-order-reversed": ("TaskOptions", MAP_REVERSED, {"options": {K1: "a", K2: "b"}}, MAP_SORTED),
}


def main(d):
    os.makedirs(os.path.join(d, "vectors"), exist_ok=True)
    os.makedirs(os.path.join(d, "projections"), exist_ok=True)
    cases = {c[0]: c[1] for c in O.CASES}
    rows = {}
    for cid, (e, proj, re) in EXPECT.items():
        fid = cid.replace("^", "p").replace("+", "plus")
        open(os.path.join(d, "vectors", fid + ".bin"), "wb").write(cases[cid])
        r = {"class": "probe", "root": "WireZoo", "expect": e, "file": "vectors/%s.bin" % fid,
             "verdict": "agreed", "permutation_accepted": False}
        if e == "accept":
            json.dump(proj, open(os.path.join(d, "projections", fid + ".json"), "w"))
            r["projection"] = "projections/%s.json" % fid
            enc = cases[cid] if re is None else re
            r["accepted_encodings"] = [{"sha256": hashlib.sha256(enc).hexdigest(), "bytes": len(enc),
                                        "forms": ["canonical (as upb re-encodes it)"]}]
        rows["P-" + fid] = r
    for cid, (root, b, proj, enc) in EXTRA.items():
        open(os.path.join(d, "vectors", cid + ".bin"), "wb").write(b)
        json.dump(proj, open(os.path.join(d, "projections", cid + ".json"), "w"))
        rows["P-" + cid] = {"class": "probe", "root": root, "expect": "accept",
                            "file": "vectors/%s.bin" % cid, "verdict": "agreed",
                            "permutation_accepted": False,
                            "projection": "projections/%s.json" % cid,
                            "accepted_encodings": [{"sha256": hashlib.sha256(enc).hexdigest(), "bytes": len(enc),
                                                    "forms": ["canonical: entries ascending by UTF-8 key"]}]}
    json.dump({"vectors": rows}, open(os.path.join(d, "manifest.json"), "w"), indent=1)
    print("%d rows in %s" % (len(rows), os.path.join(d, "manifest.json")))


if __name__ == "__main__":
    main(sys.argv[1])
