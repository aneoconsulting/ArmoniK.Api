// The arm table, shared by `conformance`, `counts` and `bench`.
//
// R3: three arms minimum, in one process. Here there are five, plus the two linkages:
//   pb        protobuf C++, non-arena              the incumbent
//   pb-arena  protobuf C++ on an Arena             the incumbent's fast configuration
//   native    the generated codec emitted into C++ the no-boundary control
//   ffi       the same codec through the C ABI     the arm under test
//   ffi-*     the decision-1 variants              zeroed fill, no batching, host transcoder
#ifndef AK_HARNESS_H
#define AK_HARNESS_H

#include <google/protobuf/arena.h>
#include <google/protobuf/io/coded_stream.h>
#include <google/protobuf/io/zero_copy_stream_impl_lite.h>

#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

#include "ak/rt.h"
#include "ak/values.h"
#include "ak_abi.h"
#include "generated/binding.h"
#include "generated/binding_borrow.h"
#include "generated/build.h"
#include "generated/cases.h"
#include "generated/core_native.h"
#include "generated/pb_build.h"
#include "generated/types.h"
#include "generated/types_borrow.h"
#include "shapes.pb.h"

namespace ns = armonik::ffi::shapes::v1;

inline std::string sha_of(const std::string &s) { return ak::values::sha256_hex(s); }
inline std::string sha_of(const ak::Enc &e) {
  return ak::values::sha256_hex(std::string((const char *)e.data(), e.size()));
}

// Two serialise paths, and the headline arm is the one a C++ consumer actually writes.
//
// `pb_serialize_default` is `SerializeToString`, protobuf's own documented call. It is the
// incumbent every ratio in this slice is against.
//
// `pb_serialize_det` forces deterministic map ordering, which BYTE IDENTITY (R2) needs on
// any message with a map. It is what `conformance` uses and it is carried as its own timed
// row, because the sort is real work: hiding it inside the incumbent would flatter this
// slice, and charging it to the incumbent's headline would handicap it.
//
// The first build of this file handicapped the incumbent three ways at once and all three
// inflated every encode ratio in this slice's favour:
//   1. `out->clear()` before `out->resize(n)`. `clear()` sets the size to 0, so the resize
//      VALUE-INITIALISES the whole output on every call -- a full zero-fill of up to 4 MB
//      that `SerializeToString` does not do.
//   2. a hand-rolled ArrayOutputStream + CodedOutputStream, so a stream was constructed per
//      call where `SerializeToString` uses the fast path.
//   3. it was used for the headline row, so protobuf paid for deterministic map ordering on
//      every payload.
inline void pb_serialize_default(const google::protobuf::MessageLite &m, std::string *out) {
  m.SerializeToString(out);
}

inline void pb_serialize_det(const google::protobuf::MessageLite &m, std::string *out) {
  // No clear(): resize() on a string that already has the right size does nothing, and on
  // a shorter one it value-initialises only the tail.
  out->resize(m.ByteSizeLong());
  google::protobuf::io::ArrayOutputStream aos(
      out->empty() ? NULL : &(*out)[0], (int)out->size());
  google::protobuf::io::CodedOutputStream cos(&aos);
  cos.SetSerializationDeterministic(true);
  m.SerializeWithCachedSizes(&cos);
}

// Kept for the call sites that want to pick at run time.
inline void pb_serialize(const google::protobuf::MessageLite &m, std::string *out,
                         bool deterministic) {
  if (deterministic) {
    pb_serialize_det(m, out);
  } else {
    pb_serialize_default(m, out);
  }
}

// R2's floor-arm rule: "a ratio far enough from 1 to be surprising gets a floor arm before
// it is reported". `native` encode rows sit at 0.25 and no arm anywhere bounded what
// PRODUCING THESE BYTES costs at all. This is that bound: one memcpy of the finished
// payload into a reused buffer. Nothing can encode faster than copying the answer.
inline void memcpy_floor(const std::string &src, std::string *out) {
  out->resize(src.size());
  if (!src.empty()) std::memcpy(&(*out)[0], src.data(), src.size());
}

struct ArmResult {
  std::string bytes;
  bool ok;
};

#endif  // AK_HARNESS_H
