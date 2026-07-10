#pragma once
/**
 * @file string_view.h
 * @brief A C++11-compatible non-owning string view type.
 */

#include <cstddef>
#include <ostream>
#include <string>
#include <vector>

namespace armonik {
namespace api {
namespace common {

/**
 * @brief Non-owning reference to a string.
 *
 * ABI-stable across C++ standards: always the same concrete type regardless of
 * whether the translation unit is compiled as C++11, 14, or 17. Optional
 * conversions to/from std::string_view are guarded by the feature macro so they
 * are compiled only on C++17 targets.
 */
class string_view {
public:
  using const_iterator = const char *;
  using size_type = std::size_t;

  static constexpr size_type npos = std::string::npos;

private:
  const char *data_;
  size_type size_;

  // Used by the runtime const char* constructor below, and by the const char* overloads of
  // find()/operator==(), for pointers whose length isn't known at compile time (e.g. returned
  // by a C API). String literals instead bind to the array-reference overloads/constructor,
  // which get their length from the array bound and never call this.
  //
  // C++11 constexpr functions are restricted to a single return statement, so a
  // loop-based strlen is not valid: https://en.cppreference.com/w/cpp/language/constexpr (see "C++11" notes section)
  static constexpr size_type clen(const char *s) noexcept { return *s ? 1 + clen(s + 1) : 0; }

public:
  constexpr string_view() noexcept : data_(nullptr), size_(0) {}

  constexpr string_view(const char *s) noexcept : data_(s), size_(clen(s)) {}

  template <std::size_t N>
  constexpr string_view(const char (&s)[N]) noexcept : data_(s), size_(N - 1) {}

  constexpr string_view(const char *s, size_type len) noexcept : data_(s), size_(len) {}

  string_view(const std::string &s) noexcept : data_(s.data()), size_(s.size()) {}

  constexpr string_view(const string_view &) noexcept = default;
  string_view &operator=(const string_view &) noexcept = default;

  // --- Accessors ---
  constexpr const char *data() const noexcept { return data_; }
  constexpr size_type size() const noexcept { return size_; }
  constexpr bool empty() const noexcept { return size_ == 0; }
  constexpr const char &operator[](size_type i) const noexcept { return data_[i]; }

  // --- Iterators ---
  constexpr const_iterator begin() const noexcept { return data_; }
  constexpr const_iterator end() const noexcept { return data_ + size_; }
  constexpr const_iterator cbegin() const noexcept { return data_; }
  constexpr const_iterator cend() const noexcept { return data_ + size_; }

  // --- Substr ---
  // pos beyond size_ is clamped (rather than throwing, unlike std::string_view::substr) since
  // this function is noexcept. Single return, and pos re-evaluated rather than bound to a local,
  // for C++11 constexpr compatibility.
  constexpr string_view substr(size_type pos, size_type len = npos) const noexcept {
    return string_view(data_ + (pos < size_ ? pos : size_),
                        (len == npos || (pos < size_ ? pos : size_) + len > size_) ? size_ - (pos < size_ ? pos : size_)
                                                                                    : len);
  }

  // --- Find ---
  size_type find(char c, size_type pos = 0) const noexcept {
    for (size_type i = pos; i < size_; ++i) {
      if (data_[i] == c) {
        return i;
      }
    }
    return npos;
  }

  size_type find(const string_view &needle, size_type pos = 0) const noexcept {
    if (pos > size_) {
      return npos;
    }
    if (needle.size_ == 0) {
      return pos;
    }
    if (needle.size_ > size_ - pos) {
      return npos;
    }
    for (size_type i = pos; i + needle.size_ <= size_; ++i) {
      bool match = true;
      for (size_type j = 0; j < needle.size_; ++j) {
        if (data_[i + j] != needle.data_[j]) {
          match = false;
          break;
        }
      }
      if (match) {
        return i;
      }
    }
    return npos;
  }

  size_type find(const char *s, size_type pos = 0) const noexcept { return find(string_view(s), pos); }

  template <std::size_t N>
  size_type find(const char (&s)[N], size_type pos = 0) const noexcept {
    return find(string_view(s, N - 1), pos);
  }

  bool contains(char c) const noexcept { return find(c) != npos; }

  // Mirrors str_split: every occurrence of delim splits the view, and the trailing piece
  // (possibly empty) is always included, even when delim is the last character.
  std::vector<string_view> split(char delim) const {
    std::vector<string_view> result;
    size_type start = 0, pos;
    while ((pos = find(delim, start)) != npos) {
      result.emplace_back(data_ + start, pos - start);
      start = pos + 1;
    }
    result.emplace_back(data_ + start, size_ - start);
    return result;
  }

  // --- Comparisons ---
  bool operator==(const string_view &other) const noexcept {
    if (size_ != other.size_) {
      return false;
    }
    for (size_type i = 0; i < size_; ++i) {
      if (data_[i] != other.data_[i]) {
        return false;
      }
    }
    return true;
  }

  bool operator!=(const string_view &other) const noexcept { return !(*this == other); }
  bool operator==(const char *s) const noexcept { return *this == string_view(s); }
  bool operator!=(const char *s) const noexcept { return !(*this == s); }

  template <std::size_t N>
  bool operator==(const char (&s)[N]) const noexcept {
    return *this == string_view(s, N - 1);
  }

  template <std::size_t N>
  bool operator!=(const char (&s)[N]) const noexcept {
    return !(*this == s);
  }

  friend bool operator==(const char *lhs, const string_view &rhs) noexcept { return rhs == lhs; }
  friend bool operator!=(const char *lhs, const string_view &rhs) noexcept { return rhs != lhs; }

  template <std::size_t N>
  friend bool operator==(const char (&lhs)[N], const string_view &rhs) noexcept {
    return rhs == lhs;
  }

  template <std::size_t N>
  friend bool operator!=(const char (&lhs)[N], const string_view &rhs) noexcept {
    return rhs != lhs;
  }

  friend std::ostream &operator<<(std::ostream &os, const string_view &sv) {
    if (!sv.empty()) {
      os.write(sv.data(), static_cast<std::streamsize>(sv.size()));
    }
    return os;
  }

  // --- Conversion ---
  explicit operator std::string() const { return std::string(data_, size_); }

  // SD-6 feature-test macro: defined (to a year-based integer) when <string_view>
  // is provided by the implementation, i.e. C++17 and later. See e.g.,
  // https://en.cppreference.com/w/cpp/feature_test
#if defined(__cpp_lib_string_view)
  string_view(std::string_view sv) noexcept : data_(sv.data()), size_(sv.size()) {}
  operator std::string_view() const noexcept { return std::string_view(data_, size_); }
#endif
};

} // namespace common

/// @cond DOXYGEN_IGNORE
using string_view = common::string_view;
/// @endcond

} // namespace api
} // namespace armonik
