#!/usr/bin/env bash
# FIX-PLAN WP12 ("both variants pass both slices' gates"): gen/gate.sh, unchanged in its steps,
# run against one h2 variant of the core, plus the RPC checks over TCP and a marker that the
# variant is in effect. Nothing timed (a container figure is instrumentation).
#
#   gen/wp12_gate.sh plain|stock|h2-batch OUT_DIR
#
#   plain     the gate as committed: every binary loads its own target directory's core (built
#             in this workspace, stock h2); only the build lock is added
#   stock     every binary loads the core of ITS feature set built by poc/codec/h2-batch/build.sh
#             stock (gen/wp12_core.sh, target-wp12-stock)
#   h2-batch  the same with build.sh h2-batch (h2 0.4.19 + h2-batch.patch, AK_H2_COALESCE unset =
#             the default 16), target-wp12-h2-batch
#
# How a variant is put in place (gen/wp12-shim/cargo): <target>/release/ak-variant -> the variant
# core of the feature set that target directory's builds compiled, and
# LD_LIBRARY_PATH='$ORIGIN/ak-variant:$ORIGIN/../ak-variant' for every process; `cargo run` becomes
# build + exec (cargo would put its deps directory first). The host side (tonic in the harness,
# the in-process servers, serve.sh's rpc_server) keeps crates.io h2, as in every timed run.
# Proof (OUT_DIR/loads.txt, gen/wp12_loads.py): the dynamic linker's record (LD_DEBUG=libs) of
# every process of every step: which libak_core.so it loaded, its sha256; the static-link check.
#
# After the gate: AK_CHECK_TRANSPORT=tcp runs of upload_check, rpc_semantics and header_diff (full
# and no-unknown builds), burst_check on serve.sh's Unix sockets and its TCP listener, and the
# marker: write syscalls per d/16 MiB call (stream_probe, /proc/self/io), cells Cf (core) and A
# (host tonic, the control), TCP.
# Every cargo build holds /tmp/claude-0/ak-codec-build.lock (the slices share poc/codec).
set -Eeuo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; RUST=$(cd "$HERE/.." && pwd)
VARIANT=${1:?usage: wp12_gate.sh plain|stock|h2-batch OUT_DIR}; OUT=${2:?out dir}
case $VARIANT in plain|stock|h2-batch) ;; *) echo "variant: plain, stock or h2-batch" >&2; exit 2 ;; esac
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
SCR=${AK_WP12_SCRATCH:-${TMPDIR:-/tmp}/ak-wp12}; LDD="$SCR/lddebug-$VARIANT"
rm -rf "$LDD"; mkdir -p "$LDD" "$OUT/tcp"
export AK_WP12_LOCK=${AK_WP12_LOCK:-/tmp/claude-0/ak-codec-build.lock}
export AK_WP12_CORES="$RUST/target-wp12-cores" AK_WP12_SHIMLOG="$OUT/shim.log" AK_WP12_LINKS="$SCR/links-$VARIANT.tsv"
: > "$AK_WP12_SHIMLOG"; : > "$AK_WP12_LINKS"
unset AK_H2_COALESCE LD_DEBUG LD_DEBUG_OUTPUT
cd "$RUST"
# no stale mapping from an earlier run (only this tooling creates these symlinks)
find "$RUST" -maxdepth 3 -path "$RUST/target*/release/ak-variant" -type l -delete
cleanup() { find "$RUST" -maxdepth 3 -path "$RUST/target*/release/ak-variant" -type l -delete; [ -n "${SPID:-}" ] && AK_SERVE_STATE="$SCR/serve-$VARIANT.state" ./serve.sh stop >/dev/null 2>&1 || true; }
trap cleanup EXIT

FULL=init-guard,rpc,unknown-fields; NOUNK=init-guard,rpc
if [ "$VARIANT" != plain ]; then
  for k in "$FULL" "$NOUNK"; do
    [ -f "$AK_WP12_CORES/$VARIANT/${k//,/+}/libak_core.so" ] || gen/wp12_core.sh "$VARIANT" "$k" "$AK_WP12_CORES/$VARIANT/${k//,/+}" >> "$AK_WP12_SHIMLOG" 2>&1
  done
fi

G="$OUT/gate.log"
{
  echo "# WP12 gate, variant $VARIANT, $(date -u +%FT%TZ), container (not the campaign machine): no figure here is a timing"
  echo "# commit $(git rev-parse HEAD)$(git diff --quiet HEAD -- . ../codec || echo ' + uncommitted changes in poc/rust or poc/codec')"
  echo "# $(rustc --version); $(cargo --version); nproc $(nproc); $(grep -m1 'model name' /proc/cpuinfo | sed 's/.*: //')"
  echo "# driver: gen/wp12_gate.sh $VARIANT (gen/gate.sh's steps unchanged; gen/wp12-shim/cargo; loads recorded by LD_DEBUG=libs, summary in loads.txt)"
  echo "# AK_H2_COALESCE: unset (the h2-batch default, 16)"
  if [ "$VARIANT" = plain ]; then
    echo "# cores: each target directory's own libak_core.so, built in this workspace (stock crates.io h2 0.4.19; poc/rust/Cargo.lock)"
  else
    for k in "$FULL" "$NOUNK"; do
      d="$AK_WP12_CORES/$VARIANT/${k//,/+}"
      echo "# core ($([ "$k" = "$FULL" ] && echo full || echo no-unknown) build, features $k): ${d#$RUST/}/libak_core.so"
      sed 's/^/#     /' "$d/info.txt"
    done
    echo "# the gate's other feature sets (count, concurrency arms, lifecycle, corpus) get their own $VARIANT core the same way; listed at the end"
  fi
  echo
} > "$G"

export PATH="$HERE/wp12-shim:$PATH" AK_GATE_LDDEBUG="$LDD"
[ "$VARIANT" != plain ] && export AK_WP12_VARIANT="$VARIANT" LD_LIBRARY_PATH='$ORIGIN/ak-variant:$ORIGIN/../ak-variant'
rc=0
gen/gate.sh 2>&1 | tee -a "$G" || rc=${PIPESTATUS[0]}
echo "gate rc=$rc" | tee -a "$G"

# ---- after the gate: the RPC checks over TCP (D10), burst_check, and the write-count marker ----
T="$OUT/tcp/checks.log"
{ echo "# WP12 RPC checks over TCP 127.0.0.1, variant $VARIANT, $(date -u +%FT%TZ); in-process server (AK_CHECK_TRANSPORT=tcp: pinned"
  echo "# configuration, TCP_NODELAY on accept) for upload_check / rpc_semantics / header_diff; serve.sh's rpc_server for burst_check"; } > "$T"
lddo() { [ -n "${AK_GATE_LDDEBUG:-}" ] && export LD_DEBUG=libs LD_DEBUG_OUTPUT="$LDD/$1"; return 0; }
unset LD_DEBUG LD_DEBUG_OUTPUT
CARGO_TARGET_DIR="$RUST/target" cargo build --release -q -p campaign --bin burst_check --bin stream_probe --bin upload_check --bin rpc_semantics --bin header_diff
CARGO_TARGET_DIR="$RUST/target-nounk" cargo build --release -q -p campaign --no-default-features --features init-guard --bin upload_check --bin rpc_semantics
trc=0
chk() {  # label, expected last line, command...
  local label=$1 want=$2; shift 2; local f="$OUT/tcp/$label.log" r=0
  lddo "tcp"; "$@" > "$f" 2>&1 || r=$?; unset LD_DEBUG LD_DEBUG_OUTPUT
  if [ "$r" = 0 ] && grep -q "$want" "$f"; then echo "PASS $label: $(grep -cE '^(PASS|ok )' "$f" || true) PASS/ok lines, $(grep -cE '^(FAIL|MISMATCH)' "$f" || true) FAIL; $(grep -m1 "$want" "$f")" >> "$T"
  else echo "FAIL $label (rc $r): see tcp/$label.log; $(grep -E '^FAIL|MISMATCH|DIFFER|panicked' "$f" | head -5 | tr '\n' ' ')" >> "$T"; trc=1; fi
}
export AK_CHECK_TRANSPORT=tcp
chk upload_check-full "UPLOAD CHECK PASSED" target/release/upload_check
chk rpc_semantics-full "RPC SEMANTICS PASSED" target/release/rpc_semantics
chk header_diff-full "send limit" target/release/header_diff
chk upload_check-nounk "UPLOAD CHECK PASSED" target-nounk/release/upload_check
chk rpc_semantics-nounk "RPC SEMANTICS PASSED" target-nounk/release/rpc_semantics
unset AK_CHECK_TRANSPORT
# serve.sh: the shared server, host side (its own target-server core, never substituted)
( unset AK_WP12_VARIANT LD_LIBRARY_PATH; ./serve.sh build >/dev/null )
SS="$SCR/serve-$VARIANT.state"
SV=$(env -u LD_LIBRARY_PATH AK_SERVE_STATE="$SS" AK_SERVER_TCP=0 AK_SERVER_THREADS=4 ./serve.sh start --out "$OUT/tcp/server")
SPID=$(sed -n 's/^pid //p' <<< "$SV"); TL=$(sed -n 's/^tcp //p' <<< "$SV")
echo "# serve.sh: pid $SPID, shipped $(sed -n 's/^shipped //p' <<< "$SV"), pinned $(sed -n 's/^pinned //p' <<< "$SV"), tcp $TL; rpc_server $(ldd target-server/release/rpc_server | grep -q libak_core && echo "needs $(ldd target-server/release/rpc_server | grep -o '/[^ ]*libak_core.so')" || echo "links no libak_core (host side only)")" >> "$T"
AK_RPC_SOCKET_SHIPPED=$(sed -n 's/^shipped //p' <<< "$SV") AK_RPC_SOCKET_PINNED=$(sed -n 's/^pinned //p' <<< "$SV") \
  chk burst_check-uds "BURST CHECK PASSED" target/release/burst_check
AK_RPC_TCP="$TL" chk burst_check-tcp "BURST CHECK PASSED" target/release/burst_check

# the marker: write syscalls per d/16 MiB call, k=1, TCP (stream_probe's io_syscw per call)
M="$OUT/marker.log"
lddo marker
AK_RPC_TARGET="http://$TL" AK_RPC_TRANSPORT=pinned AK_EXPECT_NODELAY=1 AK_OUT="$OUT/tcp/marker.jsonl" AK_PROBE_CELLS=Cf,A \
  AK_PROBE_SIZES=16MiB AK_PROBE_K=1 AK_PROBE_ROUNDS=3 AK_PROBE_CALLS=4 AK_PROBE_WARM=2 AK_PROBE_ORDER=block AK_PROBE_PROC=0 \
  target/release/stream_probe > "$OUT/tcp/marker.out" 2> "$OUT/tcp/marker.err" || trc=1
unset LD_DEBUG LD_DEBUG_OUTPUT
python3 - "$OUT/tcp/marker.jsonl" "$VARIANT" > "$M" <<'EOF'
import json, sys
rows = [json.loads(l) for l in open(sys.argv[1]) if l.startswith("{")]
print("# WP12 marker, variant %s: write syscalls per d/16 MiB call (k=1, TCP 127.0.0.1, server serve.sh pinned listener), stream_probe io_syscw per call per round" % sys.argv[2])
print("# expected: Cf (the core's client) about 1,030 on stock and about 75 on h2-batch (AK_H2_COALESCE 16); A (tonic in the host, crates.io h2) about 1,030 on both")
by = {}
for r in rows: by.setdefault(r["cell"], []).append(r["io_syscw"])
for c, v in by.items(): print("%-10s rounds %d, writes per call: %s" % (c, len(v), ", ".join("%.1f" % x for x in v)))
EOF
cat "$M" >> "$T"
echo "tcp checks rc=$trc" >> "$T"

# ---- the loads: which core every process loaded ----
python3 gen/wp12_loads.py "$LDD" "$RUST" "$VARIANT" "$AK_WP12_CORES" > "$OUT/loads.txt" || rc=$((rc ? rc : 4))
{ echo; echo "===== WP12 =====";
  echo "# cores built for this run (feature set -> core):"
  if [ -s "$AK_WP12_LINKS" ]; then
    while IFS=$'\t' read -r link key dir; do echo "#   ${link#$RUST/} [$key] -> ${dir#$RUST/} sha256 $(sed -n 's/^sha256 //p' "$dir/info.txt" | cut -c1-16), $(sed -n 's/^h2 compiled in: /h2 /p' "$dir/info.txt")"; done < "$AK_WP12_LINKS"
  else echo "#   (none: plain run)"; fi
  tail -1 "$OUT/loads.txt"
  cat "$T"
  echo "WP12 rc: gate $rc, tcp checks $trc"; } | tee -a "$G"
# keep the per-process loader records only as the summary (they are large); a short excerpt per step
for f in $(ls "$LDD" | sed 's/\..*//' | sort -u); do
  g=$(grep -l "calling init: .*libak_core.so" "$LDD/$f".* 2>/dev/null | head -1) || true
  [ -n "$g" ] && { echo "### $f: ${g##*/}"; grep -E "file=.*libak_core|search path=|trying file=.*libak_core|calling init: .*libak_core|initialize program" "$g"; }
done > "$OUT/lddebug-excerpt.txt"
[ "$rc" = 0 ] && [ "$trc" = 0 ]
