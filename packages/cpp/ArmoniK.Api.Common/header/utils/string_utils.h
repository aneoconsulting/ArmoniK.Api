#pragma once

#include <algorithm>
#include <cctype>
#include <locale>
#include <string>

namespace armonik {
namespace api {
namespace common {
namespace utils {
// trim from start (in place)
static inline void ltrim(std::string &s) {
  s.erase(s.begin(), std::find_if(s.begin(), s.end(), [](unsigned char ch) { return !std::isspace(ch); }));
}

// trim from end (in place)
static inline void rtrim(std::string &s) {
  s.erase(std::find_if(s.rbegin(), s.rend(), [](unsigned char ch) { return !std::isspace(ch); }).base(), s.end());
}

// trim from both ends (in place)
static inline void trim(std::string &s) {
  rtrim(s);
  ltrim(s);
}

// trim from start (copying)
static inline std::string ltrim_copy(std::string s) {
  ltrim(s);
  return s;
}

// trim from end (copying)
static inline std::string rtrim_copy(std::string s) {
  rtrim(s);
  return s;
}

// trim from both ends (copying)
static inline std::string trim_copy(std::string s) {
  trim(s);
  return s;
}

inline std::string pathJoin(const std::string &p1, const std::string &p2) {
#ifdef _WIN32
  constexpr char sep = '\\';
#else
  constexpr char sep = '/';
#endif
  std::string tmp = trim_copy(p1);

  if (tmp[tmp.length() - 1] != sep) {
    tmp += sep;
  }
  return tmp + trim_copy(p2);
}
} // namespace utils
} // namespace common
} // namespace api
} // namespace armonik
