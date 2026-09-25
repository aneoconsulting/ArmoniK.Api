#!/usr/bin/env bash
# WP5 step 10: the C header's two variants, each checked against the core built its way.
#
#   gen/c_variant.sh FULL_SO NOUNK_SO
#
# Renders ak_abi.h / ak_layout.h / ak_layout_names.h with poc/codec/gen/c_abi.py from the
# plan (full) and from the plan relowered with unknown="drop" (the no-unknown variant) into
# a scratch directory, compiles each as C99 and C++11, and runs a C++ host that compares
# AK_LAYOUT_HOST with the core's ak_layout_facts(). Both matched pairs must agree, and both
# MISMATCHED pairs must disagree (the check seen failing): a host that selects the wrong
# header for its core is caught at the first call.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CODEC="$HERE/../../codec/gen"
FULL_SO="$1"; NOUNK_SO="$2"
T="$(mktemp -d)"; trap 'rm -rf "$T"' EXIT
python3 -S - "$CODEC" "$T" <<'PY'
import sys, os
sys.path.insert(0, sys.argv[1])
import plan as P, c_abi, generate
p = P.load(generate.ROOTS)
for d, pp in (("full", p), ("nounk", P.relower(p, p.options.with_unknown("drop")))):
    os.makedirs(os.path.join(sys.argv[2], d))
    for n, x in zip(("ak_abi.h", "ak_layout.h", "ak_layout_names.h"), c_abi.emit(pp)):
        open(os.path.join(sys.argv[2], d, n), "w").write(x)
PY
cat > "$T/host.cc" <<'CC'
#include <cstdio>
#include "ak_layout.h"
#include "ak_layout_names.h"
int main() {
  static uint32_t core[4096];
  size_t n = ak_layout_facts(core, 4096);
#ifdef AK_NO_UNKNOWN_FIELDS
  const char *v = "no-unknown";
#else
  const char *v = "full";
#endif
  if (n != AK_LAYOUT_FACTS) { std::printf("  %s header: %d facts, core exports %zu\n", v, AK_LAYOUT_FACTS, n); return 1; }
  for (size_t i = 0; i < n; i++)
    if (core[i] != AK_LAYOUT_HOST[i]) { std::printf("  %s header: %s host %u core %u\n", v, AK_LAYOUT_NAMES[i], AK_LAYOUT_HOST[i], core[i]); return 1; }
  std::printf("  %s header: %zu layout facts agree with the core\n", v, n);
  return 0;
}
CC
printf '#include "ak_abi.h"\nint main(void){return 0;}\n' > "$T/c99.c"
for v in full nounk; do
  gcc -std=c99 -Wall -Werror -I"$T/$v" -c "$T/c99.c" -o "$T/c99-$v.o"
  g++ -std=c++11 -Wall -Werror -I"$T/$v" -c "$T/host.cc" -o "$T/host-$v.o"
done
echo "  both headers compile as C99 and C++11 (-Wall -Werror)"
cpso() { mkdir -p "$T/so-$1"; cp "$2" "$T/so-$1/libak_core.so"; }
cpso full "$FULL_SO"; cpso nounk "$NOUNK_SO"
pair() { # header-variant core-variant
  g++ -std=c++11 -I"$T/$1" "$T/host.cc" -L"$T/so-$2" -lak_core -Wl,-rpath,"$T/so-$2" -o "$T/h-$1-$2"
  "$T/h-$1-$2"; }
pair full full
pair nounk nounk
for x in "full nounk" "nounk full"; do
  set -- $x
  if out=$(pair "$1" "$2"); then echo "  MISMATCH NOT CAUGHT: $1 header on the $2 core"; exit 1; fi
  echo "  mismatch caught ($1 header, $2 core):${out#*header:}"
done
