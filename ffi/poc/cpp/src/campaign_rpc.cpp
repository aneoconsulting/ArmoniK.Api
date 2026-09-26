// design/CAMPAIGN.md section 4.2, the RPC grid's CLIENT: one process pinned by the runner to
// AK_CPU_CLIENT, talking to `campaign_server` in ANOTHER process pinned to AK_CPU_SERVER,
// over a Unix domain socket (req. 17, amended 2026-09-26: grpc++ and the core both dial
// `unix:<path>`).
//
//   | cell | codec (client side)                                   | transport                    |
//   | A    | protobuf C++ through grpc++'s generated stub (SerializationTraits, production) | grpc++, sync stub |
//   | B    | protobuf C++ (SerializeToString / ParseFromArray)     | the core, ak_call_unary      |
//   | C    | the core through the C ABI (generated binding)        | the core, ak_call_unary      |
//   | D    | the core through the C ABI                            | grpc++, raw ByteBuffer, sync |
//   | E    | host-gen (the codec generated into C++, cpp_native)   | the core, ak_call_unary      |
//   | F    | host-gen                                              | grpc++, raw ByteBuffer, sync |
//
//   unknown fields (req. 10/12): C and D in each mode the core has in this build (full:
//   C-retain, C-drop, D-retain, D-drop; no-unknown: C-nounk, D-nounk), E and F in each mode
//   host-gen has (full: E-drop, E-retain, F-drop, F-retain; no-unknown: E-nounk, F-nounk).
//   A and B run the incumbent in its default mode. retain = decode_with_*_unk (every
//   position armed, grow-backed) and encode_into_*_unk for the core; core_native_retain for
//   host-gen. `--cells ABCDEF` expands every letter; a label list selects cells one by one.
//   delivery  (req. 16): B, C and E use the core's BLOCKING delivery; A, D and F use grpc++'s
//                        synchronous call, which is what packages/cpp does (its clients call
//                        the generated stubs' blocking methods), stated.
//   directions (req. 14): a = empty request, P2.2 response, decode only; a+read = the same
//                        call, then every field read (generated traversal, touch.cpp);
//                        b = P2.2 request the server decodes, empty response
//   in flight  (req. 15): 1, 8, 16 -- k host threads, each with one blocking call out
//   transport  (req. 17): --transport shipped|pinned, applied to grpc++ AND to the core
//   channels   (req. 13): one channel (grpc++) or client (the core) PER CELL, opened at start
//                        and warmed by the warm-up before round 1
//   checks     (req. 18): every call: status OK and response length equal to the expected
//                        payload; the first failure aborts the process and leaves no sample
//   samples    (req. 21): CPU = getrusage(RUSAGE_SELF) of this client process across the
//                        batch (the server is another process), wall = CLOCK_MONOTONIC
//   rounds     (req. 22/23): every (dir, inflight) group runs its cells in an order shuffled
//                        per (launch, round, dir, inflight), recorded as order_pos; samples
//                        buffered and written only if the whole run succeeded (R-H4)
//   threads    (R-H2, req. 4): the k caller threads are created once, before any timed
//                        window; the header records them, the core's runtime workers and the
//                        process's thread count after warm-up (grpc-core's own threads)
//
//   --warm-server N  (req. 13): N calls per direction from grpc++ and N from the core's
//                    transport, every call checked, then exit (the runner warms the one
//                    server of the launch with it, per socket, before any client)
//   --count N        (req. 19, a counting build): per call, the crossings of cells B, C, D
//                    and E in each mode and direction, then exit
#include "rpc_common.h"

#include <sys/resource.h>
#include <sys/syscall.h>

#include <algorithm>
#include <condition_variable>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <mutex>
#include <random>
#include <string>
#include <thread>
#include <vector>

#include <google/protobuf/io/coded_stream.h>
#include <google/protobuf/io/zero_copy_stream_impl_lite.h>

#include "ak_abi.h"
#ifdef AK_NO_UNKNOWN_FIELDS  // WP5 step 10: the no-unknown client (nounk/include's ak_abi.h)
#include "generated/binding_nounk.h"
#else
#include "generated/binding.h"
#include "generated/core_native_retain.h"
#endif
#include "generated/build.h"
#include "generated/core_native.h"
#include "generated/pb_build.h"
#include "generated/touch.h"

using namespace akrpc;

namespace {

const char *const kPush = "/armonik.ffi.shapes.v1.Shapes/Push";
typedef shapes::ListTasksDetailedResponse Fac;
typedef svcns::ListTasksDetailedResponse Pb;

enum Mode { kDefault, kRetain, kDrop, kNoUnk };
// Which binary produced a sample: A and B run in both clients (in-process controls), so a
// sample is identified by (build, cell), never by cell alone.
#ifdef AK_NO_UNKNOWN_FIELDS
const char *const kBuild = "no-unknown";
#else
const char *const kBuild = "full";
#endif
struct Cell {
  char base;
  Mode mode;
  std::string label;
};
const char *mode_name(Mode m) {
  return m == kRetain ? "retain" : m == kDrop ? "drop" : m == kNoUnk ? "no-unknown" : "default";
}
bool grpc_cell(char c) { return c == 'A' || c == 'D' || c == 'F'; }
bool core_codec(char c) { return c == 'C' || c == 'D'; }
bool hostgen_codec(char c) { return c == 'E' || c == 'F'; }

std::vector<Cell> parse_cells(const std::string &spec) {
  std::vector<Cell> out;
  bool labels = spec.find(',') != std::string::npos || spec.find('-') != std::string::npos;
  if (!labels) {
    for (char c : spec) {
      if (c == 'C' || c == 'D' || c == 'E' || c == 'F') {
#ifdef AK_NO_UNKNOWN_FIELDS
        out.push_back(Cell{c, kNoUnk, std::string(1, c) + "-nounk"});
#else
        out.push_back(Cell{c, kRetain, std::string(1, c) + "-retain"});
        out.push_back(Cell{c, kDrop, std::string(1, c) + "-drop"});
#endif
      } else {
        out.push_back(Cell{c, kDefault, std::string(1, c)});
      }
    }
    return out;
  }
  size_t p = 0;
  while (p <= spec.size()) {
    size_t q = spec.find(',', p);
    std::string l = spec.substr(p, q == std::string::npos ? std::string::npos : q - p);
    if (!l.empty()) {
      Mode m = l.size() == 1 ? kDefault
               : l.substr(1) == "-retain" ? kRetain
               : l.substr(1) == "-nounk" ? kNoUnk : kDrop;
      out.push_back(Cell{l[0], m, l});
    }
    if (q == std::string::npos) break;
    p = q + 1;
  }
  return out;
}

struct Cfg {
  std::string target, transport = "shipped", cells = "ABCDEF", dirs = "arb";
  std::vector<int> inflight = {1, 8, 16};
  int launch = 0, rounds = 5, calls = 96, warmup = 32, workers = 2;
  int fail_after = -1;   // test only: abort after this many samples (the gate's R-H4 control)
  int warm_server = 0;   // --warm-server N
  int count = 0;         // --count N
};

[[noreturn]] void die(const char *what, long v) {
  std::fprintf(stderr, "CALL CHECK FAILED: %s (%ld) -- the run is aborted, no figure\n", what, v);
  std::fflush(stdout);
  std::_Exit(3);
}

double rusage_ns() {
  struct rusage r;
  getrusage(RUSAGE_SELF, &r);
  return (double)(r.ru_utime.tv_sec + r.ru_stime.tv_sec) * 1e9 +
         (double)(r.ru_utime.tv_usec + r.ru_stime.tv_usec) * 1e3;
}

int proc_threads() {
  std::ifstream f("/proc/self/status");
  std::string line;
  while (std::getline(f, line))
    if (line.compare(0, 8, "Threads:") == 0) return std::atoi(line.c_str() + 8);
  return -1;
}

const char *dir_label(char d) { return d == 'a' ? "a" : d == 'r' ? "a+read" : "b"; }

// ---- one connection per cell (req. 13) ------------------------------------------------
struct Conn {
  std::shared_ptr<grpc::Channel> chan;
  std::vector<std::unique_ptr<svcns::Shapes::Stub> > stubs;
  ak_client *cl = nullptr;
};

struct World {
  Cfg cfg;
  ak_runtime *rt = nullptr;
  std::vector<Cell> cells;
  std::vector<Conn> conns;   // parallel to cells
  size_t expect_a = 0;       // the P2.2 response length (from the server)
  Pb pb_req;                 // direction b's request, incumbent object
  Fac fac_req;               // the same, facade object
};

grpc::ChannelArguments channel_args(const std::string &transport, const std::string &cell) {
  // `shipped`: what packages/cpp's getChannelArguments sets (keepalive 30 s, max idle 5 min,
  // a local subchannel pool; ArmoniK.Api.Common/source/utils/ChannelArguments.cpp). Its
  // retry/timeout service config is NOT applied: the retry policy never fires on a healthy
  // local call. `pinned` adds the 4 MiB stream window with BDP off (C31: grpc-core has no
  // connection-window argument, so "pinned" on grpc++ is the stream half only). The cell's
  // label is a channel argument too, so every cell gets its own connection (req. 13).
  grpc::ChannelArguments a = pinned_channel_args(transport == "pinned");
  a.SetInt(GRPC_ARG_KEEPALIVE_TIME_MS, 30000);
  a.SetInt(GRPC_ARG_MAX_CONNECTION_IDLE_MS, 300000);
  a.SetInt(GRPC_ARG_USE_LOCAL_SUBCHANNEL_POOL, 1);
  a.SetString("ak.campaign.cell", cell);
  return a;
}

ak_client *core_client(ak_runtime *rt, const std::string &target, const std::string &transport) {
  // the core's transport: `shipped` = the stack's defaults (NULL options; packages/cpp pins no
  // window, and ak_client_new's defaults are tonic/hyper's), `pinned` = 4 MiB stream and
  // connection windows, adaptive off, Nagle off, max message 2 MiB (rpc_common.h core_opts).
  // The target is `unix:<path>`, which the core dials as a Unix domain socket.
  if (transport == "pinned") {
    ak_client_opts o = pinned_core_opts(true);
    return ak_client_new_opts(rt, (const uint8_t *)target.data(), target.size(), &o);
  }
  return ak_client_new(rt, (const uint8_t *)target.data(), target.size());
}

// Per caller thread: its encode context, its root-bound decode context, host-gen's encoders.
struct ThreadCtx {
  ak_enc_ctx *ec = nullptr;
  ak_dec_ctx *dc = nullptr;
  ak::Enc *ne = nullptr, *nre = nullptr;
  ThreadCtx() {
    ec = ak_enc_ctx_new();
    dc = shapes::ffi::dec_ctx_new_for<Fac>();  // decision 11 rule 6
    ne = new ak::Enc(shapes::native::kSites);
#ifndef AK_NO_UNKNOWN_FIELDS
    nre = new ak::Enc(shapes::native_retain::kSites);
#endif
  }
  ~ThreadCtx() {
    ak_enc_ctx_free(ec);
    ak_dec_ctx_free(dc);
    delete ne;
    delete nre;
  }
};

// ---- the client-side codecs of cells C/D (core) and E/F (host-gen), per mode -------------
// Encode: returns 0 and (p, n) or a negative code. For the core, `take` counts the timed
// loop's ak_enc_take (req. 19's counting mode).
int32_t codec_encode(char base, Mode m, ThreadCtx &tc, const Fac &v, const uint8_t **p, size_t *n) {
  if (core_codec(base)) {
#ifdef AK_NO_UNKNOWN_FIELDS
    (void)m;
    intptr_t rc = shapes::ffi::encode_into_list_tasks_detailed_response(tc.ec, v, shapes::ffi::tcs_core());
#else
    intptr_t rc = m == kRetain
                      ? shapes::ffi::encode_into_list_tasks_detailed_response_unk(tc.ec, v, shapes::ffi::tcs_core())
                      : shapes::ffi::encode_into_list_tasks_detailed_response(tc.ec, v, shapes::ffi::tcs_core());
#endif
    if (rc < 0) return (int32_t)rc;
    return ak_enc_take(tc.ec, p, n);
  }
#ifndef AK_NO_UNKNOWN_FIELDS
  if (m == kRetain) {
    shapes::native_retain::encode_into_list_tasks_detailed_response(v, tc.nre);
    *p = tc.nre->data(); *n = tc.nre->size();
    return tc.nre->err;
  }
#endif
  shapes::native::encode_into_list_tasks_detailed_response(v, tc.ne);
  *p = tc.ne->data(); *n = tc.ne->size();
  return tc.ne->err;
}

int32_t codec_decode(char base, Mode m, ThreadCtx &tc, const uint8_t *p, size_t n, Fac *f) {
  if (core_codec(base)) {
#ifdef AK_NO_UNKNOWN_FIELDS
    (void)m;
    return shapes::ffi::decode_with_list_tasks_detailed_response(tc.dc, p, n, f);
#else
    return m == kRetain ? shapes::ffi::decode_with_list_tasks_detailed_response_unk(tc.dc, p, n, f)
                        : shapes::ffi::decode_with_list_tasks_detailed_response(tc.dc, p, n, f);
#endif
  }
#ifndef AK_NO_UNKNOWN_FIELDS
  if (m == kRetain) return shapes::native_retain::decode_list_tasks_detailed_response(p, n, f);
#endif
  (void)m;
  return shapes::native::decode_list_tasks_detailed_response(p, n, f);
}

// One call of one cell in one direction. Returns a fold (so nothing is dead).
long cell_call(World &w, size_t ci, char dir, int t, ThreadCtx &tc) {
  const Cell &cl = w.cells[ci];
  Conn &cn = w.conns[ci];
  const char cell = cl.base;
  static const uint8_t kNoReq[1] = {0};
  const bool resp = dir == 'a' || dir == 'r';
  const bool read = dir == 'r';
  if (cell == 'A') {
    grpc::ClientContext ctx;
    svcns::Shapes::Stub &st = *cn.stubs[(size_t)t % cn.stubs.size()];
    if (resp) {
      svcns::Empty q;
      Pb r;
      grpc::Status s = st.Fetch(&ctx, q, &r);
      if (!s.ok()) die("A/a status", (long)s.error_code());
      if (r.tasks_size() != 500) die("A/a content", r.tasks_size());
      return read ? (long)pbtouch::touch(r) : r.tasks_size();
    }
    svcns::Empty r;
    grpc::Status s = st.Push(&ctx, w.pb_req, &r);
    if (!s.ok()) die("A/b status", (long)s.error_code());
    return 1;
  }
  if (!grpc_cell(cell)) {  // B, C, E: the core's transport
    const char *path = resp ? kFetchPath : kPush;
    std::string pbreq;
    const uint8_t *req = kNoReq;
    size_t reqn = 0;
    if (!resp) {
      if (cell == 'B') {
        w.pb_req.SerializeToString(&pbreq);
        req = (const uint8_t *)pbreq.data();
        reqn = pbreq.size();
      } else {
        int32_t rc = codec_encode(cell, cl.mode, tc, w.fac_req, &req, &reqn);
        if (rc != 0) die("C/E b encode", rc);
      }
    }
    struct ak_bytes out;
    out.ptr = NULL; out.len = 0; out.owner = NULL;
    int32_t rc = ak_call_unary(cn.cl, (const uint8_t *)path, std::strlen(path), req, reqn, &out);
    if (rc != AK_OK) die("core call status", rc);
    long n = 1;
    if (resp) {
      if (out.len != w.expect_a) die("core/a response length", (long)out.len);
      if (cell == 'B') {
        Pb m;
        if (!m.ParseFromArray(out.ptr, (int)out.len)) die("B/a decode", 0);
        n = read ? (long)pbtouch::touch(m) : m.tasks_size();
      } else {
        Fac f;
        int32_t drc = codec_decode(cell, cl.mode, tc, out.ptr, out.len, &f);
        if (drc != 0) die("C/E a decode", drc);
        n = read ? (long)shapes::touch::touch(f) : (long)f.tasks.size();
      }
    } else if (out.len != 0) {
      die("core/b response length", (long)out.len);
    }
    ak_bytes_free(&out);
    return n;
  }
  // D, F: grpc++'s transport carrying opaque bytes, the generated codec at the client.
  grpc::internal::RpcMethod method(resp ? kFetchPath : kPush, grpc::internal::RpcMethod::NORMAL_RPC);
  grpc::ClientContext ctx;
  grpc::ByteBuffer req, rsp;
  if (resp) {
    grpc::Slice e;
    req = grpc::ByteBuffer(&e, 1);
  } else {
    const uint8_t *p = NULL;
    size_t n = 0;
    int32_t rc = codec_encode(cell, cl.mode, tc, w.fac_req, &p, &n);
    if (rc != 0) die("D/F b encode", rc);
    grpc::Slice s(p, n);
    req = grpc::ByteBuffer(&s, 1);
  }
  grpc::Status s = grpc::internal::BlockingUnaryCall<grpc::ByteBuffer, grpc::ByteBuffer>(
      cn.chan.get(), method, &ctx, req, &rsp);
  if (!s.ok()) die("D/F status", (long)s.error_code());
  if (!resp) {
    if (rsp.Length() != 0) die("D/F b response length", (long)rsp.Length());
    return 1;
  }
  if (rsp.Length() != w.expect_a) die("D/F a response length", (long)rsp.Length());
  std::vector<grpc::Slice> slices;
  if (!rsp.Dump(&slices).ok()) die("D/F a dump", 0);
  std::string flat;
  const uint8_t *p;
  size_t len;
  if (slices.size() == 1) {
    p = slices[0].begin();
    len = slices[0].size();
  } else {
    // The generated decoders need one contiguous buffer; protobuf reads the slice list. The
    // concatenation is part of cells D and F (it is the integration's cost), stated.
    for (size_t i = 0; i < slices.size(); ++i) flat.append((const char *)slices[i].begin(), slices[i].size());
    p = (const uint8_t *)flat.data();
    len = flat.size();
  }
  Fac f;
  int32_t drc = codec_decode(cell, cl.mode, tc, p, len, &f);
  if (drc != 0) die("D/F a decode", drc);
  return read ? (long)shapes::touch::touch(f) : (long)f.tasks.size();
}

// R-H2: the caller threads are created ONCE, before any timed window, each with its own
// contexts, and reused by every batch. A batch hands the first k of them `per` calls each and
// waits: the window holds the calls and one condition-variable round trip per thread.
struct Pool {
  World *w;
  std::vector<std::thread> ts;
  std::mutex m;
  std::condition_variable go, done;
  uint64_t gen = 0;
  int k = 0, per = 0, pending = 0;
  size_t cell = 0;
  char dir = 'a';
  bool quit = false;
  std::vector<long> acc;

  Pool(World &wr, int n) : w(&wr), acc((size_t)n, 0) {
    for (int t = 0; t < n; ++t) ts.push_back(std::thread([this, t]() { run(t); }));
  }
  ~Pool() {
    { std::lock_guard<std::mutex> l(m); quit = true; ++gen; }
    go.notify_all();
    for (size_t i = 0; i < ts.size(); ++i) ts[i].join();
  }
  void run(int t) {
    ThreadCtx tc;
    uint64_t seen = 0;
    for (;;) {
      int myk, myper; size_t c; char d;
      {
        std::unique_lock<std::mutex> l(m);
        go.wait(l, [&] { return gen != seen; });
        seen = gen;
        if (quit) break;
        myk = k; myper = per; c = cell; d = dir;
      }
      if (t >= myk) continue;
      long n = 0;
      for (int i = 0; i < myper; ++i) n += cell_call(*w, c, d, t, tc);
      std::lock_guard<std::mutex> l(m);
      acc[(size_t)t] = n;
      if (--pending == 0) done.notify_one();
    }
  }
  long batch(size_t c, char d, int kk, int total) {
    std::unique_lock<std::mutex> l(m);
    k = kk; per = (total + kk - 1) / kk; cell = c; dir = d; pending = kk; ++gen;
    go.notify_all();
    done.wait(l, [&] { return pending == 0; });
    long s = 0;
    for (int i = 0; i < kk; ++i) s += acc[(size_t)i];
    return s;
  }
};

std::vector<int> parse_list(const char *s) {
  std::vector<int> v;
  std::string x(s);
  size_t p = 0;
  while (p < x.size()) {
    size_t q = x.find(',', p);
    v.push_back(std::atoi(x.substr(p, q == std::string::npos ? std::string::npos : q - p).c_str()));
    if (q == std::string::npos) break;
    p = q + 1;
  }
  return v;
}

std::string det(const Pb &m) {
  std::string s;
  google::protobuf::io::StringOutputStream so(&s);
  google::protobuf::io::CodedOutputStream co(&so);
  co.SetSerializationDeterministic(true);
  m.SerializeToCodedStream(&co);
  return s;
}

// --warm-server N (req. 13): N checked calls per direction from each client transport.
int warm_server(World &w, int n) {
  std::shared_ptr<grpc::Channel> ch =
      grpc::CreateCustomChannel(w.cfg.target, grpc::InsecureChannelCredentials(), channel_args(w.cfg.transport, "warm"));
  std::unique_ptr<svcns::Shapes::Stub> st = svcns::Shapes::NewStub(ch);
  ak_client *cl = core_client(w.rt, w.cfg.target, w.cfg.transport);
  if (!cl) die("ak_client_new (warm)", 0);
  std::string req;
  w.pb_req.SerializeToString(&req);
  static const uint8_t kNoReq[1] = {0};
  for (int i = 0; i < n; ++i) {
    { grpc::ClientContext c; svcns::Empty q; Pb r;
      if (!st->Fetch(&c, q, &r).ok() || r.tasks_size() != 500) die("warm grpc++ a", i); }
    { grpc::ClientContext c; svcns::Empty r;
      if (!st->Push(&c, w.pb_req, &r).ok()) die("warm grpc++ b", i); }
    struct ak_bytes out;
    out.ptr = NULL; out.len = 0; out.owner = NULL;
    if (ak_call_unary(cl, (const uint8_t *)kFetchPath, std::strlen(kFetchPath), kNoReq, 0, &out) != AK_OK ||
        out.len != w.expect_a) die("warm core a", i);
    ak_bytes_free(&out);
    out.ptr = NULL; out.len = 0; out.owner = NULL;
    if (ak_call_unary(cl, (const uint8_t *)kPush, std::strlen(kPush), (const uint8_t *)req.data(), req.size(), &out) != AK_OK ||
        out.len != 0) die("warm core b", i);
    ak_bytes_free(&out);
  }
  ak_client_destroy(cl);
  std::printf("# {\"campaign_rpc_warm_server\": {\"transport\": \"%s\", \"calls_per_direction_per_client_transport\": %d,"
              " \"client_transports\": [\"grpc++\", \"core\"]}}\n", w.cfg.transport.c_str(), n);
  return 0;
}

#ifdef AK_COUNTING
// --count N (req. 19, amended 2026-09-26): per call, every exported entry point the loop
// calls. rpc = the core's transport counters (ak_call_unary, ak_bytes_free); codec = the
// encode/decode contexts' counters (entry points, element loops, reverse calls); host =
// what the binding calls that no counter sees (ak_enc_reset inside encode_into_*, before
// the encode entry point; the two ak_dec_reset_<Root> of a retain decode, before and after
// it); take = ak_enc_take after a core encode. Cells B, C, D and E; direction a+read has
// the crossings of a (the read is host code).
int count_cells(World &w, int n) {
  std::printf("# {\"campaign_rpc_counts\": {\"build\": \"%s\", \"calls\": %d, \"retain\": \"every position armed, no pre-placed buffer, unk_grow allocates exactly the size requested\"}}\n",
              kBuild, n);
  ThreadCtx tc;
  const char dirs[2] = {'a', 'b'};
  for (size_t ci = 0; ci < w.cells.size(); ++ci) {
    const Cell &cl = w.cells[ci];
    if (cl.base != 'B' && cl.base != 'C' && cl.base != 'D' && cl.base != 'E') continue;
    for (int di = 0; di < 2; ++di) {
      char d = dirs[di];
      ak_rpc_counters_reset();
      ak_enc_counters_reset(tc.ec);
      ak_dec_counters_reset(tc.dc);
      shapes::ffi::host_calls_take();
      for (int i = 0; i < n; ++i) cell_call(w, ci, d, 0, tc);
      struct ak_rpc_counters rc;
      ak_rpc_counters(&rc);
      AkCounters ce, cd;
      ak_enc_counters(tc.ec, &ce);
      ak_dec_counters(tc.dc, &cd);
      uint64_t host = shapes::ffi::host_calls_take();
      uint64_t take = (core_codec(cl.base) && d == 'b') ? (uint64_t)n : 0;
      uint64_t codec_f = ce.forward + cd.forward, codec_r = ce.reverse + cd.reverse;
      uint64_t fwd = rc.forward + codec_f + host + take, rev = rc.reverse + codec_r;
      std::printf("  %-10s %-2s forward/call %8.3f  reverse/call %8.3f   (rpc %.3f/%.3f + codec %.3f/%.3f + host %.3f + take %.3f)\n",
                  cl.label.c_str(), dir_label(d), fwd / (double)n, rev / (double)n,
                  rc.forward / (double)n, rc.reverse / (double)n, codec_f / (double)n, codec_r / (double)n,
                  host / (double)n, take / (double)n);
    }
  }
  return 0;
}
#endif

}  // namespace

int main(int argc, char **argv) {
  akrpc::init_core_or_die();
  World w;
  Cfg &c = w.cfg;
  for (int i = 1; i + 1 < argc; i += 2) {
    std::string a = argv[i];
    const char *v = argv[i + 1];
    if (a == "--target") c.target = v;
    else if (a == "--transport") c.transport = v;
    else if (a == "--cells") c.cells = v;
    else if (a == "--dirs") c.dirs = v;
    else if (a == "--inflight") c.inflight = parse_list(v);
    else if (a == "--launch") c.launch = std::atoi(v);
    else if (a == "--rounds") c.rounds = std::atoi(v);
    else if (a == "--calls") c.calls = std::atoi(v);
    else if (a == "--warmup") c.warmup = std::atoi(v);
    else if (a == "--workers") c.workers = std::atoi(v);
    else if (a == "--expect") w.expect_a = (size_t)std::atoll(v);
    else if (a == "--fail-after") c.fail_after = std::atoi(v);
    else if (a == "--warm-server") c.warm_server = std::atoi(v);
    else if (a == "--count") c.count = std::atoi(v);
  }
  if (c.target.empty() || (c.transport != "shipped" && c.transport != "pinned") || !w.expect_a) {
    std::fprintf(stderr, "usage: campaign_rpc --target unix:PATH --expect BYTES --transport shipped|pinned ...\n");
    return 2;
  }
  w.rt = ak_runtime_new((uint32_t)c.workers);
  pbbuild::payload_p2_2(&w.pb_req);
  w.fac_req = shapes::build::payload_p2_2();
  if (c.warm_server > 0) return warm_server(w, c.warm_server);

  w.cells = parse_cells(c.cells);
  for (size_t i = 0; i < w.cells.size(); ++i) {
    const Cell &cl = w.cells[i];
    bool coded = cl.base >= 'C' && cl.base <= 'F';
    if (std::string("ABCDEF").find(cl.base) == std::string::npos || coded != (cl.mode != kDefault)
#ifdef AK_NO_UNKNOWN_FIELDS
        || cl.mode == kRetain || cl.mode == kDrop
#else
        || cl.mode == kNoUnk
#endif
        )
      die("unknown cell label", (long)i);
  }
  int maxk = 1;
  for (int k : c.inflight) maxk = k > maxk ? k : maxk;
  // One channel or client per cell (req. 13), opened here, warmed by the warm-up below.
  w.conns.resize(w.cells.size());
  for (size_t i = 0; i < w.cells.size(); ++i) {
    Conn &cn = w.conns[i];
    if (grpc_cell(w.cells[i].base)) {
      cn.chan = grpc::CreateCustomChannel(c.target, grpc::InsecureChannelCredentials(),
                                          channel_args(c.transport, w.cells[i].label));
      for (int s = 0; s < maxk; ++s) cn.stubs.emplace_back(svcns::Shapes::NewStub(cn.chan));
    } else {
      cn.cl = core_client(w.rt, c.target, c.transport);
      if (!cn.cl) die("ak_client_new", (long)i);
    }
  }

  // Before any sample: C, D, E and F in each mode decode one Fetch response, re-encode it in
  // the same mode, and the result must be what the INCUMBENT makes of the same response
  // (protobuf C++ retains unknown fields by default): both sides parsed by protobuf and
  // re-serialised deterministically, so the comparison is of messages, not of one encoder's
  // form. (Not byte identity with the wire: the server's pre-serialised P2.2 is protobuf's
  // own form, which differs from the canonical one in encoding choices, not in content.)
  {
    ThreadCtx tc;
    for (size_t i = 0; i < w.cells.size(); ++i) {
      const Cell &cl = w.cells[i];
      if (cl.mode == kDefault) continue;
      ak_client *pc = grpc_cell(cl.base) ? core_client(w.rt, c.target, c.transport) : w.conns[i].cl;
      struct ak_bytes out;
      out.ptr = NULL; out.len = 0; out.owner = NULL;
      static const uint8_t kNone[1] = {0};
      if (ak_call_unary(pc, (const uint8_t *)kFetchPath, std::strlen(kFetchPath), kNone, 0, &out) != AK_OK ||
          out.len != w.expect_a)
        die("pre-check fetch", (long)out.len);
      std::string wire((const char *)out.ptr, out.len);
      ak_bytes_free(&out);
      if (grpc_cell(cl.base)) ak_client_destroy(pc);
      Fac f;
      int32_t drc = codec_decode(cl.base, cl.mode, tc, (const uint8_t *)wire.data(), wire.size(), &f);
      if (drc != 0) die("pre-check decode", drc);
      const uint8_t *q = NULL;
      size_t qn = 0;
      if (codec_encode(cl.base, cl.mode, tc, f, &q, &qn) != 0) die("pre-check re-encode", (long)i);
      std::string ours((const char *)q, qn);
      Pb inc, back;
      if (!inc.ParseFromString(wire) || !back.ParseFromString(ours)) die("pre-check incumbent parse", 0);
      if (det(inc) != det(back)) die("pre-check: the re-encode differs from the incumbent's", (long)i);
      if ((long)f.tasks.size() != inc.tasks_size()) die("pre-check task count", (long)f.tasks.size());
    }
  }
  // Cell A's wire length, once, before the rounds.
  for (size_t i = 0; i < w.cells.size(); ++i) {
    if (w.cells[i].base != 'A') continue;
    grpc::ClientContext ctx;
    svcns::Empty q;
    Pb r;
    if (!w.conns[i].stubs[0]->Fetch(&ctx, q, &r).ok() || r.ByteSizeLong() != w.expect_a)
      die("A/a wire length (pre-check)", (long)r.ByteSizeLong());
  }
#ifdef AK_COUNTING
  if (c.count > 0) return count_cells(w, c.count);
#else
  if (c.count > 0) { std::fprintf(stderr, "--count needs a counting build\n"); return 2; }
#endif

  Pool pool(w, maxk);  // R-H2: the caller threads, created before any timed window
  for (char d : c.dirs)
    for (int k : c.inflight)
      for (size_t j = 0; j < w.cells.size(); ++j) pool.batch(j, d, k, c.warmup);  // warm-up, identical per cell
  std::printf("# {\"campaign_rpc\": {\"build\": \"%s\", \"target\": \"%s\", \"transport\": \"%s\", \"cells\": \"%s\","
              " \"dirs\": \"%s\", \"calls_per_sample\": %d, \"warmup_calls_per_cell\": %d, \"rounds\": %d,"
              " \"expect_bytes\": %zu, \"delivery\": \"B, C, E: the core's blocking ak_call_unary; A, D, F: grpc++'s"
              " synchronous call (packages/cpp's idiom)\", \"channels\": \"one per cell, opened at start, warmed\","
              " \"threads\": {\"caller_threads\": %d, \"core_runtime_workers\": %d,"
              " \"process_threads_after_warmup\": %d, \"grpcpp\": \"grpc-core sizes its own pollers and executor"
              " (no application setting); they are counted in process_threads_after_warmup\"},"
              " \"precheck\": \"C, D, E, F in each mode: decode, re-encode, equal to the incumbent's deterministic"
              " re-serialisation\"}}\n",
              kBuild, c.target.c_str(), c.transport.c_str(), c.cells.c_str(), c.dirs.c_str(), c.calls,
              c.warmup, c.rounds, w.expect_a, maxk, c.workers, proc_threads());

  // R-H4 / req 18: samples are BUFFERED and written only when the whole run succeeded, so an
  // aborted run (die -> _Exit(3)) leaves no sample at all, not the cells before the failure.
  std::string samples;
  int nsamples = 0;
  for (int r = 0; r < c.rounds; ++r) {
    for (char d : c.dirs) {
      for (int k : c.inflight) {
        size_t nc = w.cells.size();
        // R-H18 / R-H23 (req 22): the cell order of every (round, dir, in-flight) group is a
        // shuffle seeded by (launch, round, dir, in-flight); each sample records its position.
        std::vector<size_t> order(nc);
        for (size_t j = 0; j < nc; ++j) order[j] = j;
        std::mt19937 rng((uint32_t)(c.launch * 1000003 + r * 1009 + d * 31 + k));
        std::shuffle(order.begin(), order.end(), rng);
        for (size_t j = 0; j < nc; ++j) {
          const Cell &cell = w.cells[order[j]];
          double c0 = rusage_ns(), w0 = wall_ns();
          pool.batch(order[j], d, k, c.calls);
          double c1 = rusage_ns(), w1 = wall_ns();
          int iters = ((c.calls + k - 1) / k) * k;
          char line[600];
          std::snprintf(line, sizeof(line),
                        "{\"slice\":\"cpp\",\"suite\":\"rpc\",\"build\":\"%s\",\"cell\":\"%s\",\"unknown_mode\":\"%s\","
                        "\"payload\":\"P2.2\",\"dir\":\"%s\",\"transport\":\"%s\",\"socket\":\"uds\",\"inflight\":%d,"
                        "\"launch\":%d,\"round\":%d,\"order_pos\":%zu,\"cpu_ns\":%.0f,\"cpu_clock\":\"process\","
                        "\"wall_ns\":%.0f,\"iters\":%d}\n",
                        kBuild, cell.label.c_str(), mode_name(cell.mode), dir_label(d), c.transport.c_str(), k,
                        c.launch, r, j, c1 - c0, w1 - w0, iters);
          samples += line;
          if (++nsamples == c.fail_after) die("--fail-after (test control)", nsamples);
        }
      }
    }
  }
  std::fwrite(samples.data(), 1, samples.size(), stdout);
  std::fflush(stdout);
  for (size_t i = 0; i < w.conns.size(); ++i)
    if (w.conns[i].cl) ak_client_destroy(w.conns[i].cl);
  ak_runtime_destroy(w.rt);
  return 0;
}
