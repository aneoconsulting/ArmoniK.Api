#!/usr/bin/env bash
# What this machine can actually offer a Python slice.
#
# README open question 4 ("Python floor, target, and wheel policy") is partly a
# fact about the machine and partly a decision, and the decision is the
# aggregating session's.  This script reports the facts and decides nothing.
set -uo pipefail
echo "# python slice: what this container offers"
echo "# date:    $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "# machine: $(nproc) vCPU, $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | xargs)"
echo "# kernel:  $(uname -r), $(. /etc/os-release && echo "$PRETTY_NAME")"
echo

echo "## interpreters PRESENT"
for v in 3.7 3.8 3.9 3.10 3.11 3.12 3.13 3.14; do
  if command -v "python$v" >/dev/null 2>&1; then
    printf "   python%-5s %s\n" "$v" "$(python$v -VV 2>&1 | tr '\n' ' ')"
  fi
done
printf "   %-13s %s\n" "python3" "$(python3 -V 2>&1) (the default)"

echo
echo "## free-threaded builds (PEP 703): the GIL argument changes on these"
FT=0
for v in 3.13 3.14; do
  for n in "python${v}t" "python${v}-nogil"; do
    command -v "$n" >/dev/null 2>&1 && { echo "   $n present"; FT=1; }
  done
done
python3 -c 'import sysconfig,sys; sys.exit(0 if sysconfig.get_config_var("Py_GIL_DISABLED") else 1)' \
  && { echo "   the default interpreter is free-threaded"; FT=1; }
[ "$FT" -eq 0 ] && echo "   NONE. Every interpreter here holds the GIL, so a free-threaded arm"
[ "$FT" -eq 0 ] && echo "   cannot be measured on this machine at all."

echo
echo "## development headers (a C extension arm needs these)"
for d in /usr/include/python3.*; do
  [ -d "$d" ] && printf "   %-30s Python.h %s\n" "$d" \
    "$([ -f "$d/Python.h" ] && echo present || echo MISSING)"
done

echo
echo "## interpreters INSTALLABLE from this container's apt"
for v in 3.7 3.8 3.9 3.10 3.11 3.12 3.13; do
  c=$(apt-cache policy "python$v" 2>/dev/null | sed -n 's/^  Candidate: //p')
  [ -n "${c:-}" ] && [ "$c" != "(none)" ] && printf "   python%-5s %s\n" "$v" "$c"
done
echo "   (pyproject.toml declares >=3.7; see the line above for whether 3.7 is"
echo "    reachable here at all)"
grep -n "requires-python" ../../../../packages/python/pyproject.toml 2>/dev/null \
  | sed 's/^/   packages\/python\/pyproject.toml:/'

echo
echo "## the incumbent, per interpreter"
for PY in "$@"; do
  T=$("$PY" -c 'import sys;print("%d.%d"%sys.version_info[:2])')
  PB=$("$PY" -c 'import google.protobuf as p;print(p.__version__)' 2>/dev/null || echo "not installed")
  IM=$("$PY" -c 'from google.protobuf.internal import api_implementation as a;print(a.Type())' 2>/dev/null || echo "-")
  GR=$("$PY" -c 'import grpc;print(grpc.__version__)' 2>/dev/null || echo "not installed")
  WH=$("$PY" -c "
import sys
try:
    import google._upb._message as m
    print(m.__file__.rsplit('/',1)[-1])
except Exception:
    print('-')" 2>/dev/null)
  printf "   python%-5s protobuf %-9s impl %-8s grpcio %-9s upb ext %s\n" \
    "$T" "$PB" "$IM" "$GR" "$WH"
done
echo
echo "   'impl upb' is what makes this slice's comparison native against native:"
echo "   the incumbent is a C extension, not a Python codec (README section 9)."

echo
echo "## toolchain"
for t in "cc --version" "g++ --version" "clang --version" "cargo -V" "rustc -V" \
         "cmake --version" "protoc --version"; do
  printf "   %-16s %s\n" "${t%% *}" "$($t 2>&1 | head -1)"
done
echo "   protoc: absent from the system; grpcio-tools carries one and build.sh"
echo "   uses it."

echo
echo "## wheel policy (README 9.2), as facts rather than as a decision"
echo "   The abi3 artifact in build/abi3 is built with Py_LIMITED_API=0x030A0000"
echo "   and is loaded by every interpreter above in 20-conformance.log and"
echo "   30-mechanism-*.log. What it costs is measured there; what to ship is"
echo "   not this slice's call."
