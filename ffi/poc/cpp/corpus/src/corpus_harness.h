// The cpp slice's full-corpus harness (FIX-PLAN WP5 item 6.1), the C++ half: generic arm
// runners over the GENERATED facade, native codecs, binding and projection of the corpus
// reader schema. Glue: no wire rule and no layout lives here -- it calls generated code
// and prints what came back. The verdict is taken by `gen/corpus_all.py`.
#ifndef AK_CORPUS_HARNESS_H
#define AK_CORPUS_HARNESS_H

#include <cstdint>
#include <cstring>
#include <string>

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

struct Cx {
  ak_enc_ctx *enc;
  ak_dec_ctx *dec;
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
  int32_t rc = dec(cx.dec, b, n, &v);
  if (rc == 0) rc = ak_dec_err(cx.dec);
  ak_dec_err_reset(cx.dec);
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

// Generated (gen/generate.py, corpus glue): one case per root of the corpus reader schema.
Outcome run_arm(const std::string &root, Arm arm, const uint8_t *b, size_t n, Cx &cx);
extern const char *const kNotInAbi[][2];
extern const bool kFfiRetainBuilt;

}  // namespace corpus
#endif
