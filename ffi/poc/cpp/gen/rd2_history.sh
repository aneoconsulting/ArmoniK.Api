#!/usr/bin/env bash
# R-D2: were logs/cpp/rpc.log and logs/cpp/rpcflow.log taken before or after the core's
# `ak_client_opts` grew past the 3 fields `src/rpc_common.h` declared?
#
# Reads git only. Prints, for every commit that touched the struct, its field list as that
# commit's tree has it, and the date/commit stamps inside the two logs.
set -u
cd "$(dirname "$0")/../../.." || exit 2   # ffi/
RS=ffi/poc/codec/crates/ak-core/src/rpc.rs

fields() {  # <commit>
  git show "$1:$RS" 2>/dev/null | awk '/pub struct ak_client_opts/,/^}/' \
    | sed -n 's/^ *pub \([a-z_]*\): .*/\1/p' | tr '\n' ' '
}

echo "===== git log -S tcp_nagle -- poc/codec ====="
git log -S tcp_nagle --format='%h %ad %s' --date=iso -- poc/codec
echo
echo "===== every commit that touched ak_client_opts (core or this slice), oldest first ====="
for c in $(git log --reverse --format=%h -S ak_client_opts -- poc/codec poc/cpp/src/rpc_common.h); do
  printf '%s  %s\n' "$(git show -s --format='%h %ad %an' --date=iso "$c")" "$(git show -s --format=%s "$c")"
  f=$(fields "$c"); n=$(echo "$f" | wc -w)
  echo "    core fields ($n): ${f:-<none>}"
  h=$(git show "$c:ffi/poc/cpp/src/rpc_common.h" 2>/dev/null | grep -o 'struct ak_client_opts {[^}]*}' | head -1)
  echo "    cpp host decl:   ${h:-<none in tree>}"
done
echo
echo "===== the two logs: when they were taken, on what, and when committed ====="
for l in logs/cpp/rpc.log logs/cpp/rpcflow.log; do
  echo "-- $l"
  grep -m1 -E '^#   date' "$l"
  grep -m1 -E '^#   commit' "$l"
  git log --format='   committed in %h %ad %s' --date=iso -- "$l" | head -1
done
echo
echo "===== ancestry ====="
for a in bccff35 adfc1ad 908dc24 ef8fea9; do
  if git merge-base --is-ancestor "$a" af2b100; then r="IS"; else r="is NOT"; fi
  echo "$a $r an ancestor of af2b100 (the commit that added both logs)"
done
echo
echo "===== RPC logs committed by this slice after the 5/6-field struct landed ====="
git log --format='%h %ad %s' --date=iso 908dc24..HEAD -- logs/cpp/rpc.log logs/cpp/rpcflow.log
echo "(end; empty means none)"
