"""The codec arms of work unit 1, built once and shared by conformance and bench.

Six arms over the M1 subtree, encode only:

| arm | what it is |
|---|---|
| `upb` | the **incumbent**: `protobuf`, whose runtime is the upb C extension |
| `pycodec/plain`, `pycodec/slots` | **R3's no-boundary control**: the generated pure-Python encoder over each Python storage |
| `cshim/plain`, `cshim/slots` | the generated C shim reading fields with `PyObject_GetAttr` |
| `cshim/cext` | the generated C shim reading a struct member on the generated C facade type |
| `cshim/pyacc-py`, `cshim/pyacc-c` | **the premise control** (README 9.1, last bullet): the same C traversal with a Python-level accessor CALL per field, once with a Python function and once with a C-implemented one |

What is NOT in any of them, and it matters for how the numbers read: **there is
no Rust core here.**  README 9.1 gives Python three layers -- core, generated C
shim, facade -- and work unit 1 prices the **shim-to-facade** edge, which is the
one that is a crossing per field.  The shim-to-core edge is a plain C call and
`bench_mech.py` prices it (21.9 ns per forward call from Python, 1.42 ns per
C-to-C reverse call).  So a `cshim` row is a lower bound on the real thing by
the core's own codec cost, and it is an honest bound for the question being
asked, which is which accessor style and which storage the shim should use.
"""

import operator
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
TAG = "py%d.%d" % sys.version_info[:2]
BUILD = os.path.join(HERE, "build")
for p in (HERE, os.path.join(BUILD, TAG), os.path.join(BUILD, "pb2"),
          os.path.join(HERE, "gen", "out")):
    if p not in sys.path:
        sys.path.insert(0, p)

absent = []


def _try(name, fn):
    try:
        return fn()
    except Exception as e:  # noqa: BLE001
        absent.append("%s: %s" % (name, e))
        return None


import facade  # noqa: E402  (generated)
import pycodec  # noqa: E402  (generated)
import payload_values as gvalues  # noqa: E402  (generated)

_codec = _try("the generated C shim", lambda: __import__("_akcodec"))
_codec_count = _try("the generated C shim, counting build",
                    lambda: __import__("_akcodec_count"))
_pb2 = _try("the incumbent (protobuf/upb)", lambda: __import__("shapes_pb2"))

PAYLOADS = ["P1.1", "P1.2", "P1.3"]

# The three field names the pyacc control needs getters for are a function of
# the schema, so they are read off the generated facade rather than listed.
FIELD_NAMES = sorted({n for cls in (facade.PlainListResultsResponse,
                                    facade.PlainResultRaw,
                                    facade.PlainTimestamp)
                      for n in cls.__init__.__code__.co_varnames[1:cls.__init__.__code__.co_argcount]})

# A Python-level accessor: a Python function, one frame per field.
GETTERS_PY = {n: eval("lambda o: o.%s" % n) for n in FIELD_NAMES}  # noqa: S307
# A C-implemented accessor, called the same way.  It separates "the accessor is
# written in Python" from "the accessor is reached by a call", which a single
# control would mix.
GETTERS_C = {n: operator.attrgetter(n) for n in FIELD_NAMES}


def build_facades(pid):
    """One payload, in each of the three storages."""
    out = {}
    out["plain"] = gvalues.build(pid, facade.PlainListResultsResponse,
                                 facade.PlainResultRaw, facade.PlainTimestamp)
    out["slots"] = gvalues.build(pid, facade.SlotsListResultsResponse,
                                 facade.SlotsResultRaw, facade.SlotsTimestamp)
    if _codec:
        out["cext"] = gvalues.build(pid, _codec.CListResultsResponse,
                                    _codec.CResultRaw, _codec.CTimestamp)
    return out


def build_upb(pid):
    """The same values, in the incumbent's own message objects."""
    if not _pb2:
        return None
    src = gvalues.build(pid, facade.PlainListResultsResponse,
                        facade.PlainResultRaw, facade.PlainTimestamp)
    root = _pb2.ListResultsResponse()
    for e in src.results:
        m = root.results.add()
        for n in ("session_id", "name", "owner_task_id", "result_id",
                  "created_by", "opaque_id"):
            setattr(m, n, getattr(e, n))
        m.status = e.status
        m.size = e.size
        m.manual_deletion = e.manual_deletion
        for n in ("created_at", "completed_at"):
            v = getattr(e, n)
            if v is not None:
                getattr(m, n).seconds = v.seconds
                getattr(m, n).nanos = v.nanos
    root.page = src.page
    root.total = src.total
    return root


def arms(fac, upb):
    """[(name, callable() -> bytes)] for one payload.

    Kept as thunks so the harness times only the encode: building the facade
    objects is setup, and a setup cost inside a timed loop is the defect the
    Rust slice's `AK_BENCH_ONLY` note is about at a different scale.
    """
    out = []
    if upb is not None:
        out.append(("upb (incumbent)", upb.SerializeToString))
    out.append(("pycodec / plain", lambda: pycodec.encode_root(fac["plain"])))
    out.append(("pycodec / __slots__", lambda: pycodec.encode_root(fac["slots"])))
    if _codec:
        out.append(("cshim getattr / plain",
                    lambda: _codec.encode_attr(fac["plain"])))
        out.append(("cshim getattr / __slots__",
                    lambda: _codec.encode_attr(fac["slots"])))
        out.append(("cshim member / C ext type",
                    lambda: _codec.encode_cext(fac["cext"])))
        out.append(("cshim pyacc (python fn) / plain",
                    lambda: _codec.encode_pyacc(fac["plain"], GETTERS_PY)))
        out.append(("cshim pyacc (C attrgetter) / plain",
                    lambda: _codec.encode_pyacc(fac["plain"], GETTERS_C)))
    return out
