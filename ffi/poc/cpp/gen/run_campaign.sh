#!/usr/bin/env bash
# design/CAMPAIGN.md requirement 31: the cpp slice's campaign runner.
#
#   gen/run_campaign.sh --suite codec|rpc|calib|gate --out <dir>
#
# Environment (requirement 4: the CPU sets are parameters, never constants in a harness):
#   AK_CPU_CLIENT        CPU list for the measured process (codec benchmark, RPC client)  REQUIRED
#   AK_CPU_SERVER        CPU list for the RPC server process                              REQUIRED
#   AK_CAMPAIGN_LAUNCHES process launches per suite            (default 3; requirement 23)
#   AK_CAMPAIGN_ROUNDS   rounds per process                    (default 5; requirement 23)
#   AK_CAMPAIGN_BYTES    codec: payload bytes per sample       (default 33554432)
#   AK_CAMPAIGN_WARMUP   codec: warm-up bytes per arm          (default = AK_CAMPAIGN_BYTES)
#   AK_CAMPAIGN_CALLS    rpc: calls per sample                 (default 480)
#   AK_CAMPAIGN_RPC_WARMUP rpc: warm-up calls per cell         (default 96)
#   AK_CAMPAIGN_SERVER_WARMUP rpc: server warm-up calls per direction per client transport
#                        per socket, before any client            (default 200)
#   AK_CAMPAIGN_CALIB_ITERS calib: crossings per sample        (default 100000000)
#   AK_LLC_BYTES         last-level cache size (default 14417920, the i9-7900X's 13.75 MB)
#   AK_CAMPAIGN_POOL_BYTES codec: encode input pool, encoded bytes (default 2 x AK_LLC_BYTES)
#   AK_CAMPAIGN_ALLOW_DIRTY=1  SMOKE RUNS ONLY: run on a dirty tree; the header says so and
#                        every figure is container instrumentation (requirement 27 refuses it)
#   AK_CAMPAIGN_SMOKE=1  a smoke run on a CLEAN tree (R-H19): the header's "instrumentation"
#                        is true and "smoke" is true; a campaign run leaves both unset
#   AK_INCUMBENT_PREFIX  build against a protobuf/grpc++ installed under this prefix (the
#                        campaign builds twice: gRPC v1.54.0, ArmoniK's, and a current one;
#                        requirement 3). Unset: the system's.
#   BUILD                build directory (default ./build-campaign)
#
# Every suite runs the gate first (requirement 26) unless <out>/gate.ok names this very
# commit and build. Output: <out>/<suite>-launch<N>.jsonl, header lines prefixed "# " and
# one JSON object per sample (requirements 27 to 29). Nothing here summarises; see
# gen/campaign_summary.py (requirement 30).
set -u
cd "$(dirname "$0")/.." || exit 2
SLICE=$PWD
SUITE=""; OUT=""
while [ $# -gt 0 ]; do
  case "$1" in
    --suite) SUITE=$2; shift 2 ;;
    --out) OUT=$2; shift 2 ;;
    *) echo "usage: run_campaign.sh --suite codec|rpc|calib|gate --out <dir>" >&2; exit 2 ;;
  esac
done
case "$SUITE" in codec|rpc|calib|gate) ;; *) echo "--suite codec|rpc|calib|gate" >&2; exit 2 ;; esac
[ -n "$OUT" ] || { echo "--out <dir>" >&2; exit 2; }
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
TMPD=$(mktemp -d)   # scratch files of the gate and the server; never committed
: "${AK_CPU_CLIENT:?AK_CPU_CLIENT is required (requirement 4)}"
: "${AK_CPU_SERVER:?AK_CPU_SERVER is required (requirement 4)}"
LAUNCHES=${AK_CAMPAIGN_LAUNCHES:-3}
ROUNDS=${AK_CAMPAIGN_ROUNDS:-5}
BYTES=${AK_CAMPAIGN_BYTES:-33554432}
WARM=${AK_CAMPAIGN_WARMUP:-$BYTES}
CALLS=${AK_CAMPAIGN_CALLS:-480}
RPCWARM=${AK_CAMPAIGN_RPC_WARMUP:-96}
SRVWARM=${AK_CAMPAIGN_SERVER_WARMUP:-200}
CITERS=${AK_CAMPAIGN_CALIB_ITERS:-100000000}
# req. 11: the encode suite's beyond-cache input pool, in encoded bytes: 2 x the machine's
# last-level cache (AK_LLC_BYTES, default the reference i9-7900X's 13.75 MB), overridable.
LLC=${AK_LLC_BYTES:-14417920}
POOL=${AK_CAMPAIGN_POOL_BYTES:-$((2 * LLC))}
B=${BUILD:-$SLICE/build-campaign}
FFI=$(cd "$SLICE/../.." && pwd)
# The repository whose commit the logs name. AK_REPO: a snapshot run (a `git archive` of a
# commit built out of tree) names the repository it was archived from.
REPO=${AK_REPO:-$(git -C "$FFI" rev-parse --show-toplevel)}

# ---- the CPU sets: disjoint (requirement 4) -----------------------------------------
expand() { python3 -c "
import sys
out=set()
for part in sys.argv[1].split(','):
    if '-' in part: a,b=part.split('-'); out.update(range(int(a),int(b)+1))
    elif part: out.add(int(part))
print(' '.join(map(str,sorted(out))))" "$1"; }
if [ -n "$(comm -12 <(expand "$AK_CPU_CLIENT" | tr ' ' '\n' | sort) <(expand "$AK_CPU_SERVER" | tr ' ' '\n' | sort))" ]; then
  echo "REFUSED: AK_CPU_CLIENT ($AK_CPU_CLIENT) and AK_CPU_SERVER ($AK_CPU_SERVER) overlap" >&2; exit 2
fi

# ---- one NUMA node, no SMT sibling shared between CLIENT and SERVER (requirement 4) ----
TOPO=$(python3 - "$(expand "$AK_CPU_CLIENT")" "$(expand "$AK_CPU_SERVER")" <<'PYTOPO'
import glob, os, sys
cl = [int(x) for x in sys.argv[1].split()]
sv = [int(x) for x in sys.argv[2].split()]
def rng(s):
    out = set()
    for p in s.split(","):
        if "-" in p:
            a, b = p.split("-"); out.update(range(int(a), int(b) + 1))
        elif p:
            out.add(int(p))
    return out
def sib(c):
    try:
        return rng(open("/sys/devices/system/cpu/cpu%d/topology/thread_siblings_list" % c).read().strip())
    except OSError:
        return {c}
def node(c):
    n = glob.glob("/sys/devices/system/cpu/cpu%d/node*" % c)
    return os.path.basename(n[0]) if n else "node?"
bad = []
for c in cl:
    shared = (sib(c) - {c}) & set(sv)
    if shared:
        bad.append("cpu %d (CLIENT) is an SMT sibling of %s (SERVER)" % (c, sorted(shared)))
nodes = sorted(set(node(c) for c in cl + sv))
if len(nodes) > 1:
    bad.append("the sets span NUMA nodes %s" % nodes)
print("; ".join(bad))
PYTOPO
)
if [ -n "$TOPO" ]; then echo "REFUSED (requirement 4): $TOPO" >&2; exit 2; fi

# ---- a dirty tree is refused (requirement 27) ---------------------------------------
DIRTY=$(git -C "$REPO" status --porcelain -- ffi/poc/cpp ffi/poc/codec ffi/schema ffi/corpus 2>/dev/null)
if [ -n "$DIRTY" ] && [ "${AK_CAMPAIGN_ALLOW_DIRTY:-0}" != 1 ]; then
  echo "REFUSED: the tree is dirty (requirement 27):" >&2; echo "$DIRTY" | head -20 >&2; exit 2
fi
# AK_COMMIT: the commit a snapshot run was archived from (else the repository HEAD).
COMMIT=${AK_COMMIT:-$(git -C "$REPO" rev-parse HEAD)}

header() {  # header <suite> <launch> : one JSON line, prefixed "# " (requirement 27)
  python3 - "$1" "$2" <<EOF
import json, os, subprocess, glob, sys
def sh(c):
    try: return subprocess.run(c, shell=True, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True).stdout.strip()
    except Exception: return ""
def rd(p):
    try: return open(p).read().strip()
    except Exception: return None
gov = sorted(set(filter(None, (rd(p) for p in glob.glob('/sys/devices/system/cpu/cpu*/cpufreq/scaling_governor')))))
cmd = rd('/proc/cmdline') or ''
h = {
 "slice": "cpp", "suite": sys.argv[1], "launch": int(sys.argv[2]),
 "commit": "$COMMIT", "dirty": bool("""$DIRTY"""), "dirty_files": """$DIRTY""".splitlines()[:20],
 "instrumentation": bool("""$DIRTY""") or os.environ.get("AK_CAMPAIGN_ALLOW_DIRTY") == "1"
                    or os.environ.get("AK_CAMPAIGN_SMOKE") == "1",
 "smoke": os.environ.get("AK_CAMPAIGN_SMOKE") == "1",
 "machine": {
   "cpu_model": sh("grep -m1 'model name' /proc/cpuinfo | cut -d: -f2-").strip(),
   "cpus_online": rd('/sys/devices/system/cpu/online'), "nproc": sh("nproc"),
   "smt_active": rd('/sys/devices/system/cpu/smt/active'),
   "governor": gov or "unavailable",
   "no_turbo": rd('/sys/devices/system/cpu/intel_pstate/no_turbo'),
   "boost": rd('/sys/devices/system/cpu/cpufreq/boost'),
   "kernel": sh("uname -srm"),
   "isolated": rd('/sys/devices/system/cpu/isolated'),
   "isolation_cmdline": [t for t in cmd.split() if t.split('=')[0] in ('isolcpus','nohz_full','rcu_nocbs','irqaffinity')],
   "runner_cpuset": rd('/proc/self/status') and [l for l in open('/proc/self/status') if l.startswith('Cpus_allowed_list')][0].split(':')[1].strip(),
   "cpu_client": os.environ.get("AK_CPU_CLIENT"), "cpu_server": os.environ.get("AK_CPU_SERVER"),
 },
 "versions": {
   "cxx": sh("g++ --version | head -1"), "protoc": sh("protoc --version"),
   "protobuf_pkgconfig": sh("pkg-config --modversion protobuf"), "grpcpp": sh("pkg-config --modversion grpc++"),
   "rustc": sh("rustc --version"), "incumbent_prefix": os.environ.get("AK_INCUMBENT_PREFIX", "system"),
 },
 "build": {"dir": "$B", "cxx_flags": sh("grep -h '^CXX_FLAGS' $B/CMakeFiles/campaign_codec.dir/flags.make | cut -d= -f2-"),
           "std": "c++17 (target)", "linkage": "shared (libak_core.so)", "lto": "off",
           "core_features": "init-guard" + (",rpc" if sys.argv[1] in ("rpc", "calib") else ""),
           "core_profile": "cargo --release"},
 "transport": {"shipped": "grpc++: keepalive 30 s, max idle 5 min, local subchannel pool (packages/cpp getChannelArguments; its retry service config not applied); core: ak_client_new defaults",
               "pinned": "grpc++: 4 MiB stream window, BDP off (no connection-window argument exists, C31); core: 4 MiB stream and connection windows, adaptive off, Nagle off; both: 2 MiB max message"},
 "variants": {"full": "ak-core default features (unknown-fields: decision 11), binaries campaign_codec / campaign_rpc: core-ffi drop and retain, C/D-retain and -drop",
              "no-unknown": "ak-core --no-default-features (unknown fields compiled out; plan relowered with unknown=drop), nounk/include/ak_abi.h, binaries campaign_codec_nounk (codec-nounk-launch*.jsonl) / campaign_rpc_nounk: core-ffi and host-gen no-unknown (the facade without unknown_fields, R-H22), C/D-nounk; A, B and the incumbents run there too"},
 "threads": {"codec": "one benchmark thread; the core starts none for codec calls (each codec log's own line has the process thread count)",
             "rpc_client": "caller threads = the in-flight level (1, 8, 16), created before round 1; the core's runtime workers = 2 (campaign_rpc --workers default); grpc-core sizes its own pollers and executor, counted in each client's process_threads_after_warmup line",
             "rpc_server": "grpc++ callback server, grpc-core's own threads; the server's thread count at start and at exit is in the rpc log"},
 "repeats": {"launches": $LAUNCHES, "rounds": $ROUNDS},
 "warmup": {"codec_bytes_per_arm": $WARM, "rpc_calls_per_cell": $RPCWARM, "rpc_server_calls_per_direction_per_client_transport_per_socket": $SRVWARM, "allocator": "every arm runs its warm-up before round 1"},
 "sample": {"codec_bytes": $BYTES, "codec_pool_bytes": $POOL, "llc_bytes": $LLC, "rpc_calls": $CALLS, "calib_iters": $CITERS,
            "codec_clock": "Google Benchmark " + "v1.8.3 (344117638c8f, Release, built by the runner)" + ": cpu_time = process CPU per repetition (MeasureProcessCPUTime) and real_time, repetitions randomly interleaved", "rpc_clock": "getrusage(RUSAGE_SELF) of the client process + CLOCK_MONOTONIC"},
}
print("# " + json.dumps(h, sort_keys=True))
EOF
}

# ---- the gate (requirement 26), once per commit and build ----------------------------
run_gate() {
  local log=$OUT/gate.log
  {
    header gate 0
    echo "===== build: generate, configure, build every target (D39) ====="
    if [ -n "${AK_INCUMBENT_PREFIX:-}" ]; then
      export CMAKE_PREFIX_PATH=$AK_INCUMBENT_PREFIX PKG_CONFIG_PATH=$AK_INCUMBENT_PREFIX/lib/pkgconfig:${PKG_CONFIG_PATH:-}
      export PATH=$AK_INCUMBENT_PREFIX/bin:$PATH
    fi
    python3 gen/generate.py --check 2>/dev/null | grep -v '^ok' ; gck=${PIPESTATUS[0]}
    [ "$gck" = 0 ] || { echo ">>> FAIL: generate.py --check"; }
    gbench_release || echo ">>> FAIL: Google Benchmark release build"
    cmake -S . -B "$B" -DAK_RPC=ON -Dbenchmark_DIR="$GBPREFIX/lib/cmake/benchmark" > "$TMPD/cfg.log" 2>&1 && cmake --build "$B" -j"$(nproc)" > "$TMPD/build.log" 2>&1 \
      && echo ">>> ok: build" || { tail -20 "$TMPD/build.log"; echo ">>> FAIL: build"; }
    echo "===== conformance: byte identity on the payload set, every level and linkage ====="
    for b in conformance_a17_shared conformance_b17_shared conformance_c14_shared conformance_c11_shared conformance_a17_static; do
      (cd "$FFI/schema/generated" && taskset -c "$AK_CPU_CLIENT" "$B/$b" payloads > "$TMPD/c.log" 2>&1); rc=$?
      echo "  $b: $(tail -1 "$TMPD/c.log")"
      [ $rc = 0 ] || echo ">>> FAIL: $b"
    done
    (cd "$FFI/schema/generated" && "$B/conformance_a17_noinit" payloads > /dev/null 2>&1) \
      && echo ">>> FAIL: the planted no-ak_init build passed" || echo "  control noinit: failed as required"
    echo "===== the full corpus, four arms (drop and retain), target and floor ====="
    for b in corpus_all_a17 corpus_all_c11; do
      python3 gen/corpus_all.py "$B/$b" > "$TMPD/k.log" 2>/dev/null; rc=$?
      grep -E '^## |^   pass|^CORPUS' "$TMPD/k.log" | sed 's/^/  /'
      [ $rc = 0 ] || echo ">>> FAIL: corpus $b"
    done
    SUB="S-Probe,U-root,X-lenwrap-lrr,E-map,T-dec-root"
    for p in proj reenc accept; do
      python3 gen/corpus_all.py "$B/corpus_all_a17" --only "$SUB" --plant $p > /dev/null 2>&1 \
        && echo ">>> FAIL: control $p passed" || echo "  control $p: failed as required"
    done
    python3 gen/corpus_all.py "$B/corpus_all_noinit" --only "$SUB" > /dev/null 2>&1 \
      && echo ">>> FAIL: control noinit (corpus) passed" || echo "  control noinit (corpus): failed as required"
    echo "===== crossing counts (requirement 19): the counting core against the committed counts ====="
    unknown_rows
    (cd "$FFI/schema/generated" && "$B/counts_a17_shared" --corpus "$FFI/corpus/generated" --rows "$ROWS" > "$OUT/counts.log" 2>&1)
    # The committed baseline (logs/cpp/counts-baseline.log, re-taken deliberately when the
    # core's ABI changes a count, with the reason in its header).
    grep -E '^  [PU]' "$FFI/logs/cpp/counts-baseline.log" > "$TMPD/want"
    grep -E '^  [PU]' "$OUT/counts.log" > "$TMPD/got"
    if diff "$TMPD/want" "$TMPD/got" > "$TMPD/diff"; then
      echo "  $(wc -l < "$TMPD/got") count rows identical to logs/cpp/counts-baseline.log"
    else
      head -10 "$TMPD/diff"; echo ">>> FAIL: crossing counts differ from the committed ones"
    fi
    echo "===== WP5 step 10: the no-unknown build (gen/nounk_gate.sh -> logs/cpp/wp5s10-nounk.log) ====="
    bash gen/nounk_gate.sh "$B"; ngr=$?
    grep -E '^>>> |u-family exports|^nounk_gate' "$FFI/logs/cpp/wp5s10-nounk.log" | sed 's/^/  /'
    [ $ngr = 0 ] || echo ">>> FAIL: the no-unknown gate"
    echo "===== the codec campaign binary's own gate, and its planted control ====="
    unknown_rows
    (cd "$FFI/schema/generated" && "$B/campaign_codec" --rounds 0 --bytes 1 --warmup 1 --corpus "$FFI/corpus/generated" --rows "$ROWS" > "$TMPD/g.log" 2>&1); rc=$?
    grep '^#' "$TMPD/g.log" | sed 's/^/  /'
    [ $rc = 0 ] || { grep 'GATE FAIL' "$TMPD/g.log" | head; echo ">>> FAIL: campaign_codec gate"; }
    (cd "$FFI/schema/generated" && AK_CAMPAIGN_PLANT=1 "$B/campaign_codec" --rounds 0 --bytes 1 --warmup 1 --corpus "$FFI/corpus/generated" --rows "$ROWS" > "$TMPD/g.log" 2>&1) \
      && echo ">>> FAIL: the planted codec gate passed" \
      || echo "  control campaign_codec plant: $(grep -c 'GATE FAIL' "$TMPD/g.log") slots failed as required"
    for cb in campaign_codec_nounk; do   # the no-unknown codec binary: its own gate and plant
      (cd "$FFI/schema/generated" && "$B/$cb" --rounds 0 --bytes 1 --warmup 1 --corpus "$FFI/corpus/generated" --rows "$ROWS" > "$TMPD/g.log" 2>&1); rc=$?
      grep '^#' "$TMPD/g.log" | sed 's/^/  /'
      [ $rc = 0 ] || { grep 'GATE FAIL' "$TMPD/g.log" | head; echo ">>> FAIL: $cb gate"; }
      (cd "$FFI/schema/generated" && AK_CAMPAIGN_PLANT=1 "$B/$cb" --rounds 0 --bytes 1 --warmup 1 --corpus "$FFI/corpus/generated" --rows "$ROWS" > "$TMPD/g.log" 2>&1) \
        && echo ">>> FAIL: the planted $cb gate passed" \
        || echo "  control $cb plant: $(grep -c 'GATE FAIL' "$TMPD/g.log") slots failed as required"
    done
    echo "===== the RPC call check (requirement 18) seen failing, and leaving NO sample (R-H4) ====="
    start_server
    warm_server "$TMPD/warm.log" && echo "  server warm-up: $(grep -c campaign_rpc_warm_server "$TMPD/warm.log") socket(s), $SRVWARM calls per direction per client transport" \
      || echo ">>> FAIL: the server warm-up"
    for rb in campaign_rpc campaign_rpc_nounk; do
      timeout 120 taskset -c "$AK_CPU_CLIENT" "$B/$rb" --target "$(sock_of shipped)" --expect $((EXP + 1)) \
        --transport shipped --cells C --dirs a --inflight 1 --rounds 1 --calls 2 --warmup 1 > "$TMPD/r.log" 2>&1; rc=$?
      ns=$(grep -c '^{' "$TMPD/r.log")
      [ $rc != 0 ] && [ "$ns" = 0 ] && echo "  control rpc length ($rb): aborted as required (exit $rc, $ns samples: $(grep -m1 'CALL CHECK' "$TMPD/r.log"))" \
                   || echo ">>> FAIL: a wrong response length did not abort, or left $ns sample(s) ($rb)"
      # An abort AFTER samples were taken (--fail-after 2): the samples already measured must
      # not reach the output either (buffered in the client, written only on success).
      timeout 120 taskset -c "$AK_CPU_CLIENT" "$B/$rb" --target "$(sock_of shipped)" --expect "$EXP" \
        --transport shipped --cells AB --dirs a --inflight 1 --rounds 2 --calls 2 --warmup 1 --fail-after 2 > "$TMPD/r.log" 2>&1; rc=$?
      ns=$(grep -c '^{' "$TMPD/r.log")
      [ $rc != 0 ] && [ "$ns" = 0 ] && echo "  control rpc abort after 2 samples ($rb): exit $rc, $ns samples written, as required" \
                   || echo ">>> FAIL: an abort after 2 samples left $ns sample(s) (exit $rc, $rb)"
    done
    # req. 19 (amended 2026-09-26): the RPC cells' crossings per call, from the counting
    # binaries, against the committed files (a difference stops the run).
    for v in "" _nounk; do
      want=$FFI/logs/cpp/rpc-counts$( [ -n "$v" ] && echo -nounk ).log
      taskset -c "$AK_CPU_CLIENT" "$B/campaign_rpc_count$v" --target "$(sock_of shipped)" --expect "$EXP" \
        --transport shipped --count 4 > "$OUT/rpc-counts$v.log" 2>&1
      if diff <(grep -E '^  [BCDE]' "$want") <(grep -E '^  [BCDE]' "$OUT/rpc-counts$v.log") > "$TMPD/rd"; then
        echo "  $(grep -cE '^  [BCDE]' "$OUT/rpc-counts$v.log") RPC count rows identical to logs/cpp/$(basename "$want")"
      else head -8 "$TMPD/rd"; echo ">>> FAIL: RPC crossing counts differ from $(basename "$want")"; fi
    done
    # The runner's own discard (R-H4): a client that fails leaves no launch file.
    rpc_launch_file "$TMPD/rpcctl.jsonl" 1 shipped "--fail-after 1" > /dev/null 2>&1; rc=$?
    [ $rc != 0 ] && [ ! -e "$TMPD/rpcctl.jsonl" ] && echo "  control runner rpc discard: exit $rc, no file, as required" \
                 || echo ">>> FAIL: the runner kept a failed rpc launch file (exit $rc)"
    stop_server
    echo "===== campaign_calib failure propagated (R-H5) ====="
    calib_one "$TMPD/calctl.jsonl" bogus 1 > /dev/null 2>&1; rc=$?
    [ $rc != 0 ] && [ "$(grep -c '^{' "$TMPD/calctl.jsonl" 2>/dev/null || echo 0)" = 0 ] \
      && echo "  control calib failure: exit $rc, no sample, as required" \
      || echo ">>> FAIL: a failing campaign_calib was not propagated (exit $rc)"
  } > "$log" 2>&1
  if grep -q '>>> FAIL' "$log"; then
    echo "GATE FAILED (requirement 26): see $log; no figure is produced" >&2; return 1
  fi
  echo "$COMMIT $B" > "$OUT/gate.ok"
  echo "gate passed: $log"
}

ROWS=$OUT/campaign_unknown_rows.tsv
unknown_rows() {  # the corpus's unknown-class accept rows at a shapes root (requirement 7)
  python3 "$SLICE/gen/u_rows.py" "$FFI/corpus/generated" "$ROWS" 2>/dev/null
}

# Google Benchmark v1.8.3, RELEASE (requirement 22a): built once from the upstream tag into
# the build directory. The tag's commit is checked, so a moved tag is refused.
GB_TAG=v1.8.3
GB_COMMIT=344117638c8ff7e239044fd0fa7085839fc03021
GBPREFIX=$B/gbench-$GB_TAG-release
gbench_release() {
  [ -f "$GBPREFIX/lib/cmake/benchmark/benchmarkConfig.cmake" ] && return 0
  local src=$B/gbench-src
  rm -rf "$src" "$B/gbench-build"
  GIT_LFS_SKIP_SMUDGE=1 git clone -q --depth 1 --branch "$GB_TAG" https://github.com/google/benchmark "$src" 2>/dev/null || return 1
  [ "$(git -C "$src" rev-parse HEAD)" = "$GB_COMMIT" ] || { echo "Google Benchmark $GB_TAG is not $GB_COMMIT"; return 1; }
  cmake -S "$src" -B "$B/gbench-build" -DCMAKE_BUILD_TYPE=Release -DBENCHMARK_ENABLE_TESTING=OFF \
    -DBENCHMARK_ENABLE_GTEST_TESTS=OFF -DCMAKE_INSTALL_PREFIX="$GBPREFIX" > /dev/null 2>&1 \
    && cmake --build "$B/gbench-build" -j"$(nproc)" > /dev/null 2>&1 \
    && cmake --install "$B/gbench-build" > /dev/null 2>&1
}

# One RPC launch file for one transport, both clients (R-H4): each client's samples go to a
# scratch file and are appended only if it succeeded; on any failure the launch file is
# deleted, so an aborted run leaves no sample. Usage: rpc_launch_file FILE LAUNCH TRANSPORT
# [EXTRA-ARGS]; the server must be running. Returns nonzero on failure.
rpc_launch_file() {
  local f=$1 l=$2 t=$3 extra=${4:-} rb rc
  if [ $((l % 2)) = 1 ]; then RBS="campaign_rpc campaign_rpc_nounk"; else RBS="campaign_rpc_nounk campaign_rpc"; fi
  for rb in $RBS; do
    timeout 7200 taskset -c "$AK_CPU_CLIENT" "$B/$rb" --target "$(sock_of $t)" --expect "$EXP" \
      --transport "$t" --launch "$l" --rounds "$ROUNDS" --calls "$CALLS" --warmup "$RPCWARM" $extra \
      > "$TMPD/client.out" 2>&1; rc=$?
    if [ $rc != 0 ]; then
      echo "rpc client $rb ($t) failed (exit $rc): $(grep -m1 'CALL CHECK' "$TMPD/client.out")" >&2
      rm -f "$f"; return 1
    fi
    { echo "# client $rb"; cat "$TMPD/client.out"; } >> "$f"
  done
  return 0
}

# One campaign_calib direction into FILE (R-H5): appended only on success; on failure the
# file is deleted and the rc returned. Usage: calib_one FILE DIR LAUNCH.
calib_one() {
  local f=$1 d=$2 l=$3 rc
  if [ -n "${PERF:-}" ]; then
    taskset -c "$AK_CPU_CLIENT" "$PERF" stat -x, -e cycles,instructions -o "$TMPD/perf" \
      "$B/campaign_calib" --dir "$d" --launch "$l" --rounds "$ROUNDS" --iters "$CITERS" > "$TMPD/calib.out" 2>&1; rc=$?
  else
    taskset -c "$AK_CPU_CLIENT" "$B/campaign_calib" --dir "$d" --launch "$l" --rounds "$ROUNDS" --iters "$CITERS" \
      > "$TMPD/calib.out" 2>&1; rc=$?
  fi
  if [ $rc != 0 ]; then echo "campaign_calib --dir $d failed (exit $rc)" >&2; rm -f "$f"; return $rc; fi
  cat "$TMPD/calib.out" >> "$f"
  [ -n "${PERF:-}" ] && sed 's/^/# perf '"$d"': /' "$TMPD/perf" >> "$f"
  return 0
}

# req. 13 / 17 (amended 2026-09-26): ONE server process per launch, serving every cell of both
# builds on two Unix domain sockets (shipped and pinned configurations), warmed by
# $SRVWARM calls per direction from each client transport (grpc++ and the core's) on each
# socket before any client runs.
EXP=""; SPID=""; SRVTHREADS=""
SOCK_shipped=$TMPD/s.sock; SOCK_pinned=$TMPD/p.sock
sock_of() { [ "$1" = pinned ] && echo "unix:$SOCK_pinned" || echo "unix:$SOCK_shipped"; }
start_server() {
  rm -f "$SOCK_shipped" "$SOCK_pinned"
  taskset -c "$AK_CPU_SERVER" "$B/campaign_server" --uds-shipped "$SOCK_shipped" --uds-pinned "$SOCK_pinned" \
    > "$TMPD/srv.out" 2> "$TMPD/srv.err" &
  SPID=$!
  for _ in $(seq 100); do grep -q READY "$TMPD/srv.out" 2>/dev/null && break; sleep 0.1; done
  EXP=$(awk '/READY/{print $4}' "$TMPD/srv.out"); SRVTHREADS=$(awk '/READY/{print $5}' "$TMPD/srv.out")
  [ -n "$EXP" ] || { echo "the server did not start"; cat "$TMPD/srv.err"; exit 1; }
}
warm_server() {  # warm_server OUTFILE: every socket, both client transports, every call checked
  local t
  for t in shipped pinned; do
    taskset -c "$AK_CPU_CLIENT" "$B/campaign_rpc" --target "$(sock_of $t)" --expect "$EXP" --transport "$t" \
      --warm-server "$SRVWARM" >> "$1" 2>&1 || { echo "server warm-up failed ($t)" >&2; return 1; }
  done
}
stop_server() { kill "$SPID" 2>/dev/null; wait "$SPID" 2>/dev/null; SPID=""; }
trap 'stop_server; rm -rf "$TMPD"' EXIT

ensure_gate() {
  if [ -f "$OUT/gate.ok" ] && [ "$(cat "$OUT/gate.ok")" = "$COMMIT $B" ]; then return 0; fi
  run_gate
}

case "$SUITE" in
  gate) run_gate; exit $? ;;
  codec)
    ensure_gate || exit 1
    unknown_rows
    # WP5 step 10 (requirement 10's third mode): two binaries, the full build (core-ffi drop
    # and retain) and the no-unknown build (core-ffi no-unknown; host-gen drop and the
    # incumbents as in-process controls), one file each per launch, their ORDER alternated
    # by launch so neither always runs on a cold or warm machine.
    for l in $(seq 1 "$LAUNCHES"); do
     if [ $((l % 2)) = 1 ]; then CBS="campaign_codec campaign_codec_nounk"; else CBS="campaign_codec_nounk campaign_codec"; fi
     for cb in $CBS; do
      tag=${cb#campaign_codec}; tag=${tag#_}; tag=${tag:+$tag-}
      f=$OUT/codec-${tag}launch$l.jsonl
      # Google Benchmark (requirement 22a): the binary gates, warms up and runs the
      # benchmarks; its per-repetition JSON is converted to section 7's lines, and the raw
      # Google Benchmark JSON is kept beside them.
      gb=$OUT/codec-${tag}launch$l.gbench.json
      { header codec "$l"
        (cd "$FFI/schema/generated" && taskset -c "$AK_CPU_CLIENT" "$B/$cb" --launch "$l" \
           --rounds "$ROUNDS" --bytes "$BYTES" --warmup "$WARM" --corpus "$FFI/corpus/generated" \
           --rows "$ROWS" --gbench-out "$gb" --pool-bytes "$POOL" > "$TMPD/gb.console" 2>&1; echo $? > "$TMPD/gb.rc")
        grep '^#' "$TMPD/gb.console"
        python3 "$SLICE/gen/gbench_to_jsonl.py" "$gb" "$l" "$([ "$cb" = campaign_codec_nounk ] && echo no-unknown || echo full)"; } > "$f" 2>/dev/null
      if [ "$(cat "$TMPD/gb.rc")" != 0 ] || ! grep -q '^{' "$f"; then
        echo "codec launch $l failed: $f removed (no sample from a failed run)" >&2; tail -20 "$TMPD/gb.console" >&2
        rm -f "$f" "$gb"; exit 1
      fi
      echo "wrote $f"
     done
    done ;;
  rpc)
    ensure_gate || exit 1
    for l in $(seq 1 "$LAUNCHES"); do
      f=$OUT/rpc-launch$l.jsonl
      header rpc "$l" > "$f"
      # The full client (A B C-retain C-drop D-retain D-drop E-drop E-retain F-drop F-retain)
      # and the no-unknown client (A B C-nounk D-nounk E-nounk F-nounk; A and B its
      # in-process controls), order alternated by launch, against ONE server process for
      # the launch (req. 13), warmed first. R-H4: a failed client deletes the launch file.
      start_server
      echo "# server: one process for the launch, sockets $SOCK_shipped (shipped) and $SOCK_pinned (pinned), threads at start $SRVTHREADS" >> "$f"
      warm_server "$f" || { stop_server; rm -f "$f"; exit 1; }
      for t in shipped pinned; do
        rpc_launch_file "$f" "$l" "$t"; rc=$?
        [ $rc = 0 ] || { stop_server; echo "rpc launch $l ($t) failed: no file kept" >&2; exit 1; }
      done
      stop_server
      echo "# server: $(cat "$TMPD/srv.err")" >> "$f"
      echo "wrote $f"
    done ;;
  calib)
    ensure_gate || exit 1
    PERF=$(command -v perf || true)
    for l in $(seq 1 "$LAUNCHES"); do
      f=$OUT/calib-launch$l.jsonl
      header calib "$l" > "$f"
      [ -n "$PERF" ] || echo "# perf unavailable on this machine: cycles and instructions not recorded (requirement 20)" >> "$f"
      for d in forward reverse; do
        calib_one "$f" "$d" "$l" || { echo "calib launch $l ($d) failed: no file kept" >&2; exit 1; }
      done
      # The Rust slice's own crossing benchmark, on this machine (R13, requirement 20).
      ( cd "$SLICE/../rust" && CARGO_TARGET_DIR=$B/rustcal cargo build --release --bin bench > /dev/null 2>&1 \
          && AK_BENCH_ONLY=P1.1 taskset -c "$AK_CPU_CLIENT" "$B/rustcal/release/bench" 2>&1 \
             | grep -iE 'crossing|reverse|forward' | sed 's/^/# rust-slice bench: /' ) >> "$f" \
        || echo "# rust-slice bench: did not build here" >> "$f"
      echo "wrote $f"
    done ;;
esac
