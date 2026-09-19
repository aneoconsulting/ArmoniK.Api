"""Build the cffi API-mode arm.

cffi has two modes and they are not the same mechanism, which is why README 9.1
names one of them and not the other:

*   **ABI mode** (`ffi.dlopen`) reads the shared library at run time.  No
    compiler, no build step, and every call goes through libffi.
*   **API mode** (`ffi.set_source` + `ffi.compile`) emits a C extension at build
    time.  Its forward calls are ordinary C calls.  Its **callbacks are not**:
    `@ffi.def_extern()` emits a C function that re-enters the interpreter to
    call a Python function, which is the thing the design avoids.

Both are measured.  The point of measuring them is to establish what routing a
callback through the interpreter costs, not to choose one (README 9.1), and they
stay candidates for the RPC layer where the crossing count is two per call.
"""

import os
import sys

from cffi import FFI

HERE = os.path.dirname(os.path.abspath(__file__))

DECLS = """
uint64_t ak_noop(uint64_t x);
int32_t  ak_noop_buf(const uint8_t *p, size_t n, uint64_t *out);
uint64_t ak_reverse_n(uint64_t (*cb)(uint64_t), uint64_t x, size_t n);
uint64_t ak_reverse_n_floor(uint64_t x, size_t n);
"""


def build(outdir):
    ffi = FFI()
    ffi.cdef(DECLS)
    # The extern "Python" callback: cffi emits a C function of this signature
    # that calls whatever Python function is installed on it.
    ffi.cdef('extern "Python" uint64_t ak_py_cb(uint64_t);')
    ffi.set_source(
        "_akcffi_api",
        '#include <stdint.h>\n#include <stddef.h>\n' + DECLS,
        libraries=["akmech_cabi"],
        library_dirs=[os.path.join(HERE, "build")],
        runtime_library_dirs=[os.path.join(HERE, "build")],
    )
    return ffi.compile(tmpdir=outdir, verbose=False)


if __name__ == "__main__":
    tag = "py%d.%d" % sys.version_info[:2]
    out = os.path.join(HERE, "build", tag)
    os.makedirs(out, exist_ok=True)
    print(build(out))
