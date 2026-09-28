// B-8 (2026-09-28): the `ak_bytes` holder of an owned hand-off (ak_enc_take_owned into a
// grpc::Slice that releases it), recycled through one atomic slot instead of a new/delete
// per call. Used by the codec suite's transport rows and by cells D (campaign_rpc.cpp).
#ifndef AK_OWNED_HOLDER_H
#define AK_OWNED_HOLDER_H
#include <atomic>

#include "ak_abi.h"

namespace akhold {
inline std::atomic<ak_bytes *> &slot() {
  static std::atomic<ak_bytes *> s(nullptr);
  return s;
}
inline ak_bytes *get() {
  ak_bytes *b = slot().exchange(nullptr, std::memory_order_acquire);
  if (b == nullptr) b = new ak_bytes;
  b->ptr = nullptr; b->len = 0; b->owner = nullptr;
  return b;
}
inline void put(ak_bytes *b) {
  ak_bytes *e = nullptr;
  if (!slot().compare_exchange_strong(e, b, std::memory_order_release)) delete b;
}
// The grpc::Slice destroy callback: the core's buffer released, the holder recycled.
inline void free_owned(void *p) {
  ak_bytes *b = static_cast<ak_bytes *>(p);
  ak_bytes_free(b);
  put(b);
}
}  // namespace akhold
#endif
