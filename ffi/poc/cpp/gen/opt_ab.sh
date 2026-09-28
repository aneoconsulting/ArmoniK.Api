#!/usr/bin/env bash
# A narrowed, ALTERNATED A/B of two snapshots (gen/opt_snap.sh), pinned as opt_bench.sh pins:
#   gen/opt_ab.sh OUT_DIR SNAP_A SNAP_B codec|codec_nounk ARGS...    (campaign_codec arguments)
#   gen/opt_ab.sh OUT_DIR SNAP_A SNAP_B rpc|rpc_nounk ARGS...        (campaign_rpc arguments; the
#                                                                     server is started here)
# Runs A B A B ... (PAIRS, default 3), one process each, then gen/opt_ab.py OUT_DIR.
# Instrumentation, not gated (each process's own pre-check stays on).
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FFI=$(cd "$HERE/../.." && pwd)
OUT=$1 A=$2 B=$3 KIND=$4; shift 4
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
PAIRS=${PAIRS:-3}
CPU=${AK_CPU_CLIENT:-1}; export AK_CPU_SERVER=${AK_CPU_SERVER:-2,3}
SCR=$(mktemp -d); export AK_SERVE_STATE=$SCR/serve.state
trap '[ -f "$AK_SERVE_STATE" ] && bash "$FFI/poc/rust/serve.sh" stop > /dev/null 2>&1; rm -rf "$SCR"' EXIT
python3 "$HERE/gen/u_rows.py" "$FFI/corpus/generated" "$SCR/rows.tsv" 2>/dev/null
case $KIND in codec) V=full; X=campaign_codec ;; codec_nounk) V=nounk; X=campaign_codec_nounk ;;
  rpc) V=full; X=campaign_rpc ;; rpc_nounk) V=nounk; X=campaign_rpc_nounk ;; *) echo "kind?" >&2; exit 2 ;; esac
EXTRA=()
if [ "${KIND#rpc}" != "$KIND" ]; then
  bash "$FFI/poc/rust/serve.sh" start --out "$SCR/srv" > "$SCR/srv.out" 2>&1 || { cat "$SCR/srv.out"; exit 1; }
  taskset -c "$CPU" bash "$FFI/poc/rust/serve.sh" warm 50 > /dev/null 2>&1 || { echo "warm failed"; exit 1; }
  T=shipped; for a in "$@"; do [ "$a" = pinned ] && T=pinned; done
  EXP=$(sed -n 's/.*P2.2 \([0-9]*\) B.*/\1/p' "$SCR/srv/rpc-server.log" | head -1)
  EXTRA=(--target "unix:$(awk -v t=$T '$1==t{print $2}' "$SCR/srv.out")" --expect "$EXP")
else
  EXTRA=(--corpus "$FFI/corpus/generated" --rows "$SCR/rows.tsv")
fi
{ echo "# opt_ab: $KIND A=$A ($(head -1 "$A/REV")) B=$B ($(head -1 "$B/REV")) pairs=$PAIRS args: $*"; } > "$OUT/ab.head"
for i in $(seq 1 "$PAIRS"); do
  for side in A B; do
    S=$A; [ $side = B ] && S=$B
    ( cd "$FFI/schema/generated" && LD_LIBRARY_PATH="$S/$V" taskset -c "$CPU" "$S/$V/$X" "${EXTRA[@]}" "$@" \
        --gbench-out "$OUT/$side-$i.json" > "$OUT/$side-$i.console" 2>&1 ) \
      || { echo "run $side-$i failed:"; tail -5 "$OUT/$side-$i.console"; exit 1; }
    grep -m1 -o '"campaign_codec_gate": {[^}]*}' "$OUT/$side-$i.console" || true
  done
done
python3 "$HERE/gen/opt_ab.py" "$OUT"
for f in "$OUT"/[AB]-*.json; do gzip -9f "$f"; done
for f in "$OUT"/[AB]-*.console; do grep '^#' "$f" > "$f.head" 2>/dev/null; rm -f "$f"; done
