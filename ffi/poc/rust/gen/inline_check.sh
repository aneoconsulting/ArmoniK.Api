#!/usr/bin/env bash
# Is `core-native` inlined into the benchmark loop? Answered from the ARTIFACT, the way
# README R5's second half checks that the FFI boundary is real, rather than by assertion.
#
# The claim under audit: `core-native` is compiled into the harness, so rustc fuses the
# traversal into the benchmark closure and keeps writer state in registers, and the FFI arm
# cannot -- so `core-ffi-rust - core-native` charges the inlining advantage to the interface.
#
# The test: a closure that has the traversal inlined into it must be at least as large as
# the traversal. Print both sizes and let the reader do the comparison.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."

cargo build --release -q -p harness --bin bench --bin inlining 2>/dev/null

for BIN in bench inlining; do
  echo "===== target/release/$BIN ====="
  echo
  echo "-- the traversal, as emitted (size in bytes, decimal) --"
  nm -S --demangle target/release/$BIN | grep -E "core_native::(enc_list_results_response|decode_list_results_response|encode_into_list_results_response)$" \
    | while read -r a sz t nm; do printf "  %-10d %s\n" "$((16#$sz))" "$nm"; done
  echo
  echo "-- the benchmark closures, largest first (size in bytes, decimal) --"
  nm -S --demangle target/release/$BIN | grep -E "^[0-9a-f]+ [0-9a-f]+ t ${BIN}::main::\{\{closure\}\}" \
    | while read -r a sz t nm; do printf "  %-10d %s\n" "$((16#$sz))" "$nm"; done | sort -rn | head -5
  echo
  echo "-- direct call sites to the entry points (0 does NOT mean inlined: see below) --"
  objdump -d --no-show-raw-insn target/release/$BIN 2>/dev/null > /tmp/ak_inline_$BIN.asm
  for SYM in $(nm target/release/$BIN | grep -oE "_ZN6facade9generated11core_native[0-9]+(encode_into_list_results_response|decode_list_results_response)[0-9a-zA-Z_]*"); do
    printf "  %-6s %s\n" "$(grep -c "call .*<$SYM>" /tmp/ak_inline_$BIN.asm || true)" "$SYM"
  done
  echo
  echo "-- how the entry points are actually reached --"
  echo "   Both are exported globals in a PIE, so calls to them go through the GOT and the"
  echo "   disassembly shows \`call *0x..(%rip)\`, not a named direct call. The GOT slots:"
  objdump -R target/release/$BIN 2>/dev/null | grep -F "$(nm target/release/$BIN | grep -E "core_native[0-9]+encode_into_list_results_response" | awk '{print $1}' | sed 's/^0*//')" | head -2 || true
  echo
done

echo "===== what to read off this ====="
echo "If the largest benchmark closure is SMALLER than the traversal, no closure can contain"
echo "an inlined copy of it, and \`core-native\` is reached by a real call in that binary --"
echo "which is what the FFI arm does too. The inlining advantage the objection describes"
echo "would then not be present to be subtracted."
