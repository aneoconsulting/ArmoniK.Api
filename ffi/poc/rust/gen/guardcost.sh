#!/usr/bin/env bash
# What ABI v1 section 3's guard costs, asked the way R4's sharpened half says to ask it:
# a delta between arms in the same interleaved rounds of ONE process and ONE build.
#
# The two-build form (gen/guardprice.sh) does not survive its own control -- see the log.
# This does, because it carries a TWIN: a second exported function identical to `ak_noop`
# with no guard in it, which must measure zero and does not. What it measures instead is
# the floor, and the floor is the verdict.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."

if pgrep -x bench >/dev/null 2>&1; then
  echo "REFUSING: a bench process is running (README section 11)." >&2
  exit 2
fi

echo "===== configuration ====="
rustc --version
nproc
grep -m1 'model name' /proc/cpuinfo || true

echo
cargo build --release -q -p harness --bin guardcost
echo "# the boundary is real: both no-ops are undefined dynamic imports (R5)"
nm -D --undefined-only target/release/guardcost | grep -E ' ak_noop'
echo "# and the shared object that got loaded, from ldd:"
ldd target/release/guardcost | grep ak_core

for i in 1 2 3; do
  echo
  echo "----- run $i -----"
  ./target/release/guardcost
done
