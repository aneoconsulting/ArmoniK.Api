// The cpp slice's full-corpus harness, the child process: ONE vector, four arms, one JSON
// line per arm on stdout. `gen/corpus_all.py` runs one child per corpus row under a timeout
// (a hang or an abort is a row result, not the end of the run) and takes the verdict.
//
//   corpus_all ROW-FILE ROOT     run the four arms on one vector
//   corpus_all --info            the plan options the native codecs render, the build
//   corpus_all --layout          ABI v1 section 10: the corpus core's layout facts against
//                                this header's, fact by fact (exit 1 on a disagreement)
//
// AK_CORPUS_PLANT=proj|reenc|accept plants a harness defect that MUST turn rows red (the
// controls of gen/corpus_all.py). The binding's ak_init is rendered from plan.lifecycle and
// called by the binding itself; the `_noinit` build (-DAK_BINDING_SKIP_INIT) skips it, and
// against the core built with init-guard every ffi arm must then fail.
#include <cstdio>
#include <cstdlib>
#include <fstream>
#include <iterator>
#include <sstream>
#include <string>
#include <vector>

#include "corpus_harness.h"
#include "generated/ak_layout.h"
#include "generated/ak_layout_names.h"

namespace {

const char *arm_name(corpus::Arm a) {
  switch (a) {
    case corpus::kFfiDrop: return "ffi-drop";
    case corpus::kFfiRetain: return "ffi-retain";
    case corpus::kNativeDrop: return "native-drop";
    case corpus::kNativeRetain: return "native-retain";
  }
  return "?";
}

std::string hex(const std::string &s) {
  static const char *d = "0123456789abcdef";
  std::string o;
  o.reserve(s.size() * 2);
  for (size_t i = 0; i < s.size(); ++i) {
    unsigned char c = (unsigned char)s[i];
    o.push_back(d[c >> 4]);
    o.push_back(d[c & 15]);
  }
  return o;
}

int layout() {
  std::vector<uint32_t> core(AK_LAYOUT_FACTS + 1, 0);
  size_t n = ak_layout_facts(&core[0], core.size());
  int bad = 0;
  if (n != (size_t)AK_LAYOUT_FACTS) {
    std::printf("layout: the core has %zu facts, this header %d\n", n, AK_LAYOUT_FACTS);
    return 1;
  }
  for (size_t i = 0; i < n; ++i) {
    if (core[i] != AK_LAYOUT_HOST[i]) {
      std::printf("layout: MISMATCH %s: core %u host %u\n", AK_LAYOUT_NAMES[i], core[i],
                  AK_LAYOUT_HOST[i]);
      ++bad;
    }
  }
  std::printf("layout: %zu facts compared (corpus ABI), %d disagree\n", n, bad);
  return bad ? 1 : 0;
}

}  // namespace

int main(int argc, char **argv) {
  if (argc >= 2 && std::string(argv[1]) == "--info") {
    std::printf("{\"cplusplus\": %ld, \"cxx17_impl\": %d, \"utf8\": \"%s\", \"limit\": %u,"
                " \"native_unknown\": [\"%s\", \"%s\"], \"ffi_retain_built\": %s,"
                " \"skip_init\": %s, \"initialized_before\": %d}\n",
                (long)__cplusplus, (int)AK_CXX17, corpus::native::kUtf8Policy,
                corpus::native::kRecursionLimit, corpus::native::kUnknownFields,
                corpus::native_retain::kUnknownFields,
                corpus::kFfiRetainBuilt ? "true" : "false",
#ifdef AK_BINDING_SKIP_INIT
                "true",
#else
                "false",
#endif
                ak_initialized());
    return 0;
  }
  if (argc >= 2 && std::string(argv[1]) == "--layout") return layout();
  if (argc < 3) {
    std::fprintf(stderr, "usage: corpus_all ROW-FILE ROOT | --info | --layout\n");
    return 2;
  }
  std::ifstream f(argv[1], std::ios::binary);
  if (!f) {
    std::fprintf(stderr, "cannot read %s\n", argv[1]);
    return 2;
  }
  std::string buf((std::istreambuf_iterator<char>(f)), std::istreambuf_iterator<char>());
  const std::string root = argv[2];
  const char *plant = std::getenv("AK_CORPUS_PLANT");
  std::string pl = plant ? plant : "";

  corpus::Cx cx;
  cx.enc = ak_enc_ctx_new();
  cx.dec = ak_dec_ctx_new();
  cx.tcs = corpus::ffi::tcs_core();
  const corpus::Arm arms[] = {corpus::kFfiDrop, corpus::kFfiRetain, corpus::kNativeDrop,
                              corpus::kNativeRetain};
  std::string out = "{";
  for (size_t i = 0; i < 4; ++i) {
    corpus::Outcome oc = corpus::run_arm(root, arms[i], (const uint8_t *)buf.data(),
                                         buf.size(), cx);
    // The harness seen failing: each plant must turn rows red in gen/corpus_all.py.
    if (pl == "proj" && oc.kind == corpus::Outcome::kOk && oc.proj.size() >= 2)
      oc.proj = std::string("{\"__planted\":true") + (oc.proj.size() > 2 ? "," : "") +
                oc.proj.substr(1);
    if (pl == "reenc" && oc.kind == corpus::Outcome::kOk && oc.enc_ok) oc.bytes.push_back('\0');
    if (pl == "accept" && oc.kind == corpus::Outcome::kErr) {
      oc.kind = corpus::Outcome::kOk;
      oc.proj = "{}";
      oc.enc_ok = true;
      oc.bytes.clear();
    }
    if (i) out += ",";
    out += "\"";
    out += arm_name(arms[i]);
    out += "\":";
    std::ostringstream s;
    switch (oc.kind) {
      case corpus::Outcome::kOk:
        s << "{\"ok\":true,\"proj\":" << oc.proj;
        if (oc.enc_ok) s << ",\"hex\":\"" << hex(oc.bytes) << "\"}";
        else s << ",\"enc_err\":" << oc.enc_err << "}";
        break;
      case corpus::Outcome::kErr: s << "{\"ok\":false,\"err\":" << oc.err << "}"; break;
      case corpus::Outcome::kNotInAbi: s << "{\"na\":true}"; break;
      case corpus::Outcome::kNotBuilt: s << "{\"not_built\":true}"; break;
      case corpus::Outcome::kUnknownRoot: s << "{\"unknown_root\":true}"; break;
    }
    out += s.str();
  }
  out += "}";
  std::printf("%s\n", out.c_str());
  ak_enc_ctx_free(cx.enc);
  ak_dec_ctx_free(cx.dec);
  return 0;
}
