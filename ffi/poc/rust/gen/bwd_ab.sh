#!/usr/bin/env bash
# EXPERIMENT backward-encode: a NARROWED alternation of codec-suite builds, for one question
# (one payload family). CONTAINER INSTRUMENTATION.
#
#   gen/bwd_ab.sh OUT_DIR ONLY NAME=EXE[:LIBDIR] [NAME=EXE[:LIBDIR] ...]
#
# Each NAME is a codec_suite bench executable; LIBDIR (optional) is put first in
# LD_LIBRARY_PATH so that executable loads the core kept there (checked with ldd, recorded).
# Launch n of every variant uses seed n; order V1 V2 ... per launch. Settings as
# gen/bwd_bench.sh (encode, core-ffi + core-native, reused-buffer + transport-ready-tonic, hot).
set -Eeuo pipefail
OUT=${1:?usage}; ONLY=${2:?only}; shift 2
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
. "$HERE/machine_header.sh"
CPU=${AK_BWD_CPU:-1}; LAUNCHES=${AK_BWD_LAUNCHES:-3}
H="$OUT/header.txt"
{ echo "# backward-encode narrowed alternation, $(date -u +%FT%TZ). CONTAINER INSTRUMENTATION."
  machine_header
  echo "# AK_ONLY=$ONLY, encode, core-ffi + core-native, reused-buffer + transport-ready-tonic, hot; AK_WARMUP_MS=${AK_BWD_WARM:-100} AK_MEASURE_MS=${AK_BWD_MEAS:-250}; $LAUNCHES launches per variant; CPU $CPU"
  for a in "$@"; do n=${a%%=*}; r=${a#*=}; e=${r%%:*}; d=""; [ "$r" != "$e" ] && d=${r#*:}
    echo "# $n: $e (sha256 $(sha256sum "$e" | cut -c1-16)) loads $(LD_LIBRARY_PATH="$d" ldd "$e" | grep -o '/[^ ]*libak_core.so') (sha256 $(sha256sum "$(LD_LIBRARY_PATH="$d" ldd "$e" | grep -o '/[^ ]*libak_core.so')" | cut -c1-16))"; done
} > "$H"
T0=$(date +%s)
for l in $(seq 1 "$LAUNCHES"); do
  for a in "$@"; do n=${a%%=*}; r=${a#*=}; e=${r%%:*}; d=""; [ "$r" != "$e" ] && d=${r#*:}
    CH=$(mktemp -d)
    LD_LIBRARY_PATH="$d" CRITERION_HOME="$CH" AK_LAUNCH=$l AK_OUT="$OUT/codec-$n-$l.jsonl" AK_ONLY="$ONLY" \
      AK_CASE_ARMS=core-ffi,core-native AK_CASE_DIRS=encode AK_CASE_END=reused-buffer,transport-ready-tonic AK_CASE_INPUT=hot \
      AK_SAMPLES=10 AK_WARMUP_MS=${AK_BWD_WARM:-100} AK_MEASURE_MS=${AK_BWD_MEAS:-250} AK_NRESAMPLES=1000 \
      taskset -c "$CPU" "$e" --bench > "$OUT/codec-$n-$l.out" 2> "$OUT/codec-$n-$l.err"
    rm -rf "$CH"
    echo "$n launch $l: $(grep -m1 '^# precheck' "$OUT/codec-$n-$l.err")" >> "$H"
  done
done
echo "# benchmark wall: $(( $(date +%s) - T0 )) s" >> "$H"
cat "$H"
