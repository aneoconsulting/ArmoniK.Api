#include <gtest/gtest.h>
#include <sstream>
#include <string>
#include <type_traits>

#include <utils/string_view.h>

using armonik::api::common::string_view;

// Compile-time checks for constexpr correctness (C++11/14 compatible).
static_assert(string_view().empty(), "default-constructed must be empty");
static_assert(string_view().size() == 0, "default-constructed size must be 0");
static_assert(string_view("armonik").size() == 7, "literal size");
static_assert(!string_view("armonik").empty(), "non-empty literal");
static_assert(string_view("armonik")[0] == 'a', "operator[] constexpr");
static_assert(string_view("armonik").substr(1).size() == 6, "substr constexpr size");
static_assert(string_view("armonik", 3).size() == 3, "(ptr,len) constructor size");

// Alias is the exact same type, not just convertible.
static_assert(std::is_same<armonik::api::string_view, armonik::api::common::string_view>::value,
              "armonik::api::string_view must alias armonik::api::common::string_view");

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

TEST(StringView, DefaultConstruct) {
  string_view sv;
  EXPECT_EQ(sv.size(), 0u);
  EXPECT_TRUE(sv.empty());
}

TEST(StringView, FromLiteral) {
  string_view sv("armonik");
  EXPECT_EQ(sv.size(), 7u);
  EXPECT_FALSE(sv.empty());
}

TEST(StringView, FromPtrAndLen) {
  const char *raw = "armonik api";
  string_view sv(raw, 7);
  EXPECT_EQ(sv.size(), 7u);
  EXPECT_EQ(sv.data(), raw);
}

TEST(StringView, FromStdString) {
  std::string s = "greetings";
  string_view sv(s);
  EXPECT_EQ(sv.size(), s.size());
  EXPECT_EQ(sv.data(), s.data());
}

TEST(StringView, CopyConstruct) {
  string_view a("copy");
  string_view b(a);
  EXPECT_EQ(b.size(), a.size());
  EXPECT_EQ(b.data(), a.data());
}

TEST(StringView, CopyAssign) {
  string_view a("assign");
  string_view b;
  b = a;
  EXPECT_EQ(b.size(), a.size());
  EXPECT_EQ(b.data(), a.data());
}

// ---------------------------------------------------------------------------
// Accessors
// ---------------------------------------------------------------------------

TEST(StringView, DataAndSize) {
  const char *raw = "test";
  string_view sv(raw);
  EXPECT_EQ(sv.data(), raw);
  EXPECT_EQ(sv.size(), 4u);
}

TEST(StringView, SubscriptOperator) {
  string_view sv("abcde");
  EXPECT_EQ(sv[0], 'a');
  EXPECT_EQ(sv[4], 'e');
}

TEST(StringView, EmptyLiteral) {
  string_view sv("");
  EXPECT_TRUE(sv.empty());
  EXPECT_EQ(sv.size(), 0u);
}

// ---------------------------------------------------------------------------
// Iterators
// ---------------------------------------------------------------------------

TEST(StringView, IteratorRange) {
  string_view sv("iter");
  std::string built(sv.begin(), sv.end());
  EXPECT_EQ(built, "iter");
}

TEST(StringView, ConstIteratorRange) {
  string_view sv("citer");
  std::string built(sv.cbegin(), sv.cend());
  EXPECT_EQ(built, "citer");
}

TEST(StringView, RangeForLoop) {
  string_view sv("range");
  std::string built;
  for (char c : sv) {
    built += c;
  }
  EXPECT_EQ(built, "range");
}

// ---------------------------------------------------------------------------
// substr
// ---------------------------------------------------------------------------

TEST(StringView, SubstrFull) {
  string_view sv("armonik");
  EXPECT_EQ(sv.substr(0), sv);
}

TEST(StringView, SubstrFromPos) {
  string_view sv("armonik");
  string_view tail = sv.substr(2);
  EXPECT_EQ(tail.size(), 5u);
  EXPECT_EQ(std::string(tail.begin(), tail.end()), "monik");
}

TEST(StringView, SubstrPosAndLen) {
  string_view sv("hello from armonik");
  string_view word = sv.substr(11, 7);
  EXPECT_EQ(std::string(word.begin(), word.end()), "armonik");
}

TEST(StringView, SubstrLenClampedToEnd) {
  string_view sv("armonik");
  // Requesting more than available should give the remainder.
  string_view tail = sv.substr(3, 100);
  EXPECT_EQ(std::string(tail.begin(), tail.end()), "onik");
}

TEST(StringView, SubstrAtEnd) {
  string_view sv("armonik");
  string_view empty_tail = sv.substr(7);
  EXPECT_TRUE(empty_tail.empty());
}

// ---------------------------------------------------------------------------
// find(char)
// ---------------------------------------------------------------------------

TEST(StringView, FindCharFound) {
  string_view sv("armonik");
  EXPECT_EQ(sv.find('r'), 1u);
}

TEST(StringView, FindCharNotFound) {
  string_view sv("armonik");
  EXPECT_TRUE(sv.find('z') == string_view::npos);
}

TEST(StringView, FindCharFromPos) {
  string_view sv("abcabc");
  EXPECT_EQ(sv.find('a', 1), 3u);
}

TEST(StringView, FindCharPosAtEnd) {
  string_view sv("armonik");
  EXPECT_TRUE(sv.find('k', 7) == string_view::npos);
}

// ---------------------------------------------------------------------------
// find(string_view) / find(const char*)
// ---------------------------------------------------------------------------

TEST(StringView, FindSubstringFound) {
  string_view sv("hello from armonik");
  EXPECT_EQ(sv.find(string_view("armonik")), 11u);
}

TEST(StringView, FindSubstringNotFound) {
  string_view sv("hello from armonik");
  EXPECT_TRUE(sv.find(string_view("xyz")) == string_view::npos);
}

TEST(StringView, FindCStringFound) {
  string_view sv("http://example.com");
  EXPECT_EQ(sv.find("://"), 4u);
}

TEST(StringView, FindCStringAtStart) {
  string_view sv("unix:path");
  EXPECT_EQ(sv.find("unix"), 0u);
}

TEST(StringView, FindEmptyNeedle) {
  string_view sv("armonik");
  EXPECT_EQ(sv.find(string_view(""), 2), 2u);
}

TEST(StringView, FindSubstringFromPos) {
  string_view sv("abcabc");
  EXPECT_EQ(sv.find(string_view("abc"), 1), 3u);
}

TEST(StringView, FindNeedleLongerThanHaystack) {
  string_view sv("hi");
  EXPECT_TRUE(sv.find(string_view("armonik")) == string_view::npos);
}

// ---------------------------------------------------------------------------
// Comparison
// ---------------------------------------------------------------------------

TEST(StringView, EqualViews) {
  string_view a("armonik");
  string_view b("armonik");
  EXPECT_TRUE(a == b);
  EXPECT_FALSE(a != b);
}

TEST(StringView, UnequalViews) {
  string_view a("armonik");
  string_view b("world");
  EXPECT_FALSE(a == b);
  EXPECT_TRUE(a != b);
}

TEST(StringView, UnequalLength) {
  string_view a("armonik");
  string_view b("armoni");
  EXPECT_FALSE(a == b);
}

TEST(StringView, EqualCString) {
  string_view sv("armonik");
  EXPECT_TRUE(sv == "armonik");
  EXPECT_FALSE(sv != "armonik");
}

TEST(StringView, UnequalCString) {
  string_view sv("armonik");
  EXPECT_FALSE(sv == "world");
  EXPECT_TRUE(sv != "world");
}

TEST(StringView, CStringOnLeft) {
  string_view sv("armonik");
  EXPECT_TRUE("armonik" == sv);
  EXPECT_FALSE("world" == sv);
  EXPECT_TRUE("world" != sv);
}

TEST(StringView, EmptyEquality) {
  string_view a;
  string_view b("");
  EXPECT_TRUE(a == b);
}

// ---------------------------------------------------------------------------
// Conversion
// ---------------------------------------------------------------------------

TEST(StringView, ExplicitToStdString) {
  string_view sv("convert");
  std::string s = static_cast<std::string>(sv);
  EXPECT_EQ(s, "convert");
}

TEST(StringView, PtrLenToStdString) {
  string_view sv("armonik api", 7);
  std::string s(sv.data(), sv.size());
  EXPECT_EQ(s, "armonik");
}

// ---------------------------------------------------------------------------
// Stream output
// ---------------------------------------------------------------------------

TEST(StringView, StreamOutput) {
  string_view sv("stream");
  std::ostringstream oss;
  oss << sv;
  EXPECT_EQ(oss.str(), "stream");
}

TEST(StringView, StreamOutputEmpty) {
  string_view sv("");
  std::ostringstream oss;
  oss << sv;
  EXPECT_EQ(oss.str(), "");
}

TEST(StringView, StreamOutputSubstr) {
  string_view sv("hello from armonik");
  std::ostringstream oss;
  oss << sv.substr(11, 7);
  EXPECT_EQ(oss.str(), "armonik");
}

// ---------------------------------------------------------------------------
// npos
// ---------------------------------------------------------------------------

TEST(StringView, NposValue) {
  EXPECT_TRUE(string_view::npos == static_cast<string_view::size_type>(-1));
}

// ---------------------------------------------------------------------------
// Integration: typical API usage patterns
// ---------------------------------------------------------------------------

TEST(StringView, PatternSubstrFind) {
  // Mirrors the http:// stripping done in ChannelFactory / ComputePlane.
  string_view endpoint("http://localhost:5001");
  const string_view http_prefix("http://");
  EXPECT_EQ(endpoint.find(http_prefix), 0u);
  string_view without_scheme = endpoint.substr(http_prefix.size());
  EXPECT_EQ(std::string(without_scheme.begin(), without_scheme.end()), "localhost:5001");
}

TEST(StringView, PatternStartsWith) {
  auto starts_with = [](string_view s, string_view prefix) {
    return s.size() >= prefix.size() && s.substr(0, prefix.size()) == prefix;
  };
  EXPECT_TRUE(starts_with("tcp://host", "tcp"));
  EXPECT_FALSE(starts_with("unix://host", "tcp"));
  EXPECT_TRUE(starts_with("", ""));
}

TEST(StringView, ConstructFromStdStringAndCompare) {
  std::string s = "from_std";
  string_view sv(s);
  EXPECT_TRUE(sv == "from_std");
  EXPECT_EQ(sv.size(), s.size());
}

#if defined(__cpp_lib_string_view)
TEST(StringView, StdStringViewInterop) {
  std::string_view std_sv("interop");
  string_view sv(std_sv);
  EXPECT_EQ(sv.size(), std_sv.size());
  EXPECT_TRUE(sv == "interop");

  std::string_view round_trip = static_cast<std::string_view>(sv);
  EXPECT_EQ(round_trip, std_sv);
}
#endif
