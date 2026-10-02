#!/usr/bin/env bash
# EXPERIMENT backward-encode (logs/rust/opt/patches/backward-encode/): gen/gate.sh, its steps
# unchanged, run in a WORKTREE that carries the patch (the backward core, the reverse-
# delivering binding, the experiment's checks), plus the experiment's own checks.
# Nothing timed (container).
#
#   gen/bwd_gate.sh WORKTREE OUT_DIR
#
# WORKTREE is a checkout of this branch with backward-encode.patch applied (git apply) and the
# generators re-run; ffi/poc/codec of THIS checkout is never touched.
# One change to how the gate runs, and only one: step 7 (obligation 12.5's concurrency suite)
# is run and recorded but does not stop the gate. Its two planted arms (pad-widths, global-
# widths) exist to make the LEARNED WIDTH produce wrong bytes; the backward core has no learned
# width, so they cannot fail and concur.sh reports them as "did not behave as required". That
# is the obligation becoming vacuous, recorded in the log, not a pass and not a defect.
set -Eeuo pipefail
WT=${1:?usage: bwd_gate.sh WORKTREE OUT_DIR}; OUT=${2:?out dir}
WT=$(cd "$WT" && pwd); mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
R="$WT/ffi/poc/rust"
G="$OUT/gate.log"
{
  echo "# backward-encode gate, $(date -u +%FT%TZ), container (not the campaign machine): nothing here is a timing"
  echo "# worktree $WT at $(git -C "$WT" rev-parse HEAD) + the patch (git diff --stat below)"
  git -C "$WT" diff --stat HEAD | sed 's/^/#   /'
  git -C "$WT" status --short | grep '^??' | sed 's/^/#   untracked: /' || true
  echo "# $(rustc --version); $(cargo --version); nproc $(nproc)"
  echo "# driver: gen/bwd_gate.sh (gen/gate.sh's steps unchanged; step 7 recorded, not fatal: see the script header)"
  echo
} > "$G"
T="$R/gen/.gate-bwd.sh"
python3 - "$R/gen/gate.sh" "$T" <<'EOF'
import sys
s = open(sys.argv[1]).read()
old = 'gen/concur.sh 2>/dev/null | grep -vE "^\\s*$"'
assert s.count(old) == 1, "gate.sh step 7 line not found"
s = s.replace(old, '{ rc7=0; gen/concur.sh 2>/dev/null || rc7=$?; echo "  bwd_gate: concur.sh exit $rc7 (recorded, not fatal: the planted learned-width arms are vacuous on the backward core)"; } | grep -vE "^\\s*$"')
open(sys.argv[2], "w").write(s)
EOF
chmod +x "$T"
rc=0
( cd "$R" && bash "$T" ) 2>&1 | tee -a "$G" || rc=${PIPESTATUS[0]}
rm -f "$T"
echo "gate rc=$rc" | tee -a "$G"

# the experiment's own checks: full build, counting build, no-unknown build, planted control
C="$OUT/bwd_check.log"
cd "$R"
{
  echo "# bwd_check (harness::bwd_tests), $(date -u +%FT%TZ)"
  echo "## full build (target/)"
  CARGO_TARGET_DIR="$R/target" cargo build --release -q -p harness --bin bwd_check
  AK_NO_TIMING=1 "$R/target/release/bwd_check"; echo "rc=$?"
  echo; echo "## counting build (target-count/): the per-payload counters"
  CARGO_TARGET_DIR="$R/target-count" cargo build --release -q -p harness --features count --bin bwd_check
  AK_NO_TIMING=1 "$R/target-count/release/bwd_check"; echo "rc=$?"
  echo; echo "## no-unknown build (target-nounk/)"
  CARGO_TARGET_DIR="$R/target-nounk" cargo build --release -q -p harness --no-default-features --features guard,init-guard --bin bwd_check
  AK_NO_TIMING=1 "$R/target-nounk/release/bwd_check"; echo "rc=$?"
  echo; echo "## planted control: AK_BWD_PLANT=forward (the committed FORWARD delivery order against this core): MUST FAIL"
  prc=0; AK_NO_TIMING=1 AK_BWD_PLANT=forward "$R/target/release/bwd_check" > "$OUT/bwd_plant.log" 2>&1 || prc=$?
  echo "plant rc=$prc; $(grep -c '^FAIL' "$OUT/bwd_plant.log") FAIL, $(grep -c '^PASS' "$OUT/bwd_plant.log") PASS (the PASS rows are fields delivered in ONE call, or whose elements are all equal: order-invisible)"
  [ "$prc" != 0 ] && echo "PLANT DETECTED" || { echo "PLANT NOT DETECTED"; exit 1; }
} > "$C" 2>&1 || rc=$((rc ? rc : 5))
tail -3 "$C" | tee -a "$G"
echo "bwd_gate rc=$rc" | tee -a "$G"
exit $rc
