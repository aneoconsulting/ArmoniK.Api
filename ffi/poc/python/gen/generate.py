#!/usr/bin/env python3
"""The python slice's generator front end: build glue only (FIX-PLAN WP5 step 5).

Every generated file of this slice is rendered by the SHARED generator in
`poc/codec/gen/` from a PLAN (`plan.py`); this script only chooses the plans and writes the
text where the slice's build expects it. It contains no wire rule, no IR and no layout
derivation (CLAUDE.md, one generator):

| backend (poc/codec/gen/) | renders |
|---|---|
| `py_pure.emit_facade`   | `facade.py`: the Plain / `__slots__` facade and its metadata table |
| `py_pure.emit_pycodec`  | `pycodec.py` (unknown fields dropped), `pycodec_retain.py` (retained) |
| `py_capi.emit`          | `binding.c`: the CPython shim over the core, `ak_init` included |
| `c_abi.emit` (the one C header backend, plain C99) | `ak_abi.h`: groups, vtables, entry points, `plan.rpc`, `plan.lifecycle` |

Two plan sets, two output directories:

  gen/out/          `ffi/schema/shapes.json`, the seven roots every slice's core carries
  gen/out/corpus/   the conformance corpus's READER schema (CONTRACT.md rule 0): the shim
                    over the corpus-feature core for every root the C ABI can carry, and
                    the facade and pure-Python codec for every corpus message (the one
                    recursive message, `Nest`, is refused by the C ABI by name and runs
                    through the pure-Python codec only)

The shared generator is read from THIS checkout (the `AK_UPSTREAM` snapshot mechanism is
retired: the slice now reads only the committed shared generator, and a build log records
the commit and whether `poc/codec` had uncommitted changes).

  gen/generate.py            write gen/out/
  gen/generate.py --check    fail if what is committed differs, or if a python backend
                             module imports anything but plans (the shared guard, applied
                             to these modules, with its planted violation)
"""
import importlib.util
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))       # .../poc/python/gen
SLICE = os.path.dirname(HERE)
POC = os.path.dirname(SLICE)
CODECGEN = os.path.join(POC, "codec", "gen")
if CODECGEN not in sys.path:
    sys.path.insert(0, CODECGEN)

import plan as P         # noqa: E402  the rule layer
import py_pure           # noqa: E402
import py_capi           # noqa: E402
import c_abi             # noqa: E402  the ONE C header backend (plain C99)

OUT = os.path.join(HERE, "out")
PY_BACKENDS = ["py_pure.py", "py_capi.py"]

# The roots every slice's core carries (poc/codec/gen/generate.py ROOTS), read from the
# shared generator rather than listed here, so the shim and the core cannot disagree.


def _shared_generate():
    spec = importlib.util.spec_from_file_location("ak_shared_generate",
                                                  os.path.join(CODECGEN, "generate.py"))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def outputs():
    G = _shared_generate()
    p = P.load(G.ROOTS)
    for r in p.roots:
        P.check_direct(p, r)
    out = {
        "facade.py": py_pure.emit_facade(p),
        "pycodec.py": py_pure.emit_pycodec(p, "drop"),
        "pycodec_retain.py": py_pure.emit_pycodec(p, "retain"),
        "binding.c": py_capi.emit(p, "_akffi"),
        "ak_abi.h": c_abi.emit(p)[0],
    }
    # The corpus: the C ABI's roots are the ones the shared generator's corpus core carries.
    roots, _refused = G.corpus_roots()
    cp = P.load_corpus(roots)
    full = P.load_corpus(P.corpus_messages())
    out.update({
        "corpus/facade.py": py_pure.emit_facade(full),
        "corpus/pycodec.py": py_pure.emit_pycodec(full, "drop"),
        "corpus/pycodec_retain.py": py_pure.emit_pycodec(full, "retain"),
        "corpus/binding.c": py_capi.emit(cp, "_akffi_corpus", backends=("attr", "cext")),
        "corpus/ak_abi.h": c_abi.emit(cp)[0],
    })
    return out, G


def guard(G):
    sources = {b: open(os.path.join(CODECGEN, b)).read() for b in PY_BACKENDS}
    bad = G.guard_violations(sources)
    caught = G.guard_violations({"planted.py": "import ir as IR\nfrom shapes import load\n"}
                                ).get("planted.py") == ["ir", "shapes"]
    return bad, caught


def main():
    check = "--check" in sys.argv
    outs, G = outputs()
    rc = 0
    if check:
        bad, caught = guard(G)
        for mod, hit in sorted(bad.items()):
            print("GUARD %s imports %s: a backend takes plans, not the IR or the schema"
                  % (mod, ", ".join(hit)))
            rc = 1
        if not caught:
            print("GUARD the planted violation was NOT caught: the guard is blind")
            rc = 1
        if not bad and caught:
            print("guard %d python backend modules import plans only; a planted IR import is caught"
                  % len(PY_BACKENDS))
    stale = []
    for name, text in sorted(outs.items()):
        path = os.path.join(OUT, name)
        old = open(path).read() if os.path.exists(path) else None
        if old != text:
            stale.append(name)
            if not check:
                os.makedirs(os.path.dirname(path), exist_ok=True)
                with open(path, "w") as f:
                    f.write(text)
    if check:
        if stale:
            print("STALE, regenerate: %s" % ", ".join(stale))
            return 1
        print("generated tree is current with the plans (%d files)" % len(outs))
        return rc
    print("wrote %d files to %s%s" % (len(outs), OUT, (" (changed: %s)" % ", ".join(stale)) if stale else ""))
    return rc


if __name__ == "__main__":
    sys.exit(main())
