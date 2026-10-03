#!/usr/bin/env bash
# The java slice's campaign runner (design/CAMPAIGN.md, W11). The one entry point the owner
# runs on the campaign machine:
#
#   gen/run_campaign.sh --suite codec|rpc|calib|gate [--out <dir>]   (default ffi/logs/java/campaign)
#
# Environment:
#   AK_CPU_CLIENT, AK_CPU_SERVER   the CLIENT and SERVER cpu lists (req 4), for taskset.
#                                  Required unless AK_CAMPAIGN_SMOKE=1 (then recorded as unset).
#   AK_ISOLATION                   how the CPUs are isolated (req 3), recorded verbatim
#   AK_LAUNCHES (3), AK_ROUNDS (5) req 23
#   AK_CAMPAIGN_SMOKE=1            req 32: 1 launch, 1 round, reduced iterations, and every
#                                  figure in the log is marked instrumentation
#   AK_CAMPAIGN_ALLOW_DIRTY=1      development only: a dirty tree is otherwise refused
#                                  (req 27); the header says DIRTY
#   AK_CAMPAIGN_NO_BUILD=1         reuse the existing build (the header still records it)
#
# Order inside one invocation: build -> header -> correctness gate (req 26; the gate suite
# writes a stamp for the commit, and a timing suite runs the gate itself when no stamp for
# this commit exists in <out>) -> the suite. Target JDK 17 only is timed; the Java 8 floor
# (arms b and c) is run by the gate and never timed (req 5).
set -euo pipefail
cd "$(dirname "$0")/.."
HERE=$PWD
SUITE=""; OUT=""
while [ $# -gt 0 ]; do
  case "$1" in
    --suite) SUITE=$2; shift 2 ;;
    --out) OUT=$2; shift 2 ;;
    *) echo "usage: $0 --suite codec|rpc|calib|gate --out <dir>" >&2; exit 2 ;;
  esac
done
case "$SUITE" in codec|rpc|calib|gate) ;; *) echo "usage: $0 --suite codec|rpc|calib|gate --out <dir>" >&2; exit 2 ;; esac
# Req 29 (amended 0e8e9eb): logs go under ffi/logs/java/campaign/ unless --out says otherwise.
[ -n "$OUT" ] || OUT="$(cd "$(dirname "$0")/../../.." && pwd)/logs/java/campaign"
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)

J17=${J17:-/usr/lib/jvm/java-17-openjdk-amd64}
J8=${J8:-/usr/lib/jvm/java-8-openjdk-amd64}
unset JAVA_TOOL_OPTIONS || true
SMOKE=${AK_CAMPAIGN_SMOKE:-0}
LAUNCHES=${AK_LAUNCHES:-3}; ROUNDS=${AK_ROUNDS:-5}
if [ "$SMOKE" = 1 ]; then LAUNCHES=1; ROUNDS=1; fi

# ---- req 27: a dirty tree is refused
TOP=$(git rev-parse --show-toplevel)
COMMIT=$(git rev-parse --short HEAD)
# poc/codec, schema and corpus reach the build only through gen/build.sh's `git archive` of
# the commit, so only this slice's own directory can make the build differ from the commit.
DIRTY=$(cd "$TOP" && git status --porcelain -- ffi/poc/java | grep -v '^?? ffi/poc/java/build' || true)
if [ -n "$DIRTY" ]; then
  if [ "${AK_CAMPAIGN_ALLOW_DIRTY:-0}" != 1 ]; then
    echo "REFUSED: the tree is dirty (req 27):"; echo "$DIRTY" | head -20; exit 1
  fi
  COMMIT="$COMMIT-DIRTY"
fi

# ---- req 4: the CPU sets. ffi/campaign.sh exports them from ffi/campaign.machine; a direct
# run outside smoke reads the same file when the environment does not set them (R-H34).
MACHINE="$TOP/ffi/campaign.machine"
if [ "$SMOKE" != 1 ] && [ -f "$MACHINE" ] && { [ -z "${AK_CPU_CLIENT:-}" ] || [ -z "${AK_CPU_SERVER:-}" ]; }; then
  # shellcheck source=/dev/null
  . "$MACHINE"
fi
# Req 11 (R-H29): the pool input is sized from the last-level cache: 2 x AK_LLC_BYTES of
# serialised bytes (default 13.75 MB, the reference machine's L3, when campaign.machine
# does not set it).
AK_LLC_BYTES=${AK_LLC_BYTES:-14417920}
# Unix sockets live in a short directory of their own: a socket path is limited to 107
# bytes, which a deep checkout (a worktree under a scratch directory) exceeds.
SOCKDIR=$(mktemp -d /tmp/akj.XXXXXX)
trap 'rm -rf "$SOCKDIR"' EXIT
if [ -z "${AK_CPU_CLIENT:-}" ] || [ -z "${AK_CPU_SERVER:-}" ]; then
  if [ "$SMOKE" != 1 ] && [ "$SUITE" != gate ]; then
    echo "REFUSED: AK_CPU_CLIENT and AK_CPU_SERVER must be set (req 4)"; exit 1
  fi
fi
pin() {  # $1 = cpu list or empty
  if [ -n "$1" ]; then echo "taskset -c $1"; fi
}
PIN_C=$(pin "${AK_CPU_CLIENT:-}"); PIN_S=$(pin "${AK_CPU_SERVER:-}")

# ---- build (req 6: flags printed in the header)
if [ "${AK_CAMPAIGN_NO_BUILD:-0}" != 1 ]; then
  bash gen/build.sh > "$OUT/build-$COMMIT.log" 2>&1 || { echo "BUILD FAILED: $OUT/build-$COMMIT.log"; exit 1; }
fi
# The one RPC server of every slice (CAMPAIGN req 13 as amended at 9f6d579fa, FIX-PLAN WP10):
# the Rust slice's tonic rpc_server, built, started, warmed and stopped through its serve.sh
# (interface: poc/rust/SERVER.md). Its state file lives in this run's own directory, so
# another slice's server in the same machine is never touched.
# D11 as amended (owner 2026-10-03): the RPC core's h2 variant, stock (default) or h2-batch
# (poc/codec/h2-batch/); every RPC shim of this run is the variant's, every sample carries it,
# and the gate runs (and stamps) per variant. Codec cores carry no rpc feature, hence no h2.
AK_H2=${AK_H2:-stock}
case "$AK_H2" in stock) H2S= ;; h2-batch) H2S=-h2b ;; *) echo "AK_H2 must be stock or h2-batch"; exit 2 ;; esac
# D8 / D14: every pool sized to AK_WORKERS (campaign.machine; 8).
AK_WORKERS=${AK_WORKERS:-8}
# Req 25 / D9 as amended by the owner 2026-10-03: AK_CAMPAIGN_ALLOC=default|pinned (default:
# default). default = glibc's default allocator, the MAIN figures, as production; pinned =
# GLIBC_TUNABLES with glibc's trim and mmap thresholds pinned, the labelled diagnostic. Both
# suites, every measured JVM (the core and its transport allocate through glibc in the JVM).
# Each measured process checks it before timing (ak.CampaignAlloc: GLIBC_TUNABLES and a 16 MiB
# malloc read back through mallinfo2) and refuses on a disagreement; the header records the
# readback, and every sample carries `alloc`.
# CAMPAIGN section 4.0 (D18, owner 2026-10-03): AK_CAMPAIGN_GRID=core (default) runs exactly
# the campaign grid; full runs the grid of 4.1 and 4.2, every row beyond 4.0 a labelled extra.
AK_CAMPAIGN_GRID=${AK_CAMPAIGN_GRID:-core}
case "$AK_CAMPAIGN_GRID" in core|full) ;; *) echo "AK_CAMPAIGN_GRID must be core or full"; exit 2 ;; esac
GRID_NOTE_CODEC="grid (CAMPAIGN 4.0, D18): AK_CAMPAIGN_GRID=$AK_CAMPAIGN_GRID; core = the 16 shapes (P7.1 decode only), Latin-1 and wide on P2.2 only, the 7 named U-* rows through the shapes core, arms incumbent-prod (full build only), core-ffi push and host-gen in retain (full build) and no-unknown (no-unknown build), encode end state (ii) with one hot graph (encode-transport-hot) and decode-read, compact strings on every row and -XX:-CompactStrings on the content-set rows (P2.2 Latin-1 and wide) only; extras left out under core: incumbent-best, pull decode, bare decode, the other three encode variants, the drop mode, content sets on P1.2 and P2.4, the other 85 U-* rows timed, the corpus-core U-* invocation, utf16 on ASCII rows"
GRID_NOTE_RPC="grid (CAMPAIGN 4.0, D18): AK_CAMPAIGN_GRID=$AK_CAMPAIGN_GRID; core = cells A, Bf, Cf-retain, Ef-retain (full build, framed core cells, idiomatic delivery), a+read and b, c at P5.4, d at 16 MiB, each at k = 1 and 8, transport ONE configuration, armonik (4.0 as amended b58543f7b): cell A through packages/java's GrpcChannelBuilder called directly with its package defaults (NettyChannelBuilder.forAddress(host, port), maxInboundMessageSize 8 MiB, maxInboundMetadataSize 1 MiB, no keepalive, idle timeout or retry, plaintext; grpc-java's default event loop group, sized to AK_WORKERS through io.grpc.netty.shaded.io.netty.eventLoopThreads, and its default cached call executor), the core cells Bf, Cf, Ef the core's current client configuration (ak_client_new with no options: tonic's defaults, nodelay on), the server's TCP listener (pinned server configuration, TCP_NODELAY on accept); Nagle read back off on every live client socket before timing (a socket with Nagle on refuses the fork), stock h2 everywhere plus Cf-retain on h2-batch for c and d at k = 1 and 8 (labelled h2), the default allocator, plus the pinned allocator pass on A and Cf-retain for c and d at k = 1 (labelled alloc); one fork per (cell, combination) even under smoke; extras left out under core: cells B, C, D, E, F, Cc, the drop mode, the non-framed reference rows, direction a, k = 16, P5.3, d at 4 MiB, the no-unknown build, the shipped and pinned transport configurations (kept under full), h2-batch on other rows, the pinned allocator beyond its subset"
AK_CAMPAIGN_ALLOC=${AK_CAMPAIGN_ALLOC:-default}
case "$AK_CAMPAIGN_ALLOC" in
  default) ALLOC_TUNABLES= ;;
  pinned) ALLOC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432 ;;
  *) echo "AK_CAMPAIGN_ALLOC must be default or pinned"; exit 2 ;;
esac
# AK_ALLOC_PLANT=1 (a control): GLIBC_TUNABLES is left out of the measured processes while
# AK_CAMPAIGN_ALLOC=pinned, so every alloc check must refuse and the run produce no sample.
[ "${AK_ALLOC_PLANT:-0}" = 1 ] && ALLOC_TUNABLES=
# The header's readback: one JVM on the given shim, in the measured processes' environment.
alloc_readback() {  # $1 = shim (libakjni.so)
  GLIBC_TUNABLES=$ALLOC_TUNABLES "$J17/bin/java" -cp "build/cls17:$CP" -Dak.lib="$1" \
    -Dak.camp.alloc="$AK_CAMPAIGN_ALLOC" ak.CampaignAlloc 2>&1 | grep -m1 'ALLOC-CHECK'
}
SERVE="$TOP/ffi/poc/rust/serve.sh"
export AK_SERVE_STATE="$SOCKDIR/serve.state"
if [ "${AK_CAMPAIGN_NO_BUILD:-0}" != 1 ] || [ ! -x "$TOP/ffi/poc/rust/target-server/release/rpc_server" ]; then
  bash "$SERVE" build >> "$OUT/build-$COMMIT.log" 2>&1 || { echo "SERVER BUILD FAILED: $OUT/build-$COMMIT.log"; exit 1; }
fi
SS=""; SP=""; ST=""
serve_start() {  # $1 = DIR for its log, $2 = the server's cpu list (empty: unpinned); sets SS, SP
  local o
  # D10: the server's TCP listener too (AK_SERVER_TCP=0: any free port on 127.0.0.1, the
  # pinned server configuration, TCP_NODELAY on accept); every cell dials it: ST.
  o=$(AK_SERVER_TCP=0 AK_SERVER_THREADS="$AK_WORKERS" AK_CPU_SERVER="${2:-}" bash "$SERVE" start --out "$1") || return 1
  SS=$(echo "$o" | sed -n 's/^shipped //p'); SP=$(echo "$o" | sed -n 's/^pinned //p')
  ST=tcp:$(echo "$o" | sed -n 's/^tcp //p')
  [ -S "$SS" ] && [ -S "$SP" ] && [ "$ST" != tcp: ]
}
serve_stop() { bash "$SERVE" stop > /dev/null 2>&1 || true; }
trap 'serve_stop; rm -rf "$SOCKDIR"' EXIT
CP=$(cat deps/cp.txt)
export AK_CODECGEN=${AK_CODECGEN:-$HERE/build/snap/ffi/poc/codec/gen}

# Fixed JVM settings for every timed JVM (req 6, 25): heap fixed, the default collector
# (G1) on, tiered compilation on, nothing else.
# The heap is 4 GB in the campaign; a smoke run in a shared container may lower it
# (AK_SMOKE_HEAP, default 2g: a client and a server JVM of 4 GB each were killed by the
# container's memory limit while other slices ran), and the header says which it used.
HEAP=4g; [ "$SMOKE" = 1 ] && HEAP=${AK_SMOKE_HEAP:-2g}
JVM_FLAGS="-Xms$HEAP -Xmx$HEAP -XX:+UseG1GC -Xss8m -XX:ParallelGCThreads=$AK_WORKERS"
# The RPC client JVMs: the task-clock agent (req 21 as amended), the pools (D14), the CLIENT
# CPUs for the softirq record, the h2 variant label.
RPC_FLAGS="-agentpath:$HERE/build/taskclock/libaktc.so -Dak.taskclock.lib=$HERE/build/taskclock/libaktc.so -Dak.workers=$AK_WORKERS -Dio.grpc.netty.shaded.io.netty.eventLoopThreads=$AK_WORKERS -Dak.camp.clientcpus=${AK_CPU_CLIENT:-} -Dak.camp.h2=$AK_H2"

sysf() { cat "$1" 2>/dev/null | head -1 || echo "n/a"; }
header() {  # $1 = file, $2 = suite description
  local f=$1
  {
    echo "# campaign log, java slice (design/CAMPAIGN.md W11 draft)  suite=$SUITE  $2"
    echo "# commit $COMMIT   (dirty tree refused unless AK_CAMPAIGN_ALLOW_DIRTY=1)"
    [ "$SMOKE" = 1 ] && echo "# SMOKE RUN (req 32): 1 launch, 1 round, reduced iterations. EVERY FIGURE BELOW IS CONTAINER INSTRUMENTATION, NOT A RESULT."
    echo "# machine: cpu=\"$(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //')\" nproc=$(nproc) smt=$(sysf /sys/devices/system/cpu/smt/active)"
    echo "#   governor=$(sysf /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor) no_turbo=$(sysf /sys/devices/system/cpu/intel_pstate/no_turbo) boost=$(sysf /sys/devices/system/cpu/cpufreq/boost) kernel=$(uname -r)"
    echo "#   isolation=\"${AK_ISOLATION:-not stated}\" isolated_cpus=$(sysf /sys/devices/system/cpu/isolated) numa_nodes=$(ls -d /sys/devices/system/node/node* 2>/dev/null | wc -l)"
    echo "#   AK_CPU_CLIENT=${AK_CPU_CLIENT:-unset} AK_CPU_SERVER=${AK_CPU_SERVER:-unset} (taskset; OS = the rest)"
    echo "# grid (CAMPAIGN 4.0, D18): AK_CAMPAIGN_GRID=$AK_CAMPAIGN_GRID (core: the campaign grid; full: 4.1 and 4.2 with every extra)"
    echo "# h2 variant of the RPC cores (D11): $AK_H2 (the core grid's RPC suite runs stock and, for its labelled rows, h2-batch; each RPC file names its own); pools AK_WORKERS=$AK_WORKERS (D14); allocator AK_CAMPAIGN_ALLOC=$AK_CAMPAIGN_ALLOC${AK_ALLOC_PLANT:+ (PLANT: GLIBC_TUNABLES withheld)}"
    echo "# runtime: target $("$J17/bin/java" -version 2>&1 | head -1)   floor (gated only) $("$J8/bin/java" -version 2>&1 | head -1)"
    echo "# incumbent: $(echo "$CP" | tr ':' '\n' | grep -oE 'protobuf-java-[0-9.]+\.jar|grpc-(api|netty-shaded|protobuf)-[0-9.]+\.jar' | sort -u | tr '\n' ' ')"
    echo "# build: $(cat build/core-rev.txt 2>/dev/null | sed 's/^ *//')"
    echo "#   core cargo profile release, features $(grep -ho ',"features":"\[[^]]*\]' core-build/current/target/release/.fingerprint/ak-core-*/lib-ak_core.json 2>/dev/null | sort -u | tr -d '\\' | sed 's/^,"features":"//' | tr '\n' ' ') (shared library, cdylib); shim gcc -O2 -std=c11"
    echo "#   JVM: $JVM_FLAGS (tiered JIT on, default thresholds)"
    echo "# worker threads (req 4, R-H34), as the pinned JVM sizes them: $($PIN_C "$J17/bin/java" $JVM_FLAGS -XX:+PrintFlagsFinal -version 2>/dev/null | awk '$2 ~ /^(ParallelGCThreads|ConcGCThreads|CICompilerCount)$/ {printf "%s=%s ", $2, $4}')(JVM); codec: 1 benchmark thread (JMH); rpc: Netty event loops, core runtime workers and grpc-java's executor in each rpc log's meta line and the server's THREADS line"
    echo "# order (req 22): codec -- JMH runs cells in the order given, cannot randomise across forks; arm order rotated one step per launch inside each (payload, content, dir) block; rpc -- JMH forks one JVM per cell, cells rotated one step per launch, the 17 combinations cycled through JMH iterations inside each fork, start rotated per launch; the two builds alternate by launch"
    echo "# ratios (req 30, R-H24): formed from per-launch medians; the codec suite forks per cell, so every ratio is cross-process"
    echo "# repeats: $LAUNCHES launch(es) x $ROUNDS round(s) per process (req 23)"
  } > "$f"
}

# The gate stamp is keyed on what the build is made of (the trees of poc/java, poc/codec,
# schema and corpus at HEAD, plus the dirty marker), not on HEAD itself: other slices commit
# on the same branch, and a commit elsewhere must not force a re-gate, while any change to
# these four trees must.
GKEY=$(cd "$TOP" && for d in ffi/poc/java ffi/poc/codec ffi/schema ffi/corpus; do git rev-parse "HEAD:$d"; done \
       | sha256sum | cut -c1-16)${DIRTY:+-dirty}${H2S}
gate_ok() { [ -f "$OUT/gate-$GKEY.ok" ]; }

run_gate() {
  local f="$OUT/gate-$GKEY.log"
  header "$f" "gate key $GKEY; correctness gate: payload set on arms a/b/c, full corpus on 8 and 17 with controls, crossing counts against the committed reference"
  local rc=0
  { echo "## gen/gate.sh"; bash gen/gate.sh; } >> "$f" 2>&1 || rc=1
  { echo; echo "## gen/corpus.sh"; bash gen/corpus.sh; } >> "$f" 2>&1 || rc=1
  # req 19: the crossing counts are machine-independent and gate the run
  "$J17/bin/java" -cp "build/cls17:$CP" -Dak.lib="$HERE/build/jnicnt/libakjni.so" ak.RunCounts \
    2>/dev/null | grep -E '^(P[0-9]|EP )' > "$OUT/counts-$COMMIT.txt" || true
  if diff -u gen/campaign/counts.ref "$OUT/counts-$COMMIT.txt" > "$OUT/counts-$COMMIT.diff"; then
    echo "## crossing counts: identical to gen/campaign/counts.ref ($(wc -l < gen/campaign/counts.ref) rows)" >> "$f"
  else
    echo "## crossing counts DIFFER from gen/campaign/counts.ref (req 19): see counts-$COMMIT.diff" >> "$f"; rc=1
  fi
  # WP5 step 10: the NO-UNKNOWN build, gated on its own (payload set, corpus, its own counts).
  { echo; echo "## gen/gate.sh, no-unknown build"; AK_VARIANT=nounk bash gen/gate.sh; } >> "$f" 2>&1 || rc=1
  { echo; echo "## gen/corpus.sh, no-unknown build"; AK_VARIANT=nounk bash gen/corpus.sh; } >> "$f" 2>&1 || rc=1
  "$J17/bin/java" -cp "build/cls17-nounk:$CP" -Dak.lib="$HERE/build/jnicnt-nounk/libakjni.so" ak.RunCounts \
    2>/dev/null | grep -E '^(P[0-9]|EP )' > "$OUT/counts-nounk-$COMMIT.txt" || true
  if diff -u gen/campaign/counts-nounk.ref "$OUT/counts-nounk-$COMMIT.txt" > "$OUT/counts-nounk-$COMMIT.diff"; then
    echo "## crossing counts, no-unknown build: identical to gen/campaign/counts-nounk.ref ($(wc -l < gen/campaign/counts-nounk.ref) rows)" >> "$f"
  else
    echo "## crossing counts, no-unknown build, DIFFER from gen/campaign/counts-nounk.ref (req 19): see counts-nounk-$COMMIT.diff" >> "$f"; rc=1
  fi
  # Req 19 (R-H31): the RPC cells B, C, D and E, crossings per call, both builds, against
  # one server (the pinned socket), from the counting core and the counting shim. The server
  # is the shared Rust one (poc/rust/serve.sh, SERVER.md), unpinned: the gate checks
  # correctness, and campaign.machine's CPU lists need not exist where the gate runs.
  mkdir -p "$OUT/gate-rpc-server"
  if serve_start "$OUT/gate-rpc-server" ""; then
    echo "## rpc server: the Rust slice's rpc_server via poc/rust/serve.sh (poc/rust at $(cd "$TOP" && git rev-parse --short HEAD:ffi/poc/rust)); $(head -2 "$OUT/gate-rpc-server/rpc-server.log" | tr '\n' ' ')" >> "$f"
  else
    echo "## rpc counts: the server did not start (gate-rpc-server/rpc-server.log)" >> "$f"; rc=1
  fi
  local ss=$ST sp=$ST     # D10: shipped and pinned are the client's configuration, one TCP port
  { echo "## h2 variant: $AK_H2; the RPC cores of this run compiled:"; grep -E "^target-rpc(-count)?${H2S}(-nounk)? " build/h2-compiled.txt | sed 's/^/   /'; } >> "$f"
  local want=h2-0.4.19; [ "$AK_H2" = h2-batch ] && want=h2-batch-src
  if grep -E "^target-rpc(-count)?${H2S}(-nounk)? " build/h2-compiled.txt | grep -qv "$want/src/codec/framed_write"; then
    echo "## h2 variant check FAILED: an RPC core of variant $AK_H2 does not compile $want" >> "$f"; rc=1
  fi
  for v in "" -nounk; do
    "$J17/bin/java" -cp "build/cls17$v:$CP" -Dak.lib="$HERE/build/jnirpccnt$H2S$v/libakjni.so" \
      -Dak.rpclib="$HERE/build/jnirpccnt$H2S$v/libakjni.so" -Dak.camp.count=1 -Dak.camp.transport=pinned \
      -Dak.camp.socket="$sp" ak.CampaignRpc 2>/dev/null | grep -E '^RPC ' > "$OUT/rpc-counts$v-$COMMIT.txt" || true
    if diff -u gen/campaign/rpc-counts$v.ref "$OUT/rpc-counts$v-$COMMIT.txt" > "$OUT/rpc-counts$v-$COMMIT.diff"; then
      echo "## rpc crossing counts per call${v:+, no-unknown build}: identical to gen/campaign/rpc-counts$v.ref ($(wc -l < gen/campaign/rpc-counts$v.ref) rows)" >> "$f"
    else
      echo "## rpc crossing counts per call${v:+, no-unknown build} DIFFER from gen/campaign/rpc-counts$v.ref (req 19): see rpc-counts$v-$COMMIT.diff" >> "$f"; rc=1
    fi
  done
  # Req 14 (c) and (d) under req 18, both builds, both sockets: every cell's unary uploads
  # accepted and every cell's streamed uploads received with the right count and SHA-256;
  # then the plant (every upload expects one byte more) must abort.
  for v in "" -nounk; do
    # The address is tcp:127.0.0.1:PORT (D10), so the configuration is passed apart from it.
    for tr in shipped pinned; do
      local addr=$ss; [ "$tr" = pinned ] && addr=$sp
      "$J17/bin/java" -Xmx2g -cp "build/cls17$v:$CP" -Dak.lib="$HERE/build/jnirpc$H2S$v/libakjni.so" \
        -Dak.rpclib="$HERE/build/jnirpc$H2S$v/libakjni.so" -Dak.camp.uploadcheck=1 -Dak.camp.transport="$tr" \
        -Dak.camp.socket="$addr" ak.CampaignRpc > "$OUT/upload-check$v-$tr.txt" 2>&1
      if grep -q "UPLOAD CHECK PASSED" "$OUT/upload-check$v-$tr.txt"; then
        echo "## upload check${v:+, no-unknown build}, $tr over $addr: $(grep 'UPLOAD CHECK PASSED' "$OUT/upload-check$v-$tr.txt")" >> "$f"
      else
        echo "## upload check${v:+, no-unknown build}, $tr FAILED: see upload-check$v-$tr.txt" >> "$f"; rc=1
      fi
    done
    if "$J17/bin/java" -Xmx2g -cp "build/cls17$v:$CP" -Dak.lib="$HERE/build/jnirpc$H2S$v/libakjni.so" \
        -Dak.rpclib="$HERE/build/jnirpc$H2S$v/libakjni.so" -Dak.camp.uploadcheck=1 -Dak.camp.plant=1 \
        -Dak.camp.transport=pinned -Dak.camp.socket="$sp" ak.CampaignRpc > "$OUT/upload-plant$v.txt" 2>&1; then
      echo "## upload plant${v:+, no-unknown build}: PASSED -- the upload checks are blind" >> "$f"; rc=1
    else
      echo "## upload plant${v:+, no-unknown build}: aborted as required: $(grep -m1 -o 'req 18: .*' "$OUT/upload-plant$v.txt" | cut -c1-160)" >> "$f"
    fi
  done
  # WP9 item 3: the same plant through the timed harness (JMH, -foe true): the run must fail,
  # JMH must exit non-zero, and its JSON must hold no measurement.
  local JMHCP; JMHCP=$(cat deps/jmh/cp.txt)
  local prc=0
  "$J17/bin/java" -Xmx512m -cp "build/jmh17:build/cls17:$CP:$JMHCP" org.openjdk.jmh.Main 'ak.RpcJmh.batch' \
    -f 1 -foe true -wi 0 -i 17 -r 10ms -p cell=C-retain \
    -jvmArgs "-Xmx2g $RPC_FLAGS -Dak.lib=$HERE/build/jnirpc$H2S/libakjni.so -Dak.rpclib=$HERE/build/jnirpc$H2S/libakjni.so -Dak.camp.socket=$sp -Dak.camp.transport=pinned -Dak.camp.plant=1" \
    -rf json -rff "$OUT/rpc-jmh-plant.json" > "$OUT/rpc-jmh-plant.txt" 2>&1 || prc=$?
  if [ $prc != 0 ] && ! grep -q '"rawData"' "$OUT/rpc-jmh-plant.json" 2>/dev/null; then
    echo "## rpc JMH plant: aborted as required (JMH exit $prc, no measurement): $(grep -m1 -o 'req 18: .*' "$OUT/rpc-jmh-plant.txt" | cut -c1-160)" >> "$f"
  else
    echo "## rpc JMH plant: NOT aborted (JMH exit $prc) -- the timed harness's checks are blind" >> "$f"; rc=1
  fi
  rm -f "$OUT/rpc-jmh-plant.json"
  serve_stop
  { echo "## the two committed references against each other (full -> no-unknown):"
    diff gen/campaign/counts.ref gen/campaign/counts-nounk.ref | sed 's/^/   /' || true; } >> "$f"
  if [ $rc = 0 ]; then echo "GATE PASSED" >> "$f"; echo "commit $COMMIT $(date -u +%FT%TZ)" > "$OUT/gate-$GKEY.ok"
  else echo "GATE FAILED: no timing suite runs at this commit" >> "$f"; fi
  echo "gate: $(tail -1 "$f")  ($f)"
  return $rc
}

if [ "$SUITE" = gate ]; then run_gate; exit $?; fi
gate_ok || run_gate || exit 1
# The core grid's RPC suite also runs h2-batch cores (Cf on c and d): their gate too.
if [ "$SUITE" = rpc ] && [ "$AK_CAMPAIGN_GRID" = core ] && [ -z "$H2S" ] && [ ! -f "$OUT/gate-$GKEY-h2b.ok" ]; then
  AK_H2=h2-batch AK_CAMPAIGN_NO_BUILD=1 bash "$HERE/gen/run_campaign.sh" --suite gate --out "$OUT" || exit 1
fi

JAVA="$J17/bin/java $JVM_FLAGS -cp build/cls17:$CP -Dak.camp.rounds=$ROUNDS"

case "$SUITE" in
codec)
  # Req 22a (owner): the codec suite is timed by JMH. One JMH invocation per (launch, coder
  # state), `-f 1`, so JMH forks one fresh JVM per cell, and the launches are the forks of
  # req 23 (AK_LAUNCHES, default 3); the arm order inside each (payload, content, dir) block
  # is rotated between launches (req 22). SingleShotTime: one JMH iteration = one sample of
  # `iters` operations; warm-up iterations = AK_WARM (5), measurement iterations = rounds.
  # Every raw iteration is exported (gen/jmh_to_jsonl.py); JMH's own JSON is kept beside it.
  JMHCP=$(cat deps/jmh/cp.txt)
  EXTRA="-Dak.camp.poolbytes=$((2 * AK_LLC_BYTES))"; WARM=${AK_WARM:-5}
  [ "$SMOKE" = 1 ] && { EXTRA="-Dak.camp.budget=${AK_SMOKE_BUDGET:-65536} -Dak.camp.maxiters=${AK_SMOKE_MAXITERS:-50} -Dak.camp.poolbytes=${AK_SMOKE_POOLBYTES:-65536}"; WARM=${AK_SMOKE_WARM:-1}; AK_SMOKE_UROWS=${AK_SMOKE_UROWS:-6}; }
  WARM_NOTE="warm-up (req 24): AK_WARM, campaign default 5 JMH warm-up iterations per cell; under smoke AK_SMOKE_WARM, default 1"
  # WP5 step 10: two builds, each its own JMH invocation per launch (the no-unknown build is
  # another class tree and another core; one process cannot hold both), in alternating order
  # by launch. JMH forks one JVM per cell (-f 1), so NO arm shares a process with another:
  # the incumbent arms run in both builds' invocations as a control across them, and ratios
  # are formed from per-launch medians (CAMPAIGN req 30, owner R-H24), cross-process.
  # Order (req 22 as amended, R-H23): JMH runs the cells in the order given and cannot
  # randomise across forks; the arm order inside each (payload, content, dir) block is
  # rotated one step per launch (ak.CampaignCodec), and the two builds alternate by launch.
  codec_run() {  # $1 = launch, $2 = full|nounk
    local l=$1 V=$2 SX= TAG= BUILD=full
    [ "$V" = nounk ] && { SX=-nounk; TAG=-nounk; BUILD=no-unknown; }
    CELLS=$("$J17/bin/java" -cp "build/cls17$SX:$CP" -Dak.camp.launch="$l" -Dak.camp.grid="$AK_CAMPAIGN_GRID" \
            ${AK_SMOKE_UROWS:+-Dak.camp.urows=$AK_SMOKE_UROWS} ${AK_CODEC_PROPS:-} ak.CampaignCodec | tr '\n' ',' | sed 's/,$//')
    local ALLCELLS=$CELLS
    for coder in compact utf16; do
      CELLS=$ALLCELLS
      # Core grid (req 24 as narrowed by 4.0): the second string-coder state on the content-set
      # rows only.
      if [ "$AK_CAMPAIGN_GRID" = core ] && [ $coder = utf16 ]; then
        CELLS=$(echo "$ALLCELLS" | tr ',' '\n' | grep -E '\|(latin1|wide)\|' | paste -sd, || true)
      fi
      [ -n "$CELLS" ] || continue
      f="$OUT/codec$TAG-$coder-launch-$l.jsonl"; base="$OUT/codec$TAG-$coder-launch-$l"
      header "$f" "engine=JMH 1.37 SingleShotTime, -f 1 per cell, warm-up $WARM iteration(s) + $ROUNDS measurement iteration(s) per cell, build=$V coder=$coder launch=$l, $(echo "$CELLS" | tr ',' '\n' | wc -l) cells"
      echo "# $WARM_NOTE" >> "$f"
      echo "# $GRID_NOTE_CODEC" >> "$f"
      CF=""; [ "$coder" = utf16 ] && CF="-XX:-CompactStrings"
      echo "# command: $PIN_C java org.openjdk.jmh.Main ak.CodecJmh.sample -f 1 -wi $WARM -i $ROUNDS -foe true -jvmArgs '$JVM_FLAGS $CF ...'" >> "$f"
      echo "# cpu_ns: the process CPU clock (CLOCK_PROCESS_CPUTIME_ID) read inside the benchmark method, exported by JMH as an @AuxCounters counter per iteration; wall_ns: JMH's raw per-iteration time; jit_ms_during: the JVM's JIT compile time between the iteration's setup and teardown (an @AuxCounters counter)" >> "$f"
      echo "# allocator (D9, req 25 as amended): AK_CAMPAIGN_ALLOC=$AK_CAMPAIGN_ALLOC, GLIBC_TUNABLES=${ALLOC_TUNABLES:-unset}; readback: $(alloc_readback "$HERE/build/jni$SX/libakjni.so"); every fork repeats the check once before timing; no heap pre-grow (JMH's warm-up, minflt per sample); minflt on every sample: the process's minor faults over the measured span (getrusage)" >> "$f"
      GLIBC_TUNABLES=$ALLOC_TUNABLES $PIN_C "$J17/bin/java" -cp "build/jmh17$SX:build/cls17$SX:$CP:$JMHCP" org.openjdk.jmh.Main 'ak.CodecJmh.sample' \
        -f 1 -wi "$WARM" -i "$ROUNDS" -foe true -p cell="$CELLS" \
        -jvmArgs "$JVM_FLAGS $CF -Dak.camp.alloc=$AK_CAMPAIGN_ALLOC -Dak.lib=$HERE/build/jni$SX/libakjni.so $EXTRA" \
        -rf json -rff "$base.jmh.json" > "$base.jmh.txt" 2>&1 \
        || { echo "codec$TAG launch $l ($coder) FAILED (JMH, -foe true); no figure: $base.jmh.txt"; exit 1; }
      python3 -S gen/jmh_to_jsonl.py "$base.jmh.json" "$l" "$coder" "$BUILD" "$AK_CAMPAIGN_ALLOC" >> "$f" \
        || { echo "codec$TAG launch $l ($coder): conversion FAILED; no figure"; exit 1; }
      echo "codec$TAG launch $l ($coder): $(grep -c '"cpu_ns"' "$f") samples -> $f"
    done
    # Core grid (4.0): the corpus-core U-* invocation is an extra.
    [ "$AK_CAMPAIGN_GRID" = core ] && return 0
    # Req 7 (amended): every corpus U-* row of class unknown, not disputed, whose root the
    # slice implements, through the corpus description and the corpus core's shim; compact
    # strings only (the rows exercise unknown-field handling, not the String coder).
    UCELLS=$("$J17/bin/java" -cp "build/cls17$SX:$CP" -Dak.camp.launch="$l" -Dak.camp.unknown=1 \
             ${AK_SMOKE_UROWS:+-Dak.camp.urows=$AK_SMOKE_UROWS} ak.CampaignCodec | tr '\n' ',' | sed 's/,$//')
    f="$OUT/codec$TAG-unknown-launch-$l.jsonl"; base="$OUT/codec$TAG-unknown-launch-$l"
    header "$f" "engine=JMH 1.37 SingleShotTime, -f 1 per cell, warm-up $WARM + $ROUNDS iteration(s), corpus U-* rows (req 7), build=$V coder=compact launch=$l, $(echo "$UCELLS" | tr ',' '\n' | wc -l) cells"
    echo "# allocator (D9, req 25 as amended): AK_CAMPAIGN_ALLOC=$AK_CAMPAIGN_ALLOC, GLIBC_TUNABLES=${ALLOC_TUNABLES:-unset}; readback: $(alloc_readback "$HERE/build/jnicorpus$SX/libakjni.so"); every fork repeats the check once before timing; no heap pre-grow (JMH's warm-up, minflt per sample); minflt on every sample: the process's minor faults over the measured span (getrusage)" >> "$f"
    GLIBC_TUNABLES=$ALLOC_TUNABLES $PIN_C "$J17/bin/java" -cp "build/jmh17$SX:build/cls17$SX:$CP:$JMHCP" org.openjdk.jmh.Main 'ak.CodecJmh.sample' \
      -f 1 -wi "$WARM" -i "$ROUNDS" -foe true -p cell="$UCELLS" \
      -jvmArgs "$JVM_FLAGS -Dak.camp.alloc=$AK_CAMPAIGN_ALLOC -Dak.lib=$HERE/build/jnicorpus$SX/libakjni.so $EXTRA" \
      -rf json -rff "$base.jmh.json" > "$base.jmh.txt" 2>&1 \
      || { echo "codec$TAG U-rows launch $l FAILED (JMH, -foe true); no figure: $base.jmh.txt"; exit 1; }
    python3 -S gen/jmh_to_jsonl.py "$base.jmh.json" "$l" compact "$BUILD" "$AK_CAMPAIGN_ALLOC" >> "$f" \
      || { echo "codec$TAG U-rows launch $l: conversion FAILED; no figure"; exit 1; }
    echo "codec$TAG U-rows launch $l: $(grep -c '"cpu_ns"' "$f") samples -> $f"
  }
  for l in $(seq 1 "$LAUNCHES"); do
    if [ $((l % 2)) = 1 ]; then codec_run "$l" full; codec_run "$l" nounk
    else codec_run "$l" nounk; codec_run "$l" full; fi
  done ;;
rpc)
  # Req 22a as amended (owner 2026-09-27, FIX-PLAN WP9): the RPC grid is timed by JMH
  # (ak.RpcJmh), as the codec suite is. Per launch: ONE server process (req 13), warmed
  # through both client transports (req 24), then one JMH invocation per (transport, build),
  # `-f 1`, so JMH forks one JVM per cell, which opens that cell's one channel or client in
  # its trial setup and runs the 17 (direction, payload, in-flight) combinations as JMH
  # iterations in a fixed cycle (see ak.RpcJmh for the grouping and every custom piece).
  JMHCP=$(cat deps/jmh/cp.txt)
  NCOMBO=17
  # Req 22a (owner, e6c909630): the campaign runs JMH's own isolation, one fork per (cell,
  # combination) through the `combo` @Param. AK_RPC_GROUP=1 (the default under smoke only)
  # groups every combination of a cell in one fork, cycled through JMH iterations: allowed
  # for smoke and small exploration runs, stated in each header.
  GROUP=${AK_RPC_GROUP:-0}; [ "$SMOKE" = 1 ] && GROUP=${AK_RPC_GROUP:-1}
  # Req 24 as amended (8c02e7c58): every calling thread gets at least 20 calls at the cell's
  # payload before the first measured value, in the same process on the same threads. One JMH
  # batch is one call on each of the k calling threads (JMH's benchmark thread and k-1
  # persistent helpers, the same threads in warm-up and measurement), so calls per thread =
  # warm-up batches. Warm-up time per direction (owner): a, a+read, b AK_RPC_WARM_TIME_DOWN
  # (2s) and c, d AK_RPC_WARM_TIME_UP (6s), AK_WARM (2) iterations each. Sized from the slowest
  # batch seen in the container smoke (logs/java/campaign-d9, instrumentation): about 80 ms at
  # k=16 on a, a+read, b and about 260 ms on d/16MiB at k=8, so 4 s and 12 s give about 50
  # and 46 batches per thread, twice the rule. Each fork prints its warm-up batches
  # (RPCJMH-WARM), and each meta line carries them (warmup_calls_per_thread).
  WARM=${AK_WARM:-2}; RTIME=${AK_RPC_ITER_TIME:-1s}
  WTIME_DOWN=${AK_RPC_WARM_TIME_DOWN:-${AK_RPC_WARM_TIME:-2s}}; WTIME_UP=${AK_RPC_WARM_TIME_UP:-${AK_RPC_WARM_TIME:-6s}}
  SWARM=${AK_RPC_SERVER_WARM:-200}
  if [ "$SMOKE" = 1 ]; then
    WARM=${AK_SMOKE_WARM:-1}; WTIME_DOWN=${AK_SMOKE_RPC_WARM_TIME:-20ms}; WTIME_UP=$WTIME_DOWN; RTIME=${AK_SMOKE_RPC_ITER_TIME:-20ms}
    SWARM=${AK_SMOKE_RPC_SERVER_WARM:-20}
  fi
  WARM_NOTE="warm-up (req 24): the server, poc/rust/serve.sh warm AK_RPC_SERVER_WARM (campaign default 200, smoke AK_SMOKE_RPC_SERVER_WARM, default 20): on each socket N checked Fetch, Push and Upload calls and ceil(N/4) UploadStream (4 MiB) calls from a tonic client and from the core's client (SERVER.md), before JMH; each client fork, JMH's warm-up: AK_WARM x $NCOMBO iterations (campaign default 2, smoke AK_SMOKE_WARM, default 1), i.e. every combination AK_WARM times, of AK_RPC_WARM_TIME_DOWN for a, a+read, b (campaign default 2s) and AK_RPC_WARM_TIME_UP for c, d (campaign default 6s), smoke 20ms for both; req 24 as amended (8c02e7c58): at least 20 calls per calling thread at the cell's payload before the first measured value, same process, same threads -- one batch is one call on each of the k calling threads, the same threads in warm-up and measurement, so calls per thread = warm-up batches, which each fork prints (RPCJMH-WARM) and each meta line carries (warmup_calls_per_thread); the defaults give about 50 (a, a+read, b) and 46 (c, d) batches against the slowest batches seen in the container smoke (about 80 ms at k=16, 260 ms on d/16MiB at k=8), twice the rule; grouped smoke runs do not meet it and are not campaign figures; JMH's measurement: AK_ROUNDS x $NCOMBO iterations of AK_RPC_ITER_TIME (campaign default 1s, smoke 20ms); GC and JIT state between iterations are JMH's defaults (no forced GC, tiered JIT)"
  # Req 18, 22a: on any failure the launch's output is discarded, not kept beside a later one.
  discard() {  # $1 = launch, $2 = why
    local l=$1
    { echo "rpc launch $l FAILED: $2"; echo "the launch's samples were discarded (req 18, 22a); error lines from JMH:"
      cat "$OUT"/rpc-*-launch-$l.jmh.txt 2>/dev/null | grep -E 'Exception|Error|req 18|<failure>|FAIL' | head -40; } \
      > "$OUT/rpc-launch-$l.FAILED.txt"
    rm -f "$OUT"/rpc-*-launch-$l.jsonl "$OUT"/rpc-*-launch-$l.jmh.json "$OUT"/rpc-*-launch-$l.jmh.txt
    serve_stop
    echo "rpc launch $l FAILED ($2); no figure: $OUT/rpc-launch-$l.FAILED.txt"; exit 1
  }
  server_up() {  # $1 = launch: start the shared server (pinned to AK_CPU_SERVER by serve.sh), warm it
    local l=$1
    mkdir -p "$OUT/rpc-server-launch-$l"
    serve_start "$OUT/rpc-server-launch-$l" "${AK_CPU_SERVER:-}" || discard "$l" "the server did not start"
    bash "$SERVE" warm "$SWARM" > "$OUT/rpc-server-warm-launch-$l.txt" 2>&1 \
      || discard "$l" "the server warm-up (serve.sh warm $SWARM) failed"
  }
  rpc_run() {  # $1 = launch, $2 = transport, $3 = full|nounk, then optional (the core grid):
    # $4 = h2 (stock|h2-batch), $5 = allocator (default|pinned), $6 = cells (comma list, empty:
    # every cell of the build), $7 = combinations (comma list, empty: all 17), $8 = file label
    local l=$1 tr=$2 V=$3 SX= TAG= RH2=${4:-$AK_H2} RAL=${5:-$AK_CAMPAIGN_ALLOC} RCELLS=${6:-} RCOMBOS=${7:-} LBL=${8:-}
    local RH2S= RTUN=
    [ "$RH2" = h2-batch ] && RH2S=-h2b
    [ "$RAL" = pinned ] && RTUN=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
    [ "${AK_ALLOC_PLANT:-0}" = 1 ] && RTUN=
    [ "$V" = nounk ] && { SX=-nounk; TAG=-nounk; }
    local f="$OUT/rpc-$tr$TAG$RH2S$LBL-launch-$l.jsonl" base="$OUT/rpc-$tr$TAG$RH2S$LBL-launch-$l" sock=$ST
    local CELLS
    CELLS=$("$J17/bin/java" -cp "build/cls17$SX:$CP" -Dak.camp.launch="$l" ak.CampaignRpc --list)
    # The core grid's subset, in the launch's rotated order.
    [ -n "$RCELLS" ] && CELLS=$(echo "$CELLS" | tr ',' '\n' | grep -xF -f <(echo "$RCELLS" | tr ',' '\n') | paste -sd,)
    local PCOMBO=cycle WI=$((WARM * NCOMBO)) MI=$((ROUNDS * NCOMBO)) GNOTE
    GNOTE="grouping (req 22a): AK_RPC_GROUP=1, GROUPED -- one fork per cell runs all $NCOMBO combinations, cycled through JMH iterations (smoke and exploration only, not the campaign's configuration)"
    if [ "$GROUP" != 1 ] || [ -n "$RCOMBOS" ]; then
      PCOMBO=$("$J17/bin/java" -cp "build/cls17$SX:$CP" ak.CampaignRpc --combos); WI=$WARM; MI=$ROUNDS
      [ -n "$RCOMBOS" ] && PCOMBO=$(echo "$PCOMBO" | tr ',' '\n' | grep -xF -f <(echo "$RCOMBOS" | tr ',' '\n') | paste -sd,)
      GNOTE="grouping (req 22a): AK_RPC_GROUP=0, JMH's own isolation -- one fork per (cell, combination), -p combo=$PCOMBO, each fork opening its own channel or client"
    fi
    header "$f" "engine=JMH 1.37 AverageTime, -f 1 per (cell, combination), or per cell when grouped (see the grouping line), transport=$tr build=$V launch=$l, cells $CELLS (A and B the incumbent controls, in forks of their own; Bf, Cf-*, Ef-* the framed twins; Cc-* C on the copy path); combinations: ${RCOMBOS:-all $NCOMBO: directions a, a+read, b at 1/8/16 in flight, c (unary upload P5.3, P5.4) and d (client-streamed upload, 4 MiB and 16 MiB in 2 MiB chunks) at 1/8}"
    echo "# $WARM_NOTE" >> "$f"
    echo "# $GNOTE" >> "$f"
    echo "# $GRID_NOTE_RPC; this file: h2=$RH2 alloc=$RAL cells=$CELLS" >> "$f"
    echo "# command: $PIN_C java org.openjdk.jmh.Main ak.RpcJmh.batch -f 1 -foe true -wi $WI -w <per invocation, below> -i $MI -r $RTIME -p cell=<cells> -p combo=<combos|cycle> -jvmArgs '$JVM_FLAGS ...'" >> "$f"
    echo "# one invocation = one batch of k calls in flight (k = the combination's in-flight level: call 0 on JMH's thread, 1..k-1 on persistent helper threads), counted as k calls (iters); wall_ns: JMH's per-iteration score (ns per invocation) x invocations; cpu_ns (req 21 as amended 2026-10-01): perf task-clock of the WHOLE client process, softirq included, from one inherited counter the JVM agent build/taskclock/libaktc.so opens on the JVM's main thread before the JVM creates its other threads, read around every invocation and summed per iteration; process_cpu_ns: CLOCK_PROCESS_CPUTIME_ID read the same way, beside it; softirq_ticks_client: /proc/stat softirq time on the CLIENT CPUs (AK_CPU_CLIENT=${AK_CPU_CLIENT:-unset: every CPU}) over each iteration, USER_HZ ticks; all JMH @AuxCounters counters; JMH's own summary score averages unlike combinations and is not a figure" >> "$f"
    if [ "$tr" = armonik ]; then
      echo "# transport (CAMPAIGN 4.0 as amended b58543f7b, D10): TCP 127.0.0.1 ($ST); cell A: packages/java's GrpcChannelBuilder.forEndpoint(http://127.0.0.1:PORT).withUnsecureConnection().build(), called directly (build/armonik-client, compiled unchanged from the committed tree): NettyChannelBuilder.forAddress(host, port), maxInboundMessageSize 8 MiB, maxInboundMetadataSize 1 MiB, no keepalive, idle timeout or retry, plaintext, grpc-java's TCP_NODELAY; cells Bf, Cf, Ef: the core's current client configuration (ak_client_new with no options: tonic's defaults, nodelay on); the server's TCP listener: pinned server configuration, TCP_NODELAY on accept; TCP_NODELAY read back with getsockopt on every live client socket to the port in each fork before timing (RPCJMH-CELL), a socket with Nagle on refuses the fork" >> "$f"
      echo "# pools (D8, D14): AK_WORKERS=$AK_WORKERS: the core runtime (ak_runtime_new($AK_WORKERS)); cell A's channel uses grpc-java's default shared event loop group, sized to $AK_WORKERS through -Dio.grpc.netty.shaded.io.netty.eventLoopThreads, and grpc-java's default cached call executor (ArmoniK's builder sets neither); G1's ParallelGCThreads=$AK_WORKERS; ConcGCThreads and CICompilerCount are the JVM's own (header line 'worker threads'); the server's tokio runtime AK_SERVER_THREADS=$AK_WORKERS; no grpc-core in this slice" >> "$f"
    else
    echo "# transport (req 17 as amended, D10): TCP 127.0.0.1 ($ST) for every cell, Nagle off on every client socket (grpc-java: Netty ChannelOption.TCP_NODELAY=true; the core: tonic's default nodelay with no options (shipped), tcp_nagle=0 (pinned)), read back with getsockopt(TCP_NODELAY) on every live socket to the port in each fork before timing (RPCJMH-CELL: sockets with NODELAY / sockets, a fork with fewer refuses to run); the server's TCP listener runs the PINNED server configuration only, so shipped and pinned differ on the CLIENT side only: shipped = grpc-java's / tonic's client defaults, pinned = 4 MiB windows, BDP / adaptive window off, 8 MiB messages" >> "$f"
    echo "# pools (D8, D14): AK_WORKERS=$AK_WORKERS: the core runtime (ak_runtime_new($AK_WORKERS)), the Netty event loop group ($AK_WORKERS), grpc-java's call executor (a fixed pool of $AK_WORKERS), G1's ParallelGCThreads=$AK_WORKERS; ConcGCThreads and CICompilerCount are the JVM's own (header line 'worker threads'); the server's tokio runtime AK_SERVER_THREADS=$AK_WORKERS; no grpc-core in this slice" >> "$f"
    fi
    echo "# h2 (D11 as amended): the RPC cores of this run are the $RH2 variant ($(grep -E "^target-rpc${RH2S}${SX} " build/h2-compiled.txt | cut -d' ' -f2-)); every sample carries h2" >> "$f"
    echo "# order (req 22): JMH runs the (cell, combination) cross product in its own order, cells rotated one step per launch, and cannot randomise across forks; when grouped, inside a fork JMH iteration i runs combination (i + launch - 1) mod $NCOMBO (warm-up and measurement counted separately), so every round visits every combination, interleaved; the two builds alternate by launch" >> "$f"
    echo "# server (req 13 as amended at 9f6d579fa, FIX-PLAN WP10; TCP listener 127.0.0.1, pinned configuration, AK_SERVER_TCP=0): the Rust slice's tonic rpc_server, the one RPC server of every slice, via poc/rust/serve.sh (interface poc/rust/SERVER.md; poc/rust at $(cd "$TOP" && git rev-parse --short HEAD:ffi/poc/rust)$(cd "$TOP" && git status --porcelain -- ffi/poc/rust | grep -qv '^??' && echo ', DIRTY')), ONE process for launch $l pinned to AK_CPU_SERVER=${AK_CPU_SERVER:-unset}, serving every cell of both builds on two Unix sockets: shipped = tonic's server defaults, pinned = 4 MiB stream and connection windows, adaptive window off; receive limit 8 MiB; service armonik.ffi.campaign.v1.Grid (Fetch a: P2.2 pre-serialised once; Push b, Upload c: decoded with prost, empty answer; UploadStream d: every message decoded, the byte count answered); $(head -2 "$OUT/rpc-server-launch-$l/rpc-server.log" | tr '\n' ' ')" >> "$f"
    echo "# delivery (req 16): B, C, E the core's blocking call and, in d, the core's blocking client stream (ak_call_open, ak_call_send / ak_call_send_enc for C, ak_call_recv); A, D, F grpc-java's ClientCalls.blockingUnaryCall (a generated blocking stub's call; packages/java's clients use blocking stubs) and, in d, ClientCalls.asyncClientStreamingCall with a StreamObserver (the async stub's call: client streaming has no blocking stub); Bf, Cf, Ef the same cells on the core's framed send path (ak_client_set_framed), grpc-java has no second send path; C (and Cf) sends its request with ak_call_unary_enc / ak_call_send_enc (the encode context's output moved), Cc-* is C with take() + ak_call_unary (the copy path, labelled extra); D and F hand grpc-java a byte[] (take() / Enc.toBytes()): grpc-java's send path copies every message through an OutputStream into its own buffers, so an owned native buffer (ak_enc_take_owned) would still be copied, through a heap array, and D keeps take()" >> "$f"
    echo "# limits (D44): server 8 MiB receive on both sockets (P5.4 is 4,194,390 B), send unlimited (tonic's default); core client shipped tonic's defaults (4 MiB received, unlimited sent: every response here is below 1 MiB), pinned 8 MiB both ways; grpc-java client defaults (4 MiB inbound, no send limit)" >> "$f"
    echo "# allocator (D9, req 25 as amended 2026-10-03): AK_CAMPAIGN_ALLOC=$RAL for every client fork and sample (alloc) -- default: glibc's default allocator, the MAIN figures (as production); pinned: GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432, the labelled diagnostic; this run: GLIBC_TUNABLES=${RTUN:-unset}; readback: $(AK_CAMPAIGN_ALLOC=$RAL ALLOC_TUNABLES=$RTUN alloc_readback "$HERE/build/jnirpc$RH2S$SX/libakjni.so"); every fork repeats the check once before timing and refuses on a disagreement; no heap pre-grow (reverted, owner 2026-10-03): JMH's warm-up runs the real call path on every thread, and minflt per sample shows whether it sufficed; minflt per sample: the process's minor faults over the measured span (getrusage RUSAGE_SELF around every invocation, summed), faults per call = minflt / iters" >> "$f"
    # Req 24 as amended: one JMH invocation per direction group, each with its own warm-up
    # time (grouped: one invocation, every combination in each fork).
    local parts="all" part combos wt pb
    { [ "$GROUP" != 1 ] || [ -n "$RCOMBOS" ]; } && parts="down up"
    for part in $parts; do
      case $part in
        all) combos=$PCOMBO; wt=$WTIME_DOWN; pb=$base ;;
        down) combos=$(echo "$PCOMBO" | tr ',' '\n' | grep -E '^(a|a\+read|b)/' | paste -sd, || true); wt=$WTIME_DOWN; pb=$OUT/rpc-$tr$TAG$RH2S$LBL-down-launch-$l ;;
        up) combos=$(echo "$PCOMBO" | tr ',' '\n' | grep -E '^(c|d):' | paste -sd, || true); wt=$WTIME_UP; pb=$OUT/rpc-$tr$TAG$RH2S$LBL-up-launch-$l ;;
      esac
      [ -n "$combos" ] || continue
      echo "# invocation $part: -p combo=$combos -wi $WI -w $wt -i $MI -r $RTIME" >> "$f"
      GLIBC_TUNABLES=$RTUN $PIN_C "$J17/bin/java" -Xmx512m -cp "build/jmh17$SX:build/cls17$SX:build/armonik-client:$CP:$JMHCP" org.openjdk.jmh.Main 'ak.RpcJmh.batch' \
        -f 1 -foe true -wi "$WI" -w "$wt" -i "$MI" -r "$RTIME" -p cell="$CELLS" -p combo="$combos" \
        -jvmArgs "$JVM_FLAGS $RPC_FLAGS -Dak.lib=$HERE/build/jnirpc$RH2S$SX/libakjni.so -Dak.rpclib=$HERE/build/jnirpc$RH2S$SX/libakjni.so -Dak.camp.socket=$sock -Dak.camp.transport=$tr -Dak.camp.launch=$l -Dak.camp.alloc=$RAL -Dak.camp.h2=$RH2 ${AK_RPC_PROPS:-}" \
        -rf json -rff "$pb.jmh.json" > "$pb.jmh.txt" 2>&1 || discard "$l" "JMH ($tr, $V, $part), -foe true"
      python3 -S gen/rpc_jmh_to_jsonl.py "$pb.jmh.json" "$pb.jmh.txt" "$l" >> "$f" \
        || discard "$l" "conversion ($tr, $V, $part)"
    done
    echo "rpc launch $l ($tr, $V): $(grep -c '"process_cpu_ns"' "$f") samples -> $f"
  }
  for l in $(seq 1 "$LAUNCHES"); do
    server_up "$l"
    if [ "$AK_CAMPAIGN_GRID" = core ]; then
      # CAMPAIGN section 4.0 (as amended b58543f7b): the `armonik` configuration, full build, retain; the main grid on stock h2 and the
      # default allocator, then Cf on h2-batch for c and d, then the pinned allocator pass.
      rpc_run "$l" armonik full stock default "A,Bf,Cf-retain,Ef-retain" \
        "a+read/1,a+read/8,b/1,b/8,c:1/1,c:1/8,d:1/1,d:1/8" ""
      rpc_run "$l" armonik full h2-batch default "Cf-retain" "c:1/1,c:1/8,d:1/1,d:1/8" ""
      rpc_run "$l" armonik full stock pinned "A,Cf-retain" "c:1/1,d:1/1" "-allocpinned"
      serve_stop
      continue
    fi
    # AK_RPC_TRANSPORTS / AK_RPC_BUILDS (default both): a small exploration run may take one.
    for tr in ${AK_RPC_TRANSPORTS:-shipped pinned}; do
      BS=${AK_RPC_BUILDS:-full nounk}
      [ $((l % 2)) = 0 ] && BS=$(echo "$BS" | tr ' ' '\n' | tac | tr '\n' ' ')
      for V in $BS; do rpc_run "$l" "$tr" "$V"; done
    done
    serve_stop
  done ;;
calib)
  N=${AK_CALIB_ITERS:-20000000}; [ "$SMOKE" = 1 ] && N=${AK_SMOKE_CALIB_ITERS:-200000}
  PERF=""; command -v perf >/dev/null && PERF="perf stat -x, -e cycles,instructions"
  RUST=$HERE/../rust
  for l in $(seq 1 "$LAUNCHES"); do
    f="$OUT/calib-launch-$l.jsonl"
    header "$f" "launch=$l iterations=$N perf=${PERF:-unavailable in this environment}"
    $PIN_C $PERF $JAVA -Dak.lib="$HERE/build/jni/libakjni.so" -Dak.camp.launch="$l" \
      -Dak.camp.calibiters="$N" -Dak.camp.out="$f" ak.CampaignCalib 2>> "$f.perf"
    $PIN_C $PERF "$J17/bin/java" $JVM_FLAGS -cp build/probe -Dak.lib="$HERE/build/probe/libprobe.so" \
      -Dak.camp.calibiters="$N" -Dak.camp.rounds="$ROUNDS" -Dak.camp.launch="$l" CampaignRev \
      >> "$f" 2>> "$f.perf"
    # The Rust slice's own crossing benchmark, in this machine's same pinning (R13).
    ( export CARGO_TARGET_DIR=$HERE/build/rustcal
      cd "$RUST" && cargo build --release --bin bench >/dev/null 2>&1 \
      && AK_BENCH_ONLY=P1.1 $PIN_C $PERF "$CARGO_TARGET_DIR/release/bench" 2>>"$f.perf" \
         | grep -iE 'crossing' | sed 's/^/# rust-bench: /' ) >> "$f" || echo "# rust-bench: not run" >> "$f"
    [ -n "$PERF" ] || echo "# perf: unavailable here, so no cycles/instructions (req 20)" >> "$f"
    echo "calib launch $l -> $f"
  done ;;
esac
