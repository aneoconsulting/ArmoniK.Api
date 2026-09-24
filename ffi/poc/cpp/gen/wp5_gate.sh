#!/usr/bin/env bash
# FIX-PLAN WP5 step 2's correctness gate for the cpp slice. NOTHING IS TIMED: the bench and
# content-set binaries run with their gate-only switches, the concurrency suite without T7.
#
# Logs, into ffi/logs/cpp/:
#   wp5-generator.log    generate.py --check (drift + the one-generator guard over the shared
#                        C++ backend and this slice's glue), the shared core's --check,
#                        refusal_test.py, rd2_guard.sh (the RPC layout plants), audit, R0
#   wp5-conformance.log  byte identity on the payload set at C++17 target, C++17 floor,
#                        C++14, C++11 and static, against the core built WITH init-guard;
#                        and the planted build whose binding skips ak_init, which must FAIL
#   wp5-corpus.log       the FULL corpus, four arms (ffi-drop, ffi-retain, native-drop,
#                        native-retain), each row in its own process under a timeout, at
#                        C++17, C++14, C++11 and static; the four builds' outcomes compared
#                        byte for byte; controls proj/reenc/accept/noinit, each must FAIL
#   wp5-bytes.log        the C++ arms before the port against after it, row by row
#   wp5-boundary.log     boundary.sh (R5 from the artifact) and the corpus core's layout
#                        facts against the corpus header (ABI v1 section 10)
#   wp5-gates.log        groupskip, concurrency (planted cores must fail), ODR, bench gates
#                        (every arm, and the planted gate that must refuse), content sets,
#                        crossing counts (the counting core), RPC crossing counts
#
#   gen/wp5_gate.sh [BUILD_DIR]      default ./build (configured with -DAK_RPC=ON)
#   OLD_REV=<commit>                 the "before" tree for wp5-bytes.log (default aba944a)
set -u
cd "$(dirname "$0")/.." || exit 2
B=${1:-build}
L=../../logs/cpp
S=$(mktemp -d)
trap 'rm -rf "$S"' EXIT
OLD_REV=${OLD_REV:-aba944a}
PAY=../../schema/generated
FAILS=0

hdr() {
  echo "# $1"
  echo "#   date       $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "#   machine    $(uname -srm), $(nproc) vCPU, $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //')"
  echo "#   compiler   $(g++ --version | head -1); $(rustc --version)"
  echo "#   commit     $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- . ../codec || echo ' + uncommitted changes in poc/cpp or poc/codec')"
  echo "#   core       ffi/poc/codec at that commit, built --features init-guard (and corpus,init-guard"
  echo "#              for the corpus ABI); no timing is taken by anything in this log"
  echo
}
step() { echo "===== $1 ====="; }
must() {  # must <label> <expected-exit> <cmd...>
  local label=$1 want=$2; shift 2
  "$@"; local rc=$?
  if [ "$want" = 0 ] && [ $rc -ne 0 ]; then echo ">>> FAIL: $label exited $rc"; FAILS=$((FAILS+1));
  elif [ "$want" = nz ] && [ $rc -eq 0 ]; then echo ">>> FAIL: $label exited 0 (a control that cannot fail)"; FAILS=$((FAILS+1));
  else echo ">>> ok: $label (exit $rc)"; fi
  echo
}
py() { python3 "$@" 2> >(grep -v -i 'distutils\|traceback (most recent call last):$\|frozen site\|<string>\|remainder of file\|^ *$' >&2); }

{
  hdr "cpp slice, WP5 step 2: the generator"
  step "gen/generate.py --check (drift, and the one-generator guard)"
  must "generate.py --check" 0 py gen/generate.py --check
  step "poc/codec/gen/generate.py --check (the shared core this slice gates)"
  must "codec generate.py --check" 0 py ../codec/gen/generate.py --check
  step "refusal_test.py"
  must "refusal_test.py" 0 py gen/refusal_test.py
  step "rd2_guard.sh (ak_client_opts rendered from plan.rpc; two plants refused)"
  must "rd2_guard.sh" 0 bash gen/rd2_guard.sh
  step "audit_tracked.sh"
  must "audit_tracked.sh" 0 bash gen/audit_tracked.sh
  step "one_core.sh (R0)"
  must "one_core.sh" 0 bash ../codec/gen/one_core.sh
  step "one_core.sh --selftest: KNOWN DEFECT in the shared script, reported, not this slice's"
  # Its scratch copy holds `git ls-files ffi/poc ffi/schema` only; since WP5 step 1 the
  # shared generate.py --check also loads ffi/corpus (plan.load_corpus), so the clean
  # scratch copy already fails and the planted controls never run. Shown by re-running the
  # clean check on a scratch copy WITH ffi/corpus, which passes.
  bash ../codec/gen/one_core.sh --selftest > "$S/oc.log" 2>&1; rc=$?
  tail -4 "$S/oc.log"
  echo "  (one_core.sh --selftest exit $rc: not counted; reported to the aggregating session)"
  SC=$(mktemp -d)
  (cd ../../.. && git ls-files -z ffi/poc ffi/schema ffi/corpus | tar --null -T - -cf -) | (cd "$SC" && tar xf -)
  must "one_core.sh --root <scratch copy of ffi/poc + ffi/schema + ffi/corpus>" 0 \
       bash ../codec/gen/one_core.sh --root "$SC/ffi/poc"
  rm -rf "$SC"
} > "$L/wp5-generator.log" 2>&1

{
  hdr "cpp slice, WP5 step 2: conformance on the payload set (byte identity against manifest.json)"
  for b in conformance_a17_shared conformance_b17_shared conformance_c14_shared \
           conformance_c11_shared conformance_a17_static; do
    step "$b"
    (cd "$PAY" && timeout 300 "$OLDPWD/$B/$b" payloads > "$S/c.log" 2>&1; echo $? > "$S/rc")
    grep -v 'libprotobuf ERROR' "$S/c.log"
    rc=$(cat "$S/rc"); [ "$rc" = 0 ] && echo ">>> ok: $b" || { echo ">>> FAIL: $b exit $rc"; FAILS=$((FAILS+1)); }
    echo
  done
  step "conformance_a17_noinit: the binding skips ak_init (PLANTED) -- must FAIL"
  (cd "$PAY" && timeout 300 "$OLDPWD/$B/conformance_a17_noinit" payloads > "$S/c.log" 2>&1; echo $? > "$S/rc")
  grep -m5 'FAIL' "$S/c.log"; tail -1 "$S/c.log"
  rc=$(cat "$S/rc"); [ "$rc" != 0 ] && echo ">>> ok: the planted build failed (exit $rc): init-guard is in the core" \
                                     || { echo ">>> FAIL: the planted build passed"; FAILS=$((FAILS+1)); }
} > "$L/wp5-conformance.log" 2>&1

{
  hdr "cpp slice, WP5 step 2: the full conformance corpus, four arms"
  for b in corpus_all_a17 corpus_all_c14 corpus_all_c11 corpus_all_a17_static; do
    step "$b"
    must "corpus $b" 0 py gen/corpus_all.py "$B/$b" --record "$S/$b.json"
  done
  step "the four builds' outcomes, (row, arm) by (row, arm): identical across levels and linkages"
  must "compare" 0 py gen/corpus_all.py --compare "$S/corpus_all_a17.json" "$S/corpus_all_c14.json" \
       "$S/corpus_all_c11.json" "$S/corpus_all_a17_static.json"
  SUB="S-Probe,U-root,X-lenwrap-lrr,E-map,T-dec-root,U-wire-MetricsBatch"
  for p in proj reenc accept; do
    step "control: AK_CORPUS_PLANT=$p on $SUB -- must FAIL"
    py gen/corpus_all.py "$B/corpus_all_a17" --only "$SUB" --plant "$p" --record "$S/plant-$p.json" \
      > "$S/p.log" 2>&1; rc=$?
    grep -E '^## |^   pass|^CORPUS' "$S/p.log"
    [ $rc -ne 0 ] && echo ">>> ok: control $p failed as required" || { echo ">>> FAIL: control $p passed"; FAILS=$((FAILS+1)); }
    echo
  done
  step "control: --compare sees a planted difference (reenc vs clean, same rows) -- must FAIL"
  py gen/corpus_all.py "$B/corpus_all_a17" --only "$SUB" --record "$S/clean-sub.json" > /dev/null 2>&1
  must "compare control" nz py gen/corpus_all.py --compare "$S/clean-sub.json" "$S/plant-reenc.json"
  step "control: corpus_all_noinit, the binding skips ak_init, core built with init-guard -- must FAIL"
  py gen/corpus_all.py "$B/corpus_all_noinit" --only "$SUB" > "$S/p.log" 2>&1; rc=$?
  grep -E '^## |^   pass|^CORPUS|build ' "$S/p.log"; grep -m3 'FAIL .*\[ffi-' "$S/p.log"
  echo "  ffi arm-row failures: $(grep -c 'FAIL .*\[ffi-' "$S/p.log"), native arm-row failures: $(grep -c 'FAIL .*\[native-' "$S/p.log") (native has no ak_init)"
  [ $rc -ne 0 ] && echo ">>> ok: control noinit failed as required" || { echo ">>> FAIL: control noinit passed"; FAILS=$((FAILS+1)); }
} > "$L/wp5-corpus.log" 2>&1

{
  hdr "cpp slice, WP5 step 2: byte audit, the C++ arms before the port against after it"
  OLD=${OLD_CORPUS_BIN:-}
  if [ -z "$OLD" ]; then
    step "build the BEFORE harness (src/corpus.cpp) at $OLD_REV, out of tree"
    git worktree add -f "$S/old" "$OLD_REV" > /dev/null 2>&1
    (cmake -S "$S/old/ffi/poc/cpp" -B "$S/oldb" -DAK_CORE_TGT="$S/oldcore" > "$S/oldcfg.log" 2>&1 \
      && cmake --build "$S/oldb" -j"$(nproc)" --target corpus_a17_shared > "$S/oldbuild.log" 2>&1) \
      && echo "  built $S/oldb/corpus_a17_shared from $(git -C "$S/old" rev-parse --short HEAD)" \
      || { echo ">>> FAIL: the before tree does not build"; tail -5 "$S/oldbuild.log"; FAILS=$((FAILS+1)); }
    OLD="$S/oldb/corpus_a17_shared"
  fi
  step "wp5_bytes.py"
  must "wp5_bytes.py" 0 py gen/wp5_bytes.py "$OLD" "$B/corpus_all_a17"
  git worktree remove --force "$S/old" > /dev/null 2>&1 || true
} > "$L/wp5-bytes.log" 2>&1

{
  hdr "cpp slice, WP5 step 2: boundary"
  step "boundary.sh"
  must "boundary.sh" 0 ./gen/boundary.sh
  step "the corpus core's layout facts against the corpus header"
  must "corpus layout" 0 "$B/corpus_all_a17" --layout
  must "corpus layout (static)" 0 "$B/corpus_all_a17_static" --layout
} > "$L/wp5-boundary.log" 2>&1

{
  hdr "cpp slice, WP5 step 2: the other gates (nothing timed)"
  step "groupskip.sh"
  must "groupskip.sh" 0 bash gen/groupskip.sh
  step "concurrency.sh (AK_CONC_NO_T7=1; planted cores must fail)"
  must "concurrency.sh" 0 env AK_CONC_NO_T7=1 bash gen/concurrency.sh
  step "odr_check.sh"
  must "odr_check.sh" 0 bash gen/odr_check.sh
  for b in bench_a17_shared bench_a17_static bench_b17_shared bench_c14_shared bench_c11_shared; do
    step "$b, AK_BENCH_GATE_ONLY=1"
    (cd "$PAY" && AK_BENCH_GATE_ONLY=1 timeout 300 "$OLDPWD/$B/$b" 1 > "$S/g.log" 2>&1; echo $? > "$S/rc")
    grep -E 'gate|AK_BENCH' "$S/g.log" | tail -4
    rc=$(cat "$S/rc"); [ "$rc" = 0 ] && echo ">>> ok: $b" || { echo ">>> FAIL: $b exit $rc"; FAILS=$((FAILS+1)); }
    echo
  done
  step "bench_a17_gateplant (a refusing ffi-valtc): must refuse to time it and exit 1"
  (cd "$PAY" && AK_BENCH_GATE_ONLY=1 timeout 300 "$OLDPWD/$B/bench_a17_gateplant" 1 > "$S/g.log" 2>&1; echo $? > "$S/rc")
  tail -1 "$S/g.log"
  rc=$(cat "$S/rc"); [ "$rc" = 1 ] && echo ">>> ok: planted gate refused (exit 1)" || { echo ">>> FAIL: planted gate exit $rc"; FAILS=$((FAILS+1)); }
  echo
  step "contentsets_a17, AK_CS_GATE_ONLY=1"
  must "contentsets gate" 0 env AK_CS_GATE_ONLY=1 "$B/contentsets_a17" 0
  step "crossing counts (the COUNTING core, R5)"
  (cd "$PAY" && must "counts shared" 0 "$OLDPWD/$B/counts_a17_shared")
  (cd "$PAY" && must "counts static" 0 "$OLDPWD/$B/counts_a17_static")
  if [ -x "$B/rpccounts" ]; then
    step "RPC crossing counts (the binding's ak_init_once before the first RPC)"
    must "rpccounts" 0 timeout 120 "$B/rpccounts" 5
  fi
} > "$L/wp5-gates.log" 2>&1

TOTAL=0
for f in generator conformance corpus bytes boundary gates; do
  n=$(grep -c '>>> FAIL' "$L/wp5-$f.log")
  TOTAL=$((TOTAL + n))
  printf '%-14s %s failure(s)\n' "$f" "$n"
done
echo "wp5_gate: $TOTAL step(s) failed"
exit $((TOTAL ? 1 : 0))
