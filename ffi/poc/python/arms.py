"""The python slice's arms, work unit 2: the composed arm, both directions.

Every arm covers the M1 subtree over P1.1, P1.2 and P1.3, and every one of them is
byte-identical to `ffi/schema/generated/manifest.json` before anything is timed (R2).

| arm | what it is |
|---|---|
| `upb` | **the incumbent, on the path ArmoniK runs** (R14). See `BASELINE` below |
| `pycodec / plain`, `pycodec / __slots__` | **R3's no-boundary control**: the generated pure-Python codec over the same facades, no crossing at all |
| `core-ffi / plain`, `core-ffi / __slots__` | the **composed arm**: the shared Rust core at `poc/codec/` behind the generated CPython shim, facade reached with `PyObject_GetAttr` / `SetAttr` |
| `core-ffi / C ext type` | the same, with the facade a generated C extension type the shim reads and writes as a struct |
| `core-ffi / pyacc` | **the premise control** (README 9.1, last bullet): the same shim, the same crossings, each field reached by a Python-level accessor CALL |

What is finally here that work unit 1 did not have: **a core**.  Work unit 1's `mech/`
arms price the shim-to-facade edge with nothing behind it; these are the design.

R14, the baseline.  `packages/python` does not serialise directly; gRPC's generated stub
does, and with the pinned `grpcio-tools` that is `request_serializer=
ListResultsRequest.SerializeToString` and `response_deserializer=
ListResultsResponse.FromString`.  So `SerializeToString` and `FromString` ARE the
production path here, with no size pass and no buffer writer in between -- unlike C#,
where the marshaller's `CalculateSize()` turned out to be production code.  `verify_r14.py`
re-derives that from `Protos/V1/results_service.proto` rather than taking it on trust.
"""

import operator
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
TAG = "py%d.%d" % sys.version_info[:2]
for p in (HERE, os.path.join(HERE, "build", TAG), os.path.join(HERE, "gen", "out"),
          os.path.join(HERE, "mech", "build", "pb2")):
    if p not in sys.path:
        sys.path.insert(0, p)

BASELINE = ("gRPC's generated marshaller path: Message.SerializeToString on encode and "
            "Message.FromString on decode (R14)")

absent = []


def _try(name, fn):
    try:
        return fn()
    except Exception as e:  # noqa: BLE001
        absent.append("%s: %s" % (name, e))
        return None


import facade            # noqa: E402  (generated)
import pycodec           # noqa: E402  (generated)
import payload_values as V  # noqa: E402  (generated)

# Two builds of the same shim, over two builds of the same core, and they must NOT share
# a process: both cores are called `libak_core.so`, so whichever is loaded first satisfies
# the other's NEEDED entry and the second shim silently gets the first core. That is how
# the counting numbers came out as zeroes the first time, and it is the same class of
# defect as the rust slice's inlined counters -- a count that is not the build you think
# it is. `AK_USE_COUNT=1` selects the counting build, and `conformance.py` runs that pass
# in a subprocess of its own.
if os.environ.get("AK_USE_COUNT") == "1":
    _ffi = _try("the composed arm, counting build",
                lambda: __import__("_akffi_count"))
    _ffi_count = _ffi
    COUNTING = True
else:
    _ffi = _try("the composed arm (_akffi)", lambda: __import__("_akffi"))
    _ffi_count = None
    COUNTING = False
_pb2 = _try("the incumbent (protobuf/upb)", lambda: __import__("shapes_pb2"))

PAYLOADS = ["P1.1", "P1.2", "P1.3"]
SCOPE = ["ListResultsResponse", "ResultRaw", "Timestamp"]

PLAIN = (facade.PlainListResultsResponse, facade.PlainResultRaw, facade.PlainTimestamp)
SLOTS = (facade.SlotsListResultsResponse, facade.SlotsResultRaw, facade.SlotsTimestamp)
CEXT = ((_ffi.CListResultsResponse, _ffi.CResultRaw, _ffi.CTimestamp)
        if _ffi else None)

# The field names the pyacc control needs accessors for are read off the generated
# facade, not listed, so widening the scope cannot leave the control behind.
FIELD_NAMES = sorted({
    n for cls in PLAIN
    for n in cls.__init__.__code__.co_varnames[1:cls.__init__.__code__.co_argcount]})

# A Python-level accessor: a Python function, one interpreter frame per field.
ACC_PY = {}
for _n in FIELD_NAMES:
    ACC_PY["get_" + _n] = eval("lambda o: o.%s" % _n)                    # noqa: S307
    ACC_PY["set_" + _n] = eval("lambda o, v: setattr(o, %r, v)" % _n)    # noqa: S307
# A C-implemented accessor reached the same way. It separates "the accessor is written in
# Python" from "the accessor is reached by a call", which one control would mix.
ACC_C = {}
for _n in FIELD_NAMES:
    ACC_C["get_" + _n] = operator.attrgetter(_n)
    ACC_C["set_" + _n] = (lambda nm: lambda o, v: setattr(o, nm, v))(_n)

CTORS = {"ListResultsResponse": PLAIN[0], "ResultRaw": PLAIN[1],
         "Timestamp": PLAIN[2]}
CTORS_SLOTS = {"ListResultsResponse": SLOTS[0], "ResultRaw": SLOTS[1],
               "Timestamp": SLOTS[2]}


def reference(pid):
    return V.reference(pid)


def build_facade(pid, ctors):
    return V.build(pid, *ctors)


def build_upb(pid):
    """The same values, in the incumbent's own message objects."""
    if not _pb2:
        return None
    src = V.build(pid, *PLAIN)
    root = _pb2.ListResultsResponse()
    for e in src.results:
        m = root.results.add()
        for n in ("session_id", "name", "owner_task_id", "result_id",
                  "created_by", "opaque_id"):
            setattr(m, n, getattr(e, n))
        m.status, m.size, m.manual_deletion = e.status, e.size, e.manual_deletion
        for n in ("created_at", "completed_at"):
            v = getattr(e, n)
            if v is not None:
                getattr(m, n).seconds, getattr(m, n).nanos = v.seconds, v.nanos
    root.page, root.total = src.page, src.total
    return root


def encode_arms(pid, mod=None):
    """[(name, callable() -> bytes)]."""
    m = mod or _ffi
    out = []
    upb = build_upb(pid)
    if upb is not None:
        out.append(("upb (incumbent)", upb.SerializeToString))
    fp, fs = build_facade(pid, PLAIN), build_facade(pid, SLOTS)
    out.append(("pycodec / plain", lambda: pycodec.encode_root(fp)))
    out.append(("pycodec / __slots__", lambda: pycodec.encode_root(fs)))
    if m:
        fc = build_facade(pid, CEXT)
        out.append(("core-ffi / plain", lambda: m.encode("attr", fp)))
        out.append(("core-ffi / __slots__", lambda: m.encode("attr", fs)))
        out.append(("core-ffi / C ext type", lambda: m.encode("cext", fc)))
        out.append(("core-ffi / pyacc (python fn)",
                    lambda: m.encode("pyacc", fp, ACC_PY)))
        out.append(("core-ffi / pyacc (C attrgetter)",
                    lambda: m.encode("pyacc", fp, ACC_C)))
    return out


def decode_arms(pid, mod=None):
    """[(name, callable() -> a facade or a message)].

    Every arm returns a fully constructed host object, which is what makes them
    comparable: an arm that returned a lazy view would be measuring something else.
    """
    m = mod or _ffi
    buf = reference(pid)
    out = []
    if _pb2 is not None:
        out.append(("upb (incumbent)", lambda: _pb2.ListResultsResponse.FromString(buf)))
    out.append(("pycodec / plain", lambda: pycodec.decode_root(buf, CTORS)))
    out.append(("pycodec / __slots__", lambda: pycodec.decode_root(buf, CTORS_SLOTS)))
    if m:
        out.append(("core-ffi / plain", lambda: m.decode("attr", buf, PLAIN)))
        out.append(("core-ffi / __slots__", lambda: m.decode("attr", buf, SLOTS)))
        out.append(("core-ffi / C ext type", lambda: m.decode("cext", buf, CEXT)))
        out.append(("core-ffi / pyacc (python fn)",
                    lambda: m.decode("pyacc", buf, PLAIN, ACC_PY)))
        out.append(("core-ffi / pyacc (C attrgetter)",
                    lambda: m.decode("pyacc", buf, PLAIN, ACC_C)))
    return out


# The field names of one element, read off the generated facade rather than listed, so
# `touch` cannot fall behind a widened scope.
_ELEM_FIELDS = [n for n in PLAIN[1].__init__.__code__.co_varnames[
    1:PLAIN[1].__init__.__code__.co_argcount]]
_STAMP_FIELDS = [n for n in PLAIN[2].__init__.__code__.co_varnames[
    1:PLAIN[2].__init__.__code__.co_argcount]]


def touch(msg):
    """Read every field of every element into Python.

    Why this exists, and it is the most important sentence in this file:
    **upb does not construct Python objects in `FromString`.** It parses the wire into
    its own arena and materialises a Python `str`, a Python element wrapper or a nested
    message ONLY when something reads it. The core-ffi and pycodec arms construct the
    whole facade eagerly, because that is what a facade IS. So a bare `FromString`
    against a bare `core-ffi decode` compares two different amounts of work, and the
    floor arm in `bench.py` is what made that visible: constructing 1,000 bare facade
    objects and copying the input already costs 0.8 of upb's entire decode.

    Applying the SAME `touch` to every arm's output puts them back on the same work.
    Neither table is "the" answer: a caller that reads the fields pays for the
    materialisation and a caller that reads one field does not, so both are reported and
    the report says which question it is answering.
    """
    n = 0
    for e in msg.results:
        for f in _ELEM_FIELDS:
            v = getattr(e, f)
            if v is None:
                continue
            n += 1
            if not isinstance(v, (str, bytes, int, bool)):
                for g in _STAMP_FIELDS:
                    n += int(getattr(v, g) != 0)
    return n + int(msg.page) + int(msg.total)


def touch_upb(msg):
    """The same read, against the incumbent's presence rules.

    `HasField` rather than `is None`, because a protobuf message has no absent
    representation for a submessage that a plain attribute read would show.
    """
    n = 0
    for e in msg.results:
        for f in _ELEM_FIELDS:
            if f in ("created_at", "completed_at"):
                if not e.HasField(f):
                    continue
                v = getattr(e, f)
                n += 1
                for g in _STAMP_FIELDS:
                    n += int(getattr(v, g) != 0)
            else:
                getattr(e, f)
                n += 1
    return n + int(msg.page) + int(msg.total)


def decode_touch_arms(pid, mod=None):
    """decode, then read every field into Python: the like-for-like comparison."""
    out = []
    for name, fn in decode_arms(pid, mod):
        t = touch_upb if name.startswith("upb") else touch
        out.append((name, (lambda _f=fn, _t=t: (lambda: _t(_f())))()))
    return out


def reread_arms(pid, mod=None):
    """Read every field of an ALREADY-decoded object. The decode is outside the loop.

    This is the row that says the two things being compared are not the same kind of
    object. A facade holds Python values, so a second pass is attribute reads. upb holds
    an arena, and **it does not cache**: measured, a second full read of the same message
    costs 3.05 ms against 3.45 ms for the first, so all but 12 percent of the
    materialisation is paid again. A caller that reads its response twice pays upb twice.
    """
    out = []
    m = mod or _ffi
    buf = reference(pid)
    if _pb2 is not None:
        msg = _pb2.ListResultsResponse.FromString(buf)
        out.append(("upb (incumbent)", lambda _m=msg: touch_upb(_m)))
    for nm, ctors in (("plain", PLAIN), ("__slots__", SLOTS)):
        obj = pycodec.decode_root(buf, {"ListResultsResponse": ctors[0],
                                        "ResultRaw": ctors[1], "Timestamp": ctors[2]})
        out.append(("facade / " + nm, lambda _o=obj: touch(_o)))
    if m:
        obj = m.decode("cext", buf, CEXT)
        out.append(("facade / C ext type", lambda _o=obj: touch(_o)))
    return out


def reencode(name, obj):
    """Re-encode a decoded object with the arm that decoded it, so decode can be checked
    by byte identity rather than by a field-by-field comparison that would need its own
    correctness argument."""
    if name.startswith("upb"):
        return obj.SerializeToString()
    if name.startswith("pycodec"):
        return pycodec.encode_root(obj)
    if "C ext type" in name:
        return _ffi.encode("cext", obj)
    if "pyacc (python fn)" in name:
        return _ffi.encode("pyacc", obj, ACC_PY)
    if "pyacc (C attrgetter)" in name:
        return _ffi.encode("pyacc", obj, ACC_C)
    return _ffi.encode("attr", obj)
