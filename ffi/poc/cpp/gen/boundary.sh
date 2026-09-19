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
ok=0; fail=0; ctrl_fired=0
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
    | grep -E 'shapes::native::(encode_into|decode)_list_(results|tasks_detailed|probe|metrics)_response|shapes::native::(enc|dec)_(result_raw|task_detailed|probe|metrics_batch)' \
    | awk '{printf "    %8d B  %s\n", $2, substr($0, index($0,"shapes"))}' | sort -rn | head -10
  loop=$(nm -t d -S -C --defined-only $B/$bin \
        | awk '/double timed</ {if ($2+0 > m) m = $2+0} END {print m+0}')
  echo "    largest timing closure in the image: $loop B"
  # BOTH directions. README asks for both and the first version of this script printed
  # encode symbols only -- while decode is the direction where this control misbehaves,
  # so it is the direction where "is it fused?" most needed answering.
  # The per-message traversals the entry points call. `decode_list_X` is a 94 B shim that
  # calls an out-of-line `dec_list_X`, so the shim's size is not the question -- the
  # traversal's is, and it is these.
  for sym in 'shapes::native::encode_into_list_results_response' \
             'shapes::native::enc_task_detailed' \
             'shapes::native::dec_list_results_response' \
             'shapes::native::dec_list_tasks_detailed_response' \
             'shapes::native::decode_list_metrics_response'; do
    sz=$(nm -t d -S -C --defined-only $B/$bin | awk -v s="$sym" 'index($0, s) {print $2+0; exit}')
    if [ -z "$sz" ]; then
      echo "    $sym: ABSENT (inlined into its caller within the control TU, which is fine:"
      echo "      the question is whether the BENCHMARK LOOP contains it, and the loop is"
      echo "      in another TU with no LTO in the default build)"
      continue
    fi
    if [ "$bin" = bench_a17_shared_lto ]; then
      # The POSITIVE CONTROL binary. `-flto` is the condition that would break the
      # control, and it is built so the check can be seen under it. A fire HERE is the
      # desired outcome and is not a failure of the slice: no figure comes from this
      # binary. What it demonstrates is the trend -- under LTO the traversals shrink and
      # the timing closures grow, which is the direction that ends in fusion.
      if [ "$sz" -gt "${loop:-0}" ]; then
        echo "    [control] $sym is $sz B, still larger than any closure ($loop B)"
      else
        echo "    [control FIRED] $sym is only $sz B against a $loop B closure -- which is"
        echo "      what the check is for, and why no figure comes from this binary"
        ctrl_fired=$((ctrl_fired + 1))
      fi
      continue
    fi
    if [ "$sz" -gt "${loop:-0}" ]; then
      chk ok "$bin: $sym is $sz B, larger than any timing closure ($loop B)"
    else
      chk bad "$bin: $sym is only $sz B, smaller than a timing closure -- check for fusion"
    fi
  done
done
echo "  The control is reached through a FUNCTION POINTER passed to the case runner, so its"
echo "  address is taken and an out-of-line body must exist; the sizes above say the loop is"
echo "  not carrying a copy of it."
echo
echo "boundary: $ok checks passed, $fail failed"
if [ "$ctrl_fired" -gt 0 ]; then
  echo "and the -flto POSITIVE CONTROL fired on $ctrl_fired symbols, which is what says the"
  echo "check above is capable of failing. No figure in this slice comes from that binary."
else
  echo "WARNING: the -flto positive control did NOT fire, so the check above has not been"
  echo "seen failing on this build. Treat the pass as unproven."
fi
exit $((fail ? 1 : 0))
