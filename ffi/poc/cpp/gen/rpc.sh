#!/usr/bin/env bash
# The RPC arm, as design/SHAPES.md's GRID. R9 is why it measures CPU rather than wall clock.
#
# Builds nothing: run `cmake --build build --target rpcbench rpccounts` first, with
# -DAK_RPC=ON. `gen/rpcflow.sh` is the flow-control probe and writes its own log.
set -u
cd "$(dirname "$0")/.." || exit 2
CALLS="${1:-80}"
ROUNDS="${2:-9}"

echo "# The RPC arm (ABI v1 section 9, design/SHAPES.md's RPC arm), as a GRID"
echo "#   date       $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "#   machine    $(uname -srm), $(nproc) vCPU, $(grep -m1 'model name' /proc/cpuinfo | sed 's/.*: //')"
echo "#   incumbent  grpc++ $(pkg-config --modversion grpc++) + protobuf C++ $(protoc --version | awk '{print $2}') (apt). packages/cpp pins neither"
echo "#   core       tonic 0.14 over hyper 1.11, through the C ABI, shared library, 2 tokio workers"
echo "#   transport  Unix domain socket PRIMARY, loopback TCP as a labelled second row; server IN-PROCESS"
echo "#   windows    stated per run below, and established from each stack's own behaviour in logs/cpp/rpcflow.log"
echo "#   payload    P2.2, 540,422 B response, EMPTY request"
echo "#   build      -O2 -g -DNDEBUG, C++17, shared linkage"
echo "#   commit     $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo

echo "== R13: this machine, and it is NOT the machine the published tables came from =="
echo "   The cross-language crossing table is a table of absolutes, so every slice"
echo "   calibrates its own. This container is an Intel Xeon at 2.10 GHz; every other"
echo "   figure in this slice's STATE.md was taken on a 2.80 GHz one. Nothing here is"
echo "   comparable to those absolutes and the ratios inside this file are the result."
if [ -x ./build/bench_a17_shared ]; then
  echo "   This slice's own crossing, measured HERE by the unchanged bench:"
  AK_BENCH_ONLY=P1.1 ./build/bench_a17_shared 2>/dev/null \
    | sed -n '/the crossing itself/,/^$/p' | head -6 | sed 's/^/     /'
else
  echo "   build/bench_a17_shared is not built; the crossing was not re-taken."
fi
cat <<'EOF'
   The rust slice's own crossing benchmark -- what R13 actually asks each slice to
   quote -- COULD NOT BE RUN, and that is a defect in another slice rather than a
   choice here. `ffi/poc/rust/crates/facade/src/generated/core_native.rs` still calls
   `d.skip(wire)`; C24 changed the shared runtime's signature to `skip(tag, wire)`
   and swept the CPP generator's 13 emission sites, and the rust slice's generated
   tree was not regenerated. 20 E0061 errors, so `cargo build --bin bench` fails.
   `poc/rust/**` is not this slice's to write. Logged as C27.
EOF
echo

echo "== R0: what this slice ADDED to the shared core, and what it did not touch =="
cat <<'R0EOF'
   ak_client_new_opts (design/SHAPES.md wants every cell pinned and the core could not be
   pinned at all) and ak_rpc_counters / ak_rpc_counters_reset / ak_rpc_counting (R5). All
   of them inside --features rpc, so the DEFAULT artifact is untouched. Counted:
R0EOF
for d in target target-rpc target-rpc-count; do
  so="core-build/$d/release/libak_core.so"
  [ -f "$so" ] || continue
  n=$(nm -D --defined-only "$so" | grep -c " T ak_")
  echo "   $d: $n ak_ exports"
done
echo "   The default build is 86, which is what it was before this work unit. The rpc builds"
echo "   carry section 9's entry points on top and are never what a codec arm links."
echo

echo "== ABI v1 section 9: the RPC half does not know the schema =="
echo "   'It moves opaque bytes and dispatches on a path string, so there is no place a"
echo "    per-field cost could enter.' Checked rather than trusted:"
for f in ../codec/crates/ak-core/src/rpc.rs ../codec/crates/rpc/src/lib.rs; do
  [ -f "$f" ] || continue
  n=$(grep -cE 'ResultRaw|TaskDetailed|Probe|TaskSummary|UploadResultData|MetricsBatch|DualResponse|shapes' "$f")
  echo "   $f: $n mentions of any message type in this schema"
done
echo

echo "== R5: the crossing count per RPC, from a COUNTING core =="
echo "   A separate binary linking ak-core built --features rpc,count. The timed binary"
echo "   below links the core WITHOUT it, which is what R5 requires."
if [ -x ./build/rpccounts ]; then
  ./build/rpccounts 20 2>&1 | sed 's/^/   /'
else
  echo "   build/rpccounts is not built."
fi
echo

# The grid goes to a temp file first, so the cross-configuration verdict below can be
# DERIVED from it rather than typed beside it.
GRIDOUT=$(mktemp)
trap 'rm -f "$GRIDOUT"' EXIT
for tr in uds tcp; do
  for pin in pin nopin; do
    {
      echo "=================================================================================="
      echo "== the grid: transport $tr, pinning $pin =="
      ./build/rpcbench "$CALLS" "$ROUNDS" "$tr" "$pin" 2>&1
      echo
    } >> "$GRIDOUT"
  done
done
{
  echo "=================================================================================="
  echo "== the deliveries on a call small enough to SEE them: Ping, UDS, pinned =="
  echo "   The grid above sits on a 540 KB response and about 4 ms of CPU, where a 0.30 ns"
  echo "   reverse crossing cannot be seen whatever it costs. This block removes the codec"
  echo "   from both sides so what is left is the transport and the three deliveries."
  ./build/rpcbench 500 "$ROUNDS" small pin 2>&1
  echo
} >> "$GRIDOUT"
cat "$GRIDOUT"
python3 gen/rpc_verdict.py "$GRIDOUT"
