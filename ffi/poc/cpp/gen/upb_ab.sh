#!/usr/bin/env bash
# Experiment 1: the UPB_FASTTABLE A/B, and the compiler separated from it.
#
# `upb/port/def.inc:227-239` defaults UPB_FASTTABLE to 0; it is 1 only under
# -DUPB_ENABLE_FASTTABLE, or -DUPB_TRY_ENABLE_FASTTABLE where UPB_MUSTTAIL exists. The
# first upb column in this slice defined neither and did not say so, which is an R7
# omission. Three builds of identical sources, so the compiler and the define are
# separated rather than confounded:
#
#   build/          cc (gcc 13.3.0), UPB_FASTTABLE=0   -- what the first column measured
#   build-upbclang/ clang 18,        UPB_FASTTABLE=0
#   build-upbft/    clang 18,        UPB_FASTTABLE=1   -- clang because UPB_MUSTTAIL needs
#                                                        __attribute__((musttail))
#
# Each binary carries its own in-process pb and pb-arena arms, so each ratio is formed
# inside one process (R4) and only the ratios are compared across the three.
set -u
cd "$(dirname "$0")/.." || exit 2
R=${1:-9}
for d in build build-upbclang build-upbft; do
  [ -x "$d/upbbench" ] || { echo "missing $d/upbbench -- see gen/fetch_upb.sh"; continue; }
  echo "===== $d ====="
  ./"$d"/upbbench "$d"/gen/shapes.desc "$R" 2>&1 \
    | grep -E "compiler |UPB_FASTTABLE|table_mask|^payload|   dec "
  echo
done
echo "The fast dispatch is UNREACHABLE from a reflection-built minitable, whatever"
echo "UPB_FASTTABLE is: upb/wire/decode.c:766 fires only when table_mask != -1, and"
echo "upb/mini_descriptor/decode.c:698,712 sets table_mask = -1 on every minitable it"
echo "builds. The fasttable entries come from protoc-gen-upb's UPB_FASTTABLE_INIT and"
echo "from nothing else. The runtime table_mask above is the proof, and the fast-parse"
echo "function count printed by gen/fetch_upb.sh is the proof that the code IS compiled"
echo "in when the define is set -- so the null result is a reachability fact, not a"
echo "build that silently did nothing."
