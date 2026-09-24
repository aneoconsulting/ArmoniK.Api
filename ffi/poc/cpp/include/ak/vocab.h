// The post-C++11 vocabulary types, reimplemented, per README 5.1.1.
//
// The rule is about AVAILABILITY, not taste. `std::string`, `std::vector`, `std::map` and
// the rest of the C++11 library are used directly everywhere in this slice: they exist at
// every level the facade supports, so a header naming them means the same thing at each.
// A type that arrives AFTER C++11 is reimplemented here, because its presence depends on
// `-std`, which the consumer picks and we do not (README 5.1, the hard stop).
//
// The shape is the house pattern of
// `packages/cpp/ArmoniK.Api.Common/header/utils/string_view.h`: one concrete type at every
// level, with the conversions to and from the standard counterpart guarded by the feature
// macro. `gen/odr_check.sh` asserts the layout property mechanically.
#ifndef AK_VOCAB_H
#define AK_VOCAB_H

#include <cstddef>
#include <cstring>
#include <string>
#include <utility>

#if defined(__cpp_lib_string_view) && __cpp_lib_string_view >= 201606L
#define AK_HAS_STD_STRING_VIEW 1
#include <string_view>
#endif

namespace ak {

// `optional` (C++17). All data members private, no bases, no virtuals: standard layout, so
// `offsetof` on it is well defined and the ODR check can pin it.
template <class T>
class Optional {
 public:
  Optional() : has_(false), v_() {}
  Optional(const T &v) : has_(true), v_(v) {}           // NOLINT(runtime/explicit)
  bool has_value() const { return has_; }
  explicit operator bool() const { return has_; }
  const T &value() const { return v_; }
  T &value() { return v_; }
  const T &operator*() const { return v_; }
  T &operator*() { return v_; }
  const T *operator->() const { return &v_; }
  T *operator->() { return &v_; }
  // The in-place accessor a decode path needs: a message that carries a repeated or a map
  // field is filled IN PLACE, never constructed, because `apply` arrives after the runs
  // that populated it (ABI v1 section 7.1).
  T &get_or_insert() {
    if (!has_) { v_ = T(); has_ = true; }
    return v_;
  }
  void set(const T &v) { v_ = v; has_ = true; }
  // The rvalue overload is not a nicety: without it a decoded child is built by value and
  // then COPIED into the option, which on M5's 4 MB `bytes` field is a second copy of the
  // whole payload. Measured before it existed: P5.4 decode through the ABI was 3.7 ms
  // slower than the no-boundary control, all of it this copy.
  void set(T &&v) { v_ = static_cast<T &&>(v); has_ = true; }
  T &emplace() { v_ = T(); has_ = true; return v_; }
  void reset() { v_ = T(); has_ = false; }
  bool operator==(const Optional &o) const {
    return has_ == o.has_ && (!has_ || v_ == o.v_);
  }
  bool operator!=(const Optional &o) const { return !(*this == o); }

 private:
  bool has_;
  T v_;
#ifdef AK_ODR_BREAK
  // The POSITIVE CONTROL, and it is here rather than in a comment because a guard with no
  // failing test is a guard nobody has seen work. Build `odrcheck` with -DAK_ODR_BREAK=ON
  // and this member appears only at C++17, which is exactly the ODR violation README 5.1
  // calls a hard stop. The check must report it.
#if __cplusplus >= 201703L
  std::size_t break_;
#endif
#endif
};

// A singular message field that can contain its own owner (the corpus's `Nest`), which
// `Optional<T>` cannot hold because it needs a complete `T`. Owning, deep-copying, one
// pointer at every standard level. Not in any shapes.json facade (no message there is
// recursive), so it changes no existing type's layout. FIX-PLAN WP5 step 2.
template <class T>
class Box {
 public:
  Box() : p_(NULL) {}
  Box(const Box &o) : p_(o.p_ ? new T(*o.p_) : NULL) {}
  Box(Box &&o) noexcept : p_(o.p_) { o.p_ = NULL; }
  Box &operator=(const Box &o) {
    if (this != &o) { T *n = o.p_ ? new T(*o.p_) : NULL; delete p_; p_ = n; }
    return *this;
  }
  Box &operator=(Box &&o) noexcept {
    if (this != &o) { delete p_; p_ = o.p_; o.p_ = NULL; }
    return *this;
  }
  ~Box() { delete p_; }
  bool has_value() const { return p_ != NULL; }
  explicit operator bool() const { return p_ != NULL; }
  const T &operator*() const { return *p_; }
  T &operator*() { return *p_; }
  const T *operator->() const { return p_; }
  T *operator->() { return p_; }
  T &get_or_insert() {
    if (!p_) p_ = new T();
    return *p_;
  }
  T &emplace() { delete p_; p_ = new T(); return *p_; }
  void reset() { delete p_; p_ = NULL; }
  bool operator==(const Box &o) const {
    return (p_ == NULL) == (o.p_ == NULL) && (!p_ || *p_ == *o.p_);
  }
  bool operator!=(const Box &o) const { return !(*this == o); }

 private:
  T *p_;
};

// `string_view` (C++17). Only the part this slice uses.
class StringView {
 public:
  StringView() : data_(NULL), size_(0) {}
  StringView(const char *p, std::size_t n) : data_(p), size_(n) {}
  StringView(const std::string &s) : data_(s.data()), size_(s.size()) {}  // NOLINT
  const char *data() const { return data_; }
  std::size_t size() const { return size_; }
  bool empty() const { return size_ == 0; }
  std::string to_string() const { return std::string(data_, size_); }
  bool operator==(const StringView &o) const {
    return size_ == o.size_ && (size_ == 0 || std::memcmp(data_, o.data_, size_) == 0);
  }
  bool operator!=(const StringView &o) const { return !(*this == o); }
  // Ordering, so a borrowed view can be a `std::map` key. The borrowed-facade arm of
  // `bench` needs it and nothing else does; it is `std::string`'s ordering, byte-wise.
  bool operator<(const StringView &o) const {
    std::size_t n = size_ < o.size_ ? size_ : o.size_;
    int c = n ? std::memcmp(data_, o.data_, n) : 0;
    return c != 0 ? c < 0 : size_ < o.size_;
  }
  bool empty_or_null() const { return data_ == NULL || size_ == 0; }
  void clear() { data_ = NULL; size_ = 0; }
#ifdef AK_HAS_STD_STRING_VIEW
  // Guarded, exactly as the house pattern guards its own. The conversions are additive and
  // they do not change this class's layout, which is what keeps one ABI across levels.
  StringView(std::string_view s) : data_(s.data()), size_(s.size()) {}  // NOLINT
  operator std::string_view() const { return std::string_view(data_, size_); }
#endif

 private:
  const char *data_;
  std::size_t size_;
};

}  // namespace ak
#endif  // AK_VOCAB_H
