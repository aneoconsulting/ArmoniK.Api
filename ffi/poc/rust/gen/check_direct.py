"""ABI v1 section 8's generator-time refusal, exercised.

Section 8 says a direct-argument field declared on a message that makes a reverse call
"should be a generator-time refusal and currently is not". It is now, and this proves it
fires rather than asserting it: the schema is copied in memory, a direct field is added
where it must not be, and the generator is asked to accept it.

Nothing under ffi/schema/ is written.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
# The refusal is the plan layer's (FIX-PLAN WP5): this test asks `plan.py`, not the IR.
sys.path.insert(0, os.path.abspath(os.path.join(HERE, "..", "..", "codec", "gen")))

import plan as P      # noqa: E402

CASES = [
    ("a direct field on a message that needs a reverse call",
     "TaskDetailed", "ListTasksDetailedResponse"),
    ("a second direct field in the same tree",
     "UploadResultData", "UploadResultDataMessage"),
]


def main():
    bad = 0
    print("# ABI v1 section 8: the generator-time refusal")
    print()
    print("%-52s %s" % ("case", "outcome"))
    # The one configuration that MUST be accepted.
    ir = P.load(["UploadResultDataMessage"])
    try:
        P.check_direct(ir, "UploadResultDataMessage")
        print("%-52s %s" % ("M5 as the schema declares it", "accepted, as it must be"))
    except NotImplementedError as e:
        print("%-52s %s" % ("M5 as the schema declares it", "WRONGLY REFUSED: %s" % e))
        bad += 1

    for label, msg, root in CASES:
        sc = P.shapes_schema()
        sc["messages"][msg]["fields"].append(
            {"name": "payload_blob", "tag": 99, "kind": "bytes", "value": "bulk"})
        i = P.load_schema(sc, [root])
        try:
            P.check_direct(i, root)
            print("%-52s NOT REFUSED -- the check does not work" % label)
            bad += 1
        except NotImplementedError as e:
            print("%-52s refused" % label)
            print("%-52s   %s" % ("", str(e).replace("\n", " ")[:150]))

    # FIX-PLAN R-H15: a packed fixed32 has no run symbol (no ak_run_u32), so
    # plan.check_expressible refuses it at generator time, before any backend renders.
    print()
    print("# ABI v1 section 6: packed kinds without a run symbol (plan.check_expressible)")
    try:
        P.check_expressible(P.load(["ListMetricsResponse"]), "ListMetricsResponse")
        print("%-52s %s" % ("M6 as the schema declares it", "accepted, as it must be"))
    except NotImplementedError as e:
        print("%-52s %s" % ("M6 as the schema declares it", "WRONGLY REFUSED: %s" % e))
        bad += 1
    sc = P.shapes_schema()
    sc["messages"]["MetricsBatch"]["fields"].append(
        {"name": "crc32s", "tag": 99, "kind": "fixed32", "card": "packed", "count": 3})
    i = P.load_schema(sc, ["ListMetricsResponse"])
    label = "a packed fixed32 in M6's element"
    try:
        P.check_expressible(i, "ListMetricsResponse")
        print("%-52s NOT REFUSED -- the check does not work" % label)
        bad += 1
    except NotImplementedError as e:
        print("%-52s refused" % label)
        print("%-52s   %s" % ("", str(e).replace("\n", " ")[:150]))
    print()
    print("%d case(s) wrong" % bad)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
