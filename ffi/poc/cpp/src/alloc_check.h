// CAMPAIGN req 25 / D9 as amended 2026-10-03, the startup check (C#'s, made common): before any
// timing, every measured process (the codec binaries, the RPC clients) makes one 16 MiB malloc
// and reads mallinfo2's mmapped-block count before and after. glibc's default allocator serves
// it with mmap ("mmapped"); the pinned diagnostic (GLIBC_TUNABLES mmap_threshold=33554432) from
// the heap ("heap"). The process refuses to run (exit 4, no sample) if the readback disagrees
// with AK_CAMPAIGN_ALLOC (unset = default), or if AK_CAMPAIGN_ALLOC disagrees with
// GLIBC_TUNABLES. The returned JSON object is printed as a header line.
#pragma once

#include <malloc.h>

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
  struct mallinfo2 a = mallinfo2();
  void *volatile p = std::malloc(16u << 20);
  if (p) static_cast<char *>(p)[0] = 1;
  struct mallinfo2 b = mallinfo2();
  std::free(p);
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

}  // namespace akalloc
