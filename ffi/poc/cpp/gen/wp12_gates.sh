#!/usr/bin/env bash
# FIX-PLAN WP12's done criterion for the cpp slice ("both variants pass both slices' gates"): the full C++
# gates on ONE h2 variant of the core, nothing timed.
#
#   gen/wp12_gates.sh stock|h2-batch OUT_DIR
#
# Run it from a clean checkout made for the purpose (a git worktree): the gates write that checkout's
# ffi/logs/cpp/*.log, and this driver copies them into OUT_DIR. The binaries are the same for both variants;
# only the core differs:
#   stock     every binary loads the core its RUNPATH names (core-build/target-*/release), as committed;
#   h2-batch  the six rpc-feature cores (target-rpc, -rpc-count, -camp, -camp-count, -camp-nounk,
#             -camp-count-nounk) have h2-batch twins (poc/codec/h2-batch/build.sh, gen/h2batch_nounk.sh; the
#             CMake targets core_camp_h2batch / core_camp_nounk_h2batch for the two campaign ones), and every
#             gate loads them through LD_LIBRARY_PATH (AK_CORE_SWAP, gen/core_swap.sh), as the timed drivers do.
#             The codec-only cores have no h2 in their crate graph (rpc is an optional feature): the header
#             shows no h2 compiled into them, and the gates run them unchanged.
#
# Steps (WP12_STEPS, default "build wp5 asan campaign deliv q marker"):
#   build     stock: Google Benchmark v1.8.3 release (as run_campaign.sh builds it), cmake -DAK_RPC=ON into
#             build-campaign (run_campaign.sh's) and into build (wp5_gate.sh's; its checks name build/), every
#             target, serve.sh build; h2-batch: the six twins. Every build under
#             flock $AK_CODEC_LOCK (the Rust agent builds the same core variants in parallel)
#   wp5       gen/wp5_gate.sh build (C++17 target and floor, C++14, C++11, static; corpus; plants;
#             byte audit; boundary; other gates; RPC counts; nounk_gate.sh)
#   asan      gen/d11_asan.sh (ASan + LSan, both builds)
#   campaign  gen/run_campaign.sh --suite gate (Unix sockets: the gate has no TCP target)
#   deliv     gen/deliv_checks.sh over TCP 127.0.0.1 (D10): conformance and pre-check on the core,
#             --semantics 1 (24 checks per build), check-stream of A, A-cb, A-q, D, Cf, Cf-q, Cf-cb at d/4
#             and d/16, k = 1 and 8 (count and SHA-256 per call; Cf-q included since 99c18cd1), c grid
#   q         gen/q_checks.sh (queue cells: semantics, plants, grid, RPC counts; Unix socket)
#   marker    the variant marker: socket writes per d/16MiB call of Cf-retain over TCP at k = 1 and 8,
#             strace -f of one --profile process (gen/strace_threads.py; about 1027 stock, about 73 h2-batch
#             at k = 1); the strace text is not kept
# Every step runs with LD_DEBUG=libs into a scratch directory: gen/core_census.py lists, per program, the
# libak_core.so each process loaded and its sha256, and fails the step's census when a C++ process loaded a
# core of the other variant.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$HERE" || exit 2
V=${1:?usage: gen/wp12_gates.sh stock|h2-batch OUT_DIR}; OUT=${2:?usage: gen/wp12_gates.sh stock|h2-batch OUT_DIR}
case "$V" in stock|h2-batch) ;; *) echo "variant: stock or h2-batch" >&2; exit 2 ;; esac
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
FFI=$(cd "$HERE/../.." && pwd); B=$HERE/build-campaign; CB=$HERE/core-build; L=$FFI/logs/cpp
LOCK=${AK_CODEC_LOCK:-/tmp/claude-0/ak-codec-build.lock}
export AK_CPU_CLIENT=${AK_CPU_CLIENT:-1} AK_CPU_SERVER=${AK_CPU_SERVER:-2,3}
STEPS=${WP12_STEPS:-build wp5 asan campaign deliv q marker}
SCR=$(mktemp -d); trap 'rm -rf "$SCR"' EXIT
RUNLOG=$OUT/runner.log
say() { echo "[$(date -u +%FT%TZ)] $*" | tee -a "$RUNLOG"; }

# the six rpc-feature cores and their h2-batch twins: STOCK_DIR TWIN_DIR FEATURES NOUNK
RPC_CORES="target-rpc:target-rpc-h2batch:rpc:0
target-rpc-count:target-rpc-count-h2batch:rpc,count:0
target-camp:target-camp-h2batch:rpc,init-guard:0
target-camp-count:target-camp-count-h2batch:rpc,count,init-guard:0
target-camp-nounk:target-camp-nounk-h2batch:rpc,init-guard:1
target-camp-count-nounk:target-camp-count-nounk-h2batch:rpc,count,init-guard:1"
SWAP=$OUT/core-swap.txt
stock_dirs() { echo "$RPC_CORES" | while IFS=: read -r s t f n; do echo "$CB/$s/release"; done; }
twin_dirs() { echo "$RPC_CORES" | while IFS=: read -r s t f n; do echo "$CB/$t/release"; done; }
if [ "$V" = h2-batch ]; then
  : > "$SWAP"
  echo "$RPC_CORES" | while IFS=: read -r s t f n; do echo "$CB/$s/release $CB/$t/release" >> "$SWAP"; done
  export AK_CORE_SWAP=$SWAP
  CAMP=$CB/target-camp-h2batch/release; CAMPN=$CB/target-camp-nounk-h2batch/release
  FORBID=$(stock_dirs); OTHER=stock
else
  unset AK_CORE_SWAP; rm -f "$SWAP"
  CAMP=$CB/target-camp/release; CAMPN=$CB/target-camp-nounk/release
  FORBID=$(twin_dirs); OTHER=h2-batch
fi

core_line() {  # core_line DIR: path, sha256, h2 source compiled in, AK_H2_COALESCE present
  local so=$1/libak_core.so h2
  [ -f "$so" ] || { echo "  ${so#$HERE/}: not built"; return; }
  h2=$(strings "$so" | grep -o '[^/ ]*/src/codec/framed_write\.rs' | sort -u | sed 's|/src/codec/framed_write\.rs||' | tr '\n' ' ')
  echo "  ${so#$HERE/} sha256 $(sha256sum "$so" | cut -c1-64) h2: ${h2:-none compiled in}; AK_H2_COALESCE: $(strings "$so" | grep -qx 'AK_H2_COALESCE' && echo present || echo absent)"
}
header() {
  echo "# cpp slice, WP12 gates, variant $V ($(date -u +%FT%TZ))"
  echo "# commit $(git -C "$FFI" rev-parse HEAD)$(git -C "$FFI" status --porcelain -- poc/cpp poc/codec schema corpus | grep -q . && echo ' + UNCOMMITTED CHANGES')"
  echo "# checkout $FFI (a worktree; its poc/codec/Cargo.lock is its own, not the shared tree's)"
  echo "# machine $(uname -srm), $(nproc) vCPU, $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //'): a CONTAINER; nothing here is timed"
  echo "# CPU sets: client $AK_CPU_CLIENT, server $AK_CPU_SERVER"
  echo "# g++: $(g++ --version | head -1); cmake: $(cmake --version | head -1); $(rustc --version); $(cargo --version)"
  echo "# protoc: $(protoc --version); pkg-config protobuf $(pkg-config --modversion protobuf), grpc++ $(pkg-config --modversion grpc++)"
  echo "# apt: $(dpkg-query -W -f '${Package} ${Version}; ' libprotobuf-dev libgrpc++-dev protobuf-compiler protobuf-compiler-grpc 2>/dev/null)"
  echo "# Google Benchmark: $(sed -n 's/.*PACKAGE_VERSION "\([^"]*\)".*/\1/p' "$B"/gbench-v1.8.3-release/lib/cmake/benchmark/benchmarkConfigVersion.cmake 2>/dev/null | head -1) ($B/gbench-v1.8.3-release)"
  echo "# h2-batch.patch sha256 $(sha256sum "$FFI/poc/codec/h2-batch/h2-batch.patch" | cut -c1-64)"
  if [ "$V" = h2-batch ]; then echo "# AK_CORE_SWAP ($SWAP):"; sed 's/^/#   /' "$SWAP"; else echo "# AK_CORE_SWAP unset: every binary loads its RUNPATH core"; fi
  echo "# every core of this build:"
  for d in "$CB"/target*/release; do core_line "$d"; done
}

census() {  # census STEP: the cores the step's processes loaded
  local st=$1 args=() d
  for d in $FORBID; do args+=(--forbid "$d"); done
  for d in ${REQ:-}; do args+=(--require "$d"); done
  python3 gen/core_census.py "$SCR/ld-$st" "${args[@]}" --host rpc_server --host rpc_warm > "$OUT/census-$st.txt" 2>&1
  local rc=$?
  rm -rf "$SCR/ld-$st"
  say "  census $st: $( [ $rc = 0 ] && echo ok || echo FAILED) (census-$st.txt: $(grep -c '^  [^ ]' "$OUT/census-$st.txt") lines)"
  return $rc
}
step() {  # step NAME CMD...: run CMD with the loader's record on, then the census
  local st=$1; shift
  mkdir -p "$SCR/ld-$st"
  say "step $st: $*"
  LD_DEBUG=libs LD_DEBUG_OUTPUT=$SCR/ld-$st/ld "$@"; local rc=$?
  say "step $st: exit $rc"
  census "$st"; local crc=$?
  echo "$st exit=$rc census=$crc" >> "$OUT/steps.txt"
}

gbench() {  # run_campaign.sh's gbench_release, the same pinned commit
  local P=$B/gbench-v1.8.3-release src=$B/gbench-src
  [ -f "$P/lib/cmake/benchmark/benchmarkConfig.cmake" ] && return 0
  rm -rf "$src" "$B/gbench-build"
  GIT_LFS_SKIP_SMUDGE=1 git clone -q --depth 1 --branch v1.8.3 https://github.com/google/benchmark "$src" || return 1
  [ "$(git -C "$src" rev-parse HEAD)" = 344117638c8ff7e239044fd0fa7085839fc03021 ] || return 1
  cmake -S "$src" -B "$B/gbench-build" -DCMAKE_BUILD_TYPE=Release -DBENCHMARK_ENABLE_TESTING=OFF \
    -DBENCHMARK_ENABLE_GTEST_TESTS=OFF -DCMAKE_INSTALL_PREFIX="$P" -DCMAKE_INSTALL_LIBDIR=lib > /dev/null \
    && cmake --build "$B/gbench-build" -j"$(nproc)" > /dev/null && cmake --install "$B/gbench-build" > /dev/null
}

touch "$OUT/steps.txt"   # appended to: a later invocation adds its steps
say "wp12_gates.sh $V, steps: $STEPS"
for st in $STEPS; do
  case $st in
    build)
      if [ "$V" = stock ]; then
        say "build: Google Benchmark"; gbench > "$OUT/build-gbench.log" 2>&1 || { say "FAILED: Google Benchmark"; exit 1; }
        say "build: configure"
        cmake -S . -B "$B" -DAK_RPC=ON -Dbenchmark_DIR="$B/gbench-v1.8.3-release/lib/cmake/benchmark" > "$OUT/build-configure.log" 2>&1 \
          || { tail -20 "$OUT/build-configure.log"; say "FAILED: configure"; exit 1; }
        say "build: every target (flock $LOCK)"
        flock "$LOCK" cmake --build "$B" -j"$(nproc)" > "$SCR/build.log" 2>&1; rc=$?
        grep -E 'error|warning: unused|Error [0-9]|FAILED' "$SCR/build.log" | head -40 > "$OUT/build-errors.log"
        echo "exit $rc; $(grep -c 'Linking' "$SCR/build.log") executables linked; $(grep -c 'Compiling' "$SCR/build.log") crate compilations" >> "$OUT/build-errors.log"
        tail -5 "$SCR/build.log" >> "$OUT/build-errors.log"
        [ $rc = 0 ] || { say "FAILED: build (build-errors.log)"; exit 1; }
        say "build: the wp5_gate build directory ./build (cmake -DAK_RPC=ON, every target; flock $LOCK)"
        { cmake -S . -B build -DAK_RPC=ON && flock "$LOCK" cmake --build build -j"$(nproc)"; } > "$SCR/build-wp5.log" 2>&1; rc=$?
        { grep -E 'error|Error [0-9]|FAILED' "$SCR/build-wp5.log" | head -40; echo "exit $rc; $(grep -c 'Linking' "$SCR/build-wp5.log") executables linked"; tail -3 "$SCR/build-wp5.log"; } > "$OUT/build-wp5-errors.log"
        [ $rc = 0 ] || { say "FAILED: the ./build build (build-wp5-errors.log)"; exit 1; }
        say "build: serve.sh build (flock $LOCK)"
        flock "$LOCK" bash ../rust/serve.sh build > "$OUT/build-server.log" 2>&1 || { say "FAILED: serve.sh build"; exit 1; }
      else
        say "build: the h2-batch twins (flock $LOCK)"
        : > "$OUT/build-h2batch.log"
        flock "$LOCK" cmake --build "$B" --target core_camp_h2batch -j1 >> "$OUT/build-h2batch.log" 2>&1 \
          && flock "$LOCK" cmake --build "$B" --target core_camp_nounk_h2batch -j1 >> "$OUT/build-h2batch.log" 2>&1 \
          || { say "FAILED: the CMake h2-batch targets"; exit 1; }
        while IFS=: read -r s t f n; do
          case $t in target-camp-h2batch|target-camp-nounk-h2batch) continue ;; esac
          echo "== $t ($f, $([ "$n" = 1 ] && echo no-unknown || echo full))" >> "$OUT/build-h2batch.log"
          if [ "$n" = 1 ]; then
            flock "$LOCK" bash gen/h2batch_nounk.sh "$FFI/poc/codec" "$CB/target-camp-h2batch/h2-batch-src" "$CB/$t" "$f" >> "$OUT/build-h2batch.log" 2>&1
          else
            flock "$LOCK" bash "$FFI/poc/codec/h2-batch/build.sh" h2-batch "$CB/$t" "$f" >> "$OUT/build-h2batch.log" 2>&1
          fi || { say "FAILED: $t"; exit 1; }
        done <<< "$RPC_CORES"
        git -C "$FFI" diff --quiet -- poc/codec/Cargo.lock && echo "poc/codec/Cargo.lock unmodified after the builds" >> "$OUT/build-h2batch.log" \
          || { echo "poc/codec/Cargo.lock MODIFIED after the builds" >> "$OUT/build-h2batch.log"; say "FAILED: Cargo.lock left modified"; exit 1; }
      fi
      header > "$OUT/header.txt"
      say "header.txt written" ;;
    wp5)
      REQ=$( [ "$V" = h2-batch ] && echo "$CB/target-rpc-count-h2batch/release" || echo "$CB/target-rpc-count/release")
      step wp5 bash gen/wp5_gate.sh build > "$OUT/wp5-summary.log" 2>&1
      mkdir -p "$OUT/wp5"; cp "$L"/wp5-{build,generator,conformance,corpus,probe,bytes,boundary,gates}.log "$L/wp5s10-nounk.log" "$OUT/wp5/" ;;
    asan)
      REQ=""
      step asan bash gen/d11_asan.sh > "$OUT/asan-summary.log" 2>&1
      cp "$L/asan.log" "$OUT/asan.log" ;;
    campaign)
      REQ="$CAMP $CAMPN"
      rm -rf "$SCR/camp"
      step campaign bash gen/run_campaign.sh --suite gate --out "$SCR/camp" > "$OUT/campaign-summary.log" 2>&1
      mkdir -p "$OUT/campaign"; cp "$SCR/camp/gate.log" "$OUT/campaign/campaign-gate.log"
      for f in counts.log rpc-counts.log rpc-counts_nounk.log gate.ok; do [ -f "$SCR/camp/$f" ] && cp "$SCR/camp/$f" "$OUT/campaign/"; done
      cp "$L/wp5s10-nounk.log" "$OUT/campaign/wp5s10-nounk.log" ;;
    deliv)
      REQ="$CAMP $CAMPN"
      step deliv env AK_NET=tcp bash gen/deliv_checks.sh "$OUT/deliv" "$V" "$CAMP" "$CAMPN" "" > "$OUT/deliv-summary.log" 2>&1 ;;
    q)
      REQ="$CAMP $CAMPN"
      step q bash gen/q_checks.sh "$OUT/q-checks.log" > /dev/null 2>&1 ;;
    marker)
      REQ="$CAMP"
      step marker bash gen/wp12_marker.sh "$CAMP" "$SCR/mk" > "$OUT/marker.log" 2>&1 ;;
    *) say "unknown step $st"; exit 2 ;;
  esac
done
say "done: $(tr '\n' ' ' < "$OUT/steps.txt")"
