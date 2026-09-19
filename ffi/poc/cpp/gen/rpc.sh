#!/usr/bin/env bash
# The RPC arm. R9 is why it measures CPU rather than wall clock.
set -u
cd "$(dirname "$0")/.." || exit 2
echo "== ABI v1 section 9: the RPC half does not know the schema =="
echo "   'It moves opaque bytes and dispatches on a path string, so there is no place a"
echo "    per-field cost could enter.' Checked rather than trusted:"
for f in core/src/rpc.rs ../rust/crates/rpc/src/lib.rs; do
  [ -f "$f" ] || continue
  n=$(grep -cE 'ResultRaw|TaskDetailed|Probe|TaskSummary|UploadResultData|MetricsBatch|DualResponse|shapes' "$f")
  echo "   $f: $n mentions of any message type in this schema"
done
echo
echo "== the arm =="
exec ./build/rpcbench "${1:-40}"
