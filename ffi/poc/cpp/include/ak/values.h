// The deterministic field values of `ffi/schema/emit/values.py`, hand-re-derived in C++.
//
// No RNG anywhere: a value is a pure function of (field path, element index), so five
// slices in five languages produce identical bytes by implementing the same function rather
// than by sharing a data file. The manifest pins the outcome, and this file is wrong if
// `ffi/schema/generated/manifest.json` disagrees -- which is the check `conformance` makes
// before anything is timed.
#ifndef AK_VALUES_H
#define AK_VALUES_H

#include <cstdint>
#include <string>
#include <vector>

namespace ak {
namespace values {

void sha256(const uint8_t *data, std::size_t n, uint8_t out[32]);
std::string sha256_hex(const std::string &s);

uint64_t h64(const std::string &path, int idx);
std::string guid(const std::string &path, int idx);
std::string word(const std::string &path, int idx);
std::string sentence(const std::string &path, int idx);
std::string blob(const std::string &path, int idx, std::size_t n = 16);
const std::string &bulk(std::size_t n);

int32_t scalar_i32(const std::string &path, int idx);
int64_t scalar_i64(const std::string &path, int idx);
bool scalar_bool(const std::string &path, int idx);
double scalar_double(const std::string &path, int idx);

int32_t enum_value(const std::vector<int32_t> &vals, int idx);

struct Stamp { int64_t seconds; int32_t nanos; };
Stamp timestamp(int idx);
Stamp duration(int idx);

// The three content sets of design/SHAPES.md. `ascii` is the default and is what the
// manifest pins; the other two exist only for the arms that touch the string path.
enum ContentSet { kAscii = 0, kLatin1 = 1, kWide = 2 };
std::string recode(const std::string &s, ContentSet cs);

// Which set the STRING value rules return. `guid`, `word` and `sentence` honour it;
// `blob` and `bulk` do not, because a content set says what is in the *strings* and a
// `bytes` field has no encoding to be in. That distinction is C20: `ResultRaw.opaque_id`
// is `bytes`, and treating it as a string is what put a field the policy does not apply to
// into a table that priced the policy.
//
// It is process state, which is the one thing this slice spent a work unit arguing against
// -- so: it is set only by `src/contentsets.cpp`, only around a build, and only on the main
// thread before any measurement starts. `Scoped` makes "only around a build" structural
// rather than a convention. The generated builders take no parameter for it because both
// construction routes reach their values through these three functions, which is exactly
// what makes the facade arm and the protobuf arm two independent routes to one value.
ContentSet content_set();
void set_content_set(ContentSet cs);

struct ScopedContentSet {
  ContentSet prev;
  explicit ScopedContentSet(ContentSet cs) : prev(content_set()) { set_content_set(cs); }
  ~ScopedContentSet() { set_content_set(prev); }
};

}  // namespace values
}  // namespace ak
#endif  // AK_VALUES_H
