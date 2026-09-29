/* An LD_PRELOAD shim, OFF unless a driver asks for it (gen/physical_probe.sh --grpc-cpus N):
 * sysconf(_SC_NPROCESSORS_CONF) and sysconf(_SC_NPROCESSORS_ONLN) return AK_SHIM_NCPUS.
 *
 * Why: grpc-core v1.80 sizes itself from gpr_cpu_num_cores(), which is
 * sysconf(_SC_NPROCESSORS_CONF) (src/core/util/linux/cpu.cc), NOT the affinity mask: its
 * EventEngine thread pool reserves Clamp(ncpus, 4, 16) threads and 2 x ncpus connection shards
 * (src/core/lib/event_engine/posix_engine/posix_engine.h). On a 20-CPU machine that is 16
 * reserve threads whatever taskset sets. grpc++ has no application setting for it, so this is
 * the only way to size grpc-core for a CPU set. Every other call of sysconf is passed through.
 * Not linked into any binary.
 *   gcc -O2 -shared -fPIC -o ncpus_shim.so gen/ncpus_shim.c -ldl */
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
