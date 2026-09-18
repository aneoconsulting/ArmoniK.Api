#!/usr/bin/env bash
# Stage 4, the RPC arm, end to end.
#
# Deliberately smaller than the codec stages (design/SHAPES.md). One unary RPC carrying
# P2.2 over loopback against tonic, the crossing count per RPC, and CPU at 1, 8 and 16 in
# flight. What it does NOT do is listed in the log and is not substituted for.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."
cargo run --release -q -p harness --bin rpcbench 2>/dev/null
