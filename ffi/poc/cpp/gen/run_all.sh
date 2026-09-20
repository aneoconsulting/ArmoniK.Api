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
  echo "#   core            THE shared ak-core at ffi/poc/codec (R0), cdylib + staticlib;"
  echo "#                   codec emitted by ffi/poc/codec/gen/rust_abi.py (R1)"
  echo "#   rustc           $(rustc --version)"
  echo "#   commit          $(git -C ../../.. rev-parse --short HEAD)"
  echo
}

# R1 first, and it is a GATE rather than a claim: `--check` fails if what is committed is
# not what the generator would write, which is the whole of "no hand-written codec anywhere
# in the comparison". It was run by nothing before, and its output was in no log.
{
  hdr "cpp slice: R1 and the generator's guards"
  echo "===== generate.py --check: is what is committed what the generator writes? ====="
  python3 gen/generate.py --check
  gen_rc=$?
  echo "generate.py --check exit $gen_rc"
  echo
  echo "===== refusal_test.py: every guard run against input it must REJECT ====="
  python3 gen/refusal_test.py
  echo "refusal_test.py exit $?"
  echo
  echo "===== audit_tracked.sh: R4's closing rule, asked of git ====="
  ./gen/audit_tracked.sh
  echo "audit_tracked.sh exit $?"
  echo
  echo "===== utf8.sh: the decode policy's validator, against an independent oracle ====="
  # Four implementations of one predicate, every 1-, 2- and 3-byte string exhaustively,
  # plus a structured 4-byte sweep and every named malformed class. 17.8 million checks.
  bash gen/utf8.sh
  echo "utf8.sh exit $?"
  echo
  echo "===== contentsets.sh: SHAPES.md's three sets, on whole payloads ====="
  # Correctness per set first: no manifest oracle covers latin1 or wide, so every arm is
  # checked against the INCUMBENT, which is itself anchored to manifest.json on ascii.
  bash gen/contentsets.sh
  echo "contentsets.sh exit $?"
  echo
  echo "===== concurrency.sh: ABI v1 obligation 12.5 ====="
  # Two axes and four payload shapes, plus three PLANTED builds of the designs section 6
  # refused, each of which must fail.
  bash gen/concurrency.sh
  echo "concurrency.sh exit $?"
  echo
  echo "===== one_core.sh: R0, and the proof that R0 can fail ====="
  # The shared core's gate, not this slice's, but it runs here because this is where the
  # gates run and a rule checked by nobody is a rule that gets broken again. --selftest
  # plants five violations in a scratch copy of what git tracks and requires each to fail.
  bash ../codec/gen/one_core.sh --selftest
  echo "one_core.sh --selftest exit $?"
} > "$L/generator.log" 2>&1

{
  hdr "cpp slice: correctness (R2), every level and both linkages"
  for b in conformance_a17_shared conformance_b17_shared conformance_c14_shared \
           conformance_c11_shared conformance_a17_static conformance_a17_lossy; do
    echo "===== $b ====="
    (cd ../../schema/generated && "$OLDPWD/build/$b" payloads 2>&1 | grep -v 'libprotobuf ERROR')
    echo
  done
} > "$L/conformance.log" 2>&1

# C24: the GROUP skip. Its own log, because it is the one decode path a corpus generated
# from the schema that reads it can never reach, and because two PLANTED builds have to be
# seen failing for the test to mean anything.
{
  hdr "cpp slice: ak::Dec::skip over the deprecated GROUP form (C24)"
  bash gen/groupskip.sh
  echo "groupskip.sh exit $?"
} > "$L/groupskip.log" 2>&1

# W8: the conformance corpus. The oracle byte identity against a schema-generated manifest
# cannot be. Target level and the C++11 floor, because the floor is a correctness gate.
{
  hdr "cpp slice: the conformance corpus (W8)"
  echo "===== C++17 target, shared ====="
  python3 gen/corpus.py build/corpus_a17_shared
  echo "corpus.py (a17) exit $?"
  echo
  echo "===== C++11 floor, shared ====="
  python3 gen/corpus.py build/corpus_c11_shared
  echo "corpus.py (c11) exit $?"
} > "$L/corpus.log" 2>&1

./gen/boundary.sh   > "$L/boundary.log" 2>&1
./gen/odr_check.sh  > "$L/odr.log" 2>&1
./gen/calibrate.sh  > "$L/calibration-r13.log" 2>&1
{ hdr "crossing counts, from the COUNTING core (R5)"; echo "===== shared ====="; ./build/counts_a17_shared; \
  echo; echo "===== static ====="; ./build/counts_a17_static; } > "$L/counts.log" 2>&1

for b in bench_a17_shared bench_a17_static bench_b17_shared bench_c11_shared \
         bench_c14_shared bench_a17_noguard bench_a17_lossy; do
  { hdr "timings: $b"; ./build/$b "$ROUNDS"; } > "$L/$b.log" 2>&1
done

{ hdr "R4's across-build control: the same source, a neutral layout perturbation"
  ./gen/drift.sh "$ROUNDS"; } > "$L/drift.log" 2>&1

{ hdr "ABI v1 decision 1: the batching predicate at a DEARER crossing"
  ./gen/tax.sh 5; } > "$L/tax.log" 2>&1

{ hdr "is the control's decode gap a function of the optimisation level?"
  ./gen/opt.sh 5; } > "$L/opt.log" 2>&1

# Optional arms: they need libgrpc++-dev and a upb build, and neither is wanted by the
# codec gates. Run only if their binaries exist.
if [ -x build/rpcbench ]; then
  { hdr "the RPC arm (ABI v1 section 9)"
    echo "#   incumbent       grpc++ $(pkg-config --modversion grpc++ 2>/dev/null)"
    ./gen/rpc.sh 40; } > "$L/rpc.log" 2>&1
fi
if [ -x build/upbbench ]; then
  { hdr "the upb arm: a CEILING, not a candidate"
    echo "#   upb             protobuf $(cat build/upb-src/.git/HEAD 2>/dev/null | head -c 8), built by gen/fetch_upb.sh"
    ./build/upbbench build/gen/shapes.desc "$ROUNDS"; } > "$L/upb.log" 2>&1
fi
echo "logs in $L:"
ls -la "$L"
