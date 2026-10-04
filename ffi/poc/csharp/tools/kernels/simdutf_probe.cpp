// D21 step 7 item 3: which simdutf implementation simdutf selects on this CPU. Built from the
// SAME amalgamation the core links (crate simdutf 0.7.0, cpp/simdutf.cpp, C++ simdutf 7.7.1)
// with the cc crate's flags for the core's release build (-O3 -std=c++11 -fPIC
// -ffunction-sections -fdata-sections -m64), so the runtime dispatch is the core's.
//   c++ -O3 -std=c++11 -fPIC -ffunction-sections -fdata-sections -m64 -I $SIMDUTF/cpp simdutf_probe.cpp -o probe
#include "simdutf.cpp"
#include <cstdio>
#include <cstdlib>
int main() {
  std::printf("simdutf %s\n", SIMDUTF_VERSION);
  const char *force = std::getenv("SIMDUTF_FORCE_IMPLEMENTATION");
  std::printf("SIMDUTF_FORCE_IMPLEMENTATION=%s\n", force ? force : "(unset)");
  for (auto impl : simdutf::get_available_implementations())
    std::printf("available: %-10s supported_by_runtime_system=%d  (%s)\n", impl->name().c_str(),
                (int)impl->supported_by_runtime_system(), impl->description().c_str());
  const simdutf::implementation *a = simdutf::get_active_implementation();
  std::printf("active: %s (%s)\n", a->name().c_str(), a->description().c_str());
  return 0;
}
