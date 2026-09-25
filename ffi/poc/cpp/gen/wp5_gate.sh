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
#                        byte for byte; controls proj/reenc/accept/noinit, each must FAIL;
#                        retain arms may write the dropped form only on U-map-entry; the
#                        decision 11 controls (--unk-controls) at every level, and their plant
#   wp5-bytes.log        the C++ arms before the port against after it, row by row
#   wp5-boundary.log     boundary.sh (R5 from the artifact) and the corpus core's layout
#                        facts against the corpus header (ABI v1 section 10)
#   wp5-gates.log        groupskip, concurrency (planted cores must fail), ODR, bench gates
#                        (every arm, and the planted gate that must refuse), content sets,
#                        crossing counts (the counting core), RPC crossing counts
#
#   wp5-build.log        D39: the gate BUILDS what it gates (cmake configure + build, every
#                        target, every core via cargo), then REFUSES to run if any gated
#                        binary is older than the newest source it depends on
#   wp5-probe.log        the rust slice's oracle-probe manifest (poc/rust/gen/probe_corpus.py,
#                        rows the aggregating session proposes to the corpus), four arms
#
#   gen/wp5_gate.sh [BUILD_DIR]      default ./build (configured with -DAK_RPC=ON)
#   CLEAN=1                          delete BUILD_DIR first (a clean build)
#   AK_GATE_NO_BUILD=1               control only: skip the build; the freshness check must
#                                    then refuse a tree whose sources are newer
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

# ---- D39: build what is gated, and refuse a binary older than its sources ------------
# The gate once passed on binaries built before the backend commit it claimed to gate: it
# never built. It now configures and builds every target itself (cargo rebuilds the cores
# if the shared crates changed), and then checks mtimes, so a build that silently did not
# happen cannot pass either.
{
  hdr "cpp slice, WP5: build (D39)"
  [ "${CLEAN:-0}" = 1 ] && { echo "CLEAN=1: removing $B"; rm -rf "$B"; }
  step "generate.py (the tree the build compiles is the tree the generator writes)"
  py gen/generate.py | grep -v '^same' ; echo "generate.py exit ${PIPESTATUS[0]}"
  step "cmake configure + build, every target"
  if [ "${AK_GATE_NO_BUILD:-0}" = 1 ]; then
    # The D39 CONTROL only: skip the build, so the freshness check below must refuse a
    # binary older than its sources. Never used for a gate run.
    echo "  AK_GATE_NO_BUILD=1: build SKIPPED (the freshness control)"
  elif cmake -S . -B "$B" -DAK_RPC=ON > "$S/cfg.log" 2>&1 && cmake --build "$B" -j"$(nproc)" > "$S/build.log" 2>&1; then
    echo ">>> ok: build ($(grep -c 'Linking' "$S/build.log") executables relinked)"
  else
    tail -n 30 "$S/cfg.log" "$S/build.log"; echo ">>> FAIL: build"
  fi
  step "freshness: every gated binary newer than the newest input"
  # What the binaries are compiled from: the C++ sources and headers (generated ones
  # included, which generate.py above just brought current), the build file, and the core's
  # Rust sources. Harness scripts (gen/*.py) are not compiled into anything.
  NEWEST=$(find CMakeLists.txt src include corpus ../codec/crates \
             -path '*/target*' -prune -o -type f \( -name '*.cpp' -o -name '*.h' -o -name '*.inc' \
             -o -name '*.rs' -o -name '*.toml' -o -name CMakeLists.txt \) -printf '%T@ %p\n' \
           | sort -n | tail -1)
  echo "  newest input: ${NEWEST#* } ($(date -u -d @"${NEWEST%% *}" +%FT%TZ))"
  stale=0
  for b in conformance_a17_shared conformance_b17_shared conformance_c14_shared conformance_c11_shared \
           conformance_a17_static conformance_a17_noinit corpus_all_a17 corpus_all_c14 corpus_all_c11 \
           corpus_all_a17_static corpus_all_noinit bench_a17_shared bench_a17_static bench_b17_shared \
           bench_c14_shared bench_c11_shared bench_a17_gateplant contentsets_a17 counts_a17_shared \
           counts_a17_static groupskip_a17 conc_a17_shared odrcheck rpccounts \
           conformance_nounk_a17 conformance_nounk_c11 conformance_nounk_static corpus_nounk_a17 \
           corpus_nounk_c11 corpus_nounk_noinit counts_nounk; do
    t=$(stat -c %Y "$B/$b" 2>/dev/null || echo 0)
    if [ "$t" -lt "${NEWEST%%.*}" ]; then echo "  STALE $b"; stale=$((stale+1)); fi
  done
  [ $stale -eq 0 ] && echo ">>> ok: every gated binary is newer than every input" \
                   || echo ">>> FAIL: $stale gated binary(ies) older than their sources: refusing to gate them"
} > "$L/wp5-build.log" 2>&1
if grep -q '>>> FAIL' "$L/wp5-build.log"; then
  echo "wp5_gate: the build failed or is stale (see $L/wp5-build.log); nothing gated"; exit 1
fi

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
    must "corpus $b" 0 py gen/corpus_all.py "$B/$b" --record "$S/$b.json" --max-retain-gap U-map-entry
  done
  step "the four builds' outcomes, (row, arm) by (row, arm): identical across levels and linkages"
  must "compare" 0 py gen/corpus_all.py --compare "$S/corpus_all_a17.json" "$S/corpus_all_c14.json" \
       "$S/corpus_all_c11.json" "$S/corpus_all_a17_static.json"
  # WP5 step 9 (decision 11): the armed decodes of the binding on every row the C ABI
  # carries -- pool = retain, drop = retain with every bag cleared, each position zeroed
  # drops exactly that position, map-entry bytes delivered unless that position is zeroed.
  for b in corpus_all_a17 corpus_all_c14 corpus_all_c11 corpus_all_a17_static; do
    step "decision 11 controls: $b --unk-controls"
    must "unk controls $b" 0 py gen/corpus_all.py "$B/$b" --unk-controls
  done
  step "control: --unk-controls --plant clear (the expected clear skipped) -- must FAIL"
  py gen/corpus_all.py "$B/corpus_all_a17" --unk-controls --plant clear > "$S/p.log" 2>&1; rc=$?
  grep -E '^   (rows|positions)|^UNK' "$S/p.log"; grep -m3 'FAIL' "$S/p.log"
  [ $rc -ne 0 ] && echo ">>> ok: control clear failed as required" || { echo ">>> FAIL: control clear passed"; FAILS=$((FAILS+1)); }
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
  hdr "cpp slice, WP5: the oracle-probe rows (poc/rust/gen/probe_corpus.py), four arms"
  step "probe_corpus.py: the scratch manifest"
  must "probe_corpus.py" 0 py ../rust/gen/probe_corpus.py "$S/probe"
  for b in corpus_all_a17 corpus_all_c11; do
    step "$b on the probe manifest (D38: P-field-maxplus1-in-group must be REFUSED on every arm)"
    must "probe $b" 0 py gen/corpus_all.py "$B/$b" --manifest "$S/probe/manifest.json"
  done
} > "$L/wp5-probe.log" 2>&1

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
  (cd "$PAY" && must "counts shared" 0 "$OLDPWD/$B/counts_a17_shared") | tee "$S/counts.log"
  (cd "$PAY" && must "counts static" 0 "$OLDPWD/$B/counts_a17_static")
  step "crossing counts against the committed baseline (logs/cpp/counts-baseline.log)"
  grep -E '^  P' "$L/counts-baseline.log" > "$S/cwant"; grep -E '^  P' "$S/counts.log" > "$S/cgot"
  if diff "$S/cwant" "$S/cgot" > "$S/cdiff"; then echo "  $(wc -l < "$S/cgot") count rows identical"; echo ">>> ok: counts unchanged"
  else head -10 "$S/cdiff"; echo ">>> FAIL: crossing counts differ from the baseline"; FAILS=$((FAILS+1)); fi
  step "crossing counts of the retain arms (AK_COUNTS_RETAIN=1: decode_with_*_unk and *_pool)"
  (cd "$PAY" && must "counts retain" 0 env AK_COUNTS_RETAIN=1 "$OLDPWD/$B/counts_a17_shared") | grep -E 'decode|>>>' 
  if [ -x "$B/rpccounts" ]; then
    step "RPC crossing counts (the binding's ak_init_once before the first RPC)"
    must "rpccounts" 0 timeout 120 "$B/rpccounts" 5
  fi
} > "$L/wp5-gates.log" 2>&1

# WP5 step 10: the no-unknown build, gated on its own (gen/nounk_gate.sh -> wp5s10-nounk.log).
bash gen/nounk_gate.sh "$B"

TOTAL=0
for f in build generator conformance corpus probe bytes boundary gates; do
  n=$(grep -c '>>> FAIL' "$L/wp5-$f.log")
  TOTAL=$((TOTAL + n))
  printf '%-14s %s failure(s)\n' "$f" "$n"
done
n=$(grep -c '>>> FAIL' "$L/wp5s10-nounk.log")
TOTAL=$((TOTAL + n))
printf '%-14s %s failure(s)\n' "s10-nounk" "$n"
echo "wp5_gate: $TOTAL step(s) failed"
exit $((TOTAL ? 1 : 0))
