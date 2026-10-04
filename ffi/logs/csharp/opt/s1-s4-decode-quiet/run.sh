#!/usr/bin/env bash
# The owner's quiet re-measure of the final run's codec decode-read (2026-10-04): the final run's
# settings (BDN default toolchain, warm-up 25 x 40 ms, 6 rounds x 40 ms, client CPUs 0,1,
# MemoryDiagnoser on, core grid with AK_BDN_DROP=1), decode-read only, the 16 shapes (P7.1
# decode only), Latin-1 and wide on P2.2, the 7 U-* rows; 2 reps, the build order alternated
# (rep 1 full then no-unknown, rep 2 no-unknown then full). Before every process: the 1-minute
# load average must be below 0.5 and no process outside this run above 10 % CPU, waited for up
# to 10 min and recorded in header.txt. No code change: run from the slice at HEAD.
set -uo pipefail
L="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SL=/home/user/ArmoniK.Api/ffi/poc/csharp
S=/tmp/claude-0/-home-user-ArmoniK-Api/0cc1c680-33b0-5d1c-8298-5840c4c24bc0/scratchpad/q
export DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1 AK_CPU_CLIENT=0,1 AK_CAMPAIGN_GRID=core AK_CAMPAIGN_ALLOC=default AK_BDN_MEMORY=1 AK_BDN_DROP=1 AK_BDN_DIRS=decode-read
unset GLIBC_TUNABLES
quiet() {
  for i in $(seq 1 120); do
    la=$(cut -d' ' -f1 /proc/loadavg)
    hot=$(ps -eo pid,pcpu,comm --sort=-pcpu --no-headers | awk -v me=$$ '$2 > 10 && $3 != "ps" {print $3"("$2"%)"}' | head -3 | tr '\n' ' ')
    if awk -v l="$la" 'BEGIN{exit !(l < 0.5)}' && [ -z "$hot" ]; then echo "load1 $la, no other process above 10 % CPU (waited $((i * 5 - 5)) s)"; return 0; fi
    sleep 5
  done
  echo "NOT QUIET after 10 min: load1 $la, hot: $hot"
}
{ echo "# s1-s4 decode-read, quiet re-measure (owner, 2026-10-04): CONTAINER INSTRUMENTATION, not gated (Cases.Verify on)"
  echo "# commit $(git -C $SL rev-parse HEAD); core rebuilt from HEAD (D19 in: last core commit $(git -C $SL log -1 --format=%h -- ../codec/crates)), build-core.log"
  echo "# target-core sha256 $(sha256sum $SL/target-core/release/libak_core.so | cut -d' ' -f1)"
  echo "# target-core-nounk sha256 $(sha256sum $SL/target-core-nounk/release/libak_core.so | cut -d' ' -f1)"
  echo "# SDK $(dotnet --version), runtime 8.0.31; client CPUs 0,1; BDN default toolchain, warm-up 25 x 40 ms, 6 rounds x 40 ms, MemoryDiagnoser on; units incumbent-prod, core-ffi retain/drop, host-gen retain/drop (full build), core-ffi and host-gen no-unknown (no-unknown build)"; } > $L/header.txt
( cd $SL && dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release > $L/build.log 2>&1 && dotnet build src/BenchDotNet/BenchDotNet.csproj -c Release -p:AkNounk=true >> $L/build.log 2>&1 ) || exit 1
dotnet build-server shutdown > /dev/null 2>&1
for r in 1 2; do
  if [ $r = 1 ]; then B="full nounk"; else B="nounk full"; fi
  for b in $B; do
    if [ $b = full ]; then bd=$SL/src/BenchDotNet/bin/Release/net8.0; core=$SL/target-core/release/libak_core.so; else bd=$SL/src/BenchDotNet/bin-nounk/Release/net8.0; core=$SL/target-core-nounk/release/libak_core.so; fi
    cp $core $bd/
    q=$(quiet)
    t0=$(date +%s)
    ( cd $SL && exec taskset -c 0,1 dotnet $bd/BenchDotNet.dll --launch $r --out $L/q-$b-r$r.jsonl --artifacts $S/bdn-$b-$r --rounds 6 --warmup 25 --iteration-ms 40 --toolchain process ) > $L/q-$b-r$r.bdn.log 2>&1
    rc=$?
    echo "# rep $r $b: rc=$rc $(( $(date +%s) - t0 )) s; before it: $q; load1 after: $(cut -d' ' -f1 /proc/loadavg)" | tee -a $L/header.txt
    [ $rc = 0 ] || exit 1
  done
done
python3 $SL/gen/opt_ab_table.py $L > $L/table.md
echo DONE
