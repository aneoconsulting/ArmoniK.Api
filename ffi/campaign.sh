#!/usr/bin/env bash
# The measurement campaign, top level (design/CAMPAIGN.md, W13, requirement 31).
#
# Runs the slices ONE AFTER ANOTHER, never concurrently (requirement 1), each through
# its own runner, gate first (requirement 26). A slice whose gate fails produces no
# timing and the campaign moves on to the next slice; the exit status is non-zero if
# any slice failed.
#
#   AK_CPU_CLIENT=4-7 AK_CPU_SERVER=8-11 ffi/campaign.sh [options]
#
# Options:
#   --slices "rust cpp csharp java python"   subset and order (default: all, this order)
#   --suites "gate calib codec rpc"          subset; gate always runs first (default: all)
#   --launches N / --rounds N                override the slices' defaults (3 / 5)
#   --out-root DIR                           default: ffi/logs; each slice writes to
#                                            DIR/<lang>/campaign/run-<stamp>/ (req 29)
#   --smoke                                  1 launch, 1 round, reduced work: a harness
#                                            check, never a result (req 32)
#   --allow-dirty                            pass the slices' dirty-tree override (smoke only)
#   --dry-run                                print what would run, run nothing
#
# The runners do not share one spelling for launches, rounds, smoke and the dirty-tree
# override, so the table below maps each option onto each runner. The shared interface
# is `--suite S --out DIR` plus AK_CPU_CLIENT / AK_CPU_SERVER in the environment.

set -u -o pipefail

FFI="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$FFI/.." && pwd)"

SLICES="rust cpp csharp java python"
SUITES="gate calib codec rpc"
LAUNCHES=""
ROUNDS=""
OUT_ROOT="$FFI/logs"
SMOKE=0
ALLOW_DIRTY=0
DRY=0

while [ $# -gt 0 ]; do
  case "$1" in
    --slices)      SLICES="$2"; shift 2 ;;
    --suites)      SUITES="$2"; shift 2 ;;
    --launches)    LAUNCHES="$2"; shift 2 ;;
    --rounds)      ROUNDS="$2"; shift 2 ;;
    --out-root)    OUT_ROOT="$2"; shift 2 ;;
    --smoke)       SMOKE=1; shift ;;
    --allow-dirty) ALLOW_DIRTY=1; shift ;;
    --dry-run)     DRY=1; shift ;;
    -h|--help)     sed -n '2,27p' "$0"; exit 0 ;;
    *) echo "campaign.sh: unknown option $1" >&2; exit 2 ;;
  esac
done

# --- where each slice's runner is -------------------------------------------------
runner_of() {
  case "$1" in
    rust)   echo "$FFI/poc/rust/run_campaign.sh" ;;
    cpp)    echo "$FFI/poc/cpp/gen/run_campaign.sh" ;;
    csharp) echo "$FFI/poc/csharp/run_campaign.sh" ;;
    java)   echo "$FFI/poc/java/gen/run_campaign.sh" ;;
    python) echo "$FFI/poc/python/run_campaign.sh" ;;
    *) return 1 ;;
  esac
}

# --- per-runner spelling of the shared options -------------------------------------
# Prints `env` assignments (one per line, NAME=VALUE) and extra arguments (lines
# starting with '--'), which the caller splits.
knobs_of() {
  local s="$1"
  case "$s" in
    rust)
      [ -n "$LAUNCHES" ] && echo "AK_LAUNCHES=$LAUNCHES"
      [ -n "$ROUNDS" ]   && echo "AK_ROUNDS=$ROUNDS"
      [ "$SMOKE" = 1 ]   && echo "AK_SMOKE=1"
      [ "$ALLOW_DIRTY" = 1 ] && echo "AK_ALLOW_DIRTY=1" ;;
    cpp)
      [ -n "$LAUNCHES" ] && echo "AK_CAMPAIGN_LAUNCHES=$LAUNCHES"
      [ -n "$ROUNDS" ]   && echo "AK_CAMPAIGN_ROUNDS=$ROUNDS"
      if [ "$SMOKE" = 1 ]; then echo "AK_CAMPAIGN_LAUNCHES=1"; echo "AK_CAMPAIGN_ROUNDS=1"; fi
      [ "$ALLOW_DIRTY" = 1 ] && echo "AK_CAMPAIGN_ALLOW_DIRTY=1" ;;
    csharp)
      [ -n "$LAUNCHES" ] && echo "--launches $LAUNCHES"
      [ -n "$ROUNDS" ]   && echo "--rounds $ROUNDS"
      [ "$SMOKE" = 1 ]   && echo "--smoke"
      [ "$ALLOW_DIRTY" = 1 ] && echo "AK_ALLOW_DIRTY=1" ;;
    java)
      [ -n "$LAUNCHES" ] && echo "AK_LAUNCHES=$LAUNCHES"
      [ -n "$ROUNDS" ]   && echo "AK_ROUNDS=$ROUNDS"
      [ "$SMOKE" = 1 ]   && echo "AK_CAMPAIGN_SMOKE=1"
      [ "$ALLOW_DIRTY" = 1 ] && echo "AK_CAMPAIGN_ALLOW_DIRTY=1" ;;
    python)
      [ -n "$LAUNCHES" ] && echo "--launches $LAUNCHES"
      [ -n "$ROUNDS" ]   && echo "--rounds $ROUNDS"
      [ "$SMOKE" = 1 ]   && echo "--smoke"
      [ "$ALLOW_DIRTY" = 1 ] && echo "--allow-dirty" ;;
  esac
  return 0
}

# --- preconditions ------------------------------------------------------------------
if [ "$SMOKE" = 1 ] && { [ -z "${AK_CPU_CLIENT:-}" ] || [ -z "${AK_CPU_SERVER:-}" ]; }; then
  # A smoke run checks the plumbing, so any two disjoint sets will do.
  n=$(nproc)
  if [ "$n" -ge 4 ]; then export AK_CPU_CLIENT="${AK_CPU_CLIENT:-0-1}" AK_CPU_SERVER="${AK_CPU_SERVER:-2-3}"
  elif [ "$n" -ge 2 ]; then export AK_CPU_CLIENT="${AK_CPU_CLIENT:-0}" AK_CPU_SERVER="${AK_CPU_SERVER:-1}"
  else echo "campaign.sh: need at least 2 CPUs for a client and a server set" >&2; exit 2; fi
fi
if [ "$SMOKE" != 1 ] && { [ -z "${AK_CPU_CLIENT:-}" ] || [ -z "${AK_CPU_SERVER:-}" ]; }; then
  echo "campaign.sh: AK_CPU_CLIENT and AK_CPU_SERVER are required outside --smoke (CAMPAIGN.md req 4)" >&2
  exit 2
fi
if [ "$ALLOW_DIRTY" = 1 ] && [ "$SMOKE" != 1 ]; then
  echo "campaign.sh: --allow-dirty is for --smoke only (req 27 refuses a dirty tree)" >&2
  exit 2
fi
if [ "$ALLOW_DIRTY" != 1 ] && [ -n "$(git -C "$REPO" status --porcelain -- ffi)" ]; then
  echo "campaign.sh: the ffi/ tree is dirty; commit first (CAMPAIGN.md req 27)" >&2
  exit 2
fi
for s in $SLICES; do
  r="$(runner_of "$s")" || { echo "campaign.sh: unknown slice $s" >&2; exit 2; }
  [ -x "$r" ] || { echo "campaign.sh: no executable runner for $s at $r" >&2; exit 2; }
done
# gate first, then the other suites in the order given
ORDERED="gate"
for x in $SUITES; do [ "$x" = gate ] || ORDERED="$ORDERED $x"; done

STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
COMMIT="$(git -C "$REPO" rev-parse HEAD)"
SUMMARY="$OUT_ROOT/campaign-$STAMP.summary"
[ "$DRY" = 1 ] || mkdir -p "$OUT_ROOT"

say() { if [ "$DRY" = 1 ]; then echo "$*"; else echo "$*" | tee -a "$SUMMARY"; fi; }

say "campaign $STAMP commit $COMMIT$([ "$SMOKE" = 1 ] && echo ' SMOKE (instrumentation, not results)')"
say "host $(uname -n) kernel $(uname -r) cpus $(nproc)"
say "governor $(cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || echo n/a)" \
    "no_turbo $(cat /sys/devices/system/cpu/intel_pstate/no_turbo 2>/dev/null || echo n/a)" \
    "smt $(cat /sys/devices/system/cpu/smt/active 2>/dev/null || echo n/a)" \
    "isolated $(cat /sys/devices/system/cpu/isolated 2>/dev/null || echo n/a)"
say "CLIENT=${AK_CPU_CLIENT:-unset} SERVER=${AK_CPU_SERVER:-unset}"
say "slices: $SLICES  suites: $ORDERED  launches: ${LAUNCHES:-default}  rounds: ${ROUNDS:-default}"

FAILED=""
for s in $SLICES; do
  r="$(runner_of "$s")"
  out="$OUT_ROOT/$s/campaign/run-$STAMP"
  envs=(); args=()
  while IFS= read -r k; do
    [ -z "$k" ] && continue
    case "$k" in --*) read -r -a parts <<< "$k"; args+=("${parts[@]}") ;; *) envs+=("$k") ;; esac
  done < <(knobs_of "$s")
  say "== $s -> $out"
  [ "$DRY" = 1 ] || mkdir -p "$out"
  for suite in $ORDERED; do
    cmd=(env ${envs[@]+"${envs[@]}"} "$r" --suite "$suite" --out "$out" ${args[@]+"${args[@]}"})
    if [ "$DRY" = 1 ]; then
      echo "   (cd $(dirname "$r") && ${cmd[*]})"
      continue
    fi
    t0=$(date +%s)
    # Foreground: the campaign never leaves a background job (ffi/CLAUDE.md).
    ( cd "$(dirname "$r")" && "${cmd[@]}" ) > "$out/$suite.runner.log" 2>&1
    rc=$?
    say "   $suite rc=$rc $(( $(date +%s) - t0 ))s  ($out/$suite.runner.log)"
    if [ $rc -ne 0 ]; then
      FAILED="$FAILED $s:$suite"
      [ "$suite" = gate ] && { say "   gate failed: no timing for $s"; break; }
    fi
  done
done

if [ -n "$FAILED" ]; then
  say "FAILED:$FAILED"
  exit 1
fi
say "all slices completed"
