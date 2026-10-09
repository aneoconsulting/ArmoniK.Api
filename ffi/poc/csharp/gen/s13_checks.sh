#!/usr/bin/env bash
# s13 checks (net8.0; CORRECTNESS ONLY): the scalar-transcoder cores (gen/s13_cores.sh).
#   gen/s13_checks.sh   (BenchDotNet and Corpus already built, full build)
# Per core (default simdutf, tc-scalar-naive, tc-scalar-word): Cases.Verify (BenchDotNet
# --verify: every payload, content set and U-* row, every string path incl. E3) with the
# process's path E0, E1R and E1R:128; the corpus under E1R (retain strict) with the matching
# corpus core. The loaded core is identified by its sha256.
set -uo pipefail
SLICE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SLICE"
B="$SLICE/src/BenchDotNet/bin/Release/net8.0"; C="$SLICE/src/Corpus/bin/Release/net8.0"
FAILS=0
run() { local name="$1"; shift; echo "\$ $*"; "$@" 2>&1 | tail -n 3; local rc=${PIPESTATUS[0]}; echo "exit status: $rc"; [ $rc -eq 0 ] || { echo "CHECK: $name FAILED"; FAILS=$((FAILS+1)); }; }
echo "# s13_checks.sh at $(git rev-parse --short HEAD)$(git diff --quiet HEAD -- src gen ../codec || echo ' + uncommitted'); $(date -u +%FT%TZ)"
for v in "target-core target-core-corpus default-simdutf" "target-core-tcnaive target-core-corpus-tcnaive tc-scalar-naive" "target-core-tcword target-core-corpus-tcword tc-scalar-word"; do
  set -- $v
  cp "$1/release/libak_core.so" "$B/"; cp "$2/release/libak_core.so" "$C/"
  echo "## $3: shapes core $(sha256sum "$B/libak_core.so" | cut -c1-16), corpus core $(sha256sum "$C/libak_core.so" | cut -c1-16)"
  for m in E0 E1R E1R:128; do AK_STR_ENC=$m run "verify $3 $m" dotnet "$B/BenchDotNet.dll" --verify; done
  AK_STR_ENC=E1R AK_CORPUS_RETAIN_STRICT=1 run "corpus $3 E1R" "$C/corpus"
done
cp target-core/release/libak_core.so "$B/"; cp target-core-corpus/release/libak_core.so "$C/"
echo
[ $FAILS -eq 0 ] && echo "S13 CHECKS PASSED (net8.0; not the gate)" || echo "S13 CHECKS: $FAILS FAILED"
exit $FAILS
