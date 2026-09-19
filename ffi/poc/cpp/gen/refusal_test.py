#!/usr/bin/env python3
"""Every guard in this generator, run against input it is supposed to REJECT.

README R1: "a backend that has no case for a shape raises rather than skips", and
"a refusal is tested by a case that must fail. A guard with no failing test is a guard
nobody has seen work." The rust slice's first version of ABI v1 section 8's refusal walked
singular message children only, found nothing, refused nothing, and read as working.

Three groups of must-fail cases:

  A. ABI v1 section 8's direct-argument refusal, over THIS slice's invocation -- that the
     loop in `generate.py` covers every root and runs before a line is emitted.
  B. Shapes no backend has a case for: a repeated `bytes` and an UNPACKED repeated enum.
     Before this, `cpp_build` and `cpp_pbbuild` stopped testing cardinality after the
     repeated-string arm, so a repeated `bytes` fell into the singular-`bytes` arm and
     emitted a scalar store against a `std::vector`. No instance exists in `shapes.json`,
     which is exactly why it needed a test rather than a reading.
  C. A map half that is not a blob, which the map transcoder derivation must refuse.

And one positive control on the whole thing: the real schema must still emit.
"""
import copy
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
RUSTGEN = os.path.abspath(os.path.join(HERE, "..", "..", "rust", "gen"))
# HERE first: both directories contain a `generate.py`. See the note in generate.py.
sys.path.insert(0, RUSTGEN)
sys.path.insert(0, HERE)

import ir as IR                 # noqa: E402
import shapes as S              # noqa: E402
import generate                 # noqa: E402

FAILS = []
OKS = []


def must_raise(name, fn):
    try:
        fn()
    except NotImplementedError as e:
        OKS.append((name, str(e).split("\n")[0][:150]))
        return
    except Exception as e:                       # noqa: BLE001
        FAILS.append((name, "raised %s, not NotImplementedError: %s" % (type(e).__name__, e)))
        return
    FAILS.append((name, "DID NOT RAISE -- this guard has never been seen working"))


def must_not_raise(name, fn):
    try:
        fn()
        OKS.append((name, "emitted"))
    except Exception as e:                       # noqa: BLE001
        FAILS.append((name, "raised on valid input: %s: %s" % (type(e).__name__, e)))


def ir_with(mutate):
    schema = copy.deepcopy(S.load())
    mutate(schema)
    return IR.Ir(schema, generate.ROOTS)


def emit_all(ir):
    for root in ir.roots:
        IR.check_direct(ir, root)
    generate.targets(ir)


# ---- A: ABI v1 section 8's direct-argument refusal --------------------------------

def a_direct_plus_reverse_call(schema):
    """A direct-argument field on a tree that also needs a reverse call. A critical
    section and an upcall are mutually exclusive, so no host can honour both."""
    schema["messages"]["TaskDetailed"]["fields"].append(
        {"name": "bulk_blob", "tag": 99, "kind": "bytes", "value": "bulk"})


def a_two_direct_fields(schema):
    """More than one direct field in one tree: the path is built for ONE field."""
    schema["messages"]["UploadResultData"]["fields"].append(
        {"name": "second_chunk", "tag": 9, "kind": "bytes", "value": "bulk"})


# ---- B: shapes no backend has a case for ------------------------------------------

def b_repeated_bytes(schema):
    schema["messages"]["ResultRaw"]["fields"].append(
        {"name": "extra_blobs", "tag": 40, "kind": "bytes", "card": "repeated"})


def b_unpacked_repeated_enum(schema):
    schema["messages"]["ResultRaw"]["fields"].append(
        {"name": "extra_statuses", "tag": 41, "kind": "enum", "of": "ResultStatus",
         "card": "repeated"})


def b_repeated_double(schema):
    schema["messages"]["ResultRaw"]["fields"].append(
        {"name": "extra_values", "tag": 42, "kind": "double", "card": "repeated"})


# ---- C: a map half that is not a blob ---------------------------------------------

def c_map_int_value(schema):
    schema["messages"]["TaskOptions"]["fields"].append(
        {"name": "counters", "tag": 40, "kind": "map", "key": "string",
         "value_kind": "int32", "entries": 2})


def main():
    must_not_raise("positive control: the real schema emits", lambda: emit_all(IR.load(generate.ROOTS)))

    for name, mut in (
            ("A1 direct field on a tree that makes a reverse call", a_direct_plus_reverse_call),
            ("A2 two direct fields in one tree", a_two_direct_fields),
            ("B1 repeated bytes", b_repeated_bytes),
            ("B2 UNPACKED repeated enum", b_unpacked_repeated_enum),
            ("B3 repeated double", b_repeated_double),
            ("C1 map<string, int32>", c_map_int_value),
    ):
        must_raise(name, lambda mut=mut: emit_all(ir_with(mut)))

    # And per backend, because a raise from the SHARED rust walker would mask a cpp
    # backend that still falls through. `cpp_build` and `cpp_pbbuild` do not use that
    # walker at all, and they are the two that had the defect.
    import cpp_build
    import cpp_pbbuild
    import cpp_binding
    # The expectation is per (shape, backend) and not uniform, because ABI v1's table DOES
    # cover "repeated string or bytes -- element call, batchable": the binding supports a
    # repeated `bytes` and must emit it. What had no case for it was the two payload
    # builders, which is precisely what the review found, so that is what is asserted.
    for name, mut, expect in (
            ("B1 repeated bytes", b_repeated_bytes,
             {"cpp_build": "raise", "cpp_pbbuild": "raise", "cpp_binding": "emit"}),
            ("B2 UNPACKED repeated enum", b_unpacked_repeated_enum,
             {"cpp_build": "raise", "cpp_pbbuild": "raise", "cpp_binding": "raise"}),
            ("B3 repeated double", b_repeated_double,
             {"cpp_build": "raise", "cpp_pbbuild": "raise", "cpp_binding": "raise"}),
    ):
        for bname, mod in (("cpp_build", cpp_build), ("cpp_pbbuild", cpp_pbbuild),
                           ("cpp_binding", cpp_binding)):
            f = must_raise if expect[bname] == "raise" else must_not_raise
            f("%s, %s alone (must %s)" % (name, bname, expect[bname]),
              lambda mod=mod, mut=mut: mod.emit(ir_with(mut)))

    print("== guards run against input that MUST be rejected ==")
    for n, why in OKS:
        print("  [ok]   %-52s %s" % (n, why))
    for n, why in FAILS:
        print("  [FAIL] %-52s %s" % (n, why))
    print("\nrefusal_test: %d passed, %d failed" % (len(OKS), len(FAILS)))
    return 1 if FAILS else 0


if __name__ == "__main__":
    sys.exit(main())
