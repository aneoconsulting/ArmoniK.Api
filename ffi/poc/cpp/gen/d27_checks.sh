#!/usr/bin/env bash
# FIX-PLAN D27 (owner, 2026-10-10), reset on entry: the cpp slice's fast checks of the change
# (the owner's "checks, not gates" rule): conformance (full and no-unknown), the corpus (both
# builds, decision 11's controls and their plant), and the crossing counts (codec, core grid,
# RPC per call) against logs/cpp/counts-*.log and rpc-counts*.log. Nothing timed. Writes
# logs/cpp/d27/{conformance,corpus,counts}.log.
#   gen/d27_checks.sh [BUILD_DIR]     (configured with -DAK_RPC=ON; the targets below built)
set -u
cd "$(dirname "$0")/.." || exit 2
B=${1:-build}
. gen/core_swap.sh
L=../../logs/cpp; D=$L/d27; mkdir -p "$D"
PAY=../../schema/generated
S=$(mktemp -d); trap 'rm -rf "$S"' EXIT
fails=0
hdr() { echo "# $1 ($(git rev-parse --short HEAD)$(git diff --quiet HEAD -- . ../codec || echo ' + uncommitted changes in poc/cpp or poc/codec'); $(g++ --version | head -1); $(rustc --version))"; }
{ hdr "cpp D27 checks: conformance"
  for b in conformance_a17_shared conformance_nounk_a17; do
    echo "== $b"; (cd "$PAY" && timeout 300 "$OLDPWD/$B/$b" payloads > "$S/c" 2>&1; echo $? > "$S/rc"); grep -v 'libprotobuf ERROR' "$S/c" | tail -4
    [ "$(cat "$S/rc")" = 0 ] && echo ">>> ok: $b" || { echo ">>> FAIL: $b"; fails=$((fails+1)); }
  done; } > "$D/conformance.log" 2>&1
{ hdr "cpp D27 checks: corpus"
  echo "== corpus_all_a17 (six arms)"; python3 gen/corpus_all.py "$B/corpus_all_a17" --max-retain-gap U-map-entry > "$S/k" 2>&1; rc=$?
  grep -E '^## |^   pass|^CORPUS|FAIL' "$S/k"; [ $rc = 0 ] && echo ">>> ok" || echo ">>> FAIL: corpus"
  echo "== corpus_all_a17 --unk-controls"; python3 gen/corpus_all.py "$B/corpus_all_a17" --unk-controls > "$S/k" 2>&1; rc=$?
  tail -6 "$S/k"; [ $rc = 0 ] && echo ">>> ok" || echo ">>> FAIL: unk controls"
  echo "== control: --unk-controls --plant clear (must FAIL)"; python3 gen/corpus_all.py "$B/corpus_all_a17" --unk-controls --plant clear > "$S/k" 2>&1; rc=$?
  tail -1 "$S/k"; [ $rc != 0 ] && echo ">>> ok: the plant failed (exit $rc)" || echo ">>> FAIL: the plant passed"
  echo "== corpus_nounk_a17"; python3 gen/corpus_all.py "$B/corpus_nounk_a17" --expect-dropped ffi-drop,ffi-pull-drop > "$S/k" 2>&1; rc=$?
  grep -E '^## |^   pass|dropped form|^CORPUS|FAIL' "$S/k"; [ $rc = 0 ] && echo ">>> ok" || echo ">>> FAIL: corpus nounk"
} > "$D/corpus.log" 2>&1
{ hdr "cpp D27 checks: crossing counts"
  python3 gen/u_rows.py ../../corpus/generated "$S/rows.tsv"
  (cd "$PAY" && "$OLDPWD/$B/counts_a17_shared" --corpus "$OLDPWD/../../corpus/generated" --rows "$S/rows.tsv" > "$S/full" 2>&1
               "$OLDPWD/$B/counts_nounk" --corpus "$OLDPWD/../../corpus/generated" --rows "$S/rows.tsv" > "$S/nounk" 2>&1
               for v in "" _nounk; do env $(cs_env "$OLDPWD/$B/counts_grid$v") "$OLDPWD/$B/counts_grid$v" --corpus "$OLDPWD/../../corpus/generated" > "$S/grid$v" 2>&1; done)
  for x in "counts-baseline.log full" "counts-nounk-baseline.log nounk" "counts-grid.log grid" "counts-grid-nounk.log grid_nounk"; do
    set -- $x
    diff <(grep -E '^  [PU]' "$L/$1") <(grep -E '^  [PU]' "$S/$2") > /dev/null && echo ">>> ok: $(grep -cE '^  [PU]' "$S/$2") rows identical to $1" || echo ">>> FAIL: counts differ from $1"
  done
  SRV=$(mktemp -d); export AK_SERVE_STATE=$SRV/serve.state AK_CPU_SERVER=${AK_CPU_SERVER:-2,3}
  bash ../rust/serve.sh start --out "$SRV/srv" > "$SRV/out" 2>&1 || { cat "$SRV/out"; echo ">>> FAIL: server"; }
  SOCK=unix:$(awk '$1=="shipped"{print $2}' "$SRV/out"); EXP=$(sed -n 's/.*P2.2 \([0-9]*\) B.*/\1/p' "$SRV/srv/rpc-server.log" | head -1)
  for v in "" _nounk; do
    want=$L/rpc-counts$( [ -n "$v" ] && echo -nounk ).log
    env $(cs_env "$B/campaign_rpc_count$v") taskset -c "${AK_CPU_CLIENT:-1}" "$B/campaign_rpc_count$v" --target "$SOCK" --expect "$EXP" --transport shipped --count 4 > "$S/rpc$v" 2>&1
    diff <(grep -E '^  [BCDE]' "$want") <(grep -E '^  [BCDE]' "$S/rpc$v") > /dev/null && echo ">>> ok: $(grep -cE '^  [BCDE]' "$S/rpc$v") RPC rows identical to $(basename "$want")" || echo ">>> FAIL: RPC counts differ from $(basename "$want")"
  done
  bash ../rust/serve.sh stop > /dev/null 2>&1; rm -rf "$SRV"
} > "$D/counts.log" 2>&1
n=$(cat "$D"/conformance.log "$D"/corpus.log "$D"/counts.log | grep -c '>>> FAIL')
grep -h '>>> ' "$D"/conformance.log "$D"/corpus.log "$D"/counts.log
echo "cpp D27 checks: $n failure(s)"; exit $((n ? 1 : 0))
