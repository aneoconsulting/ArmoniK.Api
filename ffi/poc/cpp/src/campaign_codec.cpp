// design/CAMPAIGN.md section 4.1: the codec benchmark. One process, pinned by the runner to
// AK_CPU_CLIENT, no server. It emits one JSON object per sample (requirement 28) and NOTHING
// that summarises: summaries are gen/campaign_summary.py's, from the committed raw lines.
//
// Arms (requirement 8):
//   incumbent-prod   grpc++'s SerializationTraits<Message> -- what the generated stub calls:
//                    Serialize into a grpc::ByteBuffer, Deserialize from one (production, R14)
//   incumbent-best   protobuf C++ SerializeToString / ParseFromString (the labelled second row)
//   core-ffi         the generated binding through the C ABI (push decode), the core built
//                    with init-guard
//   host-gen         the codec generated into C++ by the same generator (no-boundary control)
// Directions (req. 9): encode; decode (the bare call); decode_read (decode, then every field
//   read by the generated traversal of src/generated/touch.cpp, over the facade or over
//   protoc's types). A message is decoded into a FRESH object every iteration and encoded
//   from the same object graph every iteration; protobuf C++ recomputes ByteSizeLong on every
//   Serialize, so no size memo is amortised (req. 11).
// Unknown fields (req. 10): host-gen in drop AND retain; core-ffi in drop AND retain (ABI v1
//   decision 11, WP5 step 9: the binding's `encode_into_*_unk` over the u-groups and
//   `decode_with_*_unk`, a decode with every position of `ak_dec_<Root>_opts` armed through
//   ak_dec_reset_<Root>, grow-backed, then disarmed); the incumbent in its default mode
//   (protobuf C++ 3.x RETAINS unknown fields), stated as unknown_mode "default".
// Payloads (req. 7): the 15 buildable payloads of SHAPES.md plus P7.1 (decode only, from its
//   vector), the Latin-1 and wide content sets on P1.2, P2.2, P3.1, P4.1 and P6.1 (the set
//   the committed content-set gate covers; SHAPES.md: string-path payloads, P6.1 the
//   control), and the corpus's unknown-class rows rooted at a shapes root (decode only).
// Timing (req. 22a): Google Benchmark, see the end of main(). This file prints no sample;
// the samples are Google Benchmark's per-repetition JSON, converted by gen/gbench_to_jsonl.py.
// Correctness first (req. 26): before round 1, every arm of every group is run once and
// checked. The canonical bytes are host-gen's encoding, required equal to the manifest's
// SHA-256 on ASCII (no manifest covers the other two sets). The core's encoders must write
// exactly those bytes; the incumbent's encoders must write bytes that DECODE to the same
// message (every field folded), because protobuf C++ writes P2.5 in the other valid form and
// its map order is unspecified; every decoder's fold of every field must equal the
// incumbent's fold of the canonical bytes, and the two independent builders must agree. A failure
// prints the group and exits 2 before any timing.
//
//   campaign_codec --launch L --rounds R --bytes B --warmup W [--only ID,ID]
//                  [--corpus DIR --rows TSV] [--payloads DIR] [--gbench-out FILE]
#include <benchmark/benchmark.h>
#ifndef AK_GBENCH_VERSION
#define AK_GBENCH_VERSION "unknown"
#endif
#include <grpcpp/grpcpp.h>
#include <grpcpp/impl/codegen/proto_utils.h>
#include <sched.h>
#include <time.h>

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <functional>
#include <iterator>
#include <sstream>
#include <string>
#include <vector>

#include "harness.h"
#ifndef AK_NO_UNKNOWN_FIELDS
#include "generated/core_native_retain.h"
#endif
#include "generated/touch.h"

// Requirement 10's switch: on since the binding exports decision 11's retain entry points
// (WP5 step 9). -DAK_CAMPAIGN_NO_FFI_RETAIN removes the arm (the gate line says so).
#if defined(AK_NO_UNKNOWN_FIELDS)
// WP5 step 10, the NO-UNKNOWN build (a separate binary, campaign_codec_nounk): unknown-field
// support compiled out of the core, the binding and (R-H22) the facade. core-ffi and host-gen
// both run in mode "no-unknown" (host-gen: the drop rendering over the facade without
// unknown_fields); the incumbents run in their default mode.
#define AK_FFI_RETAIN_ENC(s) NULL
#define AK_FFI_RETAIN_DEC(s) NULL
#define AK_FFI_RETAIN_STATE "compiled out (no-unknown build)"
#define AK_FFI_DROP_MODE "no-unknown"
#define AK_HOSTGEN_MODES 1
// R-H22: host-gen here is the drop rendering compiled against the facade WITHOUT
// unknown_fields, i.e. host-gen's no-unknown mode (CAMPAIGN req 10).
#define AK_HOSTGEN_DROP_MODE "no-unknown"
#elif !defined(AK_CAMPAIGN_NO_FFI_RETAIN)
#define AK_FFI_RETAIN_ENC(s) &shapes::ffi::encode_into_##s##_unk
#define AK_FFI_RETAIN_DEC(s) &shapes::ffi::decode_with_##s##_unk
#define AK_FFI_RETAIN_STATE "built"
#else
#define AK_FFI_RETAIN_ENC(s) NULL
#define AK_FFI_RETAIN_DEC(s) NULL
#define AK_FFI_RETAIN_STATE "removed by -DAK_CAMPAIGN_NO_FFI_RETAIN"
#endif
#ifndef AK_FFI_DROP_MODE
#define AK_FFI_DROP_MODE "drop"
#define AK_HOSTGEN_MODES 2
#define AK_HOSTGEN_DROP_MODE "drop"
#endif
// host-gen retain: rendered from the plan's retain options in the full build only (R-H22:
// the no-unknown facade has no unknown_fields member for a retain codec to fill).
#ifdef AK_NO_UNKNOWN_FIELDS
#define AK_NATR_ENC(s) NULL
#define AK_NATR_DEC(s) NULL
#else
#define AK_NATR_ENC(s) &shapes::native_retain::encode_into_##s
#define AK_NATR_DEC(s) &shapes::native_retain::decode_##s
#endif

namespace {

struct Cfg {
  int launch = 0, rounds = 5;
  double bytes = 32.0 * 1024 * 1024;  // per sample: iterations = bytes / wire size
  double warmup = -1;                 // warm-up bytes per arm; default = one sample
  std::vector<std::string> only;
  std::string corpus, payloads, rows;
  std::string gbout = "campaign_codec_gbench.json";  // Google Benchmark's JSON output
};
Cfg g_cfg;


volatile uint64_t g_sink = 0;
// Keeps a decoded object alive without reading it (the bare decode direction).
#define AK_KEEP(x) asm volatile("" : : "r"(&(x)) : "memory")

// One timed thing: an arm in one direction and one unknown mode. `run(n)` does n iterations
// and returns a fold (so nothing is dead). `check()` returns "" or what is wrong.
struct Slot {
  std::string arm, dir, mode;
  std::function<uint64_t(long)> run;
  std::function<std::string()> check;
};

struct Group {
  std::string payload, content;
  size_t wire = 0;
  std::vector<Slot> slots;
};

std::string read_file(const std::string &p) {
  std::ifstream f(p.c_str(), std::ios::binary);
  return std::string((std::istreambuf_iterator<char>(f)), std::istreambuf_iterator<char>());
}

std::string det_bytes(const google::protobuf::MessageLite &m) {
  std::string s;
  pb_serialize_det(m, &s);
  return s;
}

// ---- one root type's arms ---------------------------------------------------------------
template <class Fac, class Pb>
struct Fns {
  intptr_t (*ffi_enc)(ak_enc_ctx *, const Fac &, const shapes::ffi::Tcs &);
  int32_t (*ffi_dec)(ak_dec_ctx *, const uint8_t *, size_t, Fac *);
  void (*nat_enc)(const Fac &, ak::Enc *);
  int32_t (*nat_dec)(const uint8_t *, size_t, Fac *);
  void (*natr_enc)(const Fac &, ak::Enc *);
  int32_t (*natr_dec)(const uint8_t *, size_t, Fac *);
  // Requirement 10's hook: core-ffi in RETAIN mode (ABI v1 decision 11, every position's
  // options entry armed). NULL until the generated binding exports the decision-11 retain
  // entry points (WP5 step 7); set here, the arm appears in the gate and the rounds.
  intptr_t (*ffi_enc_retain)(ak_enc_ctx *, const Fac &, const shapes::ffi::Tcs &);
  int32_t (*ffi_dec_retain)(ak_dec_ctx *, const uint8_t *, size_t, Fac *);
};

struct Ctx {
  ak_enc_ctx *ec;
  // Decision 11 rule 6: one bound decode context per root (drop mode between decodes; the
  // retain arm arms and disarms its root's context inside decode_with_*_unk).
  shapes::ffi::DecCtxs *dcs;
  ak::Enc *ne, *nre;
};

// `fac` / `pb`: the object graph encoders start from (null for a decode-only group);
// `canon`: the canonical bytes decoders read and encoders must reproduce.
template <class Fac, class Pb>
Group make_group(const std::string &payload, const std::string &content, const Fns<Fac, Pb> &F,
                 const Fac *fac, const Pb *pb, const std::string &canon, Ctx *cx) {
  Group g;
  g.payload = payload;
  g.content = content;
  g.wire = canon.size();
  // grpc++'s production decode reads a ByteBuffer. One slice holding the canonical bytes,
  // referenced (not copied) into a fresh ByteBuffer every iteration.
  std::shared_ptr<grpc::Slice> slice(new grpc::Slice(canon.data(), canon.size()));
  const std::string *cp = new std::string(canon);  // lives for the process
  const uint8_t *cb = (const uint8_t *)cp->data();
  size_t cn = cp->size();
  // AK_CAMPAIGN_PLANT=1: the gate seen failing. core-ffi's decoders read, and its encoder is
  // compared against, a copy of the canonical bytes with one byte changed; every core-ffi
  // slot must then fail its check and nothing may be timed.
  const std::string *cpf = cp;
  if (std::getenv("AK_CAMPAIGN_PLANT") && !canon.empty()) {
    std::string *x = new std::string(canon);
    (*x)[x->size() / 2] ^= 0x01;
    (*x)[x->size() - 1] ^= 0x01;
    cpf = x;
  }
  const uint8_t *cbf = (const uint8_t *)cpf->data();
  shapes::ffi::Tcs tc = shapes::ffi::tcs_core();

  // Reference fold for the decode checks: the incumbent's decode of the canonical bytes.
  Pb ref;
  ref.ParseFromArray(cb, (int)cn);
  const uint64_t want_fold = pbtouch::touch(ref);

  if (fac && pb) {
    // ---- encode
    g.slots.push_back({"incumbent-prod", "encode", "default", [pb](long n) {
      uint64_t h = 0;
      for (long i = 0; i < n; ++i) {
        grpc::ByteBuffer bb;
        bool own = false;
        grpc::SerializationTraits<Pb>::Serialize(*pb, &bb, &own);
        h += bb.Length();
      }
      return h;
    }, [pb, want_fold]() -> std::string {
      grpc::ByteBuffer bb;
      bool own = false;
      if (!grpc::SerializationTraits<Pb>::Serialize(*pb, &bb, &own).ok()) return "Serialize failed";
      std::vector<grpc::Slice> sl;
      bb.Dump(&sl);
      std::string s;
      for (auto &x : sl) s.append((const char *)x.begin(), x.size());
      Pb m;
      if (!m.ParseFromString(s) || pbtouch::touch(m) != want_fold) return "output does not decode to the canonical message";
      return "";
    }});
    g.slots.push_back({"incumbent-best", "encode", "default", [pb](long n) {
      uint64_t h = 0;
      std::string s;
      for (long i = 0; i < n; ++i) { pb->SerializeToString(&s); h += s.size(); }
      return h;
    }, [pb, want_fold]() -> std::string {
      std::string s;
      pb->SerializeToString(&s);
      Pb m;
      if (!m.ParseFromString(s) || pbtouch::touch(m) != want_fold) return "output does not decode to the canonical message";
      if (pbtouch::touch(*pb) != want_fold) return "the incumbent's own builder and the facade builder disagree";
      return "";
    }});
    g.slots.push_back({"core-ffi", "encode", AK_FFI_DROP_MODE, [F, fac, cx, tc](long n) {
      uint64_t h = 0;
      for (long i = 0; i < n; ++i) {
        F.ffi_enc(cx->ec, *fac, tc);
        const uint8_t *p; size_t len;
        ak_enc_take(cx->ec, &p, &len);
        h += len;
      }
      return h;
    }, [F, fac, cx, tc, cpf]() -> std::string {
      intptr_t rc = F.ffi_enc(cx->ec, *fac, tc);
      const uint8_t *p = NULL; size_t len = 0;
      if (rc < 0 || ak_enc_take(cx->ec, &p, &len) != 0) return "encode refused";
      return std::string((const char *)p, len) == *cpf ? "" : "bytes differ from the canonical";
    }});
    if (F.ffi_enc_retain) {
      g.slots.push_back({"core-ffi", "encode", "retain", [F, fac, cx, tc](long n) {
        uint64_t h = 0;
        for (long i = 0; i < n; ++i) {
          F.ffi_enc_retain(cx->ec, *fac, tc);
          const uint8_t *p; size_t len;
          ak_enc_take(cx->ec, &p, &len);
          h += len;
        }
        return h;
      }, [F, fac, cx, tc, cp]() -> std::string {
        intptr_t rc = F.ffi_enc_retain(cx->ec, *fac, tc);
        const uint8_t *p = NULL; size_t len = 0;
        if (rc < 0 || ak_enc_take(cx->ec, &p, &len) != 0) return "encode refused";
        return std::string((const char *)p, len) == *cp ? "" : "bytes differ from the canonical";
      }});
    }
    for (int r = 0; r < AK_HOSTGEN_MODES; ++r) {
      bool ret = r == 1;
      g.slots.push_back({"host-gen", "encode", ret ? "retain" : AK_HOSTGEN_DROP_MODE, [F, fac, cx, ret](long n) {
        uint64_t h = 0;
        for (long i = 0; i < n; ++i) {
          if (ret) { F.natr_enc(*fac, cx->nre); h += cx->nre->size(); }
          else { F.nat_enc(*fac, cx->ne); h += cx->ne->size(); }
        }
        return h;
      }, [F, fac, cx, ret, cp]() -> std::string {
        ak::Enc *e = ret ? cx->nre : cx->ne;
        if (ret) F.natr_enc(*fac, e); else F.nat_enc(*fac, e);
        return std::string((const char *)e->data(), e->size()) == *cp ? "" : "bytes differ from the canonical";
      }});
    }
  }
  // ---- decode and decode_read
  for (int rd = 0; rd < 2; ++rd) {
    const bool read = rd == 1;
    const char *dir = read ? "decode_read" : "decode";
    g.slots.push_back({"incumbent-prod", dir, "default", [slice, read](long n) {
      uint64_t h = 0;
      for (long i = 0; i < n; ++i) {
        grpc::ByteBuffer bb(slice.get(), 1);
        Pb m;
        grpc::SerializationTraits<Pb>::Deserialize(&bb, &m);
        if (read) h += pbtouch::touch(m); else { AK_KEEP(m); ++h; }
      }
      return h;
    }, [slice, want_fold]() -> std::string {
      grpc::ByteBuffer bb(slice.get(), 1);
      Pb m;
      if (!grpc::SerializationTraits<Pb>::Deserialize(&bb, &m).ok()) return "Deserialize failed";
      return pbtouch::touch(m) == want_fold ? "" : "field fold differs";
    }});
    g.slots.push_back({"incumbent-best", dir, "default", [cp, read](long n) {
      uint64_t h = 0;
      for (long i = 0; i < n; ++i) {
        Pb m;
        m.ParseFromString(*cp);
        if (read) h += pbtouch::touch(m); else { AK_KEEP(m); ++h; }
      }
      return h;
    }, [cp, want_fold]() -> std::string {
      Pb m;
      if (!m.ParseFromString(*cp)) return "parse failed";
      return pbtouch::touch(m) == want_fold ? "" : "field fold differs";
    }});
    g.slots.push_back({"core-ffi", dir, AK_FFI_DROP_MODE, [F, cx, cbf, cn, read](long n) {
      uint64_t h = 0;
      for (long i = 0; i < n; ++i) {
        Fac v;
        h += (uint64_t)F.ffi_dec(cx->dcs->of<Fac>(), cbf, cn, &v);
        if (read) h += shapes::touch::touch(v);
      }
      return h;
    }, [F, cx, cbf, cn, want_fold]() -> std::string {
      Fac v;
      int32_t rc = F.ffi_dec(cx->dcs->of<Fac>(), cbf, cn, &v);
      if (rc != 0 || ak_dec_err(cx->dcs->of<Fac>()) != 0) { ak_dec_err_reset(cx->dcs->of<Fac>()); return "decode refused"; }
      return shapes::touch::touch(v) == want_fold ? "" : "field fold differs from the incumbent's";
    }});
    if (F.ffi_dec_retain) {
      g.slots.push_back({"core-ffi", dir, "retain", [F, cx, cb, cn, read](long n) {
        uint64_t h = 0;
        for (long i = 0; i < n; ++i) {
          Fac v;
          h += (uint64_t)F.ffi_dec_retain(cx->dcs->of<Fac>(), cb, cn, &v);
          if (read) h += shapes::touch::touch(v);
        }
        return h;
      }, [F, cx, cb, cn, want_fold]() -> std::string {
        Fac v;
        int32_t rc = F.ffi_dec_retain(cx->dcs->of<Fac>(), cb, cn, &v);
        if (rc != 0 || ak_dec_err(cx->dcs->of<Fac>()) != 0) { ak_dec_err_reset(cx->dcs->of<Fac>()); return "decode refused"; }
        return shapes::touch::touch(v) == want_fold ? "" : "field fold differs from the incumbent's";
      }});
    }
    for (int r = 0; r < AK_HOSTGEN_MODES; ++r) {
      bool ret = r == 1;
      g.slots.push_back({"host-gen", dir, ret ? "retain" : AK_HOSTGEN_DROP_MODE, [F, cb, cn, read, ret](long n) {
        uint64_t h = 0;
        for (long i = 0; i < n; ++i) {
          Fac v;
          h += (uint64_t)(ret ? F.natr_dec(cb, cn, &v) : F.nat_dec(cb, cn, &v));
          if (read) h += shapes::touch::touch(v);
        }
        return h;
      }, [F, cb, cn, ret, want_fold]() -> std::string {
        Fac v;
        int32_t rc = ret ? F.natr_dec(cb, cn, &v) : F.nat_dec(cb, cn, &v);
        if (rc != 0) return "decode refused";
        return shapes::touch::touch(v) == want_fold ? "" : "field fold differs from the incumbent's";
      }});
    }
  }
  return g;
}

bool wanted(const std::string &id) {
  if (g_cfg.only.empty()) return true;
  for (size_t i = 0; i < g_cfg.only.size(); ++i)
    if (id == g_cfg.only[i] || id.compare(0, g_cfg.only[i].size(), g_cfg.only[i]) == 0) return true;
  return false;
}

// The corpus rows: unknown class, rooted at a shapes root, accepted (decode only).
struct CorpusRow { std::string id, root, file; };
std::vector<CorpusRow> corpus_rows(const std::string &rows) {
  // The runner extracts the rows from the manifest with python (gen/run_campaign.sh) into a
  // TSV, one "id<TAB>root<TAB>file" per line, file relative to --corpus.
  std::vector<CorpusRow> v;
  std::ifstream f(rows.c_str());
  std::string line;
  while (std::getline(f, line)) {
    std::istringstream is(line);
    CorpusRow r;
    std::getline(is, r.id, '\t');
    std::getline(is, r.root, '\t');
    std::getline(is, r.file, '\t');
    if (!r.id.empty()) v.push_back(r);
  }
  return v;
}

}  // namespace

int main(int argc, char **argv) {
  for (int i = 1; i + 1 < argc; i += 2) {
    std::string a = argv[i];
    const char *v = argv[i + 1];
    if (a == "--launch") g_cfg.launch = std::atoi(v);
    else if (a == "--rounds") g_cfg.rounds = std::atoi(v);
    else if (a == "--bytes") g_cfg.bytes = std::atof(v);
    else if (a == "--warmup") g_cfg.warmup = std::atof(v);
    else if (a == "--corpus") g_cfg.corpus = v;
    else if (a == "--payloads") g_cfg.payloads = v;
    else if (a == "--rows") g_cfg.rows = v;
    else if (a == "--gbench-out") g_cfg.gbout = v;
    else if (a == "--only") {
      std::string s(v);
      size_t p = 0;
      while (p <= s.size()) {
        size_t q = s.find(',', p);
        g_cfg.only.push_back(s.substr(p, q == std::string::npos ? std::string::npos : q - p));
        if (q == std::string::npos) break;
        p = q + 1;
      }
    }
  }
  if (g_cfg.warmup < 0) g_cfg.warmup = g_cfg.bytes;
  grpc_init();
  if (shapes::ffi::ak_init_once() != AK_OK) { std::fprintf(stderr, "ak_init refused\n"); return 2; }

  Ctx cx;
  cx.ec = ak_enc_ctx_new();
  cx.dcs = new shapes::ffi::DecCtxs();
  if (!cx.dcs->ok()) { std::fprintf(stderr, "ak_dec_ctx_new_<Root> refused\n"); return 2; }
  cx.ne = new ak::Enc(shapes::native::kSites);
#ifdef AK_NO_UNKNOWN_FIELDS
  cx.nre = NULL;  // no host-gen retain in the no-unknown build
#else
  cx.nre = new ak::Enc(shapes::native_retain::kSites);
#endif

  std::vector<Group> groups;
  static const ak::values::ContentSet sets[3] = {ak::values::kAscii, ak::values::kLatin1, ak::values::kWide};
  static const char *setname[3] = {"ascii", "latin1", "wide"};
  int gate_fail = 0;

#define X(id, Root, sroot, pfx, sha, nbytes)                                                     \
  {                                                                                             \
    Fns<shapes::Root, ns::Root> F = {&shapes::ffi::encode_into_##sroot,                         \
                                     &shapes::ffi::decode_with_##sroot,                         \
                                     &shapes::native::encode_into_##sroot,                      \
                                     &shapes::native::decode_##sroot,                           \
                                     AK_NATR_ENC(sroot),               \
                                     AK_NATR_DEC(sroot),                    \
                                     AK_FFI_RETAIN_ENC(sroot), AK_FFI_RETAIN_DEC(sroot)};       \
    std::string pid(id);                                                                        \
    bool cs = pid == "P1.2" || pid == "P2.2" || pid == "P3.1" || pid == "P4.1" || pid == "P6.1"; \
    for (int s = 0; s < (cs ? 3 : 1); ++s) {                                                    \
      if (!wanted(pid)) break;                                                                  \
      ak::values::ScopedContentSet scope(sets[s]);                                              \
      shapes::Root *fac = new shapes::Root(shapes::build::payload_##pfx());                     \
      ns::Root *pb = new ns::Root();                                                            \
      pbbuild::payload_##pfx(pb);                                                               \
      shapes::native::encode_into_##sroot(*fac, cx.ne);                                        \
      std::string canon((const char *)cx.ne->data(), cx.ne->size());                           \
      if (s == 0 && sha_of(canon) != std::string(sha)) {                                        \
        std::printf("GATE FAIL %s ascii: the canonical bytes are not the manifest's\n", id);   \
        ++gate_fail;                                                                            \
      }                                                                                         \
      groups.push_back(make_group<shapes::Root, ns::Root>(pid, setname[s], F, fac, pb, canon, &cx)); \
    }                                                                                           \
  }
  AK_CASES(X)
#undef X

  // P7.1: decode only, from its committed vector (no canonical writer can produce it).
  if (wanted("P7.1")) {
    std::string dir = g_cfg.payloads.empty() ? std::string("payloads") : g_cfg.payloads;
    std::string v = read_file(dir + "/" + AK_P71_VECTOR);
    if (sha_of(v) != std::string(AK_P71_SHA)) {
      std::printf("GATE FAIL P7.1 vector missing or wrong (%s)\n", (dir + "/" + AK_P71_VECTOR).c_str());
      ++gate_fail;
    } else {
      Fns<shapes::DualResponse, ns::DualResponse> F = {
          &shapes::ffi::encode_into_dual_response, &shapes::ffi::decode_with_dual_response,
          &shapes::native::encode_into_dual_response, &shapes::native::decode_dual_response,
          AK_NATR_ENC(dual_response), AK_NATR_DEC(dual_response),
          AK_FFI_RETAIN_ENC(dual_response), AK_FFI_RETAIN_DEC(dual_response)};
      groups.push_back(make_group<shapes::DualResponse, ns::DualResponse>("P7.1", "ascii", F, NULL, NULL, v, &cx));
    }
  }
  // The corpus's unknown-class rows at a shapes root: decode only.
  if (!g_cfg.corpus.empty() && !g_cfg.rows.empty()) {
    std::vector<CorpusRow> rows = corpus_rows(g_cfg.rows);
    for (size_t i = 0; i < rows.size(); ++i) {
      if (!wanted(rows[i].id)) continue;
      std::string v = read_file(g_cfg.corpus + "/" + rows[i].file);
#define R(Root, sroot)                                                                          \
      if (rows[i].root == #Root) {                                                              \
        Fns<shapes::Root, ns::Root> F = {&shapes::ffi::encode_into_##sroot,                     \
            &shapes::ffi::decode_with_##sroot, &shapes::native::encode_into_##sroot,            \
            &shapes::native::decode_##sroot, AK_NATR_ENC(sroot),       \
            AK_NATR_DEC(sroot),                                             \
            AK_FFI_RETAIN_ENC(sroot), AK_FFI_RETAIN_DEC(sroot)};                                \
        groups.push_back(make_group<shapes::Root, ns::Root>(rows[i].id, "ascii", F, NULL, NULL, v, &cx)); \
      }
      AK_ROOTS(R)
#undef R
    }
  }

  // ---- the gate: every slot once, before any timing ----
  size_t nslots = 0;
  for (size_t gi = 0; gi < groups.size(); ++gi) {
    for (size_t s = 0; s < groups[gi].slots.size(); ++s) {
      ++nslots;
      std::string why = groups[gi].slots[s].check();
      if (!why.empty()) {
        ++gate_fail;
        std::printf("GATE FAIL %s %s %s %s %s: %s\n", groups[gi].payload.c_str(),
                    groups[gi].content.c_str(), groups[gi].slots[s].arm.c_str(),
                    groups[gi].slots[s].dir.c_str(), groups[gi].slots[s].mode.c_str(), why.c_str());
      }
    }
  }
  std::printf("# {\"campaign_codec_gate\": {\"groups\": %zu, \"slots\": %zu, \"failed\": %d,"
              " \"ffi_retain\": \"%s\"}}\n", groups.size(), nslots, gate_fail, AK_FFI_RETAIN_STATE);
  if (gate_fail) {
    std::printf("# GATE FAILED: nothing is timed\n");
    return 2;
  }
  cpu_set_t set;
  CPU_ZERO(&set);
  sched_getaffinity(0, sizeof set, &set);
  std::string cpus;
  for (int c = 0; c < CPU_SETSIZE; ++c)
    if (CPU_ISSET(c, &set)) cpus += (cpus.empty() ? "" : ",") + std::to_string(c);
  std::printf("# {\"campaign_codec\": {\"launch\": %d, \"rounds\": %d, \"bytes_per_sample\": %.0f,"
              " \"warmup_bytes_per_slot\": %.0f, \"affinity\": \"%s\", \"timer\": \"Google Benchmark %s: cpu_time (benchmark thread) and real_time\"}}\n",
              g_cfg.launch, g_cfg.rounds, g_cfg.bytes, g_cfg.warmup, cpus.c_str(), AK_GBENCH_VERSION);

  // ---- warm-up: every slot, the same byte budget, before round 1 (req. 24, 25) ----
  for (size_t gi = 0; gi < groups.size(); ++gi) {
    long n = (long)(g_cfg.warmup / (double)(groups[gi].wire ? groups[gi].wire : 1));
    if (n < 1) n = 1;
    for (size_t s = 0; s < groups[gi].slots.size(); ++s) g_sink += groups[gi].slots[s].run(n);
  }
  // ---- timing: Google Benchmark (CAMPAIGN.md requirement 22a, owner 2026-09-25) ----
  // One benchmark per slot, name "arm|payload|content|dir|unknown_mode". Fixed iterations
  // per group (the byte budget / the wire size, as before), `rounds` repetitions, every
  // repetition reported raw (no aggregate-only), repetitions interleaved across all
  // benchmarks in random order (--benchmark_enable_random_interleaving), and the
  // registration order rotated by launch so the three launches start from different arms.
  // cpu_time is Google Benchmark's default CPU timer: the benchmark thread's CPU time;
  // real_time is wall. gen/gbench_to_jsonl.py turns the JSON into section 7's lines.
  if (g_cfg.rounds > 0) {
    std::vector<std::pair<std::string, std::pair<Slot *, long> > > regs;
    for (size_t gi = 0; gi < groups.size(); ++gi) {
      Group &g = groups[gi];
      long n = (long)(g_cfg.bytes / (double)(g.wire ? g.wire : 1));
      if (n < 1) n = 1;
      for (size_t s = 0; s < g.slots.size(); ++s) {
        Slot *sl = &g.slots[s];
        regs.push_back(std::make_pair(sl->arm + "|" + g.payload + "|" + g.content + "|" + sl->dir + "|" + sl->mode,
                                      std::make_pair(sl, n)));
      }
    }
    size_t nr = regs.size(), rot = nr ? ((size_t)g_cfg.launch * 7919u) % nr : 0;
    for (size_t k = 0; k < nr; ++k) {
      const std::pair<std::string, std::pair<Slot *, long> > &r = regs[(k + rot) % nr];
      Slot *sl = r.second.first;
      benchmark::RegisterBenchmark(r.first.c_str(), [sl](benchmark::State &st) {
        for (auto _ : st) {
          uint64_t h = sl->run(1);
          benchmark::DoNotOptimize(h);
          benchmark::ClobberMemory();
        }
      })->Iterations(r.second.second)->Repetitions(g_cfg.rounds)->Unit(benchmark::kNanosecond)
        ->ReportAggregatesOnly(false);
    }
    std::string out = "--benchmark_out=" + g_cfg.gbout;
    std::vector<std::string> args = {"campaign_codec", out, "--benchmark_out_format=json",
                                     "--benchmark_enable_random_interleaving=true",
                                     "--benchmark_min_warmup_time=0",
                                     "--benchmark_format=console"};
    std::vector<char *> av;
    for (size_t k = 0; k < args.size(); ++k) av.push_back(&args[k][0]);
    int ac = (int)av.size();
    benchmark::Initialize(&ac, av.data());
    benchmark::RunSpecifiedBenchmarks();
    benchmark::Shutdown();
  }
  return 0;
}
