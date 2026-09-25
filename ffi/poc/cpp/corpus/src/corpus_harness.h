// The cpp slice's full-corpus harness (FIX-PLAN WP5 item 6.1), the C++ half: generic arm
// runners over the GENERATED facade, native codecs, binding and projection of the corpus
// reader schema. Glue: no wire rule and no layout lives here -- it calls generated code
// and prints what came back. The verdict is taken by `gen/corpus_all.py`.
#ifndef AK_CORPUS_HARNESS_H
#define AK_CORPUS_HARNESS_H

#include <cstdint>
#include <cstdio>
#include <cstring>
#include <sstream>
#include <string>
#include <vector>

#include "ak_abi.h"
#include "ak/rt.h"
#include "generated/types.h"
#include "generated/core_native.h"
#include "generated/core_native_retain.h"
#include "generated/binding.h"
#include "generated/project.h"

namespace corpus {

enum Arm { kFfiDrop, kFfiRetain, kNativeDrop, kNativeRetain };

struct Outcome {
  enum Kind { kOk, kErr, kNotInAbi, kNotBuilt, kUnknownRoot } kind;
  int32_t err;          // kErr: the decode's error code
  std::string proj;     // kOk: the projection (JSON text)
  bool enc_ok;          // kOk: whether the re-encode succeeded
  int32_t enc_err;      // kOk and !enc_ok: its error code
  std::string bytes;    // kOk and enc_ok: the re-encoding

  Outcome() : kind(kErr), err(0), enc_ok(false), enc_err(0) {}
  static Outcome error(int32_t e) { Outcome o; o.kind = kErr; o.err = e; return o; }
  static Outcome not_in_abi() { Outcome o; o.kind = kNotInAbi; return o; }
  static Outcome not_built() { Outcome o; o.kind = kNotBuilt; return o; }
  static Outcome unknown_root() { Outcome o; o.kind = kUnknownRoot; return o; }
};

// Decision 11 rule 6: a decode context is bound to its root, so an ffi arm creates the
// context of ITS root (`ffi::DecRoot<T>`); there is no shared untyped decode context.
struct Cx {
  ak_enc_ctx *enc;
  ffi::Tcs tcs;
};

template <class T>
Outcome native_arm(const uint8_t *b, size_t n, size_t sites,
                   int32_t (*dec)(const uint8_t *, size_t, T *),
                   void (*enc)(const T &, ak::Enc *), std::string (*proj)(const T &)) {
  T v;
  int32_t rc = dec(b, n, &v);
  // After a decode error the object is unspecified and discarded (R-G6).
  if (rc != 0) return Outcome::error(rc);
  Outcome o;
  o.kind = Outcome::kOk;
  o.proj = proj(v);
  ak::Enc e(sites);
  enc(v, &e);
  if (e.err != 0) {
    o.enc_ok = false;
    o.enc_err = e.err;
  } else {
    o.enc_ok = true;
    o.bytes.assign((const char *)e.data(), e.size());
  }
  return o;
}

template <class T>
Outcome ffi_arm(const uint8_t *b, size_t n, Cx &cx,
                int32_t (*dec)(ak_dec_ctx *, const uint8_t *, size_t, T *),
                intptr_t (*enc)(ak_enc_ctx *, const T &, const ffi::Tcs &),
                std::string (*proj)(const T &)) {
  T v;
  ak_dec_ctx *dc = ffi::dec_ctx_new_for<T>();
  if (dc == NULL) return Outcome::error(AK_ERR_INVALID_STATE);
  int32_t rc = dec(dc, b, n, &v);
  if (rc == 0) rc = ak_dec_err(dc);
  ak_dec_ctx_free(dc);
  if (rc != 0) return Outcome::error(rc);
  Outcome o;
  o.kind = Outcome::kOk;
  o.proj = proj(v);
  intptr_t erc = enc(cx.enc, v, cx.tcs);
  const uint8_t *p = NULL;
  size_t len = 0;
  int32_t trc = ak_enc_take(cx.enc, &p, &len);
  if (erc < 0 || trc != 0 || ak_enc_err(cx.enc) != 0) {
    o.enc_ok = false;
    o.enc_err = erc < 0 ? (int32_t)erc : (trc != 0 ? trc : ak_enc_err(cx.enc));
  } else {
    o.enc_ok = true;
    o.bytes.assign((const char *)p, len);
  }
  return o;
}

// The facade value as the retain encode writes it: every bag the facade holds, compared
// as bytes (so a NaN equals itself, S-double-nan). "E<rc>" when the encode refuses.
template <class T>
std::string unk_bytes(Cx &cx, intptr_t (*enc)(ak_enc_ctx *, const T &, const ffi::Tcs &),
                      const T &v) {
  intptr_t erc = enc(cx.enc, v, cx.tcs);
  const uint8_t *p = NULL;
  size_t len = 0;
  int32_t trc = ak_enc_take(cx.enc, &p, &len);
  if (erc < 0 || trc != 0 || ak_enc_err(cx.enc) != 0) {
    std::ostringstream s;
    s << "E" << (erc < 0 ? (int32_t)erc : (trc != 0 ? trc : ak_enc_err(cx.enc)));
    return s.str();
  }
  return std::string((const char *)p, len);
}

template <class T>
int32_t unk_dec_rc(ak_dec_ctx *dc, int32_t rc) {
  if (rc == 0) rc = ak_dec_err(dc);
  ak_dec_err_reset(dc);
  return rc;
}

// Decision 11's controls on one vector, through the binding's armed decodes:
//   * retain everywhere (`_unk`, grow only) is the reference, `all`;
//   * the POOL decode (k = 2 buffers of 8 bytes per repeated position, refilled in place
//     after every delivery, grow as fallback) must give exactly `all`;
//   * the DROP decode (context bound, NULL options) must give `all` with every position's
//     bags cleared;
//   * each position ZEROED in turn must give `all` with exactly that position's bags
//     cleared (`unk_clear_*`), and the map-entry bytes the core delivered must be the
//     reference's unless the zeroed position is the entry position.
// `plant` skips the clear, so the comparison is seen failing where the input has unknowns.
template <class T>
std::string unk_controls(const uint8_t *b, size_t n, Cx &cx,
                         intptr_t (*enc)(ak_enc_ctx *, const T &, const ffi::Tcs &), bool plant) {
  typedef ffi::DecRoot<T> R;
  std::ostringstream js;
  ak_dec_ctx *dc = R::ctx_new(NULL);
  if (dc == NULL) return "{\"crash\":\"no context\"}";
  ffi::unk_entry_bytes();
  T all;
  int32_t rc = unk_dec_rc<T>(dc, R::decode_unk(dc, b, n, &all));
  size_t eb_all = ffi::unk_entry_bytes();
  T drop;
  int32_t drc = unk_dec_rc<T>(dc, R::decode(dc, b, n, &drop));
  size_t eb_drop = ffi::unk_entry_bytes();
  T pool;
  uint64_t refills = 0;
  int32_t prc = unk_dec_rc<T>(dc, R::decode_pool(dc, b, n, &pool, 2, 8, &refills));
  size_t eb_pool = ffi::unk_entry_bytes();
  if (rc != 0) {
    // A refused vector: every mode must refuse it with the same code.
    js << "{\"err\":" << rc << ",\"drop_err\":" << drc << ",\"pool_err\":" << prc << "}";
    ak_dec_ctx_free(dc);
    return js.str();
  }
  std::string ball = unk_bytes<T>(cx, enc, all);
  T expd = all;
  for (int i = 0; i < (int)R::kPositions; ++i) R::clear(expd, i);
  bool drop_equal = drc == 0 && unk_bytes<T>(cx, enc, drop) == unk_bytes<T>(cx, enc, expd) &&
                    eb_drop == 0;
  bool pool_equal = prc == 0 && unk_bytes<T>(cx, enc, pool) == ball && eb_pool == eb_all;
  int changed = 0;
  std::vector<int> mism, emism;
  for (int i = 0; i < (int)R::kPositions; ++i) {
    typename R::Opts o;
    R::opts(&o, i);
    T got;
    int32_t grc = unk_dec_rc<T>(dc, R::decode_opts(dc, b, n, &got, &o));
    size_t eb = ffi::unk_entry_bytes();
    T exp = all;
    if (!plant) R::clear(exp, i);
    std::string bexp = unk_bytes<T>(cx, enc, exp);
    if (bexp != ball) ++changed;
    if (grc != 0 || unk_bytes<T>(cx, enc, got) != bexp) mism.push_back(i);
    if (eb != (R::is_entry(i) ? 0 : eb_all)) emism.push_back(i);
  }
  // Every buffer the binding allocated was delivered or reclaimed inside each decode.
  size_t leaked = ffi::unk_reclaim();
  ak_dec_ctx_free(dc);
  js << "{\"positions\":" << (int)R::kPositions << ",\"changed\":" << changed
     << ",\"entry_bytes\":" << eb_all << ",\"refills\":" << refills
     << ",\"drop_equal\":" << (drop_equal ? "true" : "false")
     << ",\"pool_equal\":" << (pool_equal ? "true" : "false")
     << ",\"leaked\":" << leaked << ",\"mismatched\":[";
  for (size_t i = 0; i < mism.size(); ++i) js << (i ? "," : "") << mism[i];
  js << "],\"entry_mismatch\":[";
  for (size_t i = 0; i < emism.size(); ++i) js << (i ? "," : "") << emism[i];
  js << "]}";
  return js.str();
}

// Generated (gen/generate.py, corpus glue): one case per root of the corpus reader schema.
Outcome run_arm(const std::string &root, Arm arm, const uint8_t *b, size_t n, Cx &cx);
extern const char *const kNotInAbi[][2];
extern const bool kFfiRetainBuilt;
// Decision 11's controls on one vector (`unk_controls`); "" when the root is not in the ABI.
std::string run_unk(const std::string &root, const uint8_t *b, size_t n, Cx &cx, bool plant);

}  // namespace corpus
#endif
