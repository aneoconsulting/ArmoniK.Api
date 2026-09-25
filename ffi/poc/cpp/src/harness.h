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
// WP5 step 10: the no-unknown build compiles against nounk/include's ak_abi.h, which
// defines AK_NO_UNKNOWN_FIELDS, and the binding rendered from the drop plan.
#ifdef AK_NO_UNKNOWN_FIELDS
#include "generated/binding_nounk.h"
#include "generated/binding_borrow_nounk.h"
#else
#include "generated/binding.h"
#include "generated/binding_borrow.h"
#endif
#include "generated/build.h"
#include "generated/cases.h"
#include "generated/core_native.h"
#include "generated/pb_build.h"
#include "generated/types.h"
#include "generated/types_borrow.h"
#include "shapes.pb.h"

namespace ns = armonik::ffi::shapes::v1;

// The strings the decode-side UTF-8 policy actually validates, recoded into a content set.
//
// FIVE fields, not six. `ResultRaw.opaque_id` is a `bytes` field: proto3 puts no UTF-8
// requirement on it, the generated codec reaches it through `ak_tc_bytes` and
// `decode_str_raw`, and no validator ever sees it. Including it was a defect in the
// string-path table (C20) -- 1,000 of its 6,000 strings were a field the policy does not
// apply to, and in the ASCII set those 1,000 are arbitrary bytes that the check arm
// REJECTS on the first bad byte, so it did less work than a validation and the ASCII row
// understated the cost. `src/utf8check.cpp` found it by asserting that every string it
// validates is valid, which the table never did.
//
// One definition, used by `bench.cpp` and `utf8check.cpp`, so the two cannot drift.
inline void p1_2_strings(ak::values::ContentSet cs, std::vector<std::string> *out) {
  static shapes::ListResultsResponse m = shapes::build::payload_p1_2();
  out->clear();
  out->reserve(m.results.size() * 5);
  for (std::size_t i = 0; i < m.results.size(); ++i) {
    out->push_back(ak::values::recode(m.results[i].session_id, cs));
    out->push_back(ak::values::recode(m.results[i].name, cs));
    out->push_back(ak::values::recode(m.results[i].owner_task_id, cs));
    out->push_back(ak::values::recode(m.results[i].result_id, cs));
    out->push_back(ak::values::recode(m.results[i].created_by, cs));
  }
}

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
