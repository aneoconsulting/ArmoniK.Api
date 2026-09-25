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
#   AK_CAMPAIGN_CALIB_ITERS calib: crossings per sample        (default 100000000)
#   AK_CAMPAIGN_ALLOW_DIRTY=1  SMOKE RUNS ONLY: run on a dirty tree; the header says so and
#                        every figure is container instrumentation (requirement 27 refuses it)
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
: "${AK_CPU_CLIENT:?AK_CPU_CLIENT is required (requirement 4)}"
: "${AK_CPU_SERVER:?AK_CPU_SERVER is required (requirement 4)}"
LAUNCHES=${AK_CAMPAIGN_LAUNCHES:-3}
ROUNDS=${AK_CAMPAIGN_ROUNDS:-5}
BYTES=${AK_CAMPAIGN_BYTES:-33554432}
WARM=${AK_CAMPAIGN_WARMUP:-$BYTES}
CALLS=${AK_CAMPAIGN_CALLS:-480}
RPCWARM=${AK_CAMPAIGN_RPC_WARMUP:-96}
CITERS=${AK_CAMPAIGN_CALIB_ITERS:-100000000}
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
COMMIT=$(git -C "$REPO" rev-parse HEAD)

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
 "instrumentation": bool("""$DIRTY""") or os.environ.get("AK_CAMPAIGN_ALLOW_DIRTY") == "1",
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
 "repeats": {"launches": $LAUNCHES, "rounds": $ROUNDS},
 "warmup": {"codec_bytes_per_arm": $WARM, "rpc_calls_per_cell": $RPCWARM, "allocator": "every arm runs its warm-up before round 1"},
 "sample": {"codec_bytes": $BYTES, "rpc_calls": $CALLS, "calib_iters": $CITERS,
            "codec_clock": "CLOCK_THREAD_CPUTIME_ID + CLOCK_MONOTONIC", "rpc_clock": "getrusage(RUSAGE_SELF) of the client process + CLOCK_MONOTONIC"},
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
    cmake -S . -B "$B" -DAK_RPC=ON > "$OUT/.cfg.log" 2>&1 && cmake --build "$B" -j"$(nproc)" > "$OUT/.build.log" 2>&1 \
      && echo ">>> ok: build" || { tail -20 "$OUT/.build.log"; echo ">>> FAIL: build"; }
    echo "===== conformance: byte identity on the payload set, every level and linkage ====="
    for b in conformance_a17_shared conformance_b17_shared conformance_c14_shared conformance_c11_shared conformance_a17_static; do
      (cd "$FFI/schema/generated" && taskset -c "$AK_CPU_CLIENT" "$B/$b" payloads > "$OUT/.c.log" 2>&1); rc=$?
      echo "  $b: $(tail -1 "$OUT/.c.log")"
      [ $rc = 0 ] || echo ">>> FAIL: $b"
    done
    (cd "$FFI/schema/generated" && "$B/conformance_a17_noinit" payloads > /dev/null 2>&1) \
      && echo ">>> FAIL: the planted no-ak_init build passed" || echo "  control noinit: failed as required"
    echo "===== the full corpus, four arms (drop and retain), target and floor ====="
    for b in corpus_all_a17 corpus_all_c11; do
      python3 gen/corpus_all.py "$B/$b" > "$OUT/.k.log" 2>/dev/null; rc=$?
      grep -E '^## |^   pass|^CORPUS' "$OUT/.k.log" | sed 's/^/  /'
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
    (cd "$FFI/schema/generated" && "$B/counts_a17_shared" > "$OUT/counts.log" 2>&1)
    # The committed baseline (logs/cpp/counts-baseline.log, re-taken deliberately when the
    # core's ABI changes a count, with the reason in its header).
    grep -E '^  P' "$FFI/logs/cpp/counts-baseline.log" > "$OUT/.want"
    grep -E '^  P' "$OUT/counts.log" > "$OUT/.got"
    if diff "$OUT/.want" "$OUT/.got" > "$OUT/.diff"; then
      echo "  $(wc -l < "$OUT/.got") count rows identical to logs/cpp/counts-baseline.log"
    else
      head -10 "$OUT/.diff"; echo ">>> FAIL: crossing counts differ from the committed ones"
    fi
    echo "===== the codec campaign binary's own gate, and its planted control ====="
    unknown_rows
    (cd "$FFI/schema/generated" && "$B/campaign_codec" --rounds 0 --bytes 1 --warmup 1 --corpus "$FFI/corpus/generated" --rows "$ROWS" > "$OUT/.g.log" 2>&1); rc=$?
    grep '^#' "$OUT/.g.log" | sed 's/^/  /'
    [ $rc = 0 ] || { grep 'GATE FAIL' "$OUT/.g.log" | head; echo ">>> FAIL: campaign_codec gate"; }
    (cd "$FFI/schema/generated" && AK_CAMPAIGN_PLANT=1 "$B/campaign_codec" --rounds 0 --bytes 1 --warmup 1 --corpus "$FFI/corpus/generated" --rows "$ROWS" > "$OUT/.g.log" 2>&1) \
      && echo ">>> FAIL: the planted codec gate passed" \
      || echo "  control campaign_codec plant: $(grep -c 'GATE FAIL' "$OUT/.g.log") slots failed as required"
    echo "===== the RPC call check (requirement 18) seen failing: a wrong expected length ====="
    start_server shipped
    timeout 120 taskset -c "$AK_CPU_CLIENT" "$B/campaign_rpc" --target 127.0.0.1:$PORT --expect $((EXP + 1)) \
      --transport shipped --cells C --dirs a --inflight 1 --rounds 1 --calls 2 --warmup 1 > "$OUT/.r.log" 2>&1; rc=$?
    stop_server
    [ $rc != 0 ] && echo "  control rpc length: aborted as required (exit $rc: $(grep -m1 'CALL CHECK' "$OUT/.r.log"))" \
                 || echo ">>> FAIL: a wrong response length did not abort"
  } > "$log" 2>&1
  if grep -q '>>> FAIL' "$log"; then
    echo "GATE FAILED (requirement 26): see $log; no figure is produced" >&2; return 1
  fi
  echo "$COMMIT $B" > "$OUT/gate.ok"
  echo "gate passed: $log"
}

ROWS=$OUT/campaign_unknown_rows.tsv
unknown_rows() {  # the corpus's unknown-class accept rows at a shapes root (requirement 7)
  python3 - "$FFI/corpus/generated" "$ROWS" <<'EOF'
import json, os, sys
d = sys.argv[1]
roots = {"ListResultsResponse", "ListTasksDetailedResponse", "ListProbeResponse", "ListTaskSummaryResponse",
         "UploadResultDataMessage", "ListMetricsResponse", "DualResponse"}
m = json.load(open(os.path.join(d, "manifest.json")))["vectors"]
with open(sys.argv[2], "w") as f:
    for k in sorted(m):
        r = m[k]
        if r.get("class") == "unknown" and r.get("expect") == "accept" and r.get("root") in roots \
                and r.get("verdict") != "disputed":
            f.write("%s\t%s\t%s\n" % (k, r["root"], r["file"]))
EOF
}

PORT=""; EXP=""; SPID=""
start_server() {
  taskset -c "$AK_CPU_SERVER" "$B/campaign_server" --port 0 --transport "$1" > "$OUT/.srv.out" 2> "$OUT/.srv.err" &
  SPID=$!
  for _ in $(seq 100); do grep -q READY "$OUT/.srv.out" 2>/dev/null && break; sleep 0.1; done
  PORT=$(awk '/READY/{print $2}' "$OUT/.srv.out"); EXP=$(awk '/READY/{print $3}' "$OUT/.srv.out")
  [ -n "$PORT" ] || { echo "the server did not start"; cat "$OUT/.srv.err"; exit 1; }
}
stop_server() { kill "$SPID" 2>/dev/null; wait "$SPID" 2>/dev/null; SPID=""; }
trap 'stop_server' EXIT

ensure_gate() {
  if [ -f "$OUT/gate.ok" ] && [ "$(cat "$OUT/gate.ok")" = "$COMMIT $B" ]; then return 0; fi
  run_gate
}

case "$SUITE" in
  gate) run_gate; exit $? ;;
  codec)
    ensure_gate || exit 1
    unknown_rows
    for l in $(seq 1 "$LAUNCHES"); do
      f=$OUT/codec-launch$l.jsonl
      { header codec "$l"
        (cd "$FFI/schema/generated" && taskset -c "$AK_CPU_CLIENT" "$B/campaign_codec" --launch "$l" \
           --rounds "$ROUNDS" --bytes "$BYTES" --warmup "$WARM" --corpus "$FFI/corpus/generated" --rows "$ROWS"); } > "$f" 2>&1 \
        || { echo "codec launch $l failed: $f" >&2; exit 1; }
      echo "wrote $f"
    done ;;
  rpc)
    ensure_gate || exit 1
    for l in $(seq 1 "$LAUNCHES"); do
      f=$OUT/rpc-launch$l.jsonl
      header rpc "$l" > "$f"
      for t in shipped pinned; do
        start_server "$t"
        timeout 7200 taskset -c "$AK_CPU_CLIENT" "$B/campaign_rpc" --target 127.0.0.1:$PORT --expect "$EXP" \
          --transport "$t" --launch "$l" --rounds "$ROUNDS" --calls "$CALLS" --warmup "$RPCWARM" >> "$f" 2>&1
        rc=$?
        stop_server
        echo "# server ($t): $(cat "$OUT/.srv.err")" >> "$f"
        [ $rc = 0 ] || { echo "rpc launch $l ($t) failed: $f" >&2; exit 1; }
      done
      echo "wrote $f"
    done ;;
  calib)
    ensure_gate || exit 1
    PERF=$(command -v perf || true)
    for l in $(seq 1 "$LAUNCHES"); do
      f=$OUT/calib-launch$l.jsonl
      header calib "$l" > "$f"
      for d in forward reverse; do
        if [ -n "$PERF" ]; then
          taskset -c "$AK_CPU_CLIENT" "$PERF" stat -x, -e cycles,instructions -o "$OUT/.perf" \
            "$B/campaign_calib" --dir $d --launch "$l" --rounds "$ROUNDS" --iters "$CITERS" >> "$f" 2>&1
          sed 's/^/# perf '"$d"': /' "$OUT/.perf" >> "$f"
        else
          echo "# perf unavailable on this machine: cycles and instructions not recorded (requirement 20)" >> "$f"
          taskset -c "$AK_CPU_CLIENT" "$B/campaign_calib" --dir $d --launch "$l" --rounds "$ROUNDS" --iters "$CITERS" >> "$f" 2>&1
        fi
      done
      # The Rust slice's own crossing benchmark, on this machine (R13, requirement 20).
      ( cd "$SLICE/../rust" && CARGO_TARGET_DIR=$B/rustcal cargo build --release --bin bench > /dev/null 2>&1 \
          && AK_BENCH_ONLY=P1.1 taskset -c "$AK_CPU_CLIENT" "$B/rustcal/release/bench" 2>&1 \
             | grep -iE 'crossing|reverse|forward' | sed 's/^/# rust-slice bench: /' ) >> "$f" \
        || echo "# rust-slice bench: did not build here" >> "$f"
      echo "wrote $f"
    done ;;
esac
