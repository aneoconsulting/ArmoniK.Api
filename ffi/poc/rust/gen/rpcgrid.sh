#!/usr/bin/env bash
# ABI v1 section 9's three deliveries, and the A/B/C RPC grid.
#
# 1. configuration, and the two things R5 always asks: is the boundary real, and is
#    the artifact that is loaded the one that was built (from ldd, not the build log);
# 2. that the RPC half still knows nothing about a message type, which is what makes
#    the per-field crossing count 0 BY CONSTRUCTION rather than by measurement;
# 3. the run itself: window control, deliveries, floor, grid, codec share.
#
# Refuses to start if anything else in this container is benchmarking (README section 11:
# two timed processes on four vCPUs corrupt each other).
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."

# `pgrep -x` on the BASENAME, never `pgrep -f` on a path substring. `-f` matches the full
# command line of every process, so a guard written with it matches the very shell that is
# running the guard, and this check refuses every time. That is failure mode 1 in
# `ffi/CLAUDE.md`, and it caught this script on its first run.
for b in bench pullbench guardcost concur rpcgrid contentall lifecycle; do
  if pgrep -x "$b" >/dev/null 2>&1; then
    echo "REFUSING: $b is already running on this box (README section 11)." >&2
    pgrep -ax "$b" >&2 || true
    exit 1
  fi
done

echo "===== 0. configuration ====="
rustc --version
echo "incumbent: prost $(grep -m1 'name = "prost"' -A1 Cargo.lock | grep version | cut -d'"' -f2)"
echo "           tonic  $(grep -m1 'name = "tonic"' -A1 Cargo.lock | grep version | cut -d'"' -f2)"
nproc; grep -m1 'model name' /proc/cpuinfo || true

echo
echo "===== 1. the RPC half mentions no message type ====="
echo "# Section 9 rests on this and the crossing table asserts it, so it is checked and"
echo "# not assumed. A hit here means the per-field column is a measurement, not a 0."
for f in ../codec/crates/rpc/src/lib.rs ../codec/crates/ak-core/src/rpc.rs; do
  printf '%-44s ' "$f"
  if grep -nE '\b(ResultRaw|TaskRequest|SessionRaw|Empty|m1|m2|m3|m4|m5|m6|m7)\b' "$f" >/dev/null; then
    echo "MENTIONS A MESSAGE TYPE -- section 9's claim is broken"; grep -nE '\b(ResultRaw|TaskRequest|SessionRaw)\b' "$f"
  else
    echo "clean"
  fi
done

echo
echo "===== 2. the boundary is real (R5), and the loaded artifact is the built one ====="
cargo build --release -q -p harness --bin rpcgrid 2>/dev/null
BIN=target/release/rpcgrid
echo "# the RPC entry points as undefined dynamic imports, resolved by the linker:"
nm -D --undefined-only "$BIN" | grep -E ' ak_(call|queue|client|bytes)' || echo "NONE -- the boundary is gone"
echo
echo "# and the shared object ldd says is actually loaded:"
ldd "$BIN" | grep ak_core

echo
echo "===== 3. the run ====="
./"$BIN" 2>/dev/null
