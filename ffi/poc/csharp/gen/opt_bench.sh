#!/usr/bin/env bash
# The csharp slice's OPTIMISATION BENCHMARK (the optimisation pass's short baseline and its
# later before/after runs). CONTAINER INSTRUMENTATION, never a campaign result (README 1.1).
#
#   gen/opt_bench.sh --out DIR [--suite codec|rpc|both]
#
# It runs the CORE grid (CAMPAIGN section 4.0, AK_CAMPAIGN_GRID=core) exactly as
# run_campaign.sh does it (the same binaries, the same BDN default toolchain with one child
# process per case, the same merged runs: one BDN run per codec build, one per RPC run kind:
# stock, h2-batch, pinned-allocator subset), with three differences, all stated in the header:
#   * NOT GATED: gen/gate.sh is not run (it runs at the end of an experiment). Each codec
#     process still runs Cases.Verify (byte identity of every encode arm and variant, every
#     U-* row) before timing, and every RPC call is still checked (req 18);
#   * SHORT BDN settings (one launch; OPT_* below), instead of the campaign's 3 x 5 x 10 x 100 ms;
#   * AK_BDN_MEMORY=1: BenchmarkDotNet's MemoryDiagnoser (one extra workload iteration per
#     case after the actual stage, outside the job's clock), allocated bytes and gen0/1/2 per case.
#
# Settings (defaults): OPT_CODEC_WARMUP 25, OPT_CODEC_ROUNDS 6, OPT_CODEC_ITER_MS 40 (25 x 40 ms = ~1 s of
# warm-up per child: on 2 CPUs a BDN child needs about that long before tier 1 is in place, JOURNAL 73,
# logs/csharp/opt/tier-check/); OPT_RPC_WARMUP 10 (the campaign's; req 24: >= 45 calls per caller thread),
# OPT_RPC_ROUNDS 6, OPT_RPC_ITER_MS 100 (the campaign's); OPT_SERVER_WARM 500 (campaign 2000; the server is the Rust tonic process, no JIT);
# AK_CPU_CLIENT (default 0,1: a one-CPU .NET client is refused, JOURNAL 73), AK_CPU_SERVER (default 2,3), AK_WORKERS (default 8, the runner's).
set -uo pipefail
SLICE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO="$(git -C "$SLICE" rev-parse --show-toplevel)"
OUT="" SUITES="codec rpc"
while [ $# -gt 0 ]; do
  case "$1" in
    --out) OUT="$2"; shift ;;
    --suite) case "$2" in both) SUITES="codec rpc" ;; *) SUITES="$2" ;; esac; shift ;;
    *) echo "unknown argument $1" >&2; exit 2 ;;
  esac
  shift
done
[ -n "$OUT" ] || { echo "usage: gen/opt_bench.sh --out DIR [--suite codec|rpc|both]" >&2; exit 2; }
mkdir -p "$OUT"; OUT="$(cd "$OUT" && pwd)"
export SCRATCH="${SCRATCH:-$(mktemp -d)}"
export DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1
export AK_CPU_CLIENT="${AK_CPU_CLIENT:-0,1}" AK_CPU_SERVER="${AK_CPU_SERVER:-2,3}"
export AK_WORKERS="${AK_WORKERS:-8}"; export AK_SERVER_THREADS="${AK_SERVER_THREADS:-$AK_WORKERS}"
export AK_CAMPAIGN_GRID=core AK_CAMPAIGN_ALLOC=default AK_BDN_MEMORY=1
export AK_LLC_BYTES="${AK_LLC_BYTES:-14417920}"
unset GLIBC_TUNABLES
PINNED_TUNABLES="glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432"
CW="${OPT_CODEC_WARMUP:-25}" CR="${OPT_CODEC_ROUNDS:-6}" CT="${OPT_CODEC_ITER_MS:-40}"
RW="${OPT_RPC_WARMUP:-10}" RR="${OPT_RPC_ROUNDS:-6}" RT="${OPT_RPC_ITER_MS:-100}" WARM="${OPT_SERVER_WARM:-500}"
B8="$SLICE/src/BenchDotNet/bin/Release/net8.0"; BN8="$SLICE/src/BenchDotNet/bin-nounk/Release/net8.0"
R8="$SLICE/src/Rpc/bin/Release/net8.0"; RN8="$SLICE/src/Rpc/bin-nounk/Release/net8.0"
# OPT_DROP=1 (optimisation runs, labelled extras): the codec's drop units (AK_BDN_DROP=1), the RPC
# drop framed cells Cf-drop and Ef-drop (with C-drop and E-drop's a+read), and the RPC no-unknown
# client (Cf-nounk, Ef-nounk with C-nounk and E-nounk's a+read) in its own run.
OPT_DROP="${OPT_DROP:-0}"; [ "$OPT_DROP" = 1 ] && export AK_BDN_DROP=1
COMMIT="$(git -C "$REPO" rev-parse HEAD)"
CODE="ffi/poc/csharp/src ffi/poc/csharp/gen ffi/poc/csharp/abi ffi/poc/csharp/Directory.Build.props ffi/poc/csharp/Directory.Build.targets"
DIRTY="clean"; git -C "$REPO" diff --quiet HEAD -- $CODE ffi/schema ffi/corpus || DIRTY="DIRTY (uncommitted changes in the slice, schema or corpus)"
sha() { sha256sum "$1" | cut -d' ' -f1; }
for d in target-core target-core-nounk target-core-h2b; do
  [ -f "$SLICE/$d/release/libak_core.so" ] || { echo "no $d; run gen/build_core.sh first" >&2; exit 1; }
done
now() { date +%s.%N; }
el() { python3 -c "print(f'{$2-$1:.1f}')"; }

{
  echo "# csharp slice OPTIMISATION BENCHMARK (gen/opt_bench.sh), core grid (AK_CAMPAIGN_GRID=core, CAMPAIGN section 4.0)"
  echo "# CONTAINER INSTRUMENTATION: not a campaign result (README 1.1, CAMPAIGN section 2); NOT GATED (gen/gate.sh not run); each codec process's Cases.Verify byte-identity pre-check is on, and every RPC call is checked (req 18)"
  echo "# commit:     $COMMIT ($DIRTY)"
  echo "# utc:        $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# machine:    $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2 | sed 's/^ //'); $(nproc --all) logical CPUs; kernel $(uname -r); smt $(cat /sys/devices/system/cpu/smt/active 2>/dev/null || echo n/a)"
  echo "# cpu sets:   CLIENT=$AK_CPU_CLIENT SERVER=$AK_CPU_SERVER (taskset); AK_WORKERS=$AK_WORKERS (core runtime workers, .NET thread pool min/max in the RPC client), server tokio workers AK_SERVER_THREADS=$AK_SERVER_THREADS"
  echo "# .NET:       SDK $(dotnet --version); runtimes $(dotnet --list-runtimes | awk '{print $1" "$2}' | tr '\n' ';') (Ubuntu noble-updates packages dotnet-sdk-8.0 8.0.131-0ubuntu1~24.04.1); target net8.0 Release; workstation concurrent GC, tiering/PGO at defaults"
  echo "# rust:       $(rustc --version); $(cargo --version)"
  echo "# core:       target-core (rpc,init-guard) sha256 $(sha "$SLICE/target-core/release/libak_core.so")"
  echo "#             target-core-nounk (rpc,init-guard, --no-default-features) sha256 $(sha "$SLICE/target-core-nounk/release/libak_core.so")"
  echo "#             target-core-h2b (rpc,init-guard, h2-batch) sha256 $(sha "$SLICE/target-core-h2b/release/libak_core.so")"
  echo "#             built by gen/build_core.sh from git archive HEAD ffi/poc/codec (last core commit $(git -C "$REPO" log -1 --format=%h -- ffi/poc/codec/crates))"
  echo "# BDN:        BenchmarkDotNet 0.15.8, DEFAULT toolchain (one child process per case, as the campaign), one launch (launch 1: full build first), merged runs (one per codec build; one per RPC run kind)"
  echo "#             codec: --warmup $CW --rounds $CR --iteration-ms $CT (campaign: 10 / 5 / 100, 3 launches)"
  echo "#             rpc:   --warmup $RW --rounds $RR --iteration-ms $RT (campaign: 10 / 5 / 100, 3 launches); >= $((1 + 4 + 4 * RW)) calls per caller thread before the first measured value (req 24: >= 20)"
  echo "#             warm-up length (JOURNAL 73, logs/csharp/opt/tier-check/): .NET 8 tiers up after a call-counting delay of 100 ms (x10 on a one-CPU affinity mask), restarted by each new tier-0 JIT; a BDN child measured tier-0 or mid-tier-up code with 4 x 40 ms of warm-up (1 CPU: always; 2 CPUs: often), and settled code with 10 x 100 ms or 25 x 40 ms on 2 CPUs; every timed process refuses a one-CPU affinity mask (CpuGuard) and each case's first row carries the CPUs its process saw"
  echo "#             MemoryDiagnoser on (AK_BDN_MEMORY=1): one extra workload iteration per case after the actual stage, outside the job's clock"
  echo "# grid:       codec: units incumbent-prod:default, core-ffi:retain, host-gen:retain (full build), core-ffi:no-unknown, host-gen:no-unknown (no-unknown build); encode-transport-hot and decode-read; 16 shapes (P7.1 decode only), Latin-1 and wide on P2.2, 7 U-* rows"
  echo "#             rpc: transport armonik; stock h2: A, Bf (+ B a+read), Cf-retain (+ C-retain a+read), Ef-retain (+ E-retain a+read) on a+read, b (P2.2), c (P5.4), d (16 MiB) at k = 1 and 8; h2-batch: Cf-retain on c, d at k = 1, 8; pinned allocator: A, Cf-retain on c, d at k = 1"
  echo "# dropped:    nothing of the core grid; calib not run; no plant controls; server warm-up $WARM calls per direction (campaign 2000)"
  [ "$OPT_DROP" = 1 ] && echo "# extras:     OPT_DROP=1: codec drop units core-ffi:drop and host-gen:drop (AK_BDN_DROP=1); RPC Cf-drop, Ef-drop (+ C-drop, E-drop a+read) in the stock run; the RPC no-unknown client Cf-nounk, Ef-nounk (+ C-nounk, E-nounk a+read) in its own run (rpc-nounk.jsonl)"
  echo "# allocator:  default (GLIBC_TUNABLES unset) for every process except the pinned-allocator subset (GLIBC_TUNABLES=$PINNED_TUNABLES)"
} > "$OUT/header.txt"
cat "$OUT/header.txt"

T0=$(now)
( cd "$SLICE" && dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release > "$OUT/build.log" 2>&1 \
  && dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release -p:AkNounk=true >> "$OUT/build.log" 2>&1 \
  && dotnet build src/Rpc/akrpc.csproj -c Release >> "$OUT/build.log" 2>&1 \
  && { [ "$OPT_DROP" != 1 ] || dotnet build src/Rpc/akrpc.csproj -c Release -p:AkNounk=true >> "$OUT/build.log" 2>&1; } ) || { tail -30 "$OUT/build.log"; exit 1; }
echo "# build: $(el "$T0" "$(now)") s" | tee -a "$OUT/timing.txt"

for s in $SUITES; do
case "$s" in
codec)
  for bld in full nounk; do
    if [ "$bld" = full ]; then BX="$B8"; cp "$SLICE/target-core/release/libak_core.so" "$BX/"; else BX="$BN8"; cp "$SLICE/target-core-nounk/release/libak_core.so" "$BX/"; fi
    f="$OUT/codec-$bld.jsonl"; cp "$OUT/header.txt" "$f"
    t=$(now)
    taskset -c "$AK_CPU_CLIENT" dotnet "$BX/BenchDotNet.dll" --launch 1 --out "$f" --artifacts "$SCRATCH/bdn-codec-$bld" \
      --rounds "$CR" --warmup "$CW" --iteration-ms "$CT" --toolchain process > "$OUT/codec-$bld.bdn.log" 2>&1
    rc=$?
    echo "# codec $bld: $(el "$t" "$(now)") s rc=$rc" | tee -a "$OUT/timing.txt"
    [ $rc = 0 ] || { echo "codec $bld failed ($OUT/codec-$bld.bdn.log)" >&2; exit 1; }
  done ;;
rpc)
  SERVE="$REPO/ffi/poc/rust/serve.sh"
  export AK_SERVE_STATE="$SCRATCH/ak-rpc-server.state"
  t=$(now)
  "$SERVE" build > "$OUT/rpc-server-build.log" 2>&1 || { tail -20 "$OUT/rpc-server-build.log"; exit 1; }
  echo "# rpc server build: $(el "$t" "$(now)") s" | tee -a "$OUT/timing.txt"
  sdir="$OUT/rpc-server"; mkdir -p "$sdir"
  env -u GLIBC_TUNABLES AK_SERVER_TCP=0 "$SERVE" start --out "$sdir" > "$sdir/start.out" 2>&1 || { cat "$sdir/start.out"; exit 1; }
  TL=$(sed -n 's/^tcp //p' "$sdir/start.out")
  [ -n "$TL" ] || { echo "no TCP listener" >&2; "$SERVE" stop; exit 1; }
  t=$(now)
  "$SERVE" warm "$WARM" > "$OUT/rpc-server-warm.log" 2>&1 || { echo "server warm-up failed" >&2; "$SERVE" stop; exit 1; }
  echo "# rpc server warm ($WARM): $(el "$t" "$(now)") s" | tee -a "$OUT/timing.txt"
  BDNARGS=(--toolchain process --rounds "$RR" --warmup "$RW" --iteration-ms "$RT" --artifacts "$SCRATCH/bdn-rpc")
  KINDS="stock h2-batch stock-pinned"; [ "$OPT_DROP" = 1 ] && KINDS="$KINDS nounk"
  for kind in $KINDS; do
    h2=stock; UNITS="A,Bf,Cf-retain,Ef-retain"; [ "$OPT_DROP" = 1 ] && UNITS="$UNITS,Cf-drop,Ef-drop"; UENV=(); f="$OUT/rpc-stock.jsonl"; RX="$R8"
    case "$kind" in
      nounk) UNITS="Cf-nounk,Ef-nounk"; RX="$RN8"; f="$OUT/rpc-nounk.jsonl" ;;
      h2-batch) h2=h2-batch; UNITS="Cf-retain"; UENV=(AK_RPC_ONLY_DIRS=c,d); f="$OUT/rpc-h2-batch.jsonl" ;;
      stock-pinned) UNITS="A,Cf-retain"; UENV=(AK_RPC_ONLY_DIRS=c,d AK_RPC_ONLY_K=1 GLIBC_TUNABLES="$PINNED_TUNABLES" AK_CAMPAIGN_ALLOC=pinned); f="$OUT/rpc-stock.alloc-pinned.jsonl" ;;
    esac
    hs=""; [ "$h2" = h2-batch ] && hs="-h2b"
    cp "$SLICE/target-core$hs/release/libak_core.so" "$R8/"; cp "$SLICE/target-core-nounk$hs/release/libak_core.so" "$RN8/" 2>/dev/null
    { cat "$OUT/header.txt"; echo "# run:        $kind; units $UNITS; server pid $(sed -n 's/^pid //p' "$sdir/start.out"), TCP $TL"; } > "$f"
    t=$(now)
    env "${UENV[@]}" AK_H2="$h2" taskset -c "$AK_CPU_CLIENT" dotnet "$RX/akrpc.dll" bench --sock "tcp:$TL" --transport armonik --unit "$UNITS" \
      --launch 1 --out "$f" "${BDNARGS[@]}" --inflight 1,8 > "$OUT/rpc-$kind.bdn.log" 2>&1
    rc=$?
    echo "# rpc $kind: $(el "$t" "$(now)") s rc=$rc" | tee -a "$OUT/timing.txt"
    [ $rc = 0 ] || { echo "rpc $kind failed ($OUT/rpc-$kind.bdn.log)" >&2; "$SERVE" stop; exit 1; }
  done
  cp "$SLICE/target-core/release/libak_core.so" "$R8/"
  "$SERVE" stop > /dev/null ;;
esac
done
echo "# total: $(el "$T0" "$(now)") s" | tee -a "$OUT/timing.txt"
