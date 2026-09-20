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


sys.path.insert(0, os.path.join(HERE, "gen"))
sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(HERE)), "schema", "emit"))

import shapes as _S      # noqa: E402  (the description, shared)
import walk as W         # noqa: E402  (this slice's scope guard and walker)

_SCHEMA = _S.load()

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

# design/SHAPES.md's payload set, as far as this slice's scope reaches. M1 is flat with a
# LEAF element; M2's element is not a leaf, which is the property every M1 figure rests on.
PAYLOADS_M1 = ["P1.1", "P1.2", "P1.3"]
PAYLOADS_M2 = ["P2.1", "P2.2", "P2.3", "P2.4", "P2.5"]
PAYLOADS = PAYLOADS_M1 + PAYLOADS_M2
ROOT_OF = {p: ("ListResultsResponse" if p.startswith("P1")
               else "ListTasksDetailedResponse") for p in PAYLOADS}

# The shim's HostTypes order, read off the module rather than listed, so widening the
# scope cannot leave this behind.
TYPE_NAMES = [n[1:] for n in _ffi.types()] if _ffi else []


def _ctors(prefix):
    """{message name: constructor} for one Python storage.

    `TaskOptionsOptionsEntry` is the synthetic pair message a map becomes on the wire
    (ABI v1 section 11). The FACADE holds a `dict`, so there is no pair class and nothing
    ever constructs one: the shim writes map entries straight into the dict. `dict` fills
    the slot so the tuple stays positional, and if anything ever did call it the failure
    would be loud rather than a silent wrong type.
    """
    out = {}
    for n in TYPE_NAMES:
        out[n] = getattr(facade, prefix + n, dict)
    return out


CT_PLAIN = _ctors("Plain")
CT_SLOTS = _ctors("Slots")
CT_CEXT = {n[1:]: getattr(_ffi, n) for n in _ffi.types()} if _ffi else {}

TY_PLAIN = tuple(CT_PLAIN[n] for n in TYPE_NAMES)
TY_SLOTS = tuple(CT_SLOTS[n] for n in TYPE_NAMES)
TY_CEXT = tuple(CT_CEXT[n] for n in TYPE_NAMES) if _ffi else None

# Kept for work unit 1's and 2's M1 entry points, which name the three types positionally.
PLAIN = (facade.PlainListResultsResponse, facade.PlainResultRaw, facade.PlainTimestamp)
SLOTS = (facade.SlotsListResultsResponse, facade.SlotsResultRaw, facade.SlotsTimestamp)
CEXT = ((_ffi.CListResultsResponse, _ffi.CResultRaw, _ffi.CTimestamp) if _ffi else None)

# The field names the pyacc control needs accessors for: every field of every message in
# scope, read off the generated facade rather than listed.
FIELD_NAMES = sorted({
    n for cls in (getattr(facade, "Plain" + m) for m in TYPE_NAMES
                  if hasattr(facade, "Plain" + m))
    for n in cls.__init__.__code__.co_varnames[1:cls.__init__.__code__.co_argcount]})

ACC_PY = {}
for _n in FIELD_NAMES:
    ACC_PY["get_" + _n] = eval("lambda o: o.%s" % _n)                    # noqa: S307
    ACC_PY["set_" + _n] = eval("lambda o, v: setattr(o, %r, v)" % _n)    # noqa: S307
ACC_C = {}
for _n in FIELD_NAMES:
    ACC_C["get_" + _n] = operator.attrgetter(_n)
    ACC_C["set_" + _n] = (lambda nm: lambda o, v: setattr(o, nm, v))(_n)


def reference(pid):
    return V.reference(pid)


def build_facade(pid, ctors):
    return V.build(pid, ctors)


def _pb_root(pid):
    return getattr(_pb2, ROOT_OF[pid])


def build_upb(pid):
    """The same values, in the incumbent's own message objects.

    Built by re-parsing the canonical bytes rather than by a hand-written field copy: the
    copy was M1-sized and would have to grow a case per shape, which is exactly the
    hand-written path R1 exists to keep out of a comparison. `FromString` of the
    manifest's own bytes is the same values by construction.
    """
    if not _pb2:
        return None
    return _pb_root(pid).FromString(reference(pid))


def encode_arms(pid, mod=None):
    m = mod or _ffi
    root = ROOT_OF[pid]
    out = []
    upb = build_upb(pid)
    if upb is not None:
        out.append(("upb (incumbent)", upb.SerializeToString))
        # R14's second row: the map makes the production path and the canonical path
        # differ, because SerializeToString does not sort map entries and the corpus's
        # canonical form does. On M1 the two are the same call.
        if root == "ListTasksDetailedResponse":
            out.append(("upb, deterministic=True",
                        lambda _u=upb: _u.SerializeToString(deterministic=True)))
    fp = build_facade(pid, CT_PLAIN)
    fs = build_facade(pid, CT_SLOTS)
    enc_py = getattr(pycodec, "encode_root_" + root)
    out.append(("pycodec / plain", lambda: enc_py(fp)))
    out.append(("pycodec / __slots__", lambda: enc_py(fs)))
    if m:
        fc = build_facade(pid, CT_CEXT)
        out.append(("core-ffi / plain", lambda: m.encode("attr", root, fp)))
        out.append(("core-ffi / __slots__", lambda: m.encode("attr", root, fs)))
        out.append(("core-ffi / C ext type", lambda: m.encode("cext", root, fc)))
        out.append(("core-ffi / pyacc (python fn)",
                    lambda: m.encode("pyacc", root, fp, ACC_PY)))
        out.append(("core-ffi / pyacc (C attrgetter)",
                    lambda: m.encode("pyacc", root, fp, ACC_C)))
    return out


def decode_arms(pid, mod=None):
    m = mod or _ffi
    root = ROOT_OF[pid]
    buf = reference(pid)
    dec_py = getattr(pycodec, "decode_root_" + root)
    out = []
    if _pb2 is not None:
        out.append(("upb (incumbent)", lambda: _pb_root(pid).FromString(buf)))
    out.append(("pycodec / plain", lambda: dec_py(buf, CT_PLAIN)))
    out.append(("pycodec / __slots__", lambda: dec_py(buf, CT_SLOTS)))
    if m:
        out.append(("core-ffi / plain", lambda: m.decode("attr", root, buf, TY_PLAIN)))
        out.append(("core-ffi / __slots__", lambda: m.decode("attr", root, buf, TY_SLOTS)))
        out.append(("core-ffi / C ext type", lambda: m.decode("cext", root, buf, TY_CEXT)))
        out.append(("core-ffi / pyacc (python fn)",
                    lambda: m.decode("pyacc", root, buf, TY_PLAIN, ACC_PY)))
        out.append(("core-ffi / pyacc (C attrgetter)",
                    lambda: m.decode("pyacc", root, buf, TY_PLAIN, ACC_C)))
    return out


def elem_field(pid):
    return "results" if ROOT_OF[pid] == "ListResultsResponse" else "tasks"


# ----------------------------------------------------------------------------------
# The like-for-like read.
#
# `touch` and `touch_upb` must do the SAME number of Python-level attribute reads, or
# the "decode + read every field" table measures the two readers rather than the two
# decoders. The first version walked nested messages with `dir()`, which builds a sorted
# name list per object: on M2 that is 14 nested messages per element and it made the
# facade's re-read 2.8 times upb's on a payload where M1's was CHEAPER. Nothing about the
# arms; everything about the reader. Defect D9.
#
# Both now follow one PLAN built from `ffi/schema/shapes.json` through the same walker
# every backend uses, so the two readers differ only where they must: presence.
# ----------------------------------------------------------------------------------

def _plan(msg):
    """[(name, kind, card, child plan or None)] for one message, from the description."""
    out = []
    for f, k, c in W.walk(_SCHEMA, msg):
        child = _plan(f["of"]) if k == "message" else None
        out.append((f["name"], k, c, child))
    return out


_PLANS = {m: _plan(m) for m in ("ResultRaw", "TaskDetailed")}


def _read(o, plan):
    n = 0
    for name, k, c, child in plan:
        v = getattr(o, name)
        n += 1
        if c == "map":
            for k2, v2 in v.items():
                n += len(k2) + len(v2)
        elif c == "repeated":
            for x in v:
                n += len(x) if k == "string" else _read(x, child)
        elif k == "message":
            if v is not None:
                n += _read(v, child)
        elif k in ("string", "bytes"):
            n += len(v)
    return n


def _read_pb(o, plan, present):
    n = 0
    for name, k, c, child in plan:
        if k == "message" and c == "singular":
            # The one place the two readers MUST differ: a protobuf message has no absent
            # representation an attribute read would show, so presence is a HasField call
            # where the facade's is `is None`. Same count of reads either way.
            n += 1
            if o.HasField(name):
                n += _read_pb(getattr(o, name), child, True)
            continue
        v = getattr(o, name)
        n += 1
        if c == "map":
            for k2, v2 in v.items():
                n += len(k2) + len(v2)
        elif c == "repeated":
            for x in v:
                n += len(x) if k == "string" else _read_pb(x, child, True)
        elif k in ("string", "bytes"):
            n += len(v)
    return n


def touch(msg, pid):
    """Read every field of every element into Python.

    **upb does not construct Python objects in `FromString`.** It parses into its arena
    and materialises a `str`, an element wrapper or a nested message only when something
    reads it, and it caches nothing, so it re-materialises on every read. The facade arms
    construct eagerly because that is what a facade IS. Applying the same read to both is
    what puts them on the same work.
    """
    elem = "ResultRaw" if ROOT_OF[pid] == "ListResultsResponse" else "TaskDetailed"
    plan = _PLANS[elem]
    n = 0
    for e in getattr(msg, elem_field(pid)):
        n += _read(e, plan)
    return n + int(msg.page) + int(msg.total)


def touch_upb(msg, pid):
    elem = "ResultRaw" if ROOT_OF[pid] == "ListResultsResponse" else "TaskDetailed"
    plan = _PLANS[elem]
    n = 0
    for e in getattr(msg, elem_field(pid)):
        n += _read_pb(e, plan, True)
    return n + int(msg.page) + int(msg.total)


def decode_touch_arms(pid, mod=None):
    out = []
    for name, fn in decode_arms(pid, mod):
        t = touch_upb if name.startswith("upb") else touch
        out.append((name, (lambda _f=fn, _t=t, _p=pid: (lambda: _t(_f(), _p)))()))
    return out


def reread_arms(pid, mod=None):
    """Read every field of an ALREADY-decoded object; the decode is outside the loop."""
    out = []
    m = mod or _ffi
    buf = reference(pid)
    root = ROOT_OF[pid]
    if _pb2 is not None:
        msg = _pb_root(pid).FromString(buf)
        out.append(("upb (incumbent)", lambda _m=msg: touch_upb(_m, pid)))
    dec_py = getattr(pycodec, "decode_root_" + root)
    for nm, ct in (("plain", CT_PLAIN), ("__slots__", CT_SLOTS)):
        obj = dec_py(buf, ct)
        out.append(("facade / " + nm, lambda _o=obj: touch(_o, pid)))
    if m:
        obj = m.decode("cext", root, buf, TY_CEXT)
        out.append(("facade / C ext type", lambda _o=obj: touch(_o, pid)))
    return out


def same_message(pid, a, b):
    """Do two encodings parse to the same message?

    The oracle for "this is a different but legal encoding" rather than "this is wrong".
    Uses the incumbent to parse both, because it is the implementation neither encoding
    came from in the case that matters (the canonical bytes come from
    `ffi/schema/emit`).
    """
    if not _pb2:
        return False
    R = _pb_root(pid)
    try:
        return R.FromString(a) == R.FromString(b)
    except Exception:  # noqa: BLE001
        return False


def reencode(name, obj, pid):
    root = ROOT_OF[pid]
    if name.startswith("upb"):
        return obj.SerializeToString(deterministic=True)
    if name.startswith("pycodec"):
        return getattr(pycodec, "encode_root_" + root)(obj)
    if "C ext type" in name:
        return _ffi.encode("cext", root, obj)
    if "pyacc (python fn)" in name:
        return _ffi.encode("pyacc", root, obj, ACC_PY)
    if "pyacc (C attrgetter)" in name:
        return _ffi.encode("pyacc", root, obj, ACC_C)
    return _ffi.encode("attr", root, obj)
