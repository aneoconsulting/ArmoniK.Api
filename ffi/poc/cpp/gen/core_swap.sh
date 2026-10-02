# Sourced by the gate drivers (wp5_gate.sh, nounk_gate.sh, run_campaign.sh, q_checks.sh, deferred_checks.sh).
# INERT unless AK_CORE_SWAP names a file: every gate then runs exactly as before.
#
#   AK_CORE_SWAP=FILE   lines "STOCK_CORE_DIR REPLACEMENT_CORE_DIR" (canonical paths). A binary whose
#                       libak_core.so resolves, through its RUNPATH alone, to STOCK_CORE_DIR runs with
#                       LD_LIBRARY_PATH=REPLACEMENT_CORE_DIR: the way the timed drivers load the h2-batch
#                       core beside the stock build (FIX-PLAN WP12, D11). gen/wp12_gates.sh writes the file.
#
#   cs_env BINARY       prints "LD_LIBRARY_PATH=DIR" when BINARY's core is swapped, nothing otherwise;
#                       used as `env $(cs_env "$B/x") ... "$B/x" ...`
cs_env() {
  [ -n "${AK_CORE_SWAP:-}" ] || return 0
  local so
  so=$(env -u LD_LIBRARY_PATH -u LD_DEBUG -u LD_DEBUG_OUTPUT ldd "$1" 2>/dev/null | awk '/libak_core\.so/{print $3; exit}')
  [ -n "$so" ] || return 0
  awk -v d="$(dirname "$(readlink -f "$so")")" '$1==d{print "LD_LIBRARY_PATH=" $2; exit}' "$AK_CORE_SWAP"
}
