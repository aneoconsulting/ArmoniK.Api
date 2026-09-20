// `ak::Dec::skip` over the deprecated GROUP form (wire types 3 and 4).
//
// WHY THIS FILE EXISTS, and it is the finding rather than the test.
//
// Byte identity against `ffi/schema/generated/manifest.json` cannot reach this code at
// all. The manifest is generated from a proto3 description, proto3 cannot express a
// group, so nothing the generator emits ever puts wire type 3 on the wire -- and this
// slice's `skip` sent wire type 3 to the `default:` arm and returned `ERR_MALFORMED`.
// That is a conformant message REFUSED: protobuf C++ and upb both accept an unknown
// field of the group form, because a parser has to skip what it does not know whatever
// shape it is. 443 conformance checks passed over the defect.
//
// So the test comes first and it is required to FAIL on a planted defect before it is
// trusted (README R10). Two plants, both in `include/ak/rt.h`:
//
//   AK_GROUP_PLANT=1  the skipper that counts nesting DEPTH instead of matching the
//                     field number. It is the fix everyone writes first, it accepts
//                     corpus vector `X-group-mismatched-end`, and from that point on it
//                     mis-nests every group in the message. Must fail T4 and T5.
//   AK_GROUP_PLANT=2  the regression that adding `case 3:` invites: the 32-bit `case 5:`
//                     arm silently dropped. Every payload carrying an unknown `fixed32`
//                     would hit it, and no group test would notice. Must fail T8.
//
// `gen/groupskip.sh` builds the shipped configuration at C++11, C++14 and C++17 in both
// the floor and the target implementation, requires all of them to pass, then builds the
// two plants and requires each to fail.
//
// The eight cases mirror `ffi/poc/codec/crates/ak-rt/src/dec.rs`'s `group_skip_tests`
// one for one, because the C++ runtime and the shared core have to agree on this or the
// `native` arm and the `ffi` arm decode the same bytes differently -- which R2's byte
// identity would catch only if a vector existed, and none did.
#include <cstdio>
#include <string>
#include <vector>

#include "ak/rt.h"

static int g_fail = 0;
static int g_checks = 0;

static const char *errname(int32_t e) {
  switch (e) {
    case 0: return "ok";
    case ak::ERR_MALFORMED: return "ERR_MALFORMED";
    case ak::ERR_TRUNCATED: return "ERR_TRUNCATED";
    case ak::ERR_DEPTH: return "ERR_DEPTH";
    default: return "?";
  }
}

// A field header, field number under 16 so it is one byte.
static uint8_t tag(uint32_t field, uint32_t wire) {
  return (uint8_t)((field << 3) | wire);
}

// What a generated decoder does with a field it does not know: read the header, then hand
// BOTH halves to `skip`. Returns (err, pos), so a test can fail on a skip that stopped in
// the wrong place and not only on one that errored.
struct Res {
  int32_t err;
  std::size_t pos;
};

static Res skip_all(const std::vector<uint8_t> &buf) {
  ak::Dec d(buf.empty() ? NULL : &buf[0], buf.size());
  while (!d.at_end()) {
    uint64_t k = d.varint();
    if (d.err != 0) break;
    uint32_t t = (uint32_t)(k >> 3), w = (uint32_t)(k & 7);
    if (t == 0) { d.err = ak::ERR_MALFORMED; break; }
    d.skip(t, w);
  }
  Res r;
  r.err = d.err;
  r.pos = d.pos;
  return r;
}

static void expect(const char *id, const std::vector<uint8_t> &buf, int32_t want_err,
                   bool want_whole) {
  ++g_checks;
  Res r = skip_all(buf);
  bool ok = (r.err == want_err) && (!want_whole || r.pos == buf.size());
  if (!ok) {
    ++g_fail;
    std::printf("  FAIL  %-44s got %s at %zu of %zu, wanted %s%s\n", id, errname(r.err),
                r.pos, buf.size(), errname(want_err),
                want_whole ? " consuming the whole buffer" : "");
  } else {
    std::printf("  ok    %-44s %s at %zu of %zu\n", id, errname(r.err), r.pos, buf.size());
  }
}

int main() {
  std::printf("# cpp slice: ak::Dec::skip over the GROUP form (wire types 3 and 4)\n");
  std::printf("#   -std           %ld\n", (long)__cplusplus);
  std::printf("#   implementation %s\n",
#if AK_FLOOR_IMPL
              "floor"
#else
              "target"
#endif
  );
  std::printf("#   plant          %d (0 = the shipped code)\n", (int)AK_GROUP_PLANT);
  std::printf("#   depth limit    %u\n", (unsigned)ak::MAX_GROUP_DEPTH);
  std::printf("#   oracle         ffi/poc/codec/crates/ak-rt/src/dec.rs group_skip_tests,\n");
  std::printf("#                  case for case, and the corpus vectors it names\n\n");

  // T1. A group is skipped WHOLE. The trailing known-shaped varint is what makes this a
  // test of where the skip stopped and not only of whether it errored.
  {
    std::vector<uint8_t> b;
    b.push_back(tag(15, 3));
    b.push_back(tag(1, 0)); b.push_back(0x01);
    b.push_back(tag(15, 4));
    b.push_back(tag(2, 0)); b.push_back(0x7f);
    expect("T1 a group is skipped whole", b, 0, true);
  }

  // T2. Groups nest.
  {
    std::vector<uint8_t> b;
    b.push_back(tag(15, 3));
    b.push_back(tag(14, 3));
    b.push_back(tag(1, 0)); b.push_back(0x01);
    b.push_back(tag(14, 4));
    b.push_back(tag(15, 4));
    expect("T2 groups nest", b, 0, true);
  }

  // T3. corpus X-group-unterminated: a start tag with no end is TRUNCATED, never accepted.
  {
    std::vector<uint8_t> b;
    b.push_back(tag(15, 3));
    b.push_back(tag(1, 0)); b.push_back(0x01);
    expect("T3 an unterminated group is truncated", b, ak::ERR_TRUNCATED, false);
  }

  // T4. corpus X-group-mismatched-end. THE case a depth counter gets wrong: it sees one
  // start and one end, calls it balanced, and every group after this point is mis-nested.
  {
    std::vector<uint8_t> b;
    b.push_back(tag(15, 3));
    b.push_back(tag(1, 0)); b.push_back(0x01);
    b.push_back(tag(14, 4));
    expect("T4 a mismatched end tag is malformed", b, ak::ERR_MALFORMED, false);
  }

  // T5. The same one level in, because a skipper can match at the root and count inside.
  {
    std::vector<uint8_t> b;
    b.push_back(tag(15, 3));
    b.push_back(tag(14, 3));
    b.push_back(tag(13, 4));
    b.push_back(tag(15, 4));
    expect("T5 a mismatched end inside a nest is malformed", b, ak::ERR_MALFORMED, false);
  }

  // T6. An END_GROUP with nothing open stays malformed, which is wire type 4's whole
  // meaning at the top level.
  {
    std::vector<uint8_t> b;
    b.push_back(tag(15, 4));
    expect("T6 an end tag with nothing open is malformed", b, ak::ERR_MALFORMED, false);
  }

  // T7. Bounded recursion. Without the limit this is a stack overflow inside the HOST's
  // process, which is ABI v1 open decision 7 and not something a native core may do.
  {
    std::vector<uint8_t> b;
    for (int i = 0; i < 200; ++i) b.push_back(tag(15, 3));
    expect("T7 nesting past the limit is DEPTH, not a crash", b, ak::ERR_DEPTH, false);
  }

  // T8. THE REGRESSION GUARD. Adding an arm to a switch is where an arm gets lost: the
  // first draft of the shared core's fix dropped `case 5:` and every fixed32 would have
  // been refused. One buffer with all four of the other wire types in it.
  {
    std::vector<uint8_t> b;
    b.push_back(tag(1, 0)); b.push_back(0x96); b.push_back(0x01);   // varint 150
    b.push_back(tag(2, 1));
    for (int i = 0; i < 8; ++i) b.push_back(0);                      // 64-bit
    b.push_back(tag(3, 2)); b.push_back(0x02); b.push_back(0xaa); b.push_back(0xbb);
    b.push_back(tag(4, 5));
    for (int i = 0; i < 4; ++i) b.push_back(0);                      // 32-bit
    expect("T8 the other four wire types still skip", b, 0, true);
  }

  // T9. Wire types 6 and 7 do not exist and are malformed wherever they appear. Not in
  // the core's set; here because this `switch` has a `default:` that a careless arm could
  // swallow.
  {
    std::vector<uint8_t> b;
    b.push_back(tag(1, 6));
    expect("T9 wire type 6 is malformed", b, ak::ERR_MALFORMED, false);
  }
  {
    std::vector<uint8_t> b;
    b.push_back(tag(1, 7));
    expect("T9 wire type 7 is malformed", b, ak::ERR_MALFORMED, false);
  }

  // T10. A group is skipped whole even when it carries every other wire type inside it,
  // including a nested length-delimited body that itself contains group-looking bytes.
  // The inner bytes are inside a length prefix, so they must NOT be walked.
  {
    std::vector<uint8_t> b;
    b.push_back(tag(9, 3));
    b.push_back(tag(1, 0)); b.push_back(0xff); b.push_back(0x01);
    b.push_back(tag(2, 5)); for (int i = 0; i < 4; ++i) b.push_back(0x11);
    b.push_back(tag(3, 1)); for (int i = 0; i < 8; ++i) b.push_back(0x22);
    // a length-delimited field whose BODY is a start tag with no end
    b.push_back(tag(4, 2)); b.push_back(0x01); b.push_back(tag(15, 3));
    b.push_back(tag(9, 4));
    b.push_back(tag(5, 0)); b.push_back(0x01);
    expect("T10 a group's length-delimited body is not walked", b, 0, true);
  }

  std::printf("\n%d checks, %d failures\n", g_checks, g_fail);
  return g_fail ? 1 : 0;
}
