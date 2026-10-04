#!/usr/bin/env bash
# D20 (owner, 2026-10-04): the Java and Python slices' regenerated code against the D20 core,
# as far as this container allows. Nothing timed; nothing written under poc/java or
# poc/python (everything is built into the scratch dir).
#
#   cores    a git-archive snapshot of HEAD's poc/codec, four ak-core builds: shapes
#            (init-guard), shapes no-unknown, corpus, corpus no-unknown
#   java     the JNI shim (poc/java/native/<variant>/shim.c + tax.c) compiled with gcc against
#            each variant's generated ak_abi.h (its D20 static asserts included) and linked to
#            the matching core; every generated FFM/JNI Binding.java (java17 and java8 trees,
#            shapes and corpus, full and no-unknown) compiled with JDK 21's javac --release 17
#            (this container has no JDK 17 or JDK 8 and no resolved deps/cp.txt, so the slice's
#            own build and its run-time gate cannot run here)
#   python   the CPython shim (poc/python/native/binding.c over gen/out/<variant>) compiled
#            -Werror against each variant's header and linked to the matching core; each
#            module imported; on the shapes modules (full and no-unknown) one decode of a valid
#            ListResultsResponse and one of the same message with a malformed session_id: the
#            shim passes utf8_skip = 0, so the core must accept the first and refuse the second
#
#   gen/d20_java_py.sh <scratch dir>
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
POC="$(cd "$HERE/../.." && pwd)"
REPO="$(git -C "$POC" rev-parse --show-toplevel)"
S="${1:?usage: gen/d20_java_py.sh <scratch dir>}"; mkdir -p "$S"; S="$(cd "$S" && pwd)"
fails=0
ok() { echo "  ok    $*"; }
bad() { echo "  FAIL  $*"; fails=$((fails+1)); }
echo "# commit $(git -C "$REPO" rev-parse --short HEAD)$(git -C "$REPO" diff --quiet HEAD -- ffi/poc/codec ffi/poc/java ffi/poc/python || echo ' + uncommitted changes'); $(gcc --version | head -1); $(/usr/lib/jvm/java-21-openjdk-amd64/bin/javac -version 2>&1 | tail -1); $(python3.12 --version)"

echo "===== cores (git archive HEAD ffi/poc/codec) ====="
SNAP="$S/snap"; rm -rf "$SNAP"; mkdir -p "$SNAP"
git -C "$REPO" archive HEAD ffi/poc/codec | tar -x -C "$SNAP"
declare -A LIB
corebuild() {  # name flags...
  local n=$1; shift
  if ( cd "$SNAP/ffi/poc/codec" && CARGO_TARGET_DIR="$S/core-$n" cargo build --release -q -p ak-core "$@" ) > "$S/core-$n.log" 2>&1; then
    LIB[$n]="$S/core-$n/release"
    ok "core $n ($*): $(nm -D --defined-only "${LIB[$n]}/libak_core.so" | grep -c ' T ak_dec_set_pvt_') pvt setters"
  else bad "core $n"; tail -10 "$S/core-$n.log"; fi
}
corebuild full --features init-guard
corebuild nounk --no-default-features --features init-guard
corebuild corpus --features corpus,init-guard
corebuild corpus-nounk --no-default-features --features corpus,init-guard

echo "===== java: JNI shim (gcc) and generated bindings (javac) ====="
JDK=/usr/lib/jvm/java-21-openjdk-amd64
for v in generated:full generated_nounk:nounk generated_corpus:corpus generated_corpus_nounk:corpus-nounk; do
  d=${v%%:*}; c=${v#*:}
  if gcc -O2 -fPIC -shared -std=c11 -Wall -Wextra -Wno-unused-parameter -I"$JDK/include" -I"$JDK/include/linux" \
       -I"$POC/java/native/$d" -o "$S/libakjni-$c.so" "$POC/java/native/$d/shim.c" "$POC/java/native/tax.c" \
       -L"${LIB[$c]}" -lak_core -Wl,-rpath,"${LIB[$c]}" 2> "$S/jni-$c.err"; then
    ok "JNI shim $d against core $c ($(grep -c 'warning' "$S/jni-$c.err") warnings)"
  else bad "JNI shim $d"; head -20 "$S/jni-$c.err"; fi
done
for t in generated generated_nounk generated_corpus generated_corpus_nounk; do
  case $t in generated_corpus) base=generated;; generated_corpus_nounk) base=generated_nounk;; *) base=$t;; esac
  for lv in java17 java8; do
    for f in $(cd "$POC/java" && find "src/$t/$lv" -name 'Binding.java' | sort); do
      o="$S/javac/$(echo "$f" | tr '/' '_')"; mkdir -p "$o"
      if ( cd "$POC/java" && "$JDK/bin/javac" --release 17 -nowarn -encoding UTF-8 -d "$o" \
             -sourcepath "src/java:src/$base/$lv:src/$base/shared:src/$t/$lv:src/$t/shared" "$f" ) > "$o.log" 2>&1; then
        ok "javac $f"
      else bad "javac $f"; grep -A2 "error" "$o.log" | head -10; fi
    done
  done
done

echo "===== python: shims, import, and the UTF-8 policy through the shim ====="
PY=python3.12
INC=$($PY -c 'import sysconfig;print(sysconfig.get_paths()["include"])'); PINC=$($PY -c 'import sysconfig;print(sysconfig.get_paths()["platinclude"])')
SOABI=$($PY -c 'import sysconfig;print(sysconfig.get_config_var("EXT_SUFFIX"))')
mkdir -p "$S/py"
for v in "out:full:_akffi:" "out/nounk:nounk:_akffi_nounk:-DAK_NOUNK" "out/corpus:corpus:_akffi_corpus:-DAK_CORPUS" "out/corpus-nounk:corpus-nounk:_akffi_corpus_nounk:-DAK_CORPUS -DAK_NOUNK"; do
  IFS=: read -r g c m fl <<< "$v"
  if cc -O2 -fPIC -Wall -Wextra -Werror -Wno-unused-parameter -fvisibility=hidden -shared -I"$INC" -I"$PINC" -I"$(dirname "$INC")" \
       -I"$POC/python/gen/$g" $fl -DAK_MODNAME_STR="\"$m\"" -DAK_INITFUNC="PyInit_$m" -o "$S/py/$m$SOABI" "$POC/python/native/binding.c" \
       -L"${LIB[$c]}" -lak_core -Wl,-rpath,"${LIB[$c]}" 2> "$S/py-$m.err"; then
    if out=$( cd "$S/py" && $PY -c "import $m; print(len([n for n in dir($m) if not n.startswith('_')]), 'names')" 2>&1 ); then
      ok "python shim $m (gen/$g, -Werror) builds and imports: $out"
    else bad "python import $m: $out"; fi
  else bad "python shim $m"; head -20 "$S/py-$m.err"; fi
done
cat > "$S/py/d20_smoke.py" <<'PYEOF'
# The shapes module's decode of ListResultsResponse (backend "cext", the module's own types):
# a valid session_id, then the same message with a malformed one (0xFF 0x41).
import importlib, sys
m = importlib.import_module(sys.argv[1])
T = tuple(getattr(m, n) for n in m.types())
good = bytes([0x0A, 0x04, 0x0A, 0x02, 0x41, 0x42])
bad = bytes([0x0A, 0x04, 0x0A, 0x02, 0xFF, 0x41])
r = m.decode("cext", "ListResultsResponse", good, T)
print("valid: decoded, %d result(s)" % len(r.results))
try:
    m.decode("cext", "ListResultsResponse", bad, T)
    print("malformed: ACCEPTED -- WRONG (the shim passes utf8_skip = 0)")
    sys.exit(1)
except Exception as e:
    print("malformed: refused (%s: %s)" % (type(e).__name__, e))
PYEOF
for m in _akffi _akffi_nounk; do
  if out=$( cd "$S/py" && $PY d20_smoke.py $m 2>&1 ); then ok "python $m: $(echo "$out" | tr '\n' ' ')"
  else bad "python $m: $(echo "$out" | tr '\n' ' ')"; fi
done
echo "failures: $fails"
[ "$fails" = 0 ] && echo "D20 JAVA/PYTHON CHECK PASSED" || echo "D20 JAVA/PYTHON CHECK FAILED"
