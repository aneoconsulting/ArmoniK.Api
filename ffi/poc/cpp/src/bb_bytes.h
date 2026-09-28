// R-2 (2026-09-28): the contiguous bytes of a received grpc::ByteBuffer for the generated
// decoders, without a fresh allocation per call: one slice is used in place (TrySingleSlice);
// several are copied into a buffer the caller keeps and reuses (the slice list reused too).
// Cells D and F (campaign_rpc.cpp) and the codec suite's from=bytebuffer rows use it.
#ifndef AK_BB_BYTES_H
#define AK_BB_BYTES_H
#include <grpcpp/grpcpp.h>

#include <cstring>
#include <vector>

struct BBFlat {
  grpc::Slice one;
  std::vector<grpc::Slice> slices;
  std::vector<uint8_t> flat;
};

inline bool bb_contig(const grpc::ByteBuffer &bb, BBFlat &f, const uint8_t **p, size_t *n) {
  if (bb.TrySingleSlice(&f.one).ok()) {
    *p = f.one.begin();
    *n = f.one.size();
    return true;
  }
  f.slices.clear();
  if (!bb.Dump(&f.slices).ok()) return false;
  size_t tot = 0;
  for (size_t i = 0; i < f.slices.size(); ++i) tot += f.slices[i].size();
  if (f.flat.size() < tot) f.flat.resize(tot);
  size_t o = 0;
  for (size_t i = 0; i < f.slices.size(); ++i) {
    if (f.slices[i].size()) std::memcpy(&f.flat[o], f.slices[i].begin(), f.slices[i].size());
    o += f.slices[i].size();
  }
  f.slices.clear();
  *p = f.flat.empty() ? NULL : &f.flat[0];
  *n = tot;
  return true;
}
#endif
