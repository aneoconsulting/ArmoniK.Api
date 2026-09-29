#!/usr/bin/env bash
# The physical-machine probe's pre-step checks (no timing): what must hold before
# gen/physical_probe.sh takes a figure.
#
#   gen/physical_checks.sh LOG_DIR        (logs/cpp/opt/physical-probe/checks)
#
#   1. build facts: toolchain and incumbent versions, the tree's state, and for every binary the
#      core it loads (path, sha256, variant symbols: unknown-field support, rpc);
#   2. payload byte identity, conformance at C++17, full (608 checks) and no-unknown (478);
#   3. the codec pre-check of campaign_codec and campaign_codec_nounk (--rounds 0: every group
#      and mode against the incumbent, the pull value gate);
#   4. gen/q_checks.sh (its own server): --semantics 1 on both builds and both send paths, the
#      plants on queue cells, the queue-cell grid smoke, the RPC counts against the committed files;
#   5. the grid smoke of EVERY cell (--cells ABCDEF: A-F, framed twins, queue and pull cells,
#      every mode), every direction, k = 1 and 8, one repetition, shipped and pinned, both builds:
#      each process runs campaign_rpc's pre-check and checks every call;
#   6. the send path is what the label says: allocations of at least 1 MiB per call (LD_PRELOAD
#      gen/allocprobe.c) on the reference cells (B, C, E: set_framed(c, 0), tonic's codec copies
#      each message) against their framed twins (set_framed(c, 1)), directions c and d.
# CPU sets from AK_CPU_CLIENT / AK_CPU_SERVER. Exit 1 on any failure.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$HERE" || exit 2
L=${1:?usage: gen/physical_checks.sh LOG_DIR}; mkdir -p "$L"; L=$(cd "$L" && pwd)
FFI=$(cd "$HERE/../.." && pwd); B=${BUILD:-$HERE/build-campaign}; SERVE=$FFI/poc/rust/serve.sh
: "${AK_CPU_CLIENT:?AK_CPU_CLIENT}" "${AK_CPU_SERVER:?AK_CPU_SERVER}"
export AK_CPU_CLIENT AK_CPU_SERVER
S=$(mktemp -d); export AK_SERVE_STATE=$S/serve.state
cleanup() { [ -f "$AK_SERVE_STATE" ] && bash "$SERVE" stop > /dev/null 2>&1; rm -rf "$S"; true; }
trap cleanup EXIT
F=0
ok() { echo ">>> ok: $*"; }
bad() { echo ">>> FAIL: $*"; F=$((F + 1)); }
LOG=$L/checks.log
{
  echo "# cpp slice: physical-probe checks (gen/physical_checks.sh), commit $(git -C "$FFI" rev-parse --short HEAD)$(git -C "$FFI" status --porcelain -- poc/cpp poc/codec | grep -q . && echo ' + UNCOMMITTED CHANGES'), $(date -u +%FT%TZ)"
  echo "# machine $(python3 gen/machine_facts.py "$AK_CPU_CLIENT" "$AK_CPU_SERVER")"
  echo "===== 1. build facts ====="
  nix-shell gen/shell.nix --run 'echo "g++: $(g++ --version | head -1)"; echo "cmake: $(cmake --version | head -1)"; echo "protoc: $(protoc --version)"; echo "protobuf (pkg-config): $(pkg-config --modversion protobuf)"; echo "grpc++ (pkg-config): $(pkg-config --modversion grpc++)"' 2>/dev/null
  echo "rustc: $(rustc --version); cargo: $(cargo --version)"
  echo "Google Benchmark: $(grep -m1 'benchmark_VERSION\|PACKAGE_VERSION' "$B/gbench-v1.8.3-release/lib/cmake/benchmark/benchmarkConfigVersion.cmake" | head -1)"
  for t in campaign_rpc campaign_rpc_nounk campaign_rpc_count campaign_rpc_count_nounk campaign_codec campaign_codec_nounk conformance_a17_shared conformance_nounk_a17; do
    so=$(ldd "$B/$t" | grep -o '/[^ ]*libak_core\.so' | head -1)
    unk=$(nm -D --defined-only "$so" | grep -cE ' (ak_uencode_|ak_uelem|ak_dec_reset_)'); rpc=$(nm -D --defined-only "$so" | grep -c ' ak_call_open$')
    want_unk=1; case $t in *nounk*) want_unk=0 ;; esac; want_rpc=1; case $t in conformance*) want_rpc=0 ;; esac
    echo "  $t -> ${so#$HERE/} sha256 $(sha256sum "$so" | cut -c1-16) unknown-field exports $unk rpc $rpc"
    { [ $want_unk = 1 ] && [ "$unk" = 0 ]; } || { [ $want_unk = 0 ] && [ "$unk" != 0 ]; } || { [ $want_rpc = 1 ] && [ "$rpc" = 0 ]; } \
      && bad "$t loads the wrong core variant ($so)"
  done
  echo "===== 2. payload byte identity (conformance, C++17) ====="
  for c in conformance_a17_shared conformance_nounk_a17; do
    (cd ../../schema/generated && "$B/$c" payloads > "$S/c" 2>&1); rc=$?
    r=$(grep -E 'checks,' "$S/c" | tail -1); echo "  $c: exit $rc: $r"
    [ $rc = 0 ] && echo "$r" | grep -q ' 0 failures' && ok "$c" || { grep -m5 FAIL "$S/c"; bad "$c"; }
  done
  echo "===== 3. codec pre-check ====="
  python3 gen/u_rows.py ../../corpus/generated "$S/rows.tsv" 2>/dev/null
  for c in campaign_codec campaign_codec_nounk; do
    (cd ../../schema/generated && taskset -c "$AK_CPU_CLIENT" "$B/$c" --rounds 0 --pool-bytes 1048576 \
       --corpus "$PWD/../../corpus/generated" --rows "$S/rows.tsv" > "$S/p" 2>&1); rc=$?
    g=$(grep -o '"campaign_codec_gate": {[^}]*}' "$S/p"); echo "  $c: exit $rc $g"
    [ $rc = 0 ] && echo "$g" | grep -q '"failed": 0' && ok "$c pre-check" || { grep -m5 'GATE FAIL' "$S/p"; bad "$c pre-check"; }
  done
  echo "===== 4. gen/q_checks.sh (semantics, queue plants, queue grid smoke, RPC counts) -> q-checks.log ====="
  BUILD=$B bash gen/q_checks.sh "$L/q-checks.log"; rc=$?
  grep -E '>>>|q_checks:' "$L/q-checks.log" | sed 's/^/  /'
  [ $rc = 0 ] && ok "q_checks" || bad "q_checks"
  echo "===== 5. grid smoke, every cell, every direction, k = 1 and 8, shipped and pinned, both builds ====="
  bash "$SERVE" start --out "$S/srv" > "$S/srv.out" 2>&1 || { cat "$S/srv.out"; bad "server"; }
  EXP=$(sed -n 's/.*P2.2 \([0-9]*\) B.*/\1/p' "$S/srv/rpc-server.log" | head -1)
  echo "  server: $(head -1 "$S/srv/rpc-server.log")"
  for T in shipped pinned; do
    SOCK=unix:$(awk -v t=$T '$1==t{print $2}' "$S/srv.out")
    for exe in campaign_rpc campaign_rpc_nounk; do
      rm -f "$S/g.json"
      taskset -c "$AK_CPU_CLIENT" "$B/$exe" --target "$SOCK" --expect "$EXP" --transport $T --cells ABCDEF --dirs arbcd \
        --launch 1 --rounds 1 --min-time-s 0.001 --warmup-s 0 --inflight 1,8 --gbench-out "$S/g.json" > "$S/o" 2>&1; rc=$?
      n=$(python3 gen/gbench_to_jsonl.py "$S/g.json" 1 full rpc 2>/dev/null | grep -c '^{')
      cells=$(python3 gen/gbench_to_jsonl.py "$S/g.json" 1 full rpc 2>/dev/null | grep '^{' | python3 -c "import json,sys; print(len({json.loads(l)['cell'] for l in sys.stdin}))")
      [ $rc = 0 ] && [ "$n" -gt 0 ] && ok "$exe $T: $n samples, $cells cells, every call checked, pre-check passed" \
        || { grep -m3 -E 'CALL CHECK|pre-check|failed' "$S/o"; bad "$exe $T rc=$rc"; }
    done
  done
  echo "===== 6. send path: allocations >= 1 MiB per call, reference (set_framed 0) against framed (set_framed 1) ====="
  gcc -O2 -shared -fPIC -o "$S/allocprobe.so" gen/allocprobe.c -ldl
  SOCK=unix:$(awk '$1=="pinned"{print $2}' "$S/srv.out")
  LD_PRELOAD=$S/allocprobe.so taskset -c "$AK_CPU_CLIENT" "$B/campaign_rpc" --target "$SOCK" --expect "$EXP" --transport pinned \
    --cells B,Bf,C-retain,Cf-retain,C-q-retain,Cf-q-retain,E-retain,Ef-retain --dirs cd --alloc-probe 8 > "$S/a" 2>&1; rc=$?
  grep -v '^#' "$S/a" | sed 's/^/  /'
  python3 - "$S/a" <<'PY' || bad "reference and framed rows do not differ as the send path says"
import re, sys
rows = {}
for l in open(sys.argv[1]):
    m = re.match(r"\s+(\S+)\s+(\S+)\s+(\S+)\s+big allocations per call\s+([\d.]+)", l)
    if m: rows[(m.group(1), m.group(3))] = float(m.group(4))
bad = []
for ref, fr in (("B", "Bf"), ("C-retain", "Cf-retain"), ("C-q-retain", "Cf-q-retain"), ("E-retain", "Ef-retain")):
    for p in ("P5.3", "P5.4", "4MiB", "16MiB"):
        # the reference path copies each request message into tonic's buffer, a fresh buffer of
        # >= 1 MiB per message; the framed twin sends the message's own buffer (the encode ring of
        # 3 spares still misses sometimes on 16 MiB). With both on the framed default the pair
        # would be equal. Required: reference >= messages - 0.5 and reference - framed >= messages / 2.
        msgs = {"P5.3": 1, "P5.4": 1, "4MiB": 2, "16MiB": 8}[p]
        r, f = rows.get((ref, p), -1), rows.get((fr, p), -1)
        if not (r >= msgs - 0.5 and r - f >= msgs / 2.0):
            bad.append("%s %s %.2f against %s %.2f" % (ref, p, rows.get((ref, p), -1), fr, rows.get((fr, p), -1)))
print("  reference >= one allocation per message and reference - framed >= half of that, every pair: " + ("yes" if not bad else "NO: " + "; ".join(bad)))
sys.exit(1 if bad else 0)
PY
  [ $rc = 0 ] && ok "alloc probe (reference vs framed)" || bad "alloc probe rc=$rc"
  bash "$SERVE" stop > /dev/null 2>&1
  echo "physical_checks: $F failure(s)"
} > "$LOG" 2>&1
cat "$LOG" | grep -E '>>>|physical_checks:'
[ $F = 0 ]
