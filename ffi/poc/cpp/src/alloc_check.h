// CAMPAIGN req 25 / D9 as amended 2026-10-03, the startup check (C#'s, made common): before any
// timing, every measured process (the codec binaries, the RPC clients) makes one 16 MiB malloc
// and reads mallinfo2's mmapped-block count before and after. glibc's default allocator serves
// it with mmap ("mmapped"); the pinned diagnostic (GLIBC_TUNABLES mmap_threshold=33554432) from
// the heap ("heap"). The process refuses to run (exit 4, no sample) if the readback disagrees
// with AK_CAMPAIGN_ALLOC (unset = default), or if AK_CAMPAIGN_ALLOC disagrees with
// GLIBC_TUNABLES. The returned JSON object is printed as a header line.
#pragma once

#include <malloc.h>
#include <sys/resource.h>

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>

namespace akalloc {

inline std::string check_or_exit() {
  const char *m = std::getenv("AK_CAMPAIGN_ALLOC");
  const std::string mode = (m && *m) ? m : "default";
  const char *t = std::getenv("GLIBC_TUNABLES");
  const std::string tun = t ? t : "";
  const bool tun_pinned = tun.find("glibc.malloc.mmap_threshold=33554432") != std::string::npos &&
                          tun.find("glibc.malloc.trim_threshold=268435456") != std::string::npos;
  // Once per process, and the block stays MAPPED: freeing it would raise glibc's dynamic mmap
  // threshold, so the probe would change the mode it verifies (req. 25 as amended; Java
  // 2892e207b).
  static void *volatile s_probe = NULL;
  struct mallinfo2 a = mallinfo2();
  if (!s_probe) s_probe = std::malloc(16u << 20);
  if (s_probe) static_cast<volatile char *>(s_probe)[0] = 1;
  struct mallinfo2 b = mallinfo2();
  const char *readback = b.hblks > a.hblks ? "mmapped" : "heap";
  std::string why;
  if (mode != "default" && mode != "pinned") why = "AK_CAMPAIGN_ALLOC is neither default nor pinned";
  else if ((mode == "pinned") != tun_pinned) why = "AK_CAMPAIGN_ALLOC disagrees with GLIBC_TUNABLES";
  else if (std::strcmp(readback, mode == "pinned" ? "heap" : "mmapped") != 0)
    why = "the 16 MiB malloc readback disagrees with AK_CAMPAIGN_ALLOC";
  char o[512];
  std::snprintf(o, sizeof o,
                "{\"malloc_check\": {\"ak_campaign_alloc\": \"%s\", \"glibc_tunables\": \"%s\", \"malloc_16mib\": \"%s\","
                " \"hblks_before\": %zu, \"hblks_after\": %zu, \"ok\": %s}}",
                mode.c_str(), tun.c_str(), readback, (size_t)a.hblks, (size_t)b.hblks, why.empty() ? "true" : "false");
  if (!why.empty()) {
    std::fprintf(stderr, "ALLOC CHECK FAILED: %s -- %s; nothing is timed\n", why.c_str(), o);
    std::fflush(stdout);
    std::_Exit(4);
  }
  return o;
}

// req. 25 (ii), both modes: pre-grow the heap before any timing. Allocate, touch every page of
// and free a block of `bytes` (the run's largest payload) until one more round faults nothing
// (minor faults from getrusage), at most 8 rounds, else refuse (exit 4). Returns the header
// object: the rounds taken and the last round's faults.
inline std::string pregrow_or_exit(size_t bytes) {
  long last = -1;
  int rounds = 0;
  for (rounds = 1; rounds <= 8; ++rounds) {
    struct rusage r0, r1;
    getrusage(RUSAGE_SELF, &r0);
    volatile char *p = static_cast<volatile char *>(std::malloc(bytes));
    if (!p) break;
    for (size_t i = 0; i < bytes; i += 4096) p[i] = 1;
    p[bytes - 1] = 1;
    std::free(const_cast<char *>(p));
    getrusage(RUSAGE_SELF, &r1);
    last = r1.ru_minflt - r0.ru_minflt;
    if (last == 0) break;
  }
  char o[256];
  std::snprintf(o, sizeof o, "{\"heap_pregrow\": {\"bytes\": %zu, \"rounds\": %d, \"last_round_minor_faults\": %ld, \"cap\": 8}}",
                bytes, rounds > 8 ? 8 : rounds, last);
  if (last != 0) {
    std::fprintf(stderr, "ALLOC CHECK FAILED: the heap pre-grow did not reach a fault-free round -- %s; nothing is timed\n", o);
    std::fflush(stdout);
    std::_Exit(4);
  }
  return o;
}

}  // namespace akalloc
