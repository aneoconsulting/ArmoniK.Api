#!/usr/bin/env bash
# README R5, both halves, answered FROM THE BUILT ARTIFACT rather than from a claim.
#
# Half one -- does the FFI arm really cross? With a shared-library core the entry points
# must be UNRESOLVED DYNAMIC IMPORTS; with a static core they are defined in the image and
# must still be reached by a real call instruction.
#
# Half two -- is the no-boundary control fused into the benchmark loop? An arm named "no
# boundary" is only a control if it is not, and in C++ that question is `-flto` over the
# control's own translation unit. Answered two ways: the traversal must exist as an
# out-of-line symbol, and it must be LARGER than every timing closure that calls it, so
# the closure cannot contain it. `bench_a17_shared_lto` is built so the check can be seen
# under the condition that would break it.
set -u
cd "$(dirname "$0")/.." || exit 2
B=build
ok=0; fail=0
chk() { if [ "$1" = ok ]; then ok=$((ok+1)); printf '  [ok]   %s\n' "$2"; \
        else fail=$((fail+1)); printf '  [FAIL] %s\n' "$2"; fi; }
size_of() { nm -t d -S --defined-only "$1" | awk -v s="$2" '$4==s {print $2+0; exit}'; }
csize_of() { nm -t d -S -C --defined-only "$1" | awk -F'[0-9]+ [0-9]+ [a-zA-Z] ' -v s="$2" \
             'index($0, s) {print substr($0,18,16)+0; exit}'; }

echo "== half one: does the FFI arm cross? =="
n=$(nm -D --undefined-only $B/bench_a17_shared | grep -c ' U ak_')
echo "  shared: $n ak_* symbols are UNDEFINED dynamic imports; the per-message ones are:"
nm -D --undefined-only $B/bench_a17_shared | grep ' U ak_encode_\| U ak_decode_\| U ak_elem' | sed 's/^/   /'
[ "$n" -ge 20 ] && chk ok "shared arm: the boundary is a real dynamic import" \
                || chk bad "shared arm: NOT a dynamic import -- every figure from it is about the optimiser"
echo
d=$(nm --defined-only $B/bench_a17_static | grep -c ' T ak_')
echo "  static: $d ak_* symbols are DEFINED in the image (that is what static linkage is)"
for s in ak_encode_ListResultsResponse ak_decode_ListResultsResponse ak_elem_ResultRaw \
         ak_encode_ListTasksDetailedResponse ak_elemu_TaskDetailed; do
  sz=$(size_of $B/bench_a17_static "$s")
  calls=$(objdump -d --no-show-raw-insn $B/bench_a17_static | grep -cE "call[q]?[[:space:]]+[0-9a-f]+ <$s>")
  printf '    %-38s %6s B entry point, %s call sites\n' "$s" "${sz:-?}" "$calls"
  [ "${calls:-0}" -ge 1 ] && chk ok "static arm: $s is CALLED, not inlined away" \
                          || chk bad "static arm: $s has no call site -- this arm IS the control"
done
echo
echo "  static + -flto (R5's named hazard, built on purpose):"
for s in ak_encode_ListResultsResponse ak_elem_ResultRaw; do
  calls=$(objdump -d --no-show-raw-insn $B/counts_a17_static_lto | grep -cE "call[q]?[[:space:]]+[0-9a-f]+ <$s(@plt)?>")
  printf '    %-38s %s call sites under -flto\n' "$s" "$calls"
  [ "${calls:-0}" -ge 1 ] && chk ok "-flto did NOT inline $s away" \
                          || chk bad "-flto inlined $s away: the counting build would still count"
done
echo "    Why: gcc's -flto can only inline across GIMPLE it produced, and a Rust staticlib's"
echo "    archive members are native objects. So R5's C++ hazard does not bite a C++ host"
echo "    over THIS core. That is a fact about the toolchain pair, reported rather than"
echo "    assumed, and it would not hold for a core compiled by the same LTO."
echo
echo "== half two: is the no-boundary control fused into the benchmark loop? =="
for bin in bench_a17_shared bench_a17_shared_lto; do
  echo "  $bin:"
  nm -t d -S -C --defined-only $B/$bin \
    | grep -E 'shapes::native::encode_into_list_(results|tasks_detailed)_response|shapes::native::\(anonymous|enc_(result_raw|task_detailed)' \
    | awk '{printf "    %8d B  %s\n", $2, substr($0, index($0,"shapes"))}' | head -6
  big=$(nm -t d -S -C --defined-only $B/$bin \
        | awk '/shapes::native::encode_into_list_results_response/ {print $2+0; exit}')
  loop=$(nm -t d -S -C --defined-only $B/$bin \
        | awk '/double timed</ {if ($2+0 > m) m = $2+0} END {print m+0}')
  echo "    largest timing closure in the image: $loop B; control traversal: ${big:-0} B"
  if [ "${big:-0}" -gt "${loop:-0}" ]; then
    chk ok "$bin: the traversal is out of line and larger than any closure, so no closure contains it"
  else
    chk bad "$bin: a timing closure is at least as large as the traversal -- check for fusion"
  fi
done
echo "  The control is reached through a FUNCTION POINTER passed to the case runner, so its"
echo "  address is taken and an out-of-line body must exist; the sizes above say the loop is"
echo "  not carrying a copy of it."
echo
echo "boundary: $ok checks passed, $fail failed"
exit $((fail ? 1 : 0))
