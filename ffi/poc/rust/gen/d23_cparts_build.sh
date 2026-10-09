#!/usr/bin/env bash
# D23 C isolation: build one FSM variant per set of fix-C parts. REQUIRES the probe switch, not committed:
#   git apply ffi/logs/rust/opt/d23-fsm-c-isolate/cparts-probe-switch.patch (on 75f819f8's rust_fsm.py)
# (AK_FSM_C_PARTS in
# rust_fsm.py: m = step inlined + merged next checks (C0), v = values cached at begin, f = unchecked
# frame fetch, i = branch-free input slice) into target-cp-<name>, check it with fsm_diff, restore
# the committed generated core.  gen/d23_cparts_build.sh NAME PARTS
set -euo pipefail
cd "$(dirname "$0")/.."
N=$1; P=$2
( cd ../codec && AK_FSM_C_PARTS="$P" python3 gen/generate.py --core-only >/dev/null 2>&1 )
CARGO_TARGET_DIR=$PWD/target-cp-$N cargo build --release -q -p campaign --bin fsm_attrib --bin fsm_diff 2>/dev/null
echo "$N [$P]: $(target-cp-$N/release/fsm_diff --no-malformed | tail -1) core $(sha256sum target-cp-$N/release/deps/libak_core.so | cut -c1-16)"
( cd ../codec && python3 gen/generate.py --core-only >/dev/null 2>&1 )
