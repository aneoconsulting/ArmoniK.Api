// design/CAMPAIGN.md requirement 20, this host's half: the crossing into the core priced on
// its own, FORWARD (`ak_noop`, one host -> core call) and REVERSE (`ak_noop_reverse`, one
// host -> core call that calls back into the host once, minus the forward row), one JSON
// line per round. The runner wraps each direction in `perf stat -e cycles,instructions`
// where perf exists, and runs the Rust slice's own crossing benchmark beside it.
//
//   campaign_calib --dir forward|reverse --launch L --rounds R --iters N
#include <time.h>

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>

#include "ak_abi.h"

static uint64_t host_cb(uint64_t x) { return x + 1; }

// Requirement 21 (owner, 2026-09-26, R-H25): process CPU, as every other suite and slice.
static double cpu_ns() {
  struct timespec ts;
  clock_gettime(CLOCK_PROCESS_CPUTIME_ID, &ts);
  return (double)ts.tv_sec * 1e9 + (double)ts.tv_nsec;
}

int main(int argc, char **argv) {
  std::string dir = "forward";
  int launch = 0, rounds = 5;
  long iters = 100000000;
  for (int i = 1; i + 1 < argc; i += 2) {
    if (!std::strcmp(argv[i], "--dir")) dir = argv[i + 1];
    else if (!std::strcmp(argv[i], "--launch")) launch = std::atoi(argv[i + 1]);
    else if (!std::strcmp(argv[i], "--rounds")) rounds = std::atoi(argv[i + 1]);
    else if (!std::strcmp(argv[i], "--iters")) iters = std::atol(argv[i + 1]);
  }
  // R-H5: a malformed invocation is a failure (nonzero), never an empty success.
  if ((dir != "forward" && dir != "reverse") || iters <= 0 || rounds <= 0) {
    std::fprintf(stderr, "campaign_calib: --dir forward|reverse, --iters > 0, --rounds > 0\n");
    return 2;
  }
  bool rev = dir == "reverse";
  volatile uint64_t sink = 0;
  for (long i = 0; i < iters / 10; ++i) sink = rev ? ak_noop_reverse(host_cb, sink) : ak_noop(sink);
  // Samples are buffered and written only once every round ran (R-H5, as R-H4 for RPC).
  std::string out;
  for (int r = 0; r < rounds; ++r) {
    uint64_t x = sink;
    double c0 = cpu_ns();
    if (rev) for (long i = 0; i < iters; ++i) x = ak_noop_reverse(host_cb, x);
    else for (long i = 0; i < iters; ++i) x = ak_noop(x);
    double c1 = cpu_ns();
    sink = x;
    char line[256];
    std::snprintf(line, sizeof(line),
                  "{\"slice\":\"cpp\",\"suite\":\"calib\",\"arm\":\"crossing-%s\",\"dir\":\"%s\","
                  "\"launch\":%d,\"round\":%d,\"cpu_ns\":%.0f,\"cpu_clock\":\"process\",\"iters\":%ld}\n",
                  rev ? "fwd+rev" : "fwd", dir.c_str(), launch, r, c1 - c0, iters);
    out += line;
  }
  std::fwrite(out.data(), 1, out.size(), stdout);
  return 0;
}
