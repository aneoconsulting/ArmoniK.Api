/* An LD_PRELOAD shim for the RPC client processes (FIX-PLAN WP13, D14), a copy of
 * poc/cpp/gen/ncpus_shim.c: sysconf(_SC_NPROCESSORS_CONF) and sysconf(_SC_NPROCESSORS_ONLN)
 * return AK_SHIM_NCPUS when set.
 *
 * Why: grpc-core (inside grpcio's cygrpc extension) sizes itself from gpr_cpu_num_cores(),
 * which is sysconf(_SC_NPROCESSORS_CONF), NOT the affinity mask: its EventEngine thread pool
 * reserves Clamp(ncpus, 4, 16) threads. grpcio has no application setting for it, so this is the
 * way to size grpc-core to AK_WORKERS. run_campaign.sh preloads it into the RPC grid's pyperf
 * processes only. Every other sysconf call is passed through. Not linked into any binary.
 *   cc -O2 -shared -fPIC -o build/ncpus_shim.so native/ncpus_shim.c -ldl */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdlib.h>
#include <unistd.h>

static long (*real_sysconf)(int);

/* Read lazily, on the first call: a library's static initialisers may call sysconf before a
 * preloaded library's constructor has run. */
long sysconf(int name) {
  if (name == _SC_NPROCESSORS_CONF || name == _SC_NPROCESSORS_ONLN) {
    const char *v = getenv("AK_SHIM_NCPUS");
    if (v && atol(v) > 0) return atol(v);
  }
  if (!real_sysconf) real_sysconf = (long (*)(int))dlsym(RTLD_NEXT, "sysconf");
  return real_sysconf(name);
}
