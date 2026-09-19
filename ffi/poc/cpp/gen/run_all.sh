#!/usr/bin/env bash
# One run of everything this slice measures, into ffi/logs/cpp/.
#
# Order matters: correctness gates first at every level and both linkages (R2), then the
# artifact proofs (R5), then the counting build, then the clock. A number taken before the
# gates pass is worse than no number.
set -u
cd "$(dirname "$0")/.." || exit 2
L=../../logs/cpp
mkdir -p "$L"
STAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)
PAY=../../schema/generated/payloads
ROUNDS=${ROUNDS:-9}

hdr() {
  echo "# $1"
  echo "#   date            $STAMP"
  echo "#   machine         $(uname -srm), $(nproc) vCPU, $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //')"
  echo "#   compiler        $(g++ --version | head -1)"
  echo "#   incumbent       protobuf C++ $(protoc --version | awk '{print $2}') (libprotobuf-dev, apt)"
  echo "#                   packages/cpp pins no protobuf version and sets CXX_STANDARD 14"
  echo "#   core            ak-core-cpp, codec emitted by ffi/poc/rust/gen/rust_abi.py (R1)"
  echo "#   rustc           $(rustc --version)"
  echo "#   commit          $(git -C ../../.. rev-parse --short HEAD)"
  echo
}

{
  hdr "cpp slice: correctness (R2), every level and both linkages"
  for b in conformance_a17_shared conformance_b17_shared conformance_c14_shared \
           conformance_c11_shared conformance_a17_static conformance_a17_lossy; do
    echo "===== $b ====="
    (cd ../../schema/generated && "$OLDPWD/build/$b" payloads 2>&1 | grep -v 'libprotobuf ERROR')
    echo
  done
} > "$L/conformance.log" 2>&1

./gen/boundary.sh   > "$L/boundary.log" 2>&1
./gen/odr_check.sh  > "$L/odr.log" 2>&1
./gen/calibrate.sh  > "$L/calibration-r13.log" 2>&1
{ hdr "crossing counts, from the COUNTING core (R5)"; echo "===== shared ====="; ./build/counts_a17_shared; \
  echo; echo "===== static ====="; ./build/counts_a17_static; } > "$L/counts.log" 2>&1

for b in bench_a17_shared bench_a17_static bench_b17_shared bench_c11_shared \
         bench_c14_shared bench_a17_noguard bench_a17_lossy; do
  { hdr "timings: $b"; ./build/$b "$ROUNDS"; } > "$L/$b.log" 2>&1
done
echo "logs in $L:"
ls -la "$L"
