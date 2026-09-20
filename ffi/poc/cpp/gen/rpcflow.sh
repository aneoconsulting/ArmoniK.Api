#!/usr/bin/env bash
# The flow-control probe. design/SHAPES.md requires every RPC arm to state its stream and
# connection window and whether auto-tuning is on, and names two traps that "a slice
# establishes from its own runtime's source rather than inheriting". This runs the probe.
#
# Builds nothing: `cmake --build build --target rpcflow` first, with -DAK_RPC=ON.
set -u
cd "$(dirname "$0")/.." || exit 2
echo "# The flow-control probe (design/SHAPES.md, 'each RPC arm states its stream and"
echo "# connection window and whether auto-tuning is on')"
echo "#   date     $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "#   machine  $(uname -srm), $(nproc) vCPU, $(grep -m1 'model name' /proc/cpuinfo | sed 's/.*: //')"
echo "#   commit   $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "#   method   each row is a CHILD process with grpc's own tracers on; the answers are"
echo "#            read out of grpc's trace, not out of a document"
echo
exec ./build/rpcflow
