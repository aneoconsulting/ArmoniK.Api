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
# shapes_pb2 for the incumbent arm: `mech/build.sh` writes one for the target interpreter;
# an interpreter whose protobuf runtime cannot load that gencode (the 3.7 floor: protobuf
# 4.24, build.sh --floor) has its own beside its shims, preferred when present.
_PB2 = os.path.join(HERE, "build", TAG, "pb2")
if not os.path.exists(os.path.join(_PB2, "shapes_pb2.py")):
    _PB2 = os.path.join(HERE, "mech", "build", "pb2")
# WP5 step 10: a process of the no-unknown variant (AK_VARIANT=nounk, or a `_nounk` shim
# selected) imports the variant's facade -- no `_unknown` anywhere -- and host-gen drop from
# gen/out/nounk, and has no retain codec.
NOUNK_VARIANT = (os.environ.get("AK_VARIANT") == "nounk"
                 or os.environ.get("AK_FFI_MODULE", "").endswith("_nounk")
                 or os.environ.get("AK_COUNT_MODULE", "").endswith("_nounk"))
_GEN_OUT = os.path.join(HERE, "gen", "out", "nounk") if NOUNK_VARIANT else os.path.join(HERE, "gen", "out")
for p in (HERE, os.path.join(HERE, "build", TAG), _GEN_OUT, _PB2):
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

import shapes as _S      # noqa: E402  (the description: payload specs only)

_SCHEMA = _S.load()

import facade            # noqa: E402  (generated from the plan by poc/codec/gen/py_pure.py)
import pycodec           # noqa: E402  (generated: unknown fields dropped)
pycodec_retain = None if NOUNK_VARIANT else __import__("pycodec_retain")  # (generated: retained)
import payload_values as V  # noqa: E402  (harness glue)
import facts as _F       # noqa: E402  (the facade's field facts, read from facade.MESSAGES)
import arms_plan as _AP  # noqa: E402  (the decode+read reader, shared with camp_codec.py)


class W:
    """walk.py's two entry points, over the facade's plan-rendered facts."""
    @staticmethod
    def walk(_schema, msg):
        return _F.walk(facade, msg)

    @staticmethod
    def oneof_groups(_schema, msg):
        return _F.oneof_groups(facade, msg)

# Two builds of the same shim, over two builds of the same core, and they must NOT share
# a process: both cores are called `libak_core.so`, so whichever is loaded first satisfies
# the other's NEEDED entry and the second shim silently gets the first core. That is how
# the counting numbers came out as zeroes the first time, and it is the same class of
# defect as the rust slice's inlined counters -- a count that is not the build you think
# it is. `AK_USE_COUNT=1` selects the counting build, and `conformance.py` runs that pass
# in a subprocess of its own.
# `AK_FFI_MODULE` picks WHICH shim, and it exists for the same reason `AK_USE_COUNT` does:
# there are now three builds of one `libak_core.so` -- plain, counting, and the rpc-feature
# one that pulls tonic in -- and the first of them loaded in a process satisfies the
# others' NEEDED entry by soname. So a process gets exactly one, chosen here rather than by
# import order. `rpc.py` sets `_akffi_rpc`; everything else takes the default.
if os.environ.get("AK_USE_COUNT") == "1":
    # AK_COUNT_MODULE: the no-unknown variant's counting build is `_akffi_count_nounk`.
    _CMOD = os.environ.get("AK_COUNT_MODULE", "_akffi_count")
    _ffi = _try("the composed arm, counting build (%s)" % _CMOD,
                lambda: __import__(_CMOD))
    _ffi_count = _ffi
    COUNTING = True
else:
    _MOD = os.environ.get("AK_FFI_MODULE", "_akffi")
    _ffi = _try("the composed arm (%s)" % _MOD, lambda: __import__(_MOD))
    _ffi_count = None
    COUNTING = False
_pb2 = _try("the incumbent (protobuf/upb)", lambda: __import__("shapes_pb2"))

# design/SHAPES.md's payload set, as far as this slice's scope reaches. M1 is flat with a
# LEAF element; M2's element is not a leaf, which is the property every M1 figure rests on.
PAYLOADS_M1 = ["P1.1", "P1.2", "P1.3"]
PAYLOADS_M2 = ["P2.1", "P2.2", "P2.3", "P2.4", "P2.5"]
PAYLOADS_M3 = ["P3.1"]                  # oneof and explicit presence
PAYLOADS_M4 = ["P4.1"]                  # the adapter site
PAYLOADS_M5 = ["P5.1", "P5.2", "P5.3", "P5.4"]   # bulk bytes, 36 B to 4 MB
PAYLOADS_M6 = ["P6.1"]                  # packed scalars and enums
PAYLOADS_M7 = ["P7.1"]                  # the interleaved decode-only control
PAYLOADS = (PAYLOADS_M1 + PAYLOADS_M2 + PAYLOADS_M3 + PAYLOADS_M4
            + PAYLOADS_M5 + PAYLOADS_M6 + PAYLOADS_M7)
ROOT_OF = {p: _SCHEMA["payloads"][p]["root"] for p in PAYLOADS}

# P7.1 interleaves two repeated fields, and no canonical writer can produce those bytes:
# a writer that emits a repeated field contiguously cannot interleave two of them. So it
# is validated by DECODING, and the re-encode is checked against a permutation of the same
# (tag, wire type, body) triples rather than against the manifest's hash (SHAPES.md).
DECODE_ONLY = {"P7.1"}


def elem_fields(pid):
    """The root's repeated MESSAGE fields, as (name, element type name).

    A list rather than one name: M5's root has none and M7's has two, and every helper
    below that used to say `"results" if ... else "tasks"` was a root with exactly one
    repeated field written into the slice rather than read off the description.
    """
    out = []
    for f, k, c in W.walk(_SCHEMA, ROOT_OF[pid]):
        if c == "repeated" and k == "message":
            out.append((f["name"], f["of"]))
    return out

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
    """The incumbent's message, PARSED from the canonical bytes.

    Convenient, and **not the shape production serialises**: a message that came out of
    `FromString` has an arena the parser laid out, and a message an application built has
    one protobuf's own setters laid out. That was the first suspect when the P1.2 encode
    ratio moved between work unit 2 and work unit 3, and it was WRONG -- the two arenas
    serialise within 1-3% of each other (JOURNAL.md J26, and the labelled second row in
    the bench keeps it visible). The cause was the process's allocator state; see
    `allocator.py`. This one stays as the oracle for the conformance checks, where only
    the values matter, and as that second row. `build_upb_native` is the row the ratios
    come from, because R14 asks for the path production runs, not the cheap fixture.
    """
    if not _pb2:
        return None
    return _pb_root(pid).FromString(reference(pid))


def _fill_pb(dst, src, plan):
    """Copy one facade object into a protobuf message through the public API, which is
    what an application does. Driven from the description's plan, so it needs no case per
    shape and cannot fall behind a widened scope (R1)."""
    for name, k, c, child in plan:
        if c == "oneof_case":
            hit = child[1].get(getattr(src, name))
            if hit is None:
                continue
            gn, gk, gchild = hit
            gv = getattr(src, gn)
            if gk == "message":
                # `SetInParent` is how protobuf says "this member is selected and its body
                # is empty". Assigning nothing to a submessage leaves the oneof UNSET, so
                # the payload-free member would vanish from the wire.
                child_msg = getattr(dst, gn)
                child_msg.SetInParent()
                if gchild:
                    _fill_pb(child_msg, gv, gchild)
            else:
                setattr(dst, gn, gv)
            continue
        v = getattr(src, name)
        if c == "optional":
            if v is not None:
                setattr(dst, name, v)
            continue
        if c == "packed":
            if v:
                getattr(dst, name).extend(v)
            continue
        if c == "map":
            if v:
                getattr(dst, name).update(v)
        elif c == "repeated":
            if not v:
                continue
            if k == "string":
                getattr(dst, name).extend(v)
            else:
                for x in v:
                    _fill_pb(getattr(dst, name).add(), x, child)
        elif k == "message":
            if v is not None:
                _fill_pb(getattr(dst, name), v, child)
        else:
            setattr(dst, name, v)


def build_upb_native(pid):
    """The incumbent's message, BUILT through its own API from the same values.

    This is what R14's "the path ArmoniK runs" means on the encode side: production builds
    a message with setters and then serialises it. The parsed message is kept as a
    labelled second row in the same interleaved rounds so the choice is visible; it turned
    out not to matter (1-3%), but a fixture that had never been measured against the
    alternative is not a fixture anyone should trust.
    """
    if not _pb2:
        return None
    src = build_facade(pid, CT_PLAIN)
    dst = _pb_root(pid)()
    # One plan for the whole root: M5's root has a singular child and no list at all, and
    # M7's has two lists. `_fill_pb` already walks both from the plan.
    _fill_pb(dst, src, _PLANS[ROOT_OF[pid]])
    return dst


def _has_map(msg, seen=None):
    """Whether a map is reachable from `msg`. The `deterministic=True` row exists because
    `SerializeToString` does not sort map entries and the canonical form does, so it is a
    row wherever there IS a map and nowhere else."""
    seen = seen or set()
    if msg in seen:
        return False
    seen = seen | {msg}
    for f, k, c in W.walk(_SCHEMA, msg):
        if c == "map":
            return True
        if k == "message" and _has_map(f["of"], seen):
            return True
    return False


def encode_arms(pid, mod=None):
    m = mod or _ffi
    root = ROOT_OF[pid]
    out = []
    upb = build_upb_native(pid)
    if upb is not None:
        out.append(("upb (incumbent)", upb.SerializeToString))
        # The same values in a message the PARSER laid out rather than the setters. A
        # within-arm delta (R4), and the reason it is a row: the harness used to make its
        # fixture this way by accident and it moved P1.2's headline by a factor of 1.8.
        parsed = build_upb(pid)
        out.append(("upb, message parsed not built", parsed.SerializeToString))
        # R14's second row: the map makes the production path and the canonical path
        # differ, because SerializeToString does not sort map entries and the corpus's
        # canonical form does. On M1 the two are the same call.
        if _has_map(root):
            out.append(("upb, deterministic=True",
                        lambda _u=upb: _u.SerializeToString(deterministic=True)))
    fp = build_facade(pid, CT_PLAIN)
    fs = build_facade(pid, CT_SLOTS)
    enc_py = getattr(pycodec, "encode_root_" + root)
    out.append(("pycodec / plain", lambda: enc_py(fp)))
    out.append(("pycodec / __slots__", lambda: enc_py(fs)))
    if pycodec_retain is not None:
        enc_pyr = getattr(pycodec_retain, "encode_root_" + root)
        out.append(("pycodec-retain / plain", lambda: enc_pyr(fp)))
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
    if pycodec_retain is not None:
        dec_pyr = getattr(pycodec_retain, "decode_root_" + root)
        out.append(("pycodec-retain / plain", lambda: dec_pyr(buf, CT_PLAIN)))
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
    """The FIRST repeated message field, or None where the root has none (M5)."""
    ef = elem_fields(pid)
    return ef[0][0] if ef else None


def elem_type(pid):
    ef = elem_fields(pid)
    return ef[0][1] if ef else None


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
    """The shared reader's plan for one message of the shapes facade (arms_plan.py)."""
    return _AP.plan(facade, msg)


# Keyed by ROOT rather than by element type, so one reader serves a root with two
# repeated fields, a root with none, and the scalars beside them, with no case per shape.
_PLANS = {r: _plan(r) for r in sorted({ROOT_OF[p] for p in PAYLOADS})}


_read = _AP.read
_read_pb = _AP.read_pb


def touch(msg, pid):
    """Read every field of every element into Python.

    **upb does not construct Python objects in `FromString`.** It parses into its arena
    and materialises a `str`, an element wrapper or a nested message only when something
    reads it, and it caches nothing, so it re-materialises on every read. The facade arms
    construct eagerly because that is what a facade IS. Applying the same read to both is
    what puts them on the same work.
    """
    return _read(msg, _PLANS[ROOT_OF[pid]])


def touch_upb(msg, pid):
    return _read_pb(msg, _PLANS[ROOT_OF[pid]], True)


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
    if name.startswith("pycodec-retain"):
        return getattr(pycodec_retain, "encode_root_" + root)(obj)
    if name.startswith("pycodec"):
        return getattr(pycodec, "encode_root_" + root)(obj)
    if "C ext type" in name:
        return _ffi.encode("cext", root, obj)
    if "pyacc (python fn)" in name:
        return _ffi.encode("pyacc", root, obj, ACC_PY)
    if "pyacc (C attrgetter)" in name:
        return _ffi.encode("pyacc", root, obj, ACC_C)
    return _ffi.encode("attr", root, obj)
