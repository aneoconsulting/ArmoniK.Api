#!/usr/bin/env bash
# FIX-PLAN WP5 step 5: the python slice's correctness gate, at the target AND the floor.
#
#   ./gate.sh <target-python> [<floor-python> ...]      e.g. ./gate.sh python3.12 build/py37/python3.7
#
# Correctness only; NO timings (README 1.1). Logs in ffi/logs/python/9x-wp5-*.log:
#   90  build: R0, R1 + backend guard, four cores WITH init-guard, five shims per
#       interpreter, the noinit / layout / no-AK_RPC must-fail controls
#   91  conformance on _akffi (payload-set byte identity both directions, crossing counts)
#   92  conformance on _akffi_rpc
#   93  the WHOLE corpus, five arms, every row under a timeout, and the four planted controls;
#       the floor's run is compared byte for byte with the target's (identical wire bytes
#       across levels)
#   94  rpc_gate.py (R-D3), 95 rd1_lenwrap.py (R-D1), 96 u1_map_unknown.py
#   97  the 3.7 source check: the pre-port tree fails against real 3.7 headers, this one does not
#   98  crossing counts against logs/python/85 (the pre-port shim), row for row; and the
#       rendered C header against the cpp slice's (one renderer, c_abi)
#   WP5 step 10, THE NO-UNKNOWN VARIANT (unknown fields compiled out), per interpreter:
#   100 conformance on _akffi_nounk (counting build _akffi_count_nounk)
#   101 the whole corpus through the variant's shim: every unknown row in the dropped form,
#       byte identity against the full build's drop arms at the same level, the controls
#       and the variant's own (retain refused, no positions, wrong root, per-thread contexts)
#   102 conformance on _akffi_rpc_nounk
#   105 the RPC cells B-E per call on the rpc counting builds, against counts/rpc-{full,nounk}.txt
#   104 camp_summary's test (R-H1): references and groups keyed by build, per-launch medians
#   103 crossing counts, whole numbers per call, full (drop) and no-unknown builds, each
#       against its committed file (counts/crossings-drop.txt, counts/crossings-nounk.txt),
#       and the difference between the two
# A floor interpreter from ./fetch_py37.sh is `build/py37/python3.7`.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOGS="${AK_GATE_LOGS:-$(cd "$HERE/../.." && pwd)/logs/python}"   # run_campaign.sh points it at its --out
mkdir -p "$LOGS"
cd "$HERE"
TARGET="$1"
hdr() {
  echo "# $1"
  echo "# date:      $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# commit:    $(git -C "$HERE" rev-parse --short HEAD)$(git -C "$HERE" diff --quiet HEAD -- "$HERE" ../codec || echo ' + uncommitted changes in poc/python or poc/codec')"
  echo "# phase:     correctness only; nothing here is a timing"
  echo
}
tagof() { "$1" -c 'import sys;print("py%d.%d"%sys.version_info[:2])'; }
rc=0

echo "===== 90. build ====="
{ hdr "python slice: build, WP5 step 5 (plan-rendered shim, init-guard cores)"; ./build.sh "$@"; } \
  > "$LOGS/90-wp5-build.log" 2>&1 || { echo "   BUILD FAILED"; tail -5 "$LOGS/90-wp5-build.log"; exit 1; }
tail -1 "$LOGS/90-wp5-build.log"

for PY in "$@"; do
  T=$(tagof "$PY")
  echo "===== $T ====="
  { hdr "python slice: conformance on _akffi, $T"; "$PY" conformance.py; } > "$LOGS/91-wp5-conformance-$T.log" 2>&1 || rc=1
  echo "   91 conformance _akffi:     $(tail -1 "$LOGS/91-wp5-conformance-$T.log")"
  { hdr "python slice: conformance on _akffi_rpc, $T"; AK_FFI_MODULE=_akffi_rpc "$PY" conformance.py; } \
    > "$LOGS/92-wp5-conformance-rpc-shim-$T.log" 2>&1 || rc=1
  echo "   92 conformance _akffi_rpc: $(tail -1 "$LOGS/92-wp5-conformance-rpc-shim-$T.log")"
  CMP=()
  [ "$PY" != "$TARGET" ] && CMP=(--compare "$HERE/build/corpus-$(tagof "$TARGET").json")
  { hdr "python slice: the whole corpus, five arms, controls, $T"
    "$PY" corpus.py --timeout 20 --controls --dump "$HERE/build/corpus-$T.json" "${CMP[@]}"; } \
    > "$LOGS/93-wp5-corpus-$T.log" 2>&1 || rc=1
  echo "   93 corpus:                 $(grep -E '^CORPUS' "$LOGS/93-wp5-corpus-$T.log") / $(grep -E '^CONTROLS' "$LOGS/93-wp5-corpus-$T.log")"
  { hdr "python slice: the RPC arm under injected failure, $T"; "$PY" rpc_gate.py; } > "$LOGS/94-wp5-rpc-gate-$T.log" 2>&1 || rc=1
  echo "   94 rpc gate:               $(grep -c 'ABORTED, no figure' "$LOGS/94-wp5-rpc-gate-$T.log") aborted, $(grep -c 'FIGURE PRODUCED' "$LOGS/94-wp5-rpc-gate-$T.log") timed under injection"
  { hdr "python slice: R-D1 wrapped lengths through the shim, $T"; "$PY" rd1_lenwrap.py; } > "$LOGS/95-wp5-rd1-lenwrap-$T.log" 2>&1 || rc=1
  echo "   95 rd1:                    $(tail -1 "$LOGS/95-wp5-rd1-lenwrap-$T.log")"
  { hdr "python slice: U1, a map entry with an unknown field, $T"; "$PY" u1_map_unknown.py; } > "$LOGS/96-wp5-u1-$T.log" 2>&1 || rc=1
  echo "   96 u1:                     $(grep -c 'options' "$LOGS/96-wp5-u1-$T.log") readings"
  { hdr "python slice: conformance on the no-unknown variant _akffi_nounk (WP5 step 10), $T"
    AK_FFI_MODULE=_akffi_nounk AK_COUNT_MODULE=_akffi_count_nounk "$PY" conformance.py; } \
    > "$LOGS/100-wp5s10-conformance-nounk-$T.log" 2>&1 || rc=1
  echo "  100 conformance nounk:      $(tail -1 "$LOGS/100-wp5s10-conformance-nounk-$T.log")"
  { hdr "python slice: the whole corpus through the no-unknown variant (WP5 step 10), $T"
    AK_NOUNK=1 "$PY" corpus.py --timeout 20 --controls --dump "$HERE/build/corpus-nounk-$T.json" \
      --compare "$HERE/build/corpus-$T.json"; } > "$LOGS/101-wp5s10-corpus-nounk-$T.log" 2>&1 || rc=1
  echo "  101 corpus nounk:           $(grep -E '^CORPUS' "$LOGS/101-wp5s10-corpus-nounk-$T.log") / $(grep -E '^CONTROLS' "$LOGS/101-wp5s10-corpus-nounk-$T.log") / $(grep -E '^NOUNK CONTROLS' "$LOGS/101-wp5s10-corpus-nounk-$T.log")"
  { hdr "python slice: conformance on _akffi_rpc_nounk, $T"; AK_FFI_MODULE=_akffi_rpc_nounk AK_COUNT_MODULE=_akffi_count_nounk "$PY" conformance.py; } \
    > "$LOGS/102-wp5s10-conformance-rpc-nounk-$T.log" 2>&1 || rc=1
  echo "  102 conformance rpc nounk:  $(tail -1 "$LOGS/102-wp5s10-conformance-rpc-nounk-$T.log")"
done

echo "===== 103. crossing counts, whole numbers per call: full (drop) and no-unknown builds ====="
{
  hdr "python slice: crossing counts per call, full build (drop) and no-unknown build (WP5 step 10, requirement 19)"
  for PY in "$@"; do
    T=$(tagof "$PY")
    grep '^   abs' "$LOGS/91-wp5-conformance-$T.log" > "$HERE/build/abs-drop-$T.txt"
    grep '^   abs' "$LOGS/100-wp5s10-conformance-nounk-$T.log" > "$HERE/build/abs-nounk-$T.txt"
    grep '^   abi' "$LOGS/91-wp5-conformance-$T.log" > "$HERE/build/abi-full-$T.txt"
    grep '^   abi' "$LOGS/100-wp5s10-conformance-nounk-$T.log" > "$HERE/build/abi-nounk-$T.txt"
    for v in full nounk; do   # req 19 as amended: every ABI call per call, resets and retain included
      if diff "$HERE/counts/abi-$v.txt" "$HERE/build/abi-$v-$T.txt"; then
        echo "   $T abi $v: $(wc -l < "$HERE/build/abi-$v-$T.txt") rows, IDENTICAL to counts/abi-$v.txt"
      else
        echo "   $T abi $v: DIFFERS from counts/abi-$v.txt (above)"
      fi
    done
    for v in drop nounk; do
      if diff "$HERE/counts/crossings-$v.txt" "$HERE/build/abs-$v-$T.txt"; then
        echo "   $T $v: $(wc -l < "$HERE/build/abs-$v-$T.txt") rows, IDENTICAL to counts/crossings-$v.txt"
      else
        echo "   $T $v: DIFFERS from counts/crossings-$v.txt (above)"
      fi
    done
  done
  echo
  echo "## the committed files against each other: what compiling unknown fields out changes"
  diff "$HERE/counts/crossings-drop.txt" "$HERE/counts/crossings-nounk.txt" | sed 's/^/   /'
  echo "   $(diff "$HERE/counts/crossings-drop.txt" "$HERE/counts/crossings-nounk.txt" | grep -c '^>' || true) of $(wc -l < "$HERE/counts/crossings-drop.txt") rows differ"
} > "$LOGS/103-wp5s10-counts-drop-vs-nounk.log" 2>&1
grep -E 'IDENTICAL|DIFFERS|rows differ' "$LOGS/103-wp5s10-counts-drop-vs-nounk.log"
grep -q "DIFFERS from counts/" "$LOGS/103-wp5s10-counts-drop-vs-nounk.log" && rc=1   # abi and abs rows alike

echo "===== 105. the RPC cells B, C, D, E per call, both builds (req 19 as amended) ====="
{
  hdr "python slice: RPC crossing counts per call, the rpc counting builds (req 19, R-H31)"
  for v in full nounk; do
    A=""; [ "$v" = nounk ] && A="--variant nounk"
    python3.12 rpc_counts.py $A > "$HERE/build/rpc-$v.out" 2>&1 || echo "   rpc_counts $v FAILED: $(tail -2 "$HERE/build/rpc-$v.out")"
    cat "$HERE/build/rpc-$v.out"
    grep '^   rpc ' "$HERE/build/rpc-$v.out" > "$HERE/build/rpc-$v.txt"
    if diff "$HERE/counts/rpc-$v.txt" "$HERE/build/rpc-$v.txt"; then
      echo "   rpc $v: $(wc -l < "$HERE/build/rpc-$v.txt") rows, IDENTICAL to counts/rpc-$v.txt"
    else
      echo "   rpc $v: DIFFERS from counts/rpc-$v.txt (above)"
    fi
  done
} > "$LOGS/105-wp7-rpc-counts.log" 2>&1
grep -E 'IDENTICAL|DIFFERS|FAILED' "$LOGS/105-wp7-rpc-counts.log"
grep -qE "DIFFERS|FAILED" "$LOGS/105-wp7-rpc-counts.log" && rc=1

echo "===== 104. camp_summary keyed by build, ratios from per-launch medians (R-H1) ====="
{ hdr "python slice: camp_summary's test, two builds whose incumbents differ (R-H1, R-H24)"; python3.12 test_camp_summary.py; } \
  > "$LOGS/104-wp6-camp-summary-test.log" 2>&1 || rc=1
tail -1 "$LOGS/104-wp6-camp-summary-test.log"

echo "===== 97. the 3.7 source check ====="
{ hdr "python slice: the 3.7 source check, and every other CPython header set here"; ./floor_check.sh; } > "$LOGS/97-wp5-floor-source.log" 2>&1 || rc=1
tail -2 "$LOGS/97-wp5-floor-source.log"

echo "===== 98. crossing counts against the pre-port shim (logs/python/85) ====="
{
  hdr "python slice: crossing counts, plan-rendered shim against the pre-port shim"
  # The `default (_akffi)` block of log 85 (3.12, core 6ede244, the pre-port generator).
  sed -n '/^########## default (_akffi)/,$p' "$(cd "$HERE/../.." && pwd)/logs/python/85-conformance-rpc-shim.log" | sed -n '/crossing counts/,$p' | grep -E '^\s+P[0-9]' > "$HERE/build/counts-85.txt"
  for PY in "$@"; do
    T=$(tagof "$PY")
    sed -n '/crossing counts/,$p' "$LOGS/91-wp5-conformance-$T.log" | grep -E '^\s+P[0-9]' > "$HERE/build/counts-$T.txt"
    if diff "$HERE/build/counts-85.txt" "$HERE/build/counts-$T.txt"; then
      echo "   $T: $(wc -l < "$HERE/build/counts-$T.txt") rows, IDENTICAL to logs/python/85 (3.12, pre-port shim)"
    else
      echo "   $T: DIFFERS from logs/python/85 (above)"
    fi
  done
  echo
  echo "## the C header: one renderer (c_abi) for both C-consuming slices"
  for pair in "gen/out/ak_abi.h ../cpp/include/ak_abi.h" "gen/out/corpus/ak_abi.h ../cpp/corpus/include/ak_abi.h"; do
    set -- $pair
    if cmp -s "$1" "$2"; then echo "   $1 is byte-identical to poc/${2#../}"; else echo "   $1 DIFFERS from poc/${2#../} (the cpp slice's may be at another commit)"; fi
  done
} > "$LOGS/98-wp5-counts-vs-85.log" 2>&1
grep -E 'IDENTICAL|DIFFERS' "$LOGS/98-wp5-counts-vs-85.log"
grep -q "DIFFERS from logs/python/85" "$LOGS/98-wp5-counts-vs-85.log" && rc=1   # the header comparison is information, not a gate
echo "gate exit $rc"
exit $rc
