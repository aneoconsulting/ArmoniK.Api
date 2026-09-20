#!/usr/bin/env bash
# design/SHAPES.md's content sets, on WHOLE PAYLOADS.
#
# "A slice that reports one string-path number without saying which content set it came
# from has reported half a number." This slice priced the string path over all three sets
# and every whole-payload row over ASCII only, so the whole-payload rows were the half
# number the sentence is about.
#
# Correctness first and per set, because no manifest oracle covers latin1 or wide: every
# arm is byte-identical to the INCUMBENT, and the incumbent is itself anchored to
# manifest.json on ascii, so the oracle is not unanchored on the one set where it can be
# anchored.
set -u
cd "$(dirname "$0")/.."
./build/contentsets_a17 "${1:-9}"
