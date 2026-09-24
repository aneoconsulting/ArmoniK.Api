#!/usr/bin/env bash
# R-D2's layout guard, shown compiling AND shown firing.
#
#   pass     every RPC source compiles against the generated `ak_abi.h` (C++17, as built),
#            and `ak_abi.h` alone compiles at C++11 and C++14 too.
#   plant A  the header's struct loses a field (the R-D2 shape: host smaller than core).
#            The header's own sizeof/offsetof asserts must refuse it.
#   plant B  the Rust declaration gains a SEVENTH field. The renderer re-emits the header
#            from a scratch copy of ak-abi's lib.rs; `generate.py --check` must call the
#            committed header STALE, and `rpc_common.h`'s field-count assert must refuse
#            to compile, because `core_opts()` would leave the new field unset.
set -u
cd "$(dirname "$0")/.." || exit 2
S=$(mktemp -d)
trap 'rm -rf "$S"' EXIT
GEN=build/gen
GRPC_CFLAGS=$(pkg-config --cflags grpc++ 2>/dev/null)
bad=0
ok()   { echo "  >>> ok: $1"; }
fail() { echo "  >>> FAIL: $1"; bad=$((bad + 1)); }

printf '#include "ak_abi.h"\nint main(){ struct ak_client_opts o; (void)o; return (int)sizeof o - 24; }\n' > "$S/t.cpp"

echo "===== pass: ak_abi.h alone, at C++11, C++14, C++17 ====="
for std in 11 14 17; do
  if g++ -std=c++$std -Iinclude "$S/t.cpp" -o "$S/t$std" && "$S/t$std"; then
    ok "c++$std compiles, sizeof(ak_client_opts) == 24 at run time"
  else fail "c++$std"; fi
done
echo
echo "===== pass: every RPC source against the generated header (C++17) ====="
for f in src/rpc_common.cpp src/rpcbench.cpp src/rpcflow.cpp src/rpccounts.cpp; do
  if g++ -std=c++17 -fsyntax-only -Iinclude -Isrc -I"$GEN" $GRPC_CFLAGS "$f"; then ok "$f"
  else fail "$f"; fi
done
echo
echo "===== plant A: the struct loses tcp_nagle (host smaller than core) -- must NOT compile ====="
mkdir -p "$S/a"
grep -v '^  int32_t tcp_nagle;$' include/ak_abi.h > "$S/a/ak_abi.h"
if g++ -std=c++11 -I"$S/a" "$S/t.cpp" -o "$S/ta" 2> "$S/a.err"; then
  fail "plant A compiled: the header's asserts cannot see a missing field"
else
  grep -m2 'static assertion failed\|static_assert' "$S/a.err" | sed 's/^/    /'
  ok "plant A refused"
fi
echo
echo "===== plant B: the Rust declaration gains a seventh field ====="
mkdir -p "$S/b"
python3 - "$S/b" <<'EOF' 2>&1 | grep -v -i 'distutils\|traceback\|frozen site\|string>\|remainder'
import os, re, sys
out = sys.argv[1]
sys.path.insert(0, "gen")
import generate as G
import cpp_header
src = open(cpp_header.AK_ABI_RS).read()
src2 = src.replace("    pub tcp_nagle: i32,\n}", "    pub tcp_nagle: i32,\n    pub keepalive_ms: u32,\n}", 1)
assert src2 != src, "anchor not found in ak-abi lib.rs"
scratch = os.path.join(out, "lib.rs")
open(scratch, "w").write(src2)
cpp_header.AK_ABI_RS = scratch
ir = G.IR.load(G.ROOTS)
header, _, _ = cpp_header.emit(ir)
open(os.path.join(out, "ak_abi.h"), "w").write(header)
committed = open("include/ak_abi.h").read()
print("  generate.py --check would say:", "STALE include/ak_abi.h" if header != committed else "ok (WRONG)")
EOF
if grep -q 'keepalive_ms' "$S/b/ak_abi.h"; then ok "the renderer picked up the new field"; else fail "renderer"; fi
if g++ -std=c++17 -fsyntax-only -I"$S/b" -Iinclude -Isrc -I"$GEN" $GRPC_CFLAGS src/rpc_common.cpp 2> "$S/b.err"; then
  fail "plant B compiled: core_opts() would leave the seventh field unset"
else
  grep -m2 'static assertion failed' "$S/b.err" | sed 's/^/    /'
  ok "plant B refused by rpc_common.h"
fi
echo
echo "rd2_guard: $bad check(s) did not do what they must"
exit $((bad ? 1 : 0))
