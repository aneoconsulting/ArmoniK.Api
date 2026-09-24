#!/usr/bin/env bash
# R-D1, the C++ half: the corpus's length-wrap rows (WP4 item 2, `X-lenwrap-*`), ONE ROW
# PER PROCESS under `timeout 5`, through the NATIVE arm, before and after the fix.
#
#   before  a scratch build of the corpus binary whose `include/ak/rt.h` is the one at
#           $BEFORE_REV (default: HEAD, i.e. the unfixed comparison `pos + k > len`).
#           Built in build/lenwrap-before/ with that header's directory FIRST on the
#           include path; nothing in the tree is touched.
#   after   build/corpus_a17_shared, the tree as it is.
#
# One process per row because `gen/corpus.py` runs every row in one process: a hang or a
# crash on one row takes every later row with it. The ffi arm is left out
# (AK_CORPUS_NO_FFI=1): it is the shared core's half of R-D1 and is gated by the corpus
# run, not here. The `pb` line of each row is protobuf C++'s verdict on the same bytes.
#
# A row's verdict: `refused <code>` (the decoder returned an error), `ACCEPTED`, `HANG`
# (timeout 5 killed it), or `CRASH sig N`.
set -u
cd "$(dirname "$0")/.." || exit 2
# Default: the newest commit whose rt.h still has the unchecked comparison, so the
# script means the same thing before and after the fix is committed.
if [ -z "${BEFORE_REV:-}" ]; then
  for c in $(git log --format=%h -- include/ak/rt.h); do
    if git show "$c:ffi/poc/cpp/include/ak/rt.h" | grep -q 'pos + k > len'; then
      BEFORE_REV=$c; break
    fi
  done
fi
# ASAN=1 builds BOTH binaries with AddressSanitizer (the core .so stays uninstrumented),
# so a row the unfixed decoder "refuses" by reading past the buffer until it trips over
# something is reported as the out-of-bounds read it is.
ASAN=${ASAN:-0}
SAN=""
[ "$ASAN" = 1 ] && SAN="-fsanitize=address -fno-omit-frame-pointer"
export ASAN_OPTIONS=exitcode=99:detect_leaks=0
B=build
S=$B/lenwrap-before$([ "${ASAN:-0}" = 1 ] && echo -asan)
TASKS=$B/corpus_tasks.tsv
[ -f "$TASKS" ] || { echo "run gen/corpus.py once first: it writes $TASKS"; exit 2; }

mkdir -p "$S/inc/ak" "$S/obj"
git show "$BEFORE_REV:ffi/poc/cpp/include/ak/rt.h" > "$S/inc/ak/rt.h" || exit 2
echo "# before: include/ak/rt.h at $(git rev-parse --short "$BEFORE_REV")"
grep -n 'pos + k > len\|k64 > (uint64_t)(len - pos)' "$S/inc/ak/rt.h" | sed 's/^/#   /'
echo "# after:  include/ak/rt.h as in the tree"
grep -n 'pos + k > len\|k64 > (uint64_t)(len - pos)' include/ak/rt.h | sed 's/^/#   /'

# The same compile and link lines CMake uses for corpus_a17_shared, with the scratch
# header directory in front.
D=$B/CMakeFiles/corpus_a17_shared.dir
flags=$(sed -n 's/^CXX_FLAGS = //p' "$D/flags.make")
defs=$(sed -n 's/^CXX_DEFINES = //p' "$D/flags.make")
incs=$(sed -n 's/^CXX_INCLUDES = //p' "$D/flags.make")
link=$(cat "$D/link.txt")
objs=$(echo "$link" | grep -o 'CMakeFiles/corpus_a17_shared.dir/[^ ]*\.o')
mkdir -p "$S/obj-after"
link_b=$link; link_a=$link
for o in $objs; do
  src=${o#CMakeFiles/corpus_a17_shared.dir/}; src=${src%.o}
  ob="$S/obj/$(echo "$src" | tr / _).o"
  eval g++ $flags $SAN $defs -I"$S/inc" $incs -c "$src" -o "$ob" || exit 2
  link_b=${link_b//"$o"/"$ob"}
  if [ "$ASAN" = 1 ]; then
    oa="$S/obj-after/$(echo "$src" | tr / _).o"
    eval g++ $flags $SAN $defs $incs -c "$src" -o "$oa" || exit 2
    link_a=${link_a//"$o"/"$oa"}
  fi
done
link_b=${link_b//"-o corpus_a17_shared"/"-o $S/corpus_before"}
(cd "$B" && eval "${link_b//$S/../$S} $SAN") || exit 2
echo "# built $S/corpus_before${SAN:+ ($SAN)}"
AFTER=$B/corpus_a17_shared
if [ "$ASAN" = 1 ]; then
  link_a=${link_a//"-o corpus_a17_shared"/"-o $S/corpus_after"}
  (cd "$B" && eval "${link_a//$S/../$S} $SAN") || exit 2
  AFTER=$S/corpus_after
  echo "# built $AFTER ($SAN)"
fi
echo

verdict() {  # <binary> <task line>
  local bin=$1 line=$2 t out rc arm
  t=$(mktemp); printf '%s\n' "$line" > "$t"
  out=$(AK_CORPUS_NO_FFI=1 timeout 5 "$bin" "$t" 2>&1); rc=$?
  rm -f "$t"
  if [ $rc -eq 124 ]; then echo "HANG"; return; fi
  if [ $rc -eq 99 ]; then
    echo "ASAN $(printf '%s\n' "$out" | sed -n 's/.*ERROR: AddressSanitizer: \([a-z-]*\).*/\1/p' | head -1)"
    return
  fi
  if [ $rc -gt 128 ]; then echo "CRASH sig $((rc - 128))"; return; fi
  # R: a root decode (the native arm). W: the schema-free walker, for WireZoo rows.
  arm=$(printf '%s\n' "$out" | awk -F'\t' '($1=="R" && $3=="native") {print $4, $5} $1=="W" {print $3, $4}')
  case "$arm" in
    reject*) echo "refused ${arm#reject }" ;;
    accept*) echo "ACCEPTED" ;;
    *)       echo "NO RESULT (exit $rc)" ;;
  esac
}
# FFI=1 adds a column: the same row through the C ABI into the SHARED core, from the
# "after" binary (whichever core it links). The walker rows have no ffi arm ("-").
ffiv() {
  local t out rc arm; t=$(mktemp); printf '%s\n' "$1" > "$t"
  out=$(timeout 5 "$AFTER" "$t" 2>&1); rc=$?; rm -f "$t"
  if [ $rc -eq 124 ]; then echo "HANG"; return; fi
  if [ $rc -eq 99 ]; then echo "ASAN"; return; fi
  if [ $rc -gt 128 ]; then echo "CRASH sig $((rc - 128))"; return; fi
  arm=$(printf '%s\n' "$out" | awk -F'\t' '$1=="R" && $3=="ffi" {print $4, $5}')
  case "$arm" in
    reject*) echo "refused ${arm#reject }" ;;
    accept*) echo "ACCEPTED" ;;
    *) printf '%s\n' "$out" | grep -q '^W' && echo "-" || echo "NO RESULT (exit $rc)" ;;
  esac
}
pbv() {
  local t out; t=$(mktemp); printf '%s\n' "$1" > "$t"
  out=$(AK_CORPUS_NO_FFI=1 timeout 5 "$B/corpus_a17_shared" "$t" 2>&1); rm -f "$t"
  printf '%s\n' "$out" | awk -F'\t' '$1=="R" && $3=="pb" {print $4, $5}'
}

FFI=${FFI:-0}
if [ "$FFI" = 1 ]; then
  printf '%-42s %-16s %-16s %-16s %s\n' "row" "native before" "native after" "ffi (core)" "pb (oracle)"
else
  printf '%-42s %-16s %-16s %s\n' "row" "before" "after" "pb (oracle)"
fi
nb_bad=0; na_bad=0; nf_bad=0; n=0
while IFS= read -r line; do
  id=$(printf '%s' "$line" | cut -f2)
  case "$id" in X-lenwrap-*) ;; *) continue ;; esac
  n=$((n + 1))
  vb=$(verdict "$S/corpus_before" "$line")
  va=$(verdict "$AFTER" "$line")
  if [ "$FFI" = 1 ]; then
    vf=$(ffiv "$line")
    printf '%-42s %-16s %-16s %-16s %s\n' "$id" "$vb" "$va" "$vf" "$(pbv "$line")"
    case "$vf" in refused*|-) ;; *) nf_bad=$((nf_bad + 1)) ;; esac
  else
    printf '%-42s %-16s %-16s %s\n' "$id" "$vb" "$va" "$(pbv "$line")"
  fi
  case "$vb" in refused*) ;; *) nb_bad=$((nb_bad + 1)) ;; esac
  case "$va" in refused*) ;; *) na_bad=$((na_bad + 1)) ;; esac
done < "$TASKS"
echo
echo "rows: $n"
echo "before: $nb_bad of $n NOT refused promptly (hang, crash, accept or no result)"
echo "after:  $na_bad of $n NOT refused promptly"
if [ "$FFI" = 1 ]; then
  echo "ffi:    $nf_bad of $n NOT refused promptly (walker rows have no ffi arm)"
  [ "$nf_bad" -eq 0 ] || exit 1
fi
[ "$na_bad" -eq 0 ] || exit 1
[ "$nb_bad" -gt 0 ] || { echo "the BEFORE build refused every row: this run cannot see the defect"; exit 1; }
exit 0
