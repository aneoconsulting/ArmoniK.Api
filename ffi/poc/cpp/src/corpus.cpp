// This slice as a consumer of `ffi/corpus` (README W8, `ffi/corpus/CONTRACT.md`).
//
// WHY, and it is the point of the whole exercise rather than a preamble. This slice's
// correctness gate is byte identity against `ffi/schema/generated/manifest.json`, and that
// manifest is generated from the SAME proto3 description the codec is generated from. It
// therefore cannot contain a byte sequence the generator would not write -- and proto3
// cannot express a group, so no payload in it carries wire type 3. `ak::Dec::skip` sent
// wire type 3 to `ERR_MALFORMED`, refusing a message protobuf C++ and upb both accept,
// and 443 conformance checks passed over it. The corpus is written against a SUPERSET
// schema by a different tool, which is exactly the second opinion byte identity cannot be.
//
// This binary is the C++ half: it decodes, projects, re-encodes and watches the refusals.
// The manifest, the scope arithmetic and the comparisons are in `gen/corpus.py`, because
// the obligation is stated in JSON and a JSON reader in C++ would be a second thing to be
// wrong. What crosses between them is a tab-separated task list in and tab-separated
// results out.
//
// Two arms, every row through both (CONTRACT.md section 5 item 5, which is what R2 is
// protecting): `native`, the codec emitted into C++ with no boundary, and `ffi`, the same
// codec in the shared Rust core through the C ABI. A row where the two disagree is a
// finding whatever the manifest says.
//
// Plus `walk`, which is not a root decode and is labelled as such everywhere it appears:
// `ak::Dec`'s unknown-field skipper run over a whole buffer, so that the corpus's
// wire-form vectors -- which root at `WireZoo`, a message this slice's schema does not
// have -- can still exercise the code path this work unit fixed. It parses nothing and
// projects nothing; it answers accept or reject and no more.
#include <algorithm>
#include <cstdio>
#include <cstring>
#include <fstream>
#include <iostream>
#include <sstream>
#include <string>
#include <utility>
#include <vector>

#include <google/protobuf/descriptor.h>
#include <google/protobuf/io/coded_stream.h>
#include <google/protobuf/io/zero_copy_stream_impl_lite.h>
#include <google/protobuf/message.h>

#include "ak/rt.h"
#include "ak_abi.h"
#include "generated/binding.h"
#include "generated/cases.h"
#include "generated/core_native.h"
#include "generated/project.h"
#include "generated/types.h"
#include "ak/projjson.h"
#include "ak/values.h"
#include "shapes.pb.h"

namespace ns = armonik::ffi::shapes::v1;

static std::string read_file(const std::string &p) {
  std::ifstream f(p.c_str(), std::ios::binary);
  std::ostringstream ss;
  ss << f.rdbuf();
  return ss.str();
}

// The bytes an arm wrote, as hex, for SMALL payloads only. The driver needs them for one
// question it cannot answer from a hash: an interleaved vector (P7.1) has no canonical
// writer, so a conformant re-encode is a PERMUTATION of the same (tag, wire type, body)
// triples rather than the same bytes, and "not an accepted form" and "a different message"
// have to be told apart. Capped, because a 4 MB payload in hex helps nobody.
static const std::size_t kHexCap = 8192;

static void out_hex(const std::string &id, const char *arm, const std::string &b) {
  if (b.size() > kHexCap) return;
  static const char *d = "0123456789abcdef";
  std::string h;
  h.reserve(b.size() * 2);
  for (std::size_t i = 0; i < b.size(); ++i) {
    unsigned char c = (unsigned char)b[i];
    h.push_back(d[c >> 4]);
    h.push_back(d[c & 15]);
  }
  std::printf("B\t%s\t%s\t%s\n", id.c_str(), arm, h.c_str());
}

// A field the driver reads back: tab-separated, and every value is printable.
static void out_result(const std::string &id, const char *arm, int32_t err,
                       const std::string &bytes) {
  if (err != 0) {
    std::printf("R\t%s\t%s\treject\t%d\t-\t0\n", id.c_str(), arm, (int)err);
  } else {
    std::printf("R\t%s\t%s\taccept\t0\t%s\t%zu\n", id.c_str(), arm,
                ak::values::sha256_hex(bytes).c_str(), bytes.size());
  }
}

// ---- the INCUMBENT as a third arm, through reflection --------------------------------
//
// R14 says the comparison is against what production runs, and on a correctness question
// that cuts the same way: the projection CONTRACT.md asks for is `ListFields` semantics,
// and protobuf C++ has `ListFields`. So the incumbent projects every vector too, with no
// generated projector and no second reading of section 3 -- the reflection API IS the
// presence rule. A row where the generated projector and the incumbent agree needs no
// argument; a row where they differ is a finding about one of them, and on `U-map-entry`
// it turned out to be a finding about neither.
static void proj_pb(const google::protobuf::Message &m, std::string *out);

static void pb_value(const google::protobuf::FieldDescriptor *fd,
                     const google::protobuf::Reflection *r,
                     const google::protobuf::Message &m, int index, std::string *out) {
  namespace gp = google::protobuf;
  bool rep = fd->is_repeated();
  switch (fd->cpp_type()) {
    case gp::FieldDescriptor::CPPTYPE_INT32:
      ak::proj::i64(rep ? r->GetRepeatedInt32(m, fd, index) : r->GetInt32(m, fd), out);
      break;
    case gp::FieldDescriptor::CPPTYPE_INT64:
      ak::proj::i64(rep ? r->GetRepeatedInt64(m, fd, index) : r->GetInt64(m, fd), out);
      break;
    case gp::FieldDescriptor::CPPTYPE_UINT32:
      ak::proj::i64(rep ? r->GetRepeatedUInt32(m, fd, index) : r->GetUInt32(m, fd), out);
      break;
    case gp::FieldDescriptor::CPPTYPE_UINT64:
      ak::proj::i64((long long)(rep ? r->GetRepeatedUInt64(m, fd, index)
                                    : r->GetUInt64(m, fd)), out);
      break;
    case gp::FieldDescriptor::CPPTYPE_DOUBLE:
      ak::proj::f64(rep ? r->GetRepeatedDouble(m, fd, index) : r->GetDouble(m, fd), out);
      break;
    case gp::FieldDescriptor::CPPTYPE_BOOL:
      ak::proj::boolean(rep ? r->GetRepeatedBool(m, fd, index) : r->GetBool(m, fd), out);
      break;
    case gp::FieldDescriptor::CPPTYPE_ENUM:
      // The VALUE, not the name: an enum value the descriptor does not declare has no
      // name to use, and "999" is the point of several vectors.
      ak::proj::i64(rep ? r->GetRepeatedEnumValue(m, fd, index) : r->GetEnumValue(m, fd),
                    out);
      break;
    case gp::FieldDescriptor::CPPTYPE_STRING: {
      std::string s = rep ? r->GetRepeatedString(m, fd, index) : r->GetString(m, fd);
      if (fd->type() == gp::FieldDescriptor::TYPE_BYTES) {
        ak::proj::hex(s, out);
      } else {
        ak::proj::str(s, out);
      }
      break;
    }
    case gp::FieldDescriptor::CPPTYPE_MESSAGE:
      proj_pb(rep ? r->GetRepeatedMessage(m, fd, index) : r->GetMessage(m, fd), out);
      break;
    default:
      out->append("null");
  }
}

static void proj_pb(const google::protobuf::Message &m, std::string *out) {
  namespace gp = google::protobuf;
  const gp::Reflection *r = m.GetReflection();
  std::vector<const gp::FieldDescriptor *> set;
  r->ListFields(m, &set);
  out->push_back('{');
  bool first = true;
  for (std::size_t i = 0; i < set.size(); ++i) {
    const gp::FieldDescriptor *fd = set[i];
    ak::proj::key(fd->name().c_str(), &first, out);
    if (fd->is_map()) {
      const gp::FieldDescriptor *kf = fd->message_type()->field(0);
      const gp::FieldDescriptor *vf = fd->message_type()->field(1);
      // A map is emitted in KEY ORDER so two implementations that agree on the map can
      // never disagree on the JSON. The driver parses this, so order is not load-bearing;
      // it is here so a human diff of two logs is readable.
      std::vector<std::pair<std::string, std::string> > kv;
      int n = r->FieldSize(m, fd);
      for (int j = 0; j < n; ++j) {
        const gp::Message &e = r->GetRepeatedMessage(m, fd, j);
        std::string k, v;
        ak::proj::str(e.GetReflection()->GetString(e, kf), &k);
        std::string tmp;
        pb_value(vf, e.GetReflection(), e, 0, &tmp);
        kv.push_back(std::make_pair(k, tmp));
      }
      std::sort(kv.begin(), kv.end());
      out->push_back('{');
      for (std::size_t j = 0; j < kv.size(); ++j) {
        if (j) out->push_back(',');
        out->append(kv[j].first);
        out->push_back(':');
        out->append(kv[j].second);
      }
      out->push_back('}');
    } else if (fd->is_repeated()) {
      out->push_back('[');
      int n = r->FieldSize(m, fd);
      for (int j = 0; j < n; ++j) {
        if (j) out->push_back(',');
        pb_value(fd, r, m, j, out);
      }
      out->push_back(']');
    } else {
      pb_value(fd, r, m, 0, out);
    }
  }
  out->push_back('}');
}

// One row, one root type. Instantiated once per root by AK_ROOTS below, so the set of
// roots this binary can run is the set the codec was generated for and cannot drift.
template <class F, class P>
struct Run {
  typedef int32_t (*NatDec)(const uint8_t *, size_t, F *);
  typedef void (*NatEnc)(const F &, ak::Enc *);
  typedef int32_t (*FfiDec)(ak_dec_ctx *, const uint8_t *, size_t, F *);
  typedef intptr_t (*FfiEnc)(ak_enc_ctx *, const F &, const shapes::ffi::Tcs &);
  typedef std::string (*Proj)(const F &);

  static void go(const std::string &id, const std::string &buf, NatDec nd, NatEnc ne,
                 FfiDec fd, FfiEnc fe, Proj pr) {
    const uint8_t *p = buf.empty() ? NULL : (const uint8_t *)buf.data();

    // ---- native ------------------------------------------------------------------
    F fn;
    int32_t rc = nd(p, buf.size(), &fn);
    std::string nat_bytes;
    if (rc == 0) {
      ak::Enc e(shapes::native::kSites);
      ne(fn, &e);
      nat_bytes.assign((const char *)e.data(), e.size());
    }
    out_result(id, "native", rc, nat_bytes);
    if (rc == 0) {
      std::printf("P\t%s\tnative\t%s\n", id.c_str(), pr(fn).c_str());
      out_hex(id, "native", nat_bytes);
    }

    // ---- ffi ---------------------------------------------------------------------
    F ff;
    ak_dec_ctx *dctx = ak_dec_ctx_new();
    int32_t rc2 = fd(dctx, p, buf.size(), &ff);
    int32_t derr = ak_dec_err(dctx);
    ak_dec_ctx_free(dctx);
    if (rc2 == 0 && derr != 0) rc2 = derr;
    std::string ffi_bytes;
    if (rc2 == 0) {
      ak_enc_ctx *ctx = ak_enc_ctx_new();
      shapes::ffi::Tcs tc = shapes::ffi::tcs_core();
      intptr_t erc = fe(ctx, ff, tc);
      const uint8_t *q = NULL;
      size_t n = 0;
      ak_enc_take(ctx, &q, &n);
      // The bytes are copied out BEFORE the context is freed. `ak_enc_take` hands back a
      // pointer INTO the context's buffer (ABI v1 section 4), so freeing first and
      // copying after is a use-after-free -- which is what the first run of this file
      // did, and it read as 114 rows writing "a form the manifest does not accept" with
      // the same wrong hash repeating across unrelated rows. Named because a wrong C3
      // count that looks like a codec defect is the expensive kind of mistake.
      if (erc >= 0) ffi_bytes.assign((const char *)q, n);
      ak_enc_ctx_free(ctx);
      if (erc < 0) {
        std::printf("R\t%s\tffi\tencode-failed\t%d\t-\t0\n", id.c_str(), (int)erc);
        return;
      }
    }
    out_result(id, "ffi", rc2, ffi_bytes);
    if (rc2 == 0) {
      std::printf("P\t%s\tffi\t%s\n", id.c_str(), pr(ff).c_str());
      out_hex(id, "ffi", ffi_bytes);
    }

    // ---- pb: the incumbent, same bytes, same obligations -------------------------
    P pb;
    bool pok = pb.ParseFromString(buf);
    if (!pok) {
      out_result(id, "pb", ak::ERR_MALFORMED, std::string());
    } else {
      // DETERMINISTIC map ordering, which byte identity needs on any message with a
      // map; it is what `conformance` compares against and it is carried as its own
      // timed row there rather than inside the incumbent's headline.
      std::string pbb;
      pbb.resize(pb.ByteSizeLong());
      {
        google::protobuf::io::ArrayOutputStream aos(
            pbb.empty() ? NULL : &pbb[0], (int)pbb.size());
        google::protobuf::io::CodedOutputStream cos(&aos);
        cos.SetSerializationDeterministic(true);
        pb.SerializeWithCachedSizes(&cos);
      }
      out_result(id, "pb", 0, pbb);
      std::string pj;
      proj_pb(pb, &pj);
      std::printf("P\t%s\tpb\t%s\n", id.c_str(), pj.c_str());
      out_hex(id, "pb", pbb);
    }

    // CONTRACT.md section 5 item 5: byte identity BETWEEN THIS SLICE'S OWN ARMS, on every
    // vector. Reported as its own line so the driver counts it rather than inferring it
    // from two hashes that happened to match the manifest.
    std::printf("A\t%s\t%s\n", id.c_str(),
                (rc == rc2 && fn == ff) ? "agree" : "DISAGREE");
  }
};

// The unknown-field walker. Not a decoder: it has no schema, so every field is unknown and
// every field goes through `ak::Dec::skip`. That is precisely the path a root decoder
// takes for a field it does not know, which is why the corpus's `WireZoo` vectors are
// worth running here even though this slice cannot root a decode at `WireZoo`.
static void walk(const std::string &id, const std::string &buf) {
  ak::Dec d(buf.empty() ? NULL : (const uint8_t *)buf.data(), buf.size());
  while (!d.at_end()) {
    uint64_t k = d.varint();
    if (d.err != 0) break;
    uint32_t t = (uint32_t)(k >> 3), w = (uint32_t)(k & 7);
    if (t == 0) { d.err = ak::ERR_MALFORMED; break; }
    d.skip(t, w);
  }
  std::printf("W\t%s\t%s\t%d\t%zu\t%zu\n", id.c_str(), d.err == 0 ? "accept" : "reject",
              (int)d.err, d.pos, buf.size());
}

int main(int argc, char **argv) {
  if (argc < 2) {
    std::fprintf(stderr, "usage: corpus <tasks.tsv>\n");
    return 2;
  }
  std::printf("# cpp slice: the conformance corpus, the C++ half\n");
  std::printf("#   -std           %ld\n", (long)__cplusplus);
  std::printf("#   linkage        %s\n", AK_LINKAGE);
  std::printf("#   arms           native (no boundary) and ffi (the C ABI over the shared core)\n");
  std::printf("#   walker         ak::Dec::skip over a buffer with no schema at all\n");
  std::fflush(stdout);

  std::ifstream tf(argv[1]);
  std::string line;
  while (std::getline(tf, line)) {
    if (line.empty() || line[0] == '#') continue;
    std::vector<std::string> c;
    std::string cur;
    std::istringstream ls(line);
    while (std::getline(ls, cur, '\t')) c.push_back(cur);
    if (c.size() < 4) continue;
    const std::string &kind = c[0], &id = c[1], &root = c[2], &path = c[3];
    std::string buf = read_file(path);
    if (kind == "walk") {
      walk(id, buf);
      continue;
    }
    bool dispatched = false;
#define X(Root, snake_root)                                                        \
  if (!dispatched && root == #Root) {                                              \
    dispatched = true;                                                             \
    Run<shapes::Root, ns::Root>::go(id, buf,                                       \
                          &shapes::native::decode_##snake_root,                    \
                          &shapes::native::encode_into_##snake_root,               \
                          &shapes::ffi::decode_with_##snake_root,                  \
                          &shapes::ffi::encode_into_##snake_root,                  \
                          &shapes::project::project_##snake_root);                 \
  }
    AK_ROOTS(X)
#undef X
    if (!dispatched) std::printf("U\t%s\t%s\n", id.c_str(), root.c_str());
  }
  return 0;
}
