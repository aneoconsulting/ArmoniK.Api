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
#include <string>
#include <vector>

#include "ak/rt.h"
#include "ak/values.h"
#include "ak_abi.h"
#include "generated/binding.h"
#include "generated/build.h"
#include "generated/cases.h"
#include "generated/core_native.h"
#include "generated/pb_build.h"
#include "generated/types.h"
#include "shapes.pb.h"

namespace ns = armonik::ffi::shapes::v1;

inline std::string sha_of(const std::string &s) { return ak::values::sha256_hex(s); }
inline std::string sha_of(const ak::Enc &e) {
  return ak::values::sha256_hex(std::string((const char *)e.data(), e.size()));
}

// protobuf C++ serialises map entries in an unspecified order unless deterministic
// serialisation is asked for, so BYTE IDENTITY (R2) requires it on. It is on in every
// conformance run and in the headline timing arm; `bench` also carries the non-
// deterministic form on M2 as a separate row, because the sort is a real cost and hiding
// it inside the incumbent would flatter this slice.
inline void pb_serialize(const google::protobuf::MessageLite &m, std::string *out,
                         bool deterministic) {
  out->clear();
  out->resize(m.ByteSizeLong());
  google::protobuf::io::ArrayOutputStream aos(
      out->empty() ? NULL : &(*out)[0], (int)out->size());
  google::protobuf::io::CodedOutputStream cos(&aos);
  cos.SetSerializationDeterministic(deterministic);
  m.SerializeWithCachedSizes(&cos);
}

struct ArmResult {
  std::string bytes;
  bool ok;
};

#endif  // AK_HARNESS_H
