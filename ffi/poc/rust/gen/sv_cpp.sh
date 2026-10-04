#!/usr/bin/env bash
# Static decode vtables (owner, 2026-10-04): the C++ slice on its regenerated bindings (one
# `static const struct ak_dvt_<Root> k_dvt_<Root>` per root, positional aggregate initialisation).
# Nothing written under poc/cpp or logs/cpp: every build directory and the core's cargo output
# are under the scratch dir (AK_CORE_TGT); the core sources are this checkout's poc/codec.
#
#   check   configure poc/cpp (-DAK_RPC=ON, Google Benchmark v1.8.3 release from
#           poc/cpp/build-campaign/gbench-v1.8.3-release, as gen/run_campaign.sh), build the
#           conformance arms a17 shared / a17 static / c11 shared / c14 shared / b17 shared and
#           nounk a17 / nounk c11 / nounk static, the corpus arms, the counting arms and
#           campaign_codec(_nounk); RUN each conformance arm (`payloads`, exit 0), the corpus on
#           both builds (corpus_all_a17 / _c11 / _a17_static, corpus_nounk_a17), the counts
#           against logs/cpp/counts-baseline.log (shared, static) and counts-nounk-baseline.log,
#           each campaign_codec's own pre-check (--rounds 0); the k_dvt_ objects in the binaries
#           (nm); every generated binding compiled at -std=c++11, c++14, c++17 with
#           -Wall -Wextra -Werror=missing-field-initializers (a member left out of an aggregate
#           is an error); the same with a member PLANTED out (the check must fail).
#   asan    an ASan + LSan build (as poc/cpp/gen/d11_asan.sh): conformance_a17_shared and
#           conformance_nounk_a17 (`payloads`), corpus_all_a17 and corpus_nounk_a17 under ASan.
#
#   gen/sv_cpp.sh <scratch dir> check|asan
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
POC="$(cd "$HERE/../.." && pwd)"
REPO="$(git -C "$POC" rev-parse --show-toplevel)"
S="${1:?usage: gen/sv_cpp.sh <scratch dir> check|asan}"; mkdir -p "$S"; S="$(cd "$S" && pwd)"
WHAT="${2:?mode}"
GB="$POC/cpp/build-campaign/gbench-v1.8.3-release/lib/cmake/benchmark"
CORE_ROOT="$POC/codec"; CORE_TGT="$S/cpp-core"
PAY="$REPO/ffi/schema/generated"; COR="$REPO/ffi/corpus/generated"
fails=0
ok() { echo "  ok    $*"; }
bad() { echo "  FAIL  $*"; fails=$((fails+1)); }
echo "# sv_cpp $WHAT at $(git -C "$REPO" rev-parse --short HEAD)$(git -C "$REPO" diff --quiet HEAD -- ffi/poc/codec ffi/poc/cpp || echo ' + uncommitted changes'); $(cmake --version | head -1); $(g++ --version | head -1); $(date -u +%FT%TZ)"
configure() {  # src build-dir [extra cmake args...]
  local src=$1 b=$2; shift 2
  cmake -S "$src" -B "$b" -DCMAKE_BUILD_TYPE=Release -DAK_CORE_ROOT="$CORE_ROOT" -DAK_CORE_TGT="$CORE_TGT" "$@" \
    > "$b.configure.log" 2>&1 && ok "configure $b" || { bad "configure $b"; tail -20 "$b.configure.log"; }
}
build() {  # build-dir targets...
  local b=$1; shift
  cmake --build "$b" -j"$(nproc)" --target "$@" > "$b.build.log" 2>&1 && ok "build $*" \
    || { bad "build"; grep -E "error|undefined" "$b.build.log" | head -20; }
}
python3 "$POC/cpp/gen/u_rows.py" "$COR" "$S/rows.tsv" 2>/dev/null
# The core's cargo output starts as a copy of the C++ slice's (only to save build time; cargo
# rebuilds whatever its fingerprints say is stale). The sources are this checkout's poc/codec.
[ -d "$CORE_TGT" ] || cp -a "$POC/cpp/core-build" "$CORE_TGT"

case "$WHAT" in
check)
  B="$S/cpp-after"
  if [ -z "${AK_SV_PLANT_ONLY:-}" ]; then
  configure "$POC/cpp" "$B" -DAK_RPC=ON -Dbenchmark_DIR="$GB"
  CONF="conformance_a17_shared conformance_a17_static conformance_c11_shared conformance_c14_shared conformance_b17_shared conformance_nounk_a17 conformance_nounk_c11 conformance_nounk_static"
  CORP="corpus_all_a17 corpus_all_c11 corpus_all_a17_static corpus_nounk_a17"
  build "$B" $CONF $CORP counts_a17_shared counts_a17_static counts_nounk campaign_codec campaign_codec_nounk
  echo "  core: $CORE_TGT/target/release/libak_core.so"
  for c in $CONF; do
    (cd "$PAY" && "$B/$c" payloads > "$S/conf-$c.log" 2>&1); rc=$?
    [ $rc = 0 ] && ok "$c: exit 0: $(grep -E 'checks,' "$S/conf-$c.log" | tail -1)" \
      || { bad "$c: exit $rc"; grep -m5 FAIL "$S/conf-$c.log"; }
  done
  for c in corpus_all_a17 corpus_all_c11 corpus_all_a17_static; do
    (cd "$POC/cpp" && python3 gen/corpus_all.py "$B/$c" --timeout 60 --max-retain-gap U-map-entry > "$S/corp-$c.log" 2>&1); rc=$?
    grep -E '^   pass|^CORPUS' "$S/corp-$c.log" | sed "s/^/        $c: /"
    [ $rc = 0 ] && ok "corpus $c" || bad "corpus $c"
  done
  (cd "$POC/cpp" && python3 gen/corpus_all.py "$B/corpus_nounk_a17" --timeout 60 --expect-dropped ffi-drop,ffi-pull-drop > "$S/corp-nounk.log" 2>&1); rc=$?
  grep -E '^   pass|^CORPUS' "$S/corp-nounk.log" | sed "s/^/        corpus_nounk_a17: /"
  [ $rc = 0 ] && ok "corpus corpus_nounk_a17" || bad "corpus corpus_nounk_a17"
  for v in a17_shared a17_static nounk; do
    exe=counts_$v; base=counts-baseline.log; [ $v = nounk ] && base=counts-nounk-baseline.log
    (cd "$PAY" && "$B/$exe" --corpus "$COR" --rows "$S/rows.tsv" > "$S/counts-$v.log" 2>&1)
    if diff <(grep -E '^  [PU]' "$REPO/ffi/logs/cpp/$base") <(grep -E '^  [PU]' "$S/counts-$v.log") > "$S/counts-$v.diff"; then
      ok "counts $exe: $(grep -cE '^  [PU]' "$S/counts-$v.log") rows identical to logs/cpp/$base"
    else bad "counts $exe differ from logs/cpp/$base"; head -10 "$S/counts-$v.diff"; fi
  done
  for c in campaign_codec campaign_codec_nounk; do
    (cd "$PAY" && "$B/$c" --rounds 0 --pool-bytes 1048576 --corpus "$COR" --rows "$S/rows.tsv" > "$S/pre-$c.log" 2>&1); rc=$?
    [ $rc = 0 ] && ok "$c pre-check: $(grep -o '"campaign_codec_gate": {[^}]*}' "$S/pre-$c.log")" \
      || { bad "$c pre-check exit $rc"; grep -m5 'GATE FAIL' "$S/pre-$c.log"; }
  done
  for c in conformance_a17_shared conformance_c11_shared conformance_nounk_a17 campaign_codec campaign_codec_nounk; do
    echo "        $c: $(nm -C "$B/$c" | grep -cE ' [rRdD] .*k_dvt_[A-Za-z]+$') k_dvt_ objects in a read-only / data section (nm -C; 7 roots)"
  done
  # Every generated binding at every standard level, a member left out of an aggregate = error.
  for t in conformance_c11_shared:src/generated/binding.cpp conformance_c11_shared:src/generated/binding_borrow.cpp \
           conformance_nounk_c11:src/generated/binding_nounk.cpp conformance_nounk_c11:src/generated/binding_borrow_nounk.cpp \
           corpus_all_c11:corpus/src/generated/binding.cpp corpus_nounk_a17:corpus/src/generated/binding_nounk.cpp; do
    tg=${t%%:*}; f=${t#*:}; fm="$B/CMakeFiles/$tg.dir/flags.make"
    inc=$(grep '^CXX_INCLUDES' "$fm" | cut -d= -f2-); def=$(grep '^CXX_DEFINES' "$fm" | cut -d= -f2-)
    for std in 11 14 17; do
      fl="-std=c++$std -O1 -fsyntax-only -Wall -Wextra -Werror=missing-field-initializers -Wno-unused-parameter"
      [ $std = 11 ] && def2="$def" || def2=$(echo "$def" | sed 's/-DAK_FLOOR_IMPL=1//')
      if eval g++ $fl $def2 $inc "$POC/cpp/$f" > "$S/syn.log" 2>&1; then :; else bad "$f -std=c++$std"; head -5 "$S/syn.log"; fi
    done
    ok "$f compiles at -std=c++11/14/17 with -Werror=missing-field-initializers ($(grep -c '^static const struct ak_dvt_' "$POC/cpp/$f") static vtables)"
  done
  fi  # AK_SV_PLANT_ONLY
  # The control: one member planted out of one aggregate must fail that compile.
  fm="$B/CMakeFiles/conformance_c11_shared.dir/flags.make"
  inc=$(grep '^CXX_INCLUDES' "$fm" | cut -d= -f2-); def=$(grep '^CXX_DEFINES' "$fm" | cut -d= -f2-)
  P="$S/sv_plant_binding.cpp"
  python3 - "$POC/cpp/src/generated/binding.cpp" "$P" <<'PY'
import sys
s = open(sys.argv[1]).read()
k = s.index("static const struct ak_dvt_ListTasksDetailedResponse k_dvt_ListTasksDetailedResponse = {")
e = s.index("};", k)
blk = s[k:e]
lines = [x for x in blk.split("\n") if x.strip()]
# drop the last member, and the comma of the member before it
lines = lines[:-2] + [lines[-2].rstrip(",")]
t = s[:k] + "\n".join(lines) + "\n" + s[e:]
assert t != s
open(sys.argv[2], "w").write(t)
PY
  if eval g++ -std=c++11 -fsyntax-only -Wall -Wextra -Werror=missing-field-initializers -Wno-unused-parameter $def -iquote "$POC/cpp/src/generated" $inc "$P" > "$S/plant.log" 2>&1; then
    bad "planted missing member NOT caught"
  elif grep -q 'missing initializer for member' "$S/plant.log"; then
    ok "planted missing member caught: $(grep -m1 -o 'missing initializer for member[^]]*' "$S/plant.log")"
  else bad "the plant failed for another reason: $(head -3 "$S/plant.log" | tr '\n' ' ')"; fi
  rm -f "$P"
  ;;
asan)
  A="$S/cpp-asan"
  configure "$POC/cpp" "$A" -DCMAKE_CXX_FLAGS="-fsanitize=address -fno-omit-frame-pointer" -DCMAKE_EXE_LINKER_FLAGS="-fsanitize=address"
  build "$A" conformance_a17_shared conformance_nounk_a17 corpus_all_a17 corpus_nounk_a17
  for c in conformance_a17_shared conformance_nounk_a17; do
    (cd "$PAY" && ASAN_OPTIONS=detect_leaks=1 "$A/$c" payloads > "$S/asan-$c.log" 2>&1); rc=$?
    grep -E 'ERROR: (Leak|Address)Sanitizer' "$S/asan-$c.log"
    [ $rc = 0 ] && ok "$c under ASan: exit 0: $(grep -E 'checks,' "$S/asan-$c.log" | tail -1)" || bad "$c under ASan: exit $rc"
  done
  (cd "$POC/cpp" && ASAN_OPTIONS=detect_leaks=1 python3 gen/corpus_all.py "$A/corpus_all_a17" --timeout 120 --max-retain-gap U-map-entry > "$S/asan-corp.log" 2>&1); rc=$?
  grep -E '^   pass|^CORPUS|crash' "$S/asan-corp.log" | head -12 | sed 's/^/        /'
  [ $rc = 0 ] && ok "corpus_all_a17 under ASan" || bad "corpus_all_a17 under ASan"
  (cd "$POC/cpp" && ASAN_OPTIONS=detect_leaks=1 python3 gen/corpus_all.py "$A/corpus_nounk_a17" --timeout 120 --expect-dropped ffi-drop,ffi-pull-drop > "$S/asan-corpn.log" 2>&1); rc=$?
  grep -E '^   pass|^CORPUS|crash' "$S/asan-corpn.log" | head -12 | sed 's/^/        /'
  [ $rc = 0 ] && ok "corpus_nounk_a17 under ASan" || bad "corpus_nounk_a17 under ASan"
  ;;
*) echo "mode?"; exit 2 ;;
esac
echo "failures: $fails"
[ "$fails" = 0 ] && echo "SV CPP $WHAT PASSED" || echo "SV CPP $WHAT FAILED"
[ "$fails" = 0 ]
