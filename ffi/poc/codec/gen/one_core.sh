#!/usr/bin/env bash
# README R0, mechanically: ONE core, and every slice reaching it by path.
#
# R0 exists because the branch broke it without noticing. `codec.rs` was byte-identical in
# three slices -- so the generator was genuinely shared -- while the hand-written runtime
# beside it had forked three ways, each fork made by a slice that needed to ADD something
# and had nowhere to contribute it. Nothing was wrong and no measurement broke, which is
# exactly why it went unseen. A rule that cannot be checked is a rule that gets broken
# again, so this is the check; `--selftest` is the proof that it FAILS when the rule is
# broken, because a rule with no failing test is a rule nobody has seen work.
#
#   gen/one_core.sh              check the tree
#   gen/one_core.sh --selftest   plant each violation in a scratch copy and require a fail
#   gen/one_core.sh --root DIR   check some other ffi/poc (used by --selftest)
set -u

ROOT=""
SELFTEST=0
while [ $# -gt 0 ]; do
  case "$1" in
    --selftest) SELFTEST=1; shift ;;
    --root) ROOT=$2; shift 2 ;;
    *) echo "usage: one_core.sh [--selftest] [--root DIR]" >&2; exit 2 ;;
  esac
done
[ -n "$ROOT" ] || ROOT=$(cd "$(dirname "$0")/../.." && pwd)   # ffi/poc

bad=0
pass() { echo "  [ok]   $1"; }
fail() { echo "  FAIL   $1"; bad=$((bad + 1)); }

# Source files only. Build output is not the rule's business and a target directory holds
# copies of everything by construction.
srcs() {
  find "$ROOT" \
    -name target -prune -o -name 'target-*' -prune -o -name 'core-build' -prune -o \
    -name build -prune -o -name 'build-*' -prune -o -name __pycache__ -prune -o \
    -name obj -prune -o -name bin -prune -o \
    -type f -name "$1" -print
}

rel() { echo "${1#"$ROOT"/}"; }

echo "== R0: one core, at poc/codec, reached by path =="
echo

# ---- 1. one definition of the C entry points ---------------------------------------
# Not "one file called lib.rs": the question is whether two files DEFINE the ABI. Three
# sentinels, one per family, because a partial fork is the likely shape -- a slice copies
# the file and adds a transcoder, so the encode entry points agree and the string ones do
# not.
echo "# the ABI's entry points are defined once"
# All four are hand-written in lib.rs, not emitted, which is where the fork was.
# WP5 step 8: `ak_dec_ctx_new` is gone (decision 11 rule 6); `ak_dec_ctx_free` is the
# decode family's hand-written sentinel now.
for sym in ak_enc_ctx_new ak_dec_ctx_free ak_tc_utf8 ak_tc_utf16; do
  hits=$(grep -rlE "extern \"C\" fn $sym\b" $(srcs '*.rs') 2>/dev/null | sort -u)
  n=$(printf '%s\n' "$hits" | grep -c . )
  if [ "$n" != 1 ]; then
    fail "$sym is defined in $n files:"; printf '%s\n' "$hits" | sed 's/^/           /'
  elif [ "$(rel "$hits")" != "codec/crates/ak-core/src/lib.rs" ]; then
    fail "$sym is defined in $(rel "$hits"), not in the shared core"
  else
    pass "$sym: one definition, in codec/crates/ak-core/src/lib.rs"
  fi
done

# ---- 2. one copy of each generated core file ---------------------------------------
echo
echo "# the emitted core exists once"
# `codec/crates/*/src/generated_corpus/` is not a second core: it is the SAME core rendered
# by the same generator (`gen/generate.py`, one plan layer, one backend) for the corpus's
# reader schema, behind the test-only `corpus` feature of ak-abi/ak-core (FIX-PLAN WP5
# item 6.1). It lives inside codec/ and nowhere else, which is what this check guards;
# a copy of it outside codec/ is still counted below.
for f in codec.rs layout.rs abi.rs; do
  hits=$(srcs "$f" | grep -v '/poc/codec/crates/[a-z-]*/src/generated_corpus/' | sort)
  n=$(printf '%s\n' "$hits" | grep -c .)
  if [ "$n" != 1 ]; then
    fail "$f exists in $n places:"; printf '%s\n' "$hits" | sed 's/^/           /'
  else
    pass "$f: one copy, at $(rel "$hits")"
  fi
done

# ---- 3. one package per core crate ---------------------------------------------------
echo
echo "# one Cargo package per core crate"
for pkg in ak-abi ak-rt ak-core rpc; do
  hits=$(grep -rlE "^name = \"$pkg\"\$" $(srcs 'Cargo.toml') 2>/dev/null | sort)
  n=$(printf '%s\n' "$hits" | grep -c .)
  if [ "$n" != 1 ]; then
    fail "package $pkg is declared in $n manifests:"; printf '%s\n' "$hits" | sed 's/^/           /'
  elif [ "$(dirname "$(rel "$hits")")" != "codec/crates/$pkg" ]; then
    fail "package $pkg is declared outside the shared core, at $(rel "$hits")"
  else
    pass "$pkg: one package, at codec/crates/$pkg"
  fi
done

# ---- 4. every path dependency resolves INTO the shared core --------------------------
# The check the rule is really about. A slice may carry no copy and still build a core of
# its own by pointing somewhere else, and nothing above would see it.
echo
echo "# every path dependency on a core crate resolves to codec/crates/"
while IFS= read -r m; do
  while IFS= read -r line; do
    dep=${line%% *}
    p=$(printf '%s' "$line" | sed -n 's/.*path = "\([^"]*\)".*/\1/p')
    [ -n "$p" ] || continue
    abs=$(cd "$(dirname "$m")" && cd "$p" 2>/dev/null && pwd)
    want="$ROOT/codec/crates/$dep"
    wantabs=$(cd "$want" 2>/dev/null && pwd)
    if [ -z "$abs" ]; then
      fail "$(rel "$m"): $dep -> $p does not exist"
    elif [ "$abs" != "$wantabs" ]; then
      fail "$(rel "$m"): $dep resolves to $abs, not to the shared core"
    else
      pass "$(rel "$m"): $dep -> codec/crates/$dep"
    fi
  done < <(grep -E "^(ak-abi|ak-rt|ak-core|rpc) *=.*path *= *\"" "$m")
done < <(srcs 'Cargo.toml' | sort)

# ---- 5. the shared emitters exist once ----------------------------------------------
echo
echo "# the emitters are shared too, which was the other half of the same defect"
# Every module of the shared generator (WP5 step 6: all backends, not a hand list that went
# stale when rust_core.py was deleted), except generate.py, which each slice has its own of.
for f in $(cd "$ROOT/codec/gen" && ls *.py | grep -vx generate.py); do
  hits=$(srcs "$f" | sort)
  n=$(printf '%s\n' "$hits" | grep -c .)
  if [ "$n" != 1 ]; then
    # csharp/gen/ir.py is a DIFFERENT module (a C#-specific IR over the same shapes.py),
    # not a copy of this one; it is named here so the exception is visible rather than
    # silently tolerated by a looser pattern.
    only_cs=$(printf '%s\n' "$hits" | grep -v '/csharp/gen/' | grep -c .)
    if [ "$f" = "ir.py" ] && [ "$only_cs" = 1 ]; then
      pass "$f: one shared copy; csharp/gen/ir.py is a separate C# IR, not a copy"
      continue
    fi
    fail "$f exists in $n places:"; printf '%s\n' "$hits" | sed 's/^/           /'
  elif [ "$(rel "$hits")" != "codec/gen/$f" ]; then
    fail "$f lives at $(rel "$hits"), not in codec/gen"
  else
    pass "$f: one copy, at codec/gen/$f"
  fi
done

# ---- 6. no build names a per-slice core library --------------------------------------
echo
echo "# no build links a per-slice core library"
# This script names the forbidden spellings in order to look for them, so it excludes
# itself -- the one grep that would otherwise always find a hit.
hits=$(grep -rlE 'libak_core_[a-z]+|-lak_core_[a-z]+|ak-core-[a-z]+' \
        $(srcs '*.sh') $(srcs 'CMakeLists.txt') $(srcs '*.csproj') $(srcs '*.toml') \
        2>/dev/null | grep -v '/codec/gen/one_core.sh$' | sort)
if [ -n "$hits" ]; then
  fail "a build still names a per-slice core:"; printf '%s\n' "$hits" | sed 's/^/           /'
else
  pass "every build links libak_core, the shared one"
fi

# ---- 7. the shared core is not stale -------------------------------------------------
echo
echo "# and what is committed is what the shared emitter writes (R1)"
if out=$(cd "$ROOT/codec" && python3 gen/generate.py --check --core-only 2>&1); then
  pass "codec/gen/generate.py --check: $(printf '%s' "$out" | grep -c '^ok') files ok"
else
  fail "codec/gen/generate.py --check:"; printf '%s\n' "$out" | sed 's/^/           /'
fi

echo
echo "one_core: $bad failure(s)"
[ "$SELFTEST" = 1 ] || exit $((bad ? 1 : 0))

# ---- the positive control ------------------------------------------------------------
[ "$bad" = 0 ] || { echo "SELFTEST ABORTED: the tree already fails, so a planted fault proves nothing"; exit 1; }
echo
echo "== positive control: each violation planted in a scratch copy, each must FAIL =="
SELF=$(cd "$(dirname "$0")" && pwd)/one_core.sh
REPO=$(cd "$ROOT/../.." && pwd)
SCRATCH=$(mktemp -d)
trap 'rm -rf "$SCRATCH"' EXIT
# `ffi/schema` and `ffi/corpus` come too: the R1 freshness check below re-runs the
# generator, whose front end imports `ffi/schema/emit/shapes.py` and, for the corpus-schema
# core it also writes, `ffi/corpus/emit/spec.py` over `ffi/corpus/corpus.json` (R-G14: the
# copy lacked `ffi/corpus` from WP5 step 1 on, so the scratch copy could not pass clean).
(cd "$REPO" && git ls-files -z ffi/poc ffi/schema ffi/corpus | tar --null -T - -cf -) | (cd "$SCRATCH" && tar xf -)
BASE=$SCRATCH/ffi/poc
if "$SELF" --root "$BASE" >/dev/null 2>&1; then
  echo "  [ok]   the scratch copy passes before anything is planted"
else
  echo "  FAIL   the scratch copy does not even pass clean; the control proves nothing"
  echo "         (the usual cause is an uncommitted change: the copy is of what git TRACKS)"
  "$SELF" --root "$BASE" 2>&1 | grep FAIL | sed "s/^/         /"
  exit 1
fi

ctl=0
plant() {   # $1 = name, $2 = shell that plants the fault
  local snap="$SCRATCH/snap"
  rm -rf "$snap"; cp -a "$BASE" "$snap"
  ( cd "$BASE" && eval "$2" ) >/dev/null 2>&1
  if "$SELF" --root "$BASE" >/dev/null 2>&1; then
    echo "  FAIL   planted '$1' and the check still PASSED"; ctl=$((ctl + 1))
  else
    echo "  [ok]   planted '$1' -> the check fails, as R0 says it must"
  fi
  rm -rf "$BASE"; mv "$snap" "$BASE"
}

plant "a second copy of the core crate" \
      'mkdir -p cpp/core/src/generated && cp codec/crates/ak-core/src/lib.rs cpp/core/src/ && cp codec/crates/ak-core/Cargo.toml cpp/core/ && cp codec/crates/ak-core/src/generated/codec.rs cpp/core/src/generated/'
plant "a slice pointing its core dependency somewhere else" \
      'mkdir -p java/mycore && cp -a codec/crates/ak-rt java/myrt && sed -i "s|path = \"../ak-rt\"|path = \"../../../java/myrt\"|" codec/crates/ak-core/Cargo.toml'
plant "a second copy of a shared emitter" \
      'cp codec/gen/rust_abi.py cpp/gen/rust_abi.py'
plant "a build linking a per-slice core library" \
      'sed -i "s|libak_core\.so|libak_core_cpp.so|" cpp/CMakeLists.txt'
plant "a stale shared core (the emitter and the committed file disagree)" \
      'sed -i "1s|.*|// drifted|" codec/crates/ak-core/src/generated/codec.rs'

echo
echo "one_core --selftest: $ctl control(s) did not fire"
exit $((ctl ? 1 : 0))
