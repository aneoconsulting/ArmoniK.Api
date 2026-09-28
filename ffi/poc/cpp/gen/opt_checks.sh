#!/usr/bin/env bash
# The per-step correctness checks of the optimisation unit (not the gate): generator current and
# one core, conformance at C++17 and C++11 on both builds, the corpus on both builds, crossing
# counts against the committed files, and each codec binary's own pre-check. Log:
# ffi/logs/cpp/opt/checks/NAME.log. Exit 1 on any failure.
#   gen/opt_checks.sh NAME
set -u
cd "$(dirname "$0")/.." || exit 2
NAME=${1:?name}; B=${BUILD:-$PWD/build-campaign}; L=../../logs/cpp/opt/checks; mkdir -p "$L"
LOG=$L/$NAME.log; S=$(mktemp -d); F=0
bad() { echo ">>> FAIL: $*"; F=$((F+1)); }
{
  echo "# opt_checks $NAME at $(git rev-parse --short HEAD)$(git status --porcelain -- . ../codec | grep -q . && echo ' + uncommitted')  $(date -u +%FT%TZ)"
  python3 gen/generate.py --check > "$S/g" 2>&1 || { grep -v '^ok' "$S/g" | head; bad "generate.py --check"; }
  echo "generate.py --check: $(grep -c '^ok' "$S/g") ok lines"
  bash ../codec/gen/one_core.sh > "$S/o" 2>&1 && echo "one_core.sh: ok" || { tail -3 "$S/o"; bad one_core.sh; }
  cmake --build "$B" -j"$(nproc)" --target conformance_a17_shared conformance_c11_shared conformance_nounk_a17 \
    conformance_nounk_c11 corpus_all_a17 corpus_nounk_a17 counts_a17_shared counts_nounk campaign_codec \
    campaign_codec_nounk campaign_rpc campaign_rpc_nounk campaign_rpc_count campaign_rpc_count_nounk > "$S/b" 2>&1 \
    || { tail -20 "$S/b"; bad build; }
  for c in conformance_a17_shared conformance_c11_shared conformance_nounk_a17 conformance_nounk_c11; do
    (cd ../../schema/generated && "$B/$c" payloads > "$S/c" 2>&1); rc=$?
    echo "$c: exit $rc: $(grep -E 'checks,' "$S/c" | tail -1)"; [ $rc = 0 ] || { grep -m5 FAIL "$S/c"; bad "$c"; }
  done
  python3 gen/corpus_all.py "$B/corpus_all_a17" --timeout 60 --max-retain-gap U-map-entry > "$S/k" 2>&1; rc=$?
  grep -E '^   pass|^CORPUS' "$S/k" | sed 's/^/  full corpus: /'; [ $rc = 0 ] || bad "corpus full"
  python3 gen/corpus_all.py "$B/corpus_nounk_a17" --timeout 60 --expect-dropped ffi-drop,ffi-pull-drop > "$S/k" 2>&1; rc=$?
  grep -E '^   pass|^CORPUS' "$S/k" | sed 's/^/  nounk corpus: /'; [ $rc = 0 ] || bad "corpus nounk"
  python3 gen/u_rows.py ../../corpus/generated "$S/rows.tsv" 2>/dev/null
  for v in full nounk; do
    exe=counts_a17_shared; base=counts-baseline.log; [ $v = nounk ] && { exe=counts_nounk; base=counts-nounk-baseline.log; }
    (cd ../../schema/generated && "$B/$exe" --corpus "$PWD/../../corpus/generated" --rows "$S/rows.tsv" > "$S/n.$v" 2>&1)
    if diff <(grep -E '^  [PU]' "../../logs/cpp/$base") <(grep -E '^  [PU]' "$S/n.$v") > "$S/d.$v"; then
      echo "counts $v: identical to $base ($(grep -cE '^  [PU]' "$S/n.$v") rows)"
    else echo "counts $v: DIFFER from $base:"; head -20 "$S/d.$v"; cp "$S/n.$v" "$L/$NAME-counts-$v.log"; bad "counts $v differ (kept $NAME-counts-$v.log)"; fi
  done
  for c in campaign_codec campaign_codec_nounk; do
    (cd ../../schema/generated && taskset -c "${AK_CPU_CLIENT:-1}" "$B/$c" --rounds 0 --pool-bytes 1048576 \
       --corpus "$PWD/../../corpus/generated" --rows "$S/rows.tsv" > "$S/p" 2>&1); rc=$?
    echo "$c pre-check: exit $rc $(grep -o '"campaign_codec_gate": {[^}]*}' "$S/p")"; [ $rc = 0 ] || { grep -m5 'GATE FAIL' "$S/p"; bad "$c pre-check"; }
  done
  echo "opt_checks $NAME: $F failure(s)"
} > "$LOG" 2>&1
rm -rf "$S"
cat "$LOG"
[ $F = 0 ]
