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
echo
echo "  C18. This used to ask whether the control function was LARGER than the largest"
echo "  timing closure in the image and take that as evidence it was not copied into one."
echo "  Size is a proxy for fusion, not a test of it, and its positive control was -flto,"
echo "  which fires only when the optimiser happens to fuse something. After the core moved"
echo "  (W10) the largest closure went from 1433 B to 911 B, the 1155 B control landed on"
echo "  the other side of the line, and the control went quiet without anyone deciding it"
echo "  should. A size-threshold control is fragile by construction."
echo
echo "  Two direct properties instead, and a fixture that proves the checker can report"
echo "  both. The control is reached through a FUNCTION POINTER handed to the case runner,"
echo "  so there is no direct call site to count -- what fusion would destroy is (1) the"
echo "  out-of-line body and (2) the call instruction in the closure."
echo

# Count call instructions inside one symbol's disassembly.
calls_in() {  # $1 = binary, $2 = exact symbol name
  objdump -d --no-show-raw-insn -C "$1" 2>/dev/null \
    | awk -v s="$2" '
        /^[0-9a-f]+ <.*>:$/ { inside = (index($0, "<" s ">:") > 0); next }
        # NOT /\<call\>/. mawk is what is installed and \< \> are gawk-only word
        # boundaries -- mawk silently matches nothing, so the counter returned 0 for every
        # closure and half two reported total fusion. The same gawk-only trap as strtonum,
        # in the same file, twice.
        inside && /[ \t]call/ { n++ }
        END { print n + 0 }'
}

# Does this symbol have an out-of-line body, and how big is it?
body_of() {   # $1 = binary, $2 = symbol substring
  nm -t d -S -C --defined-only "$1" 2>/dev/null \
    | awk -v s="$2" 'index($0, s) { print $2 + 0; exit }'
}

echo "  the checker's own control (src/fusion_probe.cpp): one function that MUST be called"
echo "  and one that MUST be fused, both by construction rather than by optimisation level"
if [ -x "$B/fusion_probe" ]; then
  c_called=$(calls_in "$B/fusion_probe" "ak_probe_loop_called")
  c_fused=$(calls_in "$B/fusion_probe" "ak_probe_loop_fused")
  b_called=$(body_of "$B/fusion_probe" " ak_probe_called")
  if [ "${c_called:-0}" -ge 1 ]; then
    chk ok "the counter sees the call in ak_probe_loop_called ($c_called)"
  else
    chk bad "the counter reports NO call in ak_probe_loop_called -- the counter is broken"
  fi
  if [ "${c_fused:-1}" -eq 0 ]; then
    chk ok "the counter sees fusion: ak_probe_loop_fused has 0 calls"
  else
    chk bad "ak_probe_loop_fused has $c_fused calls -- it was supposed to be inlined, so"
    chk bad "  the control proves nothing"
  fi
  if [ -n "$b_called" ] && [ "$b_called" -gt 0 ]; then
    chk ok "ak_probe_called has an out-of-line body ($b_called B)"
  else
    chk bad "ak_probe_called has no out-of-line body -- the body test is broken"
  fi
else
  chk bad "fusion_probe is not built, so half two has no control and proves nothing"
fi
echo

for bin in bench_a17_shared bench_a17_shared_lto; do
  [ -x "$B/$bin" ] || continue
  echo "  $bin:"
  # (1) every timing closure must still contain a call. A closure with none has absorbed
  #     whatever it was timing.
  zero=0
  total=0
  while IFS= read -r sym; do
    total=$((total + 1))
    n=$(calls_in "$B/$bin" "$sym")
    [ "${n:-0}" -eq 0 ] && { zero=$((zero + 1)); echo "      ZERO calls: $sym"; }
  done < <(nm -C --defined-only "$B/$bin" | sed -n 's/^[0-9a-f]* [tT] \(double timed<.*\)$/\1/p')
  if [ "$zero" -eq 0 ]; then
    chk ok "$bin: all $total timing closures still call out"
  else
    chk bad "$bin: $zero of $total timing closures contain no call at all"
  fi
  # (2) every control traversal must still have an out-of-line body.
  for sym in 'shapes::native::encode_into_list_results_response' \
             'shapes::native::enc_task_detailed' \
             'shapes::native::dec_list_results_response' \
             'shapes::native::dec_list_tasks_detailed_response' \
             'shapes::native::decode_list_metrics_response'; do
    sz=$(body_of "$B/$bin" "$sym")
    if [ -z "$sz" ]; then
      echo "      $sym: ABSENT -- inlined into its caller WITHIN the control TU, which is"
      echo "        fine; the question is whether the benchmark loop carries it, and the"
      echo "        loop is in another TU"
    elif [ "$sz" -gt 0 ]; then
      chk ok "$bin: $sym has an out-of-line body ($sz B)"
    else
      chk bad "$bin: $sym has a zero-length body"
    fi
  done
done
echo "  The control is reached through a FUNCTION POINTER passed to the case runner, so its"
echo "  address is taken and an out-of-line body must exist. Both properties above are the"
echo "  question itself rather than a proxy for it, and the fixture says the checker can"
echo "  report either answer."
echo
echo "boundary: $ok checks passed, $fail failed"
if [ "$ctrl_fired" -gt 0 ]; then
  echo "and the -flto positive control fired on $ctrl_fired symbols."
fi
exit $((fail ? 1 : 0))
