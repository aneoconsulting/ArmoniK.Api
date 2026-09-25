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

# ---- req 4: the CPU sets
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
CP=$(cat deps/cp.txt)
export AK_CODECGEN=${AK_CODECGEN:-$HERE/build/snap/ffi/poc/codec/gen}

# Fixed JVM settings for every timed JVM (req 6, 25): heap fixed, the default collector
# (G1) on, tiered compilation on, nothing else.
JVM_FLAGS="-Xms4g -Xmx4g -XX:+UseG1GC -Xss8m"

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
    echo "# runtime: target $("$J17/bin/java" -version 2>&1 | head -1)   floor (gated only) $("$J8/bin/java" -version 2>&1 | head -1)"
    echo "# incumbent: $(echo "$CP" | tr ':' '\n' | grep -oE 'protobuf-java-[0-9.]+\.jar|grpc-(api|netty-shaded|protobuf)-[0-9.]+\.jar' | sort -u | tr '\n' ' ')"
    echo "# build: $(cat build/core-rev.txt 2>/dev/null | sed 's/^ *//')"
    echo "#   core cargo profile release, features $(grep -ho ',"features":"\[[^]]*\]' core-build/current/target/release/.fingerprint/ak-core-*/lib-ak_core.json 2>/dev/null | sort -u | tr -d '\\' | sed 's/^,"features":"//' | tr '\n' ' ') (shared library, cdylib); shim gcc -O2 -std=c11"
    echo "#   JVM: $JVM_FLAGS (tiered JIT on, default thresholds)"
    echo "# repeats: $LAUNCHES launch(es) x $ROUNDS round(s) per process (req 23)"
  } > "$f"
}

# The gate stamp is keyed on what the build is made of (the trees of poc/java, poc/codec,
# schema and corpus at HEAD, plus the dirty marker), not on HEAD itself: other slices commit
# on the same branch, and a commit elsewhere must not force a re-gate, while any change to
# these four trees must.
GKEY=$(cd "$TOP" && for d in ffi/poc/java ffi/poc/codec ffi/schema ffi/corpus; do git rev-parse "HEAD:$d"; done \
       | sha256sum | cut -c1-16)${DIRTY:+-dirty}
gate_ok() { [ -f "$OUT/gate-$GKEY.ok" ]; }

run_gate() {
  local f="$OUT/gate-$GKEY.log"
  header "$f" "gate key $GKEY; correctness gate: payload set on arms a/b/c, full corpus on 8 and 17 with controls, crossing counts against the committed reference"
  local rc=0
  { echo "## gen/gate.sh"; bash gen/gate.sh; } >> "$f" 2>&1 || rc=1
  { echo; echo "## gen/corpus.sh"; bash gen/corpus.sh; } >> "$f" 2>&1 || rc=1
  # req 19: the crossing counts are machine-independent and gate the run
  "$J17/bin/java" -cp "build/cls17:$CP" -Dak.lib="$HERE/build/jnicnt/libakjni.so" ak.RunCounts \
    2>/dev/null | grep -E '^P[0-9]' > "$OUT/counts-$COMMIT.txt" || true
  if diff -u gen/campaign/counts.ref "$OUT/counts-$COMMIT.txt" > "$OUT/counts-$COMMIT.diff"; then
    echo "## crossing counts: identical to gen/campaign/counts.ref ($(wc -l < gen/campaign/counts.ref) rows)" >> "$f"
  else
    echo "## crossing counts DIFFER from gen/campaign/counts.ref (req 19): see counts-$COMMIT.diff" >> "$f"; rc=1
  fi
  # WP5 step 10: the NO-UNKNOWN build, gated on its own (payload set, corpus, its own counts).
  { echo; echo "## gen/gate.sh, no-unknown build"; AK_VARIANT=nounk bash gen/gate.sh; } >> "$f" 2>&1 || rc=1
  { echo; echo "## gen/corpus.sh, no-unknown build"; AK_VARIANT=nounk bash gen/corpus.sh; } >> "$f" 2>&1 || rc=1
  "$J17/bin/java" -cp "build/cls17-nounk:$CP" -Dak.lib="$HERE/build/jnicnt-nounk/libakjni.so" ak.RunCounts \
    2>/dev/null | grep -E '^P[0-9]' > "$OUT/counts-nounk-$COMMIT.txt" || true
  if diff -u gen/campaign/counts-nounk.ref "$OUT/counts-nounk-$COMMIT.txt" > "$OUT/counts-nounk-$COMMIT.diff"; then
    echo "## crossing counts, no-unknown build: identical to gen/campaign/counts-nounk.ref ($(wc -l < gen/campaign/counts-nounk.ref) rows)" >> "$f"
  else
    echo "## crossing counts, no-unknown build, DIFFER from gen/campaign/counts-nounk.ref (req 19): see counts-nounk-$COMMIT.diff" >> "$f"; rc=1
  fi
  { echo "## the two committed references against each other (full -> no-unknown):"
    diff gen/campaign/counts.ref gen/campaign/counts-nounk.ref | sed 's/^/   /' || true; } >> "$f"
  if [ $rc = 0 ]; then echo "GATE PASSED" >> "$f"; echo "commit $COMMIT $(date -u +%FT%TZ)" > "$OUT/gate-$GKEY.ok"
  else echo "GATE FAILED: no timing suite runs at this commit" >> "$f"; fi
  echo "gate: $(tail -1 "$f")  ($f)"
  return $rc
}

if [ "$SUITE" = gate ]; then run_gate; exit $?; fi
gate_ok || run_gate || exit 1

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
  EXTRA=""; WARM=${AK_WARM:-5}
  [ "$SMOKE" = 1 ] && { EXTRA="-Dak.camp.budget=${AK_SMOKE_BUDGET:-65536} -Dak.camp.maxiters=${AK_SMOKE_MAXITERS:-50}"; WARM=1; AK_SMOKE_UROWS=${AK_SMOKE_UROWS:-6}; }
  # WP5 step 10: two builds, each its own JMH invocation per launch (the no-unknown build is
  # another class tree and another core; one process cannot hold both), in alternating order
  # by launch. The incumbent arms run in both, the in-process controls that carry a ratio
  # across (absolutes do not travel between the two builds' processes).
  codec_run() {  # $1 = launch, $2 = full|nounk
    local l=$1 V=$2 SX= TAG= BUILD=full
    [ "$V" = nounk ] && { SX=-nounk; TAG=-nounk; BUILD=no-unknown; }
    CELLS=$("$J17/bin/java" -cp "build/cls17$SX:$CP" -Dak.camp.launch="$l" ${AK_CODEC_PROPS:-} ak.CampaignCodec | tr '\n' ',' | sed 's/,$//')
    for coder in compact utf16; do
      f="$OUT/codec$TAG-$coder-launch-$l.jsonl"; base="$OUT/codec$TAG-$coder-launch-$l"
      header "$f" "engine=JMH 1.37 SingleShotTime, -f 1 per cell, warm-up $WARM iteration(s) + $ROUNDS measurement iteration(s) per cell, build=$V coder=$coder launch=$l, $(echo "$CELLS" | tr ',' '\n' | wc -l) cells"
      CF=""; [ "$coder" = utf16 ] && CF="-XX:-CompactStrings"
      echo "# command: $PIN_C java org.openjdk.jmh.Main ak.CodecJmh.sample -f 1 -wi $WARM -i $ROUNDS -foe true -jvmArgs '$JVM_FLAGS $CF ...'" >> "$f"
      echo "# cpu_ns: the measuring thread's CPU clock read inside the benchmark method; wall_ns: JMH's raw per-iteration time" >> "$f"
      rm -f "$base.cpu.tsv"
      $PIN_C "$J17/bin/java" -cp "build/jmh17$SX:build/cls17$SX:$CP:$JMHCP" org.openjdk.jmh.Main 'ak.CodecJmh.sample' \
        -f 1 -wi "$WARM" -i "$ROUNDS" -foe true -p cell="$CELLS" \
        -jvmArgs "$JVM_FLAGS $CF -Dak.lib=$HERE/build/jni$SX/libakjni.so -Dak.jmh.cpuout=$base.cpu.tsv $EXTRA" \
        -rf json -rff "$base.jmh.json" > "$base.jmh.txt" 2>&1 \
        || { echo "codec$TAG launch $l ($coder) FAILED (JMH, -foe true); no figure: $base.jmh.txt"; exit 1; }
      python3 -S gen/jmh_to_jsonl.py "$base.jmh.json" "$base.cpu.tsv" "$l" "$coder" "$BUILD" >> "$f" \
        || { echo "codec$TAG launch $l ($coder): conversion FAILED; no figure"; exit 1; }
      echo "codec$TAG launch $l ($coder): $(grep -c '"cpu_ns"' "$f") samples -> $f"
    done
    # Req 7 (amended): every corpus U-* row of class unknown, not disputed, whose root the
    # slice implements, through the corpus description and the corpus core's shim; compact
    # strings only (the rows exercise unknown-field handling, not the String coder).
    UCELLS=$("$J17/bin/java" -cp "build/cls17$SX:$CP" -Dak.camp.launch="$l" -Dak.camp.unknown=1 \
             ${AK_SMOKE_UROWS:+-Dak.camp.urows=$AK_SMOKE_UROWS} ak.CampaignCodec | tr '\n' ',' | sed 's/,$//')
    f="$OUT/codec$TAG-unknown-launch-$l.jsonl"; base="$OUT/codec$TAG-unknown-launch-$l"
    header "$f" "engine=JMH 1.37 SingleShotTime, -f 1 per cell, warm-up $WARM + $ROUNDS iteration(s), corpus U-* rows (req 7), build=$V coder=compact launch=$l, $(echo "$UCELLS" | tr ',' '\n' | wc -l) cells"
    rm -f "$base.cpu.tsv"
    $PIN_C "$J17/bin/java" -cp "build/jmh17$SX:build/cls17$SX:$CP:$JMHCP" org.openjdk.jmh.Main 'ak.CodecJmh.sample' \
      -f 1 -wi "$WARM" -i "$ROUNDS" -foe true -p cell="$UCELLS" \
      -jvmArgs "$JVM_FLAGS -Dak.lib=$HERE/build/jnicorpus$SX/libakjni.so -Dak.jmh.cpuout=$base.cpu.tsv $EXTRA" \
      -rf json -rff "$base.jmh.json" > "$base.jmh.txt" 2>&1 \
      || { echo "codec$TAG U-rows launch $l FAILED (JMH, -foe true); no figure: $base.jmh.txt"; exit 1; }
    python3 -S gen/jmh_to_jsonl.py "$base.jmh.json" "$base.cpu.tsv" "$l" compact "$BUILD" >> "$f" \
      || { echo "codec$TAG U-rows launch $l: conversion FAILED; no figure"; exit 1; }
    echo "codec$TAG U-rows launch $l: $(grep -c '"cpu_ns"' "$f") samples -> $f"
  }
  for l in $(seq 1 "$LAUNCHES"); do
    if [ $((l % 2)) = 1 ]; then codec_run "$l" full; codec_run "$l" nounk
    else codec_run "$l" nounk; codec_run "$l" full; fi
  done ;;
rpc)
  EXTRA=""; WARM=${AK_WARM:-2}
  [ "$SMOKE" = 1 ] && { EXTRA="-Dak.camp.calls=${AK_SMOKE_CALLS:-64} -Dak.camp.chunk=16"; WARM=1; }
  rpc_run() {  # $1 = launch, $2 = transport, $3 = full|nounk
    local l=$1 tr=$2 V=$3 SX= TAG= CELLS="A, B, C-retain, C-drop, D-retain, D-drop"
    [ "$V" = nounk ] && { SX=-nounk; TAG=-nounk; CELLS="A, B (in-process controls), C-nounk, D-nounk"; }
    local f="$OUT/rpc-$tr$TAG-launch-$l.jsonl"
    local sock="$HERE/build/campaign-$$-$tr$TAG-$l.sock"
    header "$f" "transport=$tr build=$V launch=$l warm-up=$WARM sample(s) per (dir,inflight,cell) before round 1 (cells $CELLS in ONE client process, server in its own process)"
    echo "# server: $PIN_S java ... ak.CampaignRpc --serve (grpc-java, pre-serialised P2.2; direction b parses with protobuf-java; the full build's classes, no core codec on the server)" >> "$f"
    $PIN_S $JAVA -Dak.camp.transport="$tr" -Dak.lib="$HERE/build/jnirpc/libakjni.so" \
      ak.CampaignRpc --serve "$sock" > "$OUT/rpc-server-$tr$TAG-$l.txt" 2>&1 &
    local spid=$!
    for i in $(seq 1 120); do grep -q SERVING "$OUT/rpc-server-$tr$TAG-$l.txt" 2>/dev/null && break; sleep 0.5; done
    grep -q SERVING "$OUT/rpc-server-$tr$TAG-$l.txt" || { kill $spid; echo "server did not start"; exit 1; }
    local rc=0
    $PIN_C "$J17/bin/java" $JVM_FLAGS -cp "build/cls17$SX:$CP" -Dak.camp.rounds=$ROUNDS \
      -Dak.camp.transport="$tr" -Dak.camp.socket="$sock" -Dak.camp.launch="$l" \
      -Dak.lib="$HERE/build/jnirpc$SX/libakjni.so" -Dak.rpclib="$HERE/build/jnirpc$SX/libakjni.so" \
      -Dak.camp.out="$f" -Dak.camp.warm="$WARM" $EXTRA ${AK_RPC_PROPS:-} ak.CampaignRpc || rc=$?
    kill $spid 2>/dev/null || true; wait $spid 2>/dev/null || true
    rm -f "$sock"
    [ $rc = 0 ] || { echo "rpc launch $l ($tr, $V) FAILED (req 18); no figure"; exit 1; }
    echo "rpc launch $l ($tr, $V): $(grep -c '"cpu_ns"' "$f") samples -> $f"
  }
  for l in $(seq 1 "$LAUNCHES"); do
    for tr in shipped pinned; do
      if [ $((l % 2)) = 1 ]; then rpc_run "$l" "$tr" full; rpc_run "$l" "$tr" nounk
      else rpc_run "$l" "$tr" nounk; rpc_run "$l" "$tr" full; fi
    done
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
