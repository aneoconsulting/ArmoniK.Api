#!/usr/bin/env bash
# R-D4: the one generated shim source against REAL CPython 3.7 headers (./fetch_py37.sh),
# with a control that must fail: the same check on the pre-port tree (commit aba944a, the
# last one whose shim came from poc/python/gen/py_binding.py).
#
# What this establishes: every translation unit this slice builds -- the payload-set shim,
# counting, RPC, corpus, and the chunk256 test build -- compiles with -Wall -Wextra -Werror
# against 3.7.5's headers, so no C-API name newer than 3.7 is reached on the 3.7 side of a
# PY_VERSION_HEX conditional (an undeclared function is -Werror=implicit-function-declaration).
# It also lists where the version conditionals are, so the floor path is visible.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE"
INC="$HERE/build/py37/root/usr/include"
[ -f "$INC/python3.7m/Python.h" ] || { echo "no 3.7 headers: run ./fetch_py37.sh"; exit 1; }
CF="-fsyntax-only -O2 -Wall -Wextra -Werror -Werror=implicit-function-declaration -Wno-unused-parameter -I$INC/python3.7m -I$INC"
echo "# 3.7.5 headers: $INC/python3.7m ($(grep -m1 'define PY_VERSION ' "$INC/python3.7m/patchlevel.h"))"
rc=0
echo
echo "## this tree"
for v in "gen/out|" "gen/out|-DAK_COUNT" "gen/out|-DAK_RPC" "gen/out/corpus|-DAK_CORPUS" "gen/out/corpus|-DAK_CORPUS -DAK_CHUNK_BYTES=256 -DAK_CHUNK_PACKED=3"; do
  d=${v%%|*}; f=${v#*|}
  if out=$(cc $CF -I"$d" $f -DAK_MODNAME_STR='"_akffi"' -DAK_INITFUNC=PyInit__akffi native/binding.c 2>&1); then
    echo "   ok    $d $f"
  else
    echo "   FAIL  $d $f"; echo "$out" | head -20; rc=1
  fi
done
echo
echo "## the version conditionals in the sources compiled above"
grep -n "PY_VERSION_HEX" gen/out/binding.c native/binding.c | sed 's/^/   /'
echo "   post-3.7 names reached only through them:"
for n in Py_NewRef PyObject_CallNoArgs PyObject_CallOneArg PyModule_AddObjectRef; do
  echo "     $n: $(grep -c "\b$n\b" gen/out/binding.c) in gen/out/binding.c, $(grep -c "\b$n\b" gen/out/corpus/binding.c) in gen/out/corpus/binding.c, $(grep -c "\b$n\b" native/binding.c) in native/binding.c"
done
echo
echo "## the same five variants against every other CPython header set reachable here"
# 3.9 is the one level where the two conditionals split (PyObject_CallNoArgs exists,
# Py_NewRef does not); its headers are focal's libpython3.9-dev (headers only, sha-pinned).
P39="$HERE/build/py39"
DEB39=libpython3.9-dev_3.9.5-3ubuntu0~20.04.1_amd64.deb
SHA39=b46f3ee14f34019f6f9294338b3c2bc6042df2e6844a01f1dee163cba9ba1df7
mkdir -p "$P39"
[ -f "$P39/$DEB39" ] || curl -sS -o "$P39/$DEB39" "http://archive.ubuntu.com/ubuntu/pool/universe/p/python3.9/$DEB39"
if echo "$SHA39  $P39/$DEB39" | sha256sum -c --quiet - && dpkg-deb -x "$P39/$DEB39" "$P39/root"; then
  LEVELS="3.9|-I$P39/root/usr/include/python3.9 -I$P39/root/usr/include"
else
  echo "   3.9 headers unavailable"; LEVELS=""
fi
for v in 3.10 3.11 3.12 3.13; do
  [ -f "/usr/include/python$v/Python.h" ] && LEVELS="$LEVELS
$v|-I/usr/include/python$v"
done
while IFS= read -r lv; do
  [ -z "$lv" ] && continue
  ver=${lv%%|*}; inc=${lv#*|}
  bad=0
  for v in "gen/out|" "gen/out|-DAK_COUNT" "gen/out|-DAK_RPC" "gen/out/corpus|-DAK_CORPUS" "gen/out/corpus|-DAK_CORPUS -DAK_CHUNK_BYTES=256 -DAK_CHUNK_PACKED=3"; do
    d=${v%%|*}; f=${v#*|}
    cc -fsyntax-only -O2 -Wall -Wextra -Werror -Werror=implicit-function-declaration -Wno-unused-parameter $inc \
       -I"$d" $f -DAK_MODNAME_STR='"_akffi"' -DAK_INITFUNC=PyInit__akffi native/binding.c 2>/dev/null || bad=$((bad+1))
  done
  if [ $bad -eq 0 ]; then echo "   ok    CPython $ver headers: 5 of 5 variants"; else echo "   FAIL  CPython $ver headers: $bad of 5 variants"; rc=1; fi
done <<< "$LEVELS"

echo
echo "## control: the pre-port tree (aba944a) must FAIL the same check"
T="$HERE/build/floor-control"
rm -rf "$T"; mkdir -p "$T/native" "$T/gen/out"
for f in native/binding.c gen/out/binding.c gen/out/ak_abi.h; do
  git -C "$HERE" show "aba944a:ffi/poc/python/$f" > "$T/$f"
done
if out=$(cd "$T" && cc $CF -Igen/out -DAK_MODNAME_STR='"_akffi"' -DAK_INITFUNC=PyInit__akffi native/binding.c 2>&1); then
  echo "   CONTROL PASSED -- the check cannot see a post-3.7 call"; rc=1
else
  echo "   failed as required:"
  echo "$out" | grep -oE "implicit declaration of function '[A-Za-z_]+'" | sort | uniq -c | sed 's/^/     /'
fi
echo
[ $rc -eq 0 ] && echo "FLOOR SOURCE CHECK PASSES (compile against 3.7.5 headers; the control fails)" || echo "FLOOR SOURCE CHECK FAILED"
exit $rc
