#!/usr/bin/env bash
# FIX-PLAN WP10 (CAMPAIGN req 13 as amended at 9f6d579fa): THE one RPC server of every slice.
# The interface a client codes against is poc/rust/SERVER.md; this is how any slice's runner
# starts, warms and stops it. No dependency on the caller's slice.
#
#   serve.sh build                 build rpc_server and rpc_warm once (release, their own
#                                  target directory, poc/rust/target-server/)
#   serve.sh start --out DIR       start ONE server process, pinned to AK_CPU_SERVER (taskset;
#                                  unpinned if unset), on two Unix sockets: the shipped and the
#                                  pinned server configuration, in a short mktemp directory.
#                                  Prints `shipped PATH`, `pinned PATH` and `pid PID`; the log
#                                  goes to DIR/rpc-server.log; the state to $AK_SERVE_STATE
#   serve.sh warm N                N checked calls per direction (a, b, c; d: ceil(N/4)) from a
#                                  tonic client and from the core's client, on BOTH sockets
#   serve.sh stop                  stop it and remove its socket directory
#
# Environment: AK_CPU_SERVER (taskset list), AK_SERVER_THREADS (tokio workers, default
# AK_WORKERS, else 8: D14),
# AK_SERVE_STATE (default ${TMPDIR:-/tmp}/ak-rpc-server.state), AK_SERVER_TCP (unset = Unix
# sockets only; a port, 0 = any free port: also a TCP listener on 127.0.0.1 with the pinned
# configuration and TCP_NODELAY; start prints and the state file holds `tcp 127.0.0.1:PORT`;
# warm then warms it too).
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TGT="$HERE/target-server"
STATE=${AK_SERVE_STATE:-${TMPDIR:-/tmp}/ak-rpc-server.state}
SRV="$TGT/release/rpc_server"; WARM="$TGT/release/rpc_warm"

cmd=${1:-}; shift || true
case "$cmd" in
  build)
    # gen/cargo-shim: the build directory is the target directory, whatever build.build-dir
    # the machine's cargo configuration sets (so rpc_server loads target-server's own core).
    ( cd "$HERE" && PATH="$HERE/gen/cargo-shim:$PATH" CARGO_TARGET_DIR="$TGT" cargo build --release -q -p campaign --bin rpc_server --bin rpc_warm )
    echo "built $SRV and $WARM" ;;

  start)
    OUT=""
    while [ $# -gt 0 ]; do case "$1" in --out) OUT=$2; shift 2 ;; *) echo "usage: serve.sh start --out DIR" >&2; exit 2 ;; esac; done
    [ -n "$OUT" ] || { echo "usage: serve.sh start --out DIR" >&2; exit 2; }
    [ -x "$SRV" ] || { echo "serve.sh: no $SRV; run serve.sh build first" >&2; exit 1; }
    if [ -f "$STATE" ] && kill -0 "$(sed -n 's/^pid //p' "$STATE")" 2>/dev/null; then
      echo "serve.sh: a server is already running ($STATE); serve.sh stop first" >&2; exit 1
    fi
    mkdir -p "$OUT"
    # A short directory: a Unix socket path must fit sun_path (107 bytes on Linux).
    D=$(mktemp -d /tmp/aksrv.XXXXXX)
    S1="$D/shipped.sock"; S2="$D/pinned.sock"; RF="$D/ready"
    PIN=(); [ -n "${AK_CPU_SERVER:-}" ] && PIN=(taskset -c "$AK_CPU_SERVER")
    TCP=(); [ -n "${AK_SERVER_TCP:-}" ] && TCP=(--tcp "$AK_SERVER_TCP")
    AK_SERVER_THREADS=${AK_SERVER_THREADS:-${AK_WORKERS:-8}} "${PIN[@]}" "$SRV" --socket-shipped "$S1" --socket-pinned "$S2" "${TCP[@]}" \
      --ready-file "$RF" > /dev/null 2> "$OUT/rpc-server.log" < /dev/null &
    SP=$!
    for _ in $(seq 200); do [ -s "$RF" ] && break; kill -0 $SP 2>/dev/null || break; sleep 0.05; done
    [ -s "$RF" ] || { echo "serve.sh: rpc_server did not start (see $OUT/rpc-server.log)" >&2; kill $SP 2>/dev/null || true; rm -rf "$D"; exit 1; }
    TL=$(sed -n 's/^tcp //p' "$RF")
    printf 'shipped %s\npinned %s\npid %s\ndir %s\n' "$S1" "$S2" "$SP" "$D" > "$STATE"
    [ -n "$TL" ] && printf 'tcp %s\n' "$TL" >> "$STATE"
    printf 'shipped %s\npinned %s\npid %s\n' "$S1" "$S2" "$SP"
    [ -n "$TL" ] && printf 'tcp %s\n' "$TL"
    true ;;

  warm)
    N=${1:?usage: serve.sh warm N}
    [ -f "$STATE" ] || { echo "serve.sh: no server running ($STATE)" >&2; exit 1; }
    for T in shipped pinned; do
      "$WARM" --socket "$(sed -n "s/^$T //p" "$STATE")" --transport "$T" --n "$N"
    done
    TL=$(sed -n 's/^tcp //p' "$STATE")
    if [ -n "$TL" ]; then "$WARM" --target "http://$TL" --transport pinned --n "$N"; fi ;;

  stop)
    [ -f "$STATE" ] || { echo "serve.sh: no server running ($STATE)"; exit 0; }
    SP=$(sed -n 's/^pid //p' "$STATE"); D=$(sed -n 's/^dir //p' "$STATE")
    kill "$SP" 2>/dev/null || true
    for _ in $(seq 100); do kill -0 "$SP" 2>/dev/null || break; sleep 0.05; done
    kill -9 "$SP" 2>/dev/null || true
    [ -n "$D" ] && rm -rf "$D"
    rm -f "$STATE"
    echo "stopped $SP" ;;

  *) echo "usage: serve.sh build | start --out DIR | warm N | stop" >&2; exit 2 ;;
esac
