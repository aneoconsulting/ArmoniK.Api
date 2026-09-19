"""Work unit 1, part 1: what a crossing costs a Python host, per mechanism.

README 9.1 fixes the candidate list and says why: the core calls CPython
primitives rather than calling back into Python, so the candidates for the codec
path are **a generated C extension module and PyO3**, and `ctypes` and `cffi` in
ABI mode are measured to establish what routing a callback through the
interpreter costs rather than as candidates.  They stay candidates for the RPC
layer, where the crossing count is two per call rather than one per field.

Four groups, and each answers one question:

1.  **forward** -- what it costs a Python host to reach native code at all.
    Every arm calls the SAME callee (`ak_noop` in `libakmech_cabi.so`), so the
    rows differ in the mechanism and in nothing else (README R7).
2.  **reverse (into the interpreter)** -- what a callback costs.  Driven from
    inside the C library, `n` calls per forward call, so what is divided is the
    reverse call and not the forward one.
3.  **reverse (a C-API call on a primitive)** -- the design's default, priced
    against group 2.  This is the comparison README 9.1 rests on.
4.  **a facade field read** -- README 9.1's third bullet, priced at the
    primitive level.  `bench_codec.py` prices the same choice over a whole
    message, which is the number that decides it.

Usage:  python bench_mech.py [--json out.json]
"""

import ctypes
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
TAG = "py%d.%d" % sys.version_info[:2]
BUILD = os.path.join(HERE, "build")
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(BUILD, TAG))

import harness  # noqa: E402
from harness import Case  # noqa: E402

CABI = os.path.join(BUILD, "libakmech_cabi.so")

# --------------------------------------------------------------------------
# The arms.  Anything that fails to load is NAMED and the run continues without
# it; a mechanism silently missing from a table is the failure mode README R1 is
# about, one level up from the generator.
# --------------------------------------------------------------------------

missing = []


def _try(name, fn):
    try:
        return fn()
    except Exception as e:  # noqa: BLE001 -- reporting is the point
        missing.append("%s: %s" % (name, e))
        return None


_akmech = _try("C extension (full C-API)", lambda: __import__("_akmech"))
_akmech3 = _try("C extension (abi3)", lambda: __import__("_akmech3"))
_pyo3 = _try("PyO3", lambda: __import__("_akmech_pyo3"))
_cffi_api = _try("cffi API mode", lambda: __import__("_akcffi_api"))


def _load_abi3_crossver():
    """The abi3 artifact built against 3.10, loaded by THIS interpreter.

    README 9.2's "one wheel across 3.x" is a claim about exactly this, so it is
    demonstrated rather than asserted: a different interpreter loading the same
    .so file.
    """
    import importlib.util

    p = os.path.join(BUILD, "abi3", "_akmech3.abi3.so")
    if not os.path.exists(p):
        raise FileNotFoundError(p)
    # The module name must be the one the init function is named after
    # (PyInit__akmech3); only the PATH differs from this interpreter's own
    # build.  It is deliberately NOT registered in sys.modules, so both copies
    # are live at once and the table can put them side by side.
    spec = importlib.util.spec_from_file_location("_akmech3", p)
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


_abi3x = _try("C extension (abi3, the 3.10-built artifact)", _load_abi3_crossver)


def _load_pyo3_abi3():
    import importlib.util

    p = os.path.join(BUILD, "abi3", "_akmech_pyo3.abi3.so")
    if not os.path.exists(p):
        raise FileNotFoundError(p)
    spec = importlib.util.spec_from_file_location("_akmech_pyo3", p)
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


_pyo3_abi3 = _try("PyO3 (abi3, the 3.10-built artifact)", _load_pyo3_abi3)

# ctypes, both spellings: with argtypes/restype declared (what a real binding
# does) and without (what a quick script does, and what pays a conversion
# search per argument).
_ct = ctypes.CDLL(CABI)
_ct_typed = ctypes.CDLL(CABI)
_ct_typed.ak_noop.argtypes = [ctypes.c_uint64]
_ct_typed.ak_noop.restype = ctypes.c_uint64
_ct_typed.ak_reverse_n.argtypes = [
    ctypes.CFUNCTYPE(ctypes.c_uint64, ctypes.c_uint64),
    ctypes.c_uint64,
    ctypes.c_size_t,
]
_ct_typed.ak_reverse_n.restype = ctypes.c_uint64

_CT_CB = ctypes.CFUNCTYPE(ctypes.c_uint64, ctypes.c_uint64)


def _py_cb(x):
    return x ^ 2


_ct_cb = _CT_CB(_py_cb)


def _load_cffi_abi():
    from cffi import FFI

    import cffi_build

    ffi = FFI()
    ffi.cdef(cffi_build.DECLS)
    return ffi, ffi.dlopen(CABI)


_cffi = _try("cffi ABI mode", _load_cffi_abi)

if _cffi_api is not None:
    @_cffi_api.ffi.def_extern()
    def ak_py_cb(x):
        return x ^ 2


# --------------------------------------------------------------------------
# README R2, at this benchmark's level: correctness before timing.
#
# Every forward arm calls the same callee, so every forward arm must return the
# same value for the same input -- including an input that does not fit in an
# `int`.  This gate is not decoration: it is what caught the untyped-ctypes arm
# returning a TRUNCATED result, which made it look like the fastest ctypes
# spelling when it is simply not doing the work.
# --------------------------------------------------------------------------

GATE_INPUTS = [0, 1, 7, (1 << 31) - 1, 1 << 31, (1 << 62) + 12345, (1 << 64) - 3]


def gate(arms, out=sys.stdout):
    """`arms` is [(name, callable)].  Returns the names that are WRONG."""
    bad = []
    print("\n## correctness gate: every forward arm, same callee, same answer",
          file=out)
    for name, f in arms:
        errs = []
        for x in GATE_INPUTS:
            want = x ^ 2
            try:
                got = f(x) & ((1 << 64) - 1)
            except Exception as e:  # noqa: BLE001
                errs.append("%d -> %s" % (x, e))
                continue
            if got != want:
                errs.append("%d -> %d, want %d" % (x, got, want))
        if errs:
            bad.append(name)
            print("   WRONG  %-42s %s" % (name, "; ".join(errs[:2])), file=out)
        else:
            print("   ok     %s" % name, file=out)
    return bad


def py_noop(x):
    return x ^ 2


class _PyObj:
    def m(self, x):
        return x ^ 2


_inst = _PyObj()


def forward_cases():
    cs = []

    def add(arm, f, note=""):
        if f is None:
            return
        # The loop is identical in every arm and the empty-loop row below is
        # what it costs, so a reader can subtract it once rather than trusting
        # that it cancels.
        def run(reps, _f=f):
            for _ in range(reps):
                _f(7)
        cs.append(Case("1. forward: one crossing, Python host -> native", arm, run, note))

    def empty(reps):
        for _ in range(reps):
            pass

    cs.append(Case(
        "1. forward: one crossing, Python host -> native",
        "-- empty Python for-loop",
        empty,
        "EVERY row in this group carries this; subtract it once",
    ))
    add("-- a pure-Python function call", py_noop,
        "not a crossing. The floor a Python host cannot go below")
    add("-- a pure-Python method call", _inst.m)
    if _akmech:
        add("C extension, METH_O", _akmech.fwd_noop)
        add("C extension, METH_O, no crossing", _akmech.fwd_selfcontained,
            "the same entry point with the call into the C library removed")
        add("C extension, METH_FASTCALL", _akmech.fwd_fast,
            "what a generated binding emits; ctypes and cffi cannot produce it")
    if _akmech3:
        add("C extension, abi3, METH_O", _akmech3.fwd_noop)
    if _abi3x:
        add("C extension, abi3 built on 3.10", _abi3x.fwd_noop,
            "the SAME .so file on every interpreter: README 9.2's claim, run")
    if _pyo3:
        add("PyO3", _pyo3.fwd_noop)
        add("PyO3, no crossing", _pyo3.fwd_selfcontained)
        # PyO3 measured far above the C extension on the same callee, so
        # README R2's "a surprising ratio gets a floor arm before it is
        # reported" applies.  These three split the wrapper's prologue from its
        # argument extraction: if the no-argument row is already high, the cost
        # is the prologue and not the conversion.
        if hasattr(_pyo3, "fwd_noargs"):
            cs.append(Case("1. forward: one crossing, Python host -> native",
                           "PyO3, no arguments at all",
                           lambda reps: [_pyo3.fwd_noargs() for _ in range(reps)]
                           and None,
                           "isolates PyO3's per-call prologue from its argument "
                           "extraction"))
            add("PyO3, one i64 argument", _pyo3.fwd_i64)
            add("PyO3, one untyped argument", _pyo3.fwd_any)
    if _pyo3_abi3:
        add("PyO3, abi3 built on 3.10", _pyo3_abi3.fwd_noop)
    add("ctypes, argtypes declared", _ct_typed.ak_noop)
    add("ctypes, no argtypes", _ct.ak_noop,
        "a conversion search per argument, which is the usual spelling")
    if _cffi:
        add("cffi ABI mode (dlopen)", _cffi[1].ak_noop)
    if _cffi_api:
        add("cffi API mode (compiled)", _cffi_api.lib.ak_noop)
    return cs


# --------------------------------------------------------------------------
# Group 2 and 3: reverse.
#
# Every arm drives `ak_reverse_n` inside the C library, so the loop and the
# indirect call are the same in all of them and what differs is what the
# callback does when it is reached.
# --------------------------------------------------------------------------


def reverse_cases():
    cs = []
    g = "2. reverse: native -> host, per reverse call"

    def add(arm, run, note=""):
        cs.append(Case(g, arm, run, note))

    if _akmech:
        add("-- loop in C, no call at all",
            lambda reps: _akmech.rev_floor(reps),
            "the floor: what the loop costs with the crossing removed")
        add("-- C -> C through a function pointer",
            lambda reps: _akmech.rev_cfloor(reps),
            "a real indirect call, no interpreter. The second floor")
        add("C extension: PyObject_CallFunctionObjArgs",
            lambda reps: _akmech.rev_python(_py_cb, reps),
            "into the interpreter: a frame, an argument tuple, bytecode dispatch")
    if _akmech3:
        add("C extension abi3: PyObject_CallFunctionObjArgs",
            lambda reps: _akmech3.rev_python(_py_cb, reps))
    if _pyo3:
        add("PyO3: call1 on a Py<PyAny>",
            lambda reps: _pyo3.rev_python(_py_cb, reps))
    add("ctypes: a CFUNCTYPE callback",
        lambda reps: _ct_typed.ak_reverse_n(_ct_cb, 1, reps),
        "what README 9.1 says the design avoids")
    if _cffi:
        ffi, lib = _cffi
        cb = ffi.callback("uint64_t(uint64_t)", _py_cb)
        add("cffi ABI mode: ffi.callback",
            lambda reps, _cb=cb, _lib=lib: _lib.ak_reverse_n(_cb, 1, reps))
    if _cffi_api:
        add("cffi API mode: extern \"Python\"",
            lambda reps: _cffi_api.lib.ak_reverse_n(
                _cffi_api.lib.ak_py_cb, 1, reps),
            "compiled, but the callback still re-enters the interpreter")
    return cs


# --------------------------------------------------------------------------
# Group 3: the same direction, as a C-API call on a primitive.
# --------------------------------------------------------------------------

GUID36 = "8c8deaf0-3e8d-bdcc-20d5-19afe07cc6c9"


# design/SHAPES.md's three content sets.  The ASCII one is what the real schema
# holds (every id is an ASCII GUID); the other two are where CPython's UTF-8
# handling stops being a pointer return.
ASCII36 = GUID36
LATIN36 = "8c8deaf0-3e8d-bdcc-" + "\u00e9" * 17
WIDE36 = "8c8deaf0-3e8d-bdcc-" + "\u4e2d" * 17


def content_cases():
    """The string path across design/SHAPES.md's three content sets.

    Two things are separated here that a single row would mix:

    *   **cached**: CPython stores a non-ASCII str's UTF-8 form on the object at
        the first `PyUnicode_AsUTF8AndSize`, so a repeated read is a pointer
        return whatever the content is.  An ASCII str needs no cache at all: its
        internal buffer already IS the UTF-8.
    *   **uncached**: a string that just came off the wire has no cache, which
        is the state an encode-after-decode round trip is actually in.  Priced
        as a within-arm delta (build+read minus build), which is what README R4
        asks for rather than a ratio to a third arm.
    """
    cs = []
    g = "3c. the string path across the three content sets (design/SHAPES.md)"
    if not _akmech:
        return cs
    for label, sval in (("ASCII", ASCII36), ("Latin-1", LATIN36),
                        ("above U+00FF", WIDE36)):
        nb = len(sval.encode())
        for op, tag in (("PyUnicode_AsUTF8AndSize", "cached read"),
                        ("PyUnicode_AsUTF8AndSize (uncached: build + read)",
                         "build + read"),
                        ("PyUnicode_AsUTF8AndSize (uncached: build only)",
                         "build only"),
                        ("PyUnicode_FromStringAndSize(36)", "-")):
            if op == "PyUnicode_FromStringAndSize(36)":
                continue
            cs.append(Case(g, "%-13s %s" % (label, tag),
                           lambda reps, _op=op, _o=sval:
                           _akmech.capi(_op, reps, _o),
                           "%d wire bytes" % nb if tag == "cached read" else ""))
    return cs


def capi_cases():
    cs = []
    g = "3. reverse as a C-API call on a primitive (the design's default)"
    if not _akmech:
        return cs
    obj_for = {
        "PyLong_AsLongLong": 1 << 40,
        "PyUnicode_AsUTF8AndSize": GUID36,
        "PyUnicode_AsUTF8String": GUID36,
        "PyBytes_AsStringAndSize": b"\x01" * 16,
    }
    lst = [None, None, None, None]
    obj_for["PyList_SetItem"] = lst
    obj_for["PyList_SET_ITEM"] = lst
    obj_for["PyList_Append"] = []
    obj_for["PyObject_CallNoArgs(type)"] = _akmech.CStamp
    for op in _akmech.available()["ops"]:
        if op.startswith(("PyObject_GetAttr", "PyObject_SetAttr", "struct member")):
            continue  # group 4
        if "uncached" in op:
            continue  # group 3c
        o = obj_for.get(op, None)
        cs.append(Case(g, op, lambda reps, _op=op, _o=o: _akmech.capi(_op, reps, _o)))
    return cs


def capi_abi3_cases():
    """The same primitives through the limited API.

    README 9.2 states the trade (one wheel per minor version against one wheel
    across 3.x) and says the report has to state it rather than discover it.
    What it costs is a measurement, and this is it.
    """
    cs = []
    g = "3b. the same primitives, abi3 build (README 9.2)"
    if not (_akmech and _akmech3):
        return cs
    lst = [None, None, None, None]
    obj_for = {
        "PyLong_AsLongLong": 1 << 40,
        "PyUnicode_AsUTF8AndSize": GUID36,
        "PyUnicode_AsUTF8String": GUID36,
        "PyBytes_AsStringAndSize": b"\x01" * 16,
        "PyList_SetItem": lst,
        "PyList_Append": [],
        "PyObject_CallNoArgs(type)": _akmech.CStamp,
    }
    interesting = [
        "PyLong_FromLongLong",
        "PyUnicode_FromStringAndSize(36)",
        "PyUnicode_AsUTF8AndSize",
        "PyBytes_FromStringAndSize(16)",
        "PyList_SetItem",
        "PyList_Append",
        "PyObject_CallNoArgs(type)",
    ]
    for op in interesting:
        o = obj_for.get(op, None)
        cs.append(Case(g, "full API: " + op,
                       lambda reps, _op=op, _o=o: _akmech.capi(_op, reps, _o)))
        cs.append(Case(g, "abi3:     " + op,
                       lambda reps, _op=op, _o=o: _akmech3.capi(_op, reps, _o)))
    return cs


# --------------------------------------------------------------------------
# Group 4: a facade field read, at the primitive level.
# --------------------------------------------------------------------------


class PlainFacade:
    """A plain class: attributes live in an instance __dict__."""

    def __init__(self):
        self.session_id = GUID36
        self.size = 1 << 40


class SlotsFacade:
    """__slots__: a descriptor with a fixed offset, no instance dict."""

    __slots__ = ("session_id", "size")

    def __init__(self):
        self.session_id = GUID36
        self.size = 1 << 40


def pyo3_capi_cases():
    """The same primitives in PyO3's idiom.

    README 9.1 calls PyO3 "the same C-API calls with a Rust spelling".  The
    forward row says the wrapper is not free; this group says whether the
    PRIMITIVES are, which is the half that matters, because a shim makes one
    forward call and thousands of primitive calls.
    """
    cs = []
    g = "3d. the same primitives, PyO3 spelling against the C-API spelling"
    if not (_akmech and _pyo3):
        return cs
    obj_for = {
        "PyLong_AsLongLong": 1 << 40,
        "PyUnicode_AsUTF8AndSize": GUID36,
        "PyObject_CallNoArgs(type)": _akmech.CStamp,
        "PyList_Append": [],
    }
    for op in _pyo3.available()["ops"]:
        if op == "floor" or op.startswith(("PyObject_GetAttr", "PyObject_SetAttr")):
            continue
        o = obj_for.get(op, None)
        if op in _akmech.available()["ops"]:
            cs.append(Case(g, "C-API: " + op,
                           lambda reps, _op=op, _o=o: _akmech.capi(_op, reps, _o)))
        cs.append(Case(g, "PyO3:  " + op,
                       lambda reps, _op=op, _o=o: _pyo3.capi(_op, reps, _o)))
    return cs


def storage_cases():
    cs = []
    g = "4. a facade field read, as the shim sees it (README 9.1)"
    if not _akmech:
        return cs
    plain, slots = PlainFacade(), SlotsFacade()
    cext = _akmech.CResultRaw(session_id=GUID36, size=1 << 40)
    key = "session_id"

    def add(arm, obj, op="PyObject_GetAttr", note="", mod=None):
        m = mod or _akmech
        cs.append(Case(g, arm,
                       lambda reps, _o=obj, _op=op, _m=m: _m.capi(_op, reps, _o, key),
                       note))

    cs.append(Case(g, "-- loop floor (no read)",
                   lambda reps: _akmech.capi("floor", reps)))
    add("plain class, PyObject_GetAttr", plain, note="a dict lookup")
    add("__slots__ class, PyObject_GetAttr", slots, note="a descriptor with an offset")
    add("C extension type, PyObject_GetAttr", cext,
        note="a member descriptor: still a crossing")
    add("C extension type, struct member read", cext, op="struct member read",
        note="not a crossing at all: the shim casts and loads")
    add("C extension type, struct member read (int64)", cext,
        op="struct member read (int64)",
        note="an unboxed scalar: no PyLong on either side")
    add("plain class, PyObject_GetAttrString", plain, op="PyObject_GetAttrString",
        note="the spelling that rebuilds the name string on every call")
    if _pyo3:
        add("PyO3 .getattr(), plain class", plain, mod=_pyo3)
        add("PyO3 .getattr(), __slots__ class", slots, mod=_pyo3)

    # The Python-level control: what the same read costs in bytecode.  It is not
    # a crossing, so it is the floor a shim is trying to beat by not being one.
    def py_read(reps, o=plain):
        for _ in range(reps):
            o.session_id
    cs.append(Case(g, "-- pure Python: o.session_id (plain)", py_read,
                   "bytecode, no crossing. The control for this group"))

    def py_read_s(reps, o=slots):
        for _ in range(reps):
            o.session_id
    cs.append(Case(g, "-- pure Python: o.session_id (__slots__)", py_read_s))

    # The two rows above carry the Python for-loop and the C rows do not, so a
    # naive comparison across them is a comparison of loops. These give the
    # MARGINAL cost of one bytecode attribute read as a within-arm delta
    # (README R4): (4 reads - 1 read) / 3, with the loop identical in both.
    def py_read4(reps, o=plain):
        for _ in range(reps):
            o.session_id
            o.session_id
            o.session_id
            o.session_id
    cs.append(Case(g, "-- pure Python: 4x o.session_id (plain)", py_read4,
                   "marginal read = (this - the 1x row) / 3, loop cancelled"))
    return cs


def main():
    out = sys.stdout
    print("# python slice, work unit 1: binding mechanism and facade storage", file=out)
    print("#", file=out)
    print("# interpreter:   %s" % sys.version.replace("\n", " "), file=out)
    print("# executable:    %s" % sys.executable, file=out)
    print("# build dir:     %s" % os.path.join(BUILD, TAG), file=out)
    print("# rounds:        %d, interleaved; per-case target %.0f ms"
          % (harness.ROUNDS, harness.TARGET_NS / 1e6), file=out)
    print("# gc:            disabled for the measured rounds", file=out)
    if _akmech:
        print("# full-API ops:  %d" % len(_akmech.available()["ops"]), file=out)
    if _akmech3:
        a3 = _akmech3.available()
        print("# abi3 ops:      %d, Py_LIMITED_API=0x%08x"
              % (len(a3["ops"]), a3["limited_api"]), file=out)
        if _akmech:
            gone = sorted(set(_akmech.available()["ops"]) - set(a3["ops"]))
            print("# absent under abi3: %s" % (", ".join(gone) or "none"), file=out)
    for m in missing:
        print("# ARM NOT LOADED: %s" % m, file=out)

    arms = [("-- a pure-Python function call", py_noop)]
    if _akmech:
        arms += [("C extension, METH_O", _akmech.fwd_noop),
                 ("C extension, METH_FASTCALL", _akmech.fwd_fast)]
    if _akmech3:
        arms += [("C extension, abi3, METH_O", _akmech3.fwd_noop)]
    if _abi3x:
        arms += [("C extension, abi3 built on 3.10", _abi3x.fwd_noop)]
    if _pyo3:
        arms += [("PyO3", _pyo3.fwd_noop)]
    if _pyo3_abi3:
        arms += [("PyO3, abi3 built on 3.10", _pyo3_abi3.fwd_noop)]
    arms += [("ctypes, argtypes declared", _ct_typed.ak_noop),
             ("ctypes, no argtypes", _ct.ak_noop)]
    if _cffi:
        arms += [("cffi ABI mode (dlopen)", _cffi[1].ak_noop)]
    if _cffi_api:
        arms += [("cffi API mode (compiled)", _cffi_api.lib.ak_noop)]
    bad = gate(arms, out)
    if bad:
        print("\n#  NOTE: the arms above marked WRONG are still timed, and their",
              file=out)
        print("#  rows carry this line.  A mechanism that does not do the work is",
              file=out)
        print("#  not a faster mechanism (README R2).", file=out)

    cases = forward_cases() + reverse_cases() + capi_cases() + content_cases() \
        + capi_abi3_cases() + pyo3_capi_cases() + storage_cases()
    harness.run(cases)

    def base(group):
        if group.startswith("1."):
            return "-- a pure-Python function call"
        if group.startswith("2."):
            return "-- C -> C through a function pointer"
        if group.startswith("4."):
            return "C extension type, struct member read"
        return None

    rows = harness.report(cases, base, out)
    if "--json" in sys.argv:
        harness.dump_json(rows, sys.argv[sys.argv.index("--json") + 1])


if __name__ == "__main__":
    main()
