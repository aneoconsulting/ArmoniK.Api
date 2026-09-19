// The upb arm: a CEILING, not a candidate.
//
// design/SHAPES.md's comparison is against what ArmoniK ships, which is protobuf C++.
// upb is the fastest C protobuf available and bounds how fast a C protobuf can be, which
// bounds the core's wins from the direction R2's memcpy floor does not reach. Both bounds
// in one table is what R2's "a ratio far enough from 1 to be surprising gets a floor arm"
// asks for, from both ends at once.
//
// **Not apt's upb.** `libupb-dev` is 0.0.0~git200730, a July 2020 snapshot from before upb
// was merged into the protobuf repository, with a different API and different codegen;
// measuring it and calling it the fastest comparator would be a statement about a library
// nobody ships. This builds upb from the protobuf repository at the pinned tag.
//
// **No Bazel, and no generated accessors.** `protoc-gen-upb` is Bazel-only, and a
// hand-written minitable would be a hand-written codec, which R1 forbids. Instead the
// minitables come from upb's own REFLECTION: a `upb_DefPool` loaded from the descriptor
// set `protoc` emits. That costs nothing in the measurement, because `upb_Encode` and
// `upb_Decode` work off the minitable and do not care where it came from -- the codec
// being timed is exactly upb's, and only the (untimed) setup differs from a generated
// build.
//
// Everything is in ONE process: protobuf C++, protobuf C++ on an arena, upb, the
// no-boundary control and the C ABI arm, so every ratio is formed inside one process (R4).
#include <upb/mem/arena.h>
#include <upb/message/message.h>
#include <upb/reflection/def.h>
#include <upb/wire/decode.h>
#include <upb/wire/encode.h>

#include "google/protobuf/descriptor.upb.h"  // upb's bootstrap accessors (stage0)

#include <algorithm>
#include <cstdio>
#include <cstring>
#include <fstream>
#include <sstream>
#include <string>
#include <vector>

#include "harness.h"

#if defined(__GNUC__)
#define AK_SINK(x) asm volatile("" : : "r"(x))
#define AK_SINK_MEM(x) asm volatile("" : : "r,m"(x) : "memory")
#else
#define AK_SINK(x) (void)(x)
#define AK_SINK_MEM(x) (void)(x)
#endif

static double now_ns() {
  struct timespec ts;
  clock_gettime(CLOCK_MONOTONIC, &ts);
  return (double)ts.tv_sec * 1e9 + (double)ts.tv_nsec;
}

template <class Fn>
static double timed(Fn f, int iters) {
  int per = iters / 5;
  if (per < 1) per = 1;
  double best = 1e300;
  for (int b = 0; b < 5; ++b) {
    double t0 = now_ns();
    for (int i = 0; i < per; ++i) f();
    double d = (now_ns() - t0) / per;
    if (d < best) best = d;
  }
  return best;
}

template <class Fn>
static int calibrate(Fn f) {
  double one = timed(f, 1);
  if (one <= 0) one = 1;
  int n = (int)(40e6 / one);
  if (n < 5) n = 5;
  if (n > 2000000) n = 2000000;
  return n;
}

static int g_rounds = 9;
static int g_fail = 0;

static std::string read_file(const std::string &p) {
  std::ifstream f(p.c_str(), std::ios::binary);
  std::ostringstream ss;
  ss << f.rdbuf();
  return ss.str();
}

struct Upb {
  upb_DefPool *pool;
  upb_Arena *setup;
  bool ok;
};

static Upb load_defs(const std::string &desc_path) {
  Upb u;
  u.ok = false;
  u.setup = upb_Arena_New();
  u.pool = upb_DefPool_New();
  std::string raw = read_file(desc_path);
  if (raw.empty()) {
    std::printf("upb: cannot read %s\n", desc_path.c_str());
    return u;
  }
  google_protobuf_FileDescriptorSet *set =
      google_protobuf_FileDescriptorSet_parse(raw.data(), raw.size(), u.setup);
  if (!set) {
    std::printf("upb: cannot parse the descriptor set\n");
    return u;
  }
  size_t n = 0;
  const google_protobuf_FileDescriptorProto *const *files =
      google_protobuf_FileDescriptorSet_file(set, &n);
  for (size_t i = 0; i < n; ++i) {
    upb_Status st;
    upb_Status_Clear(&st);
    if (!upb_DefPool_AddFile(u.pool, files[i], &st)) {
      std::printf("upb: AddFile failed: %s\n", upb_Status_ErrorMessage(&st));
      return u;
    }
  }
  u.ok = true;
  return u;
}

static void check(bool ok, const char *what) {
  if (!ok) {
    ++g_fail;
    std::printf("  FAIL %s\n", what);
  }
}

// ---------------------------------------------------------------- one payload

template <class P>
static void run_case(Upb &u, const char *id, const char *msg_name, void (*pbmk)(P *),
                     const char *want_sha) {
  const upb_MessageDef *md = upb_DefPool_FindMessageByName(u.pool, msg_name);
  if (!md) {
    std::printf("  %-5s upb: no message def for %s\n", id, msg_name);
    ++g_fail;
    return;
  }
  const upb_MiniTable *mt = upb_MessageDef_MiniTable(md);

  P pb;
  pbmk(&pb);
  google::protobuf::Arena arena;
  P *pba = google::protobuf::Arena::CreateMessage<P>(&arena);
  pbmk(pba);

  std::string wire;
  pb_serialize_det(pb, &wire);

  // The upb message, built by DECODING the wire once. Untimed, and it means the upb arm
  // holds exactly the message the other arms hold rather than one built through a
  // reflection API that generated code would not use.
  upb_Arena *hold = upb_Arena_New();
  upb_Message *um = upb_Message_New(mt, hold);
  upb_DecodeStatus ds = upb_Decode(wire.data(), wire.size(), um, mt, NULL, 0, hold);
  check(ds == kUpb_DecodeStatus_Ok, "upb_Decode of the incumbent's bytes");

  // BYTE IDENTITY FIRST (R2), before anything is timed.
  {
    upb_Arena *a = upb_Arena_New();
    char *buf = NULL;
    size_t sz = 0;
    upb_EncodeStatus es =
        upb_Encode(um, mt, kUpb_EncodeOption_Deterministic, a, &buf, &sz);
    check(es == kUpb_EncodeStatus_Ok, "upb_Encode");
    std::string out(buf, sz);
    std::string got = sha_of(out);
    if (got != std::string(want_sha)) {
      // design/SHAPES.md records that P2.5 has TWO VALID ENCODINGS: protobuf C++ and upb
      // both write a map entry's value unconditionally, and the canonical form omits an
      // implicit-presence leaf holding the proto zero. Two independent Google runtimes,
      // the same +80 B. So a mismatch is reported with the form its encoder wrote, and is
      // only a failure if it also disagrees with the incumbent in this same process.
      std::printf("  %-5s upb writes %zu B; the manifest records a different form.\n", id,
                  out.size());
      check(sha_of(out) == sha_of(wire),
            "upb agrees with protobuf C++ in this process (the other valid form)");
    }
    upb_Arena_Free(a);
  }

  // --- timed
  std::string s1;
  auto f_pb = [&]() { pb_serialize_default(pb, &s1); AK_SINK_MEM(s1); };
  auto f_pba = [&]() { pb_serialize_default(*pba, &s1); AK_SINK_MEM(s1); };
  // ENCODE with a REUSED initial block, which is upb's documented allocation-free
  // pattern and the like-for-like shape against `pb` writing into a reused std::string.
  // With a fresh `upb_Arena_New()` per call -- the first version -- upb measured 1.2 to
  // 2.7 of protobuf C++ on encode, and that was the malloc, not the encoder: a ceiling
  // arm that is slower than the thing it is supposed to bound is a harness defect.
  std::vector<char> scratch(4u * 1024 * 1024 + wire.size() * 2 + 4096);
  auto f_upb = [&]() {
    upb_Arena *a = upb_Arena_Init(&scratch[0], scratch.size(), &upb_alloc_global);
    char *buf = NULL;
    size_t sz = 0;
    upb_Encode(um, mt, 0, a, &buf, &sz);
    AK_SINK(sz);
    upb_Arena_Free(a);
  };
  auto f_pbd = [&]() { P m; m.ParseFromString(wire); AK_SINK_MEM(m); };
  auto f_pbad = [&]() {
    google::protobuf::Arena ar;
    P *m = google::protobuf::Arena::CreateMessage<P>(&ar);
    m->ParseFromString(wire);
    AK_SINK_MEM(m);
  };
  // DECODE keeps a FRESH arena per call, because the decoded message is the thing you
  // keep: that is the same shape as `pb-arena`, which also builds a fresh arena.
  auto f_upbd = [&]() {
    upb_Arena *a = upb_Arena_New();
    upb_Message *m = upb_Message_New(mt, a);
    upb_Decode(wire.data(), wire.size(), m, mt, NULL, 0, a);
    AK_SINK(m);
    upb_Arena_Free(a);
  };

  int i1 = calibrate(f_pb), i2 = calibrate(f_pba), i3 = calibrate(f_upb);
  int i4 = calibrate(f_pbd), i5 = calibrate(f_pbad), i6 = calibrate(f_upbd);
  double e_pb = 1e300, e_pba = 1e300, e_upb = 1e300;
  double d_pb = 1e300, d_pba = 1e300, d_upb = 1e300;
  for (int r = 0; r < g_rounds; ++r) {
    // rotate, as bench.cpp does
    switch (r % 3) {
      case 0:
        e_pb = std::min(e_pb, timed(f_pb, i1));
        e_pba = std::min(e_pba, timed(f_pba, i2));
        e_upb = std::min(e_upb, timed(f_upb, i3));
        break;
      case 1:
        e_pba = std::min(e_pba, timed(f_pba, i2));
        e_upb = std::min(e_upb, timed(f_upb, i3));
        e_pb = std::min(e_pb, timed(f_pb, i1));
        break;
      default:
        e_upb = std::min(e_upb, timed(f_upb, i3));
        e_pb = std::min(e_pb, timed(f_pb, i1));
        e_pba = std::min(e_pba, timed(f_pba, i2));
        break;
    }
    switch (r % 3) {
      case 0:
        d_pb = std::min(d_pb, timed(f_pbd, i4));
        d_pba = std::min(d_pba, timed(f_pbad, i5));
        d_upb = std::min(d_upb, timed(f_upbd, i6));
        break;
      case 1:
        d_pba = std::min(d_pba, timed(f_pbad, i5));
        d_upb = std::min(d_upb, timed(f_upbd, i6));
        d_pb = std::min(d_pb, timed(f_pbd, i4));
        break;
      default:
        d_upb = std::min(d_upb, timed(f_upbd, i6));
        d_pb = std::min(d_pb, timed(f_pbd, i4));
        d_pba = std::min(d_pba, timed(f_pbad, i5));
        break;
    }
  }
  std::printf("%-6s %-4s %12.1f %12.1f %12.1f %10.3f %10.3f\n", id, "enc", e_pb, e_pba,
              e_upb, e_pba / e_pb, e_upb / e_pb);
  std::printf("%-6s %-4s %12.1f %12.1f %12.1f %10.3f %10.3f\n", id, "dec", d_pb, d_pba,
              d_upb, d_pba / d_pb, d_upb / d_pb);
  upb_Arena_Free(hold);
}

int main(int argc, char **argv) {
  std::string desc = argc > 1 ? argv[1] : "shapes.desc";
  if (argc > 2) g_rounds = atoi(argv[2]);
  std::printf("upb arm: a CEILING, not a candidate. protobuf C++ remains the incumbent.\n");
  std::printf("upb %s, built from the protobuf repository with its own CMake; minitables\n"
              "from upb reflection (no Bazel, no protoc-gen-upb, no hand-written codec).\n",
              AK_UPB_VERSION);
  Upb u = load_defs(desc);
  if (!u.ok) return 2;

  std::printf("\n%-6s %-4s %12s %12s %12s %10s %10s\n", "payload", "dir", "pb ns",
              "pb-arena ns", "upb ns", "arena/pb", "upb/pb");
#define X(id, Root, sroot, pfx, sha, nbytes)                                        \
  run_case<ns::Root>(u, id, "armonik.ffi.shapes.v1." #Root, &pbbuild::payload_##pfx, sha);
  AK_CASES(X)
#undef X
  std::printf("\n%d failures\n", g_fail);
  return g_fail ? 1 : 0;
}
