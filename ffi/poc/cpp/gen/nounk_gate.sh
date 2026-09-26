#!/usr/bin/env bash
# FIX-PLAN WP5 step 10: the NO-UNKNOWN build (unknown fields compiled out) gated on its own.
# Correctness only; nothing is timed.
#
#   gen/nounk_gate.sh [BUILD_DIR]     default ./build (already built: gen/wp5_gate.sh builds it)
# Log: ffi/logs/cpp/wp5s10-nounk.log
#
#   1. each variant binary loads the core of its variant: zero u-family exports
#      (ak_uencode_*, ak_uelem*, ak_dec_reset_*) in the libak_core it resolves; the full
#      binaries resolve a core that has them (the check seen distinguishing)
#   2. the two C headers against the two cores (poc/rust/gen/c_variant.sh, run read-only):
#      matched pairs agree, mismatched pairs are caught
#   3. payload byte identity + the variant's layout (240 facts) + rule 6 + the drop of an
#      unknown field: conformance_nounk at C++17, C++11 (floor) and static
#   4. the full corpus on the variant, C++17 and C++11: every arm passes and ffi-drop writes
#      every unknown row in its DROPPED form (--expect-dropped); outcomes identical across
#      the two levels; controls proj/reenc/accept and noinit must fail; the dropped-form
#      check seen failing (the full build's native-retain must fail it)
#   5. crossing counts against the committed logs/cpp/counts-nounk-baseline.log, and the
#      difference against the full build's drop counts (logs/cpp/counts-baseline.log)
set -u
cd "$(dirname "$0")/.." || exit 2
B=${1:-build}
B=$(cd "$B" && pwd) || exit 2   # absolute: the steps below run from other directories
L=../../logs/cpp
PAY=../../schema/generated
S=$(mktemp -d)
trap 'rm -rf "$S"' EXIT
FAILS=0
ok() { echo ">>> ok: $1"; }
bad() { echo ">>> FAIL: $1"; FAILS=$((FAILS+1)); }
py() { python3 "$@" 2> >(grep -v -i 'distutils\|traceback (most recent call last):$\|frozen site\|<string>\|remainder of file\|^ *$' >&2); }
ufam() { nm -D --defined-only "$1" 2>/dev/null | grep -cE ' (ak_uencode_|ak_uelem|ak_dec_reset_)'; }
{
  echo "# cpp slice, WP5 step 10: the no-unknown build"
  echo "#   date       $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "#   machine    $(uname -srm), $(nproc) vCPU, $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //')"
  echo "#   compiler   $(g++ --version | head -1); $(rustc --version)"
  echo "#   commit     $(git rev-parse --short HEAD 2>/dev/null)$(git diff --quiet HEAD -- . ../codec 2>/dev/null || echo ' + uncommitted changes in poc/cpp or poc/codec')"
  echo "#   variant    plan relowered with unknown=\"drop\" (plan.py THE NO-UNKNOWN VARIANT); core"
  echo "#              ak-core --no-default-features --features init-guard(+count|corpus|rpc),"
  echo "#              own target dirs core-build/target-*-nounk; no timing in this log"
  echo
  echo "===== 1. which core each binary loads ====="
  for b in conformance_nounk_a17 conformance_nounk_c11 corpus_nounk_a17 corpus_nounk_c11 counts_nounk \
           campaign_codec_nounk campaign_rpc_nounk; do
    [ -x "$B/$b" ] || { bad "$b not built"; continue; }
    so=$(ldd "$B/$b" | awk '/libak_core/{print $3}'); n=$(ufam "$so")
    echo "  $b -> ${so#$PWD/} : u-family exports $n"
    [ "$n" = 0 ] || bad "$b loads a core WITH the u-family"
  done
  n=$(nm "$B/conformance_nounk_static" | grep -cE ' T (ak_uencode_|ak_uelem|ak_dec_reset_)')
  echo "  conformance_nounk_static (static core): u-family symbols $n"; [ "$n" = 0 ] || bad "static variant has the u-family"
  for b in conformance_a17_shared corpus_all_a17 campaign_codec campaign_rpc; do
    so=$(ldd "$B/$b" | awk '/libak_core/{print $3}'); n=$(ufam "$so")
    echo "  (full) $b -> ${so#$PWD/} : u-family exports $n"
    [ "$n" -gt 0 ] || bad "full binary $b loads a core WITHOUT the u-family"
  done
  [ $FAILS = 0 ] && ok "every variant binary resolves a no-unknown core, every full one a full core"
  echo
  echo "===== 2. the two C headers against the two cores (poc/rust/gen/c_variant.sh) ====="
  if bash ../rust/gen/c_variant.sh core-build/target/release/libak_core.so \
       core-build/target-nounk/release/libak_core.so; then ok "c_variant"; else bad "c_variant"; fi
  echo
  echo "===== 3. payload byte identity, layout, rule 6: conformance_nounk ====="
  for b in conformance_nounk_a17 conformance_nounk_c11 conformance_nounk_static; do
    (cd "$PAY" && timeout 300 "$B/$b" payloads > "$S/c.log" 2>&1; echo $? > "$S/rc")
    grep -E 'layout facts|no-unknown build|checks,|FAIL' "$S/c.log" | sed 's/^/  /'
    [ "$(cat "$S/rc")" = 0 ] && ok "$b" || bad "$b exit $(cat "$S/rc")"
  done
  echo
  echo "===== 4. the full corpus on the no-unknown build ====="
  for b in corpus_nounk_a17 corpus_nounk_c11; do
    py gen/corpus_all.py "$B/$b" --expect-dropped ffi-drop --record "$S/$b.json" > "$S/k.log"; rc=$?
    grep -E '^## |^   pass|dropped form|^# rows|^CORPUS|FAIL' "$S/k.log" | sed 's/^/  /'
    [ $rc = 0 ] && ok "corpus $b" || bad "corpus $b"
  done
  py gen/corpus_all.py --compare "$S/corpus_nounk_a17.json" "$S/corpus_nounk_c11.json" && ok "C++17 = C++11 outcomes" || bad "compare"
  SUB="S-Probe,U-root,X-lenwrap-lrr,E-map,T-dec-root,U-wire-MetricsBatch"
  for p in proj reenc accept; do
    py gen/corpus_all.py "$B/corpus_nounk_a17" --only "$SUB" --plant $p > "$S/p.log"; rc=$?
    [ $rc != 0 ] && ok "control $p failed as required" || bad "control $p passed"
  done
  py gen/corpus_all.py "$B/corpus_nounk_noinit" --only "$SUB" > "$S/p.log"; rc=$?
  echo "  noinit: ffi arm-row failures $(grep -c 'FAIL .*\[ffi-' "$S/p.log")"
  [ $rc != 0 ] && ok "control noinit failed as required" || bad "control noinit passed"
  # R-H22: the no-unknown build has no retaining arm left, so the check is seen failing on
  # the FULL build's native-retain arm (corpus_all_a17), which retains.
  py gen/corpus_all.py "$B/corpus_all_a17" --only U- --expect-dropped native-retain > "$S/p.log"; rc=$?
  grep -E 'dropped form' "$S/p.log" | sed 's/^/  /'
  [ $rc != 0 ] && ok "the dropped-form check fails on a retaining arm (seen failing)" || bad "dropped-form check blind"
  echo
  echo "===== 5. crossing counts ====="
  python3 gen/u_rows.py ../../corpus/generated "$S/rows.tsv" 2>/dev/null
  (cd "$PAY" && "$B/counts_nounk" --corpus "$OLDPWD/../../corpus/generated" --rows "$S/rows.tsv" > "$S/counts.log" 2>&1) || bad "counts_nounk exit"
  grep -E '^  [PU]' "$L/counts-nounk-baseline.log" > "$S/want"; grep -E '^  [PU]' "$S/counts.log" > "$S/got"
  if diff "$S/want" "$S/got" > "$S/d"; then ok "$(wc -l < "$S/got") rows identical to counts-nounk-baseline.log"
  else head "$S/d"; bad "counts differ from counts-nounk-baseline.log"; fi
  echo "  against the full build in drop mode (counts-baseline.log):"
  grep -E '^  [PU]' "$L/counts-baseline.log" | grep -v ' retain ' > "$S/full"
  diff "$S/full" "$S/got" | sed 's/^/    /'
  echo
  echo "nounk_gate: $FAILS failure(s)"
} > "$L/wp5s10-nounk.log" 2>&1
[ $FAILS = 0 ] && ! grep -q '>>> FAIL' "$L/wp5s10-nounk.log"
