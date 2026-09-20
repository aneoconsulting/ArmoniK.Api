#!/usr/bin/env bash
# The content sets on EVERY payload, where gen/content.sh covers P1.2 and P2.2.
#
# It carries the D20 regression's ancestor: the pass that found it did so because it runs
# the payloads in an order that makes them SHARE one encode context across message types,
# which nothing else in this slice does. Keep that property if this script is ever rewritten.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."
if pgrep -x bench >/dev/null 2>&1; then
  echo "REFUSING: a bench process is running (README section 11)." >&2; exit 2
fi
echo "===== 0. configuration ====="; rustc --version; nproc
grep -m1 'model name' /proc/cpuinfo || true
echo; echo "===== 1. the generator is current ====="; python3 gen/generate.py --check
echo; echo "===== 2. D20's regression, in the conformance gate ====="
cargo run --release -q -p harness --bin conformance 2>/dev/null | grep -E "after-direct|D20|VERDICT"
echo; echo "===== 3. the content sets, every payload, one process ====="
cargo run --release -q -p harness --bin contentall 2>/dev/null
