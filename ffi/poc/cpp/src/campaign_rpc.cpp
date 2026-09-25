// design/CAMPAIGN.md section 4.2, the RPC grid's CLIENT: one process pinned by the runner to
// AK_CPU_CLIENT, talking to `campaign_server` in ANOTHER process pinned to AK_CPU_SERVER.
//
//   | cell | codec (client side)                        | transport                        |
//   | A    | protobuf C++ through grpc++'s generated stub (SerializationTraits: production) | grpc++ |
//   | B    | protobuf C++ (SerializeToString / ParseFromArray)  | the core, ak_call_unary (blocking) |
//   | C    | the core through the C ABI (generated binding)     | the core, ak_call_unary (blocking) |
//   | D    | the core through the C ABI                         | grpc++, raw ByteBuffer methods     |
//
//   directions (req. 14): a = empty request, P2.2 response (Fetch)
//                         b = P2.2 request the server decodes, empty response (Push)
//   in flight  (req. 15): 1, 8, 16 -- k host threads, each with one blocking call out
//   transport  (req. 17): --transport shipped|pinned, applied to grpc++ AND to the core
//   delivery   (req. 16): B and C use the core's BLOCKING delivery only
//   checks     (req. 18): every call: status OK and response length equal to the expected
//                         payload; the first failure aborts the process (no figure)
//   samples    (req. 21): CPU = getrusage(RUSAGE_SELF) of this client process across the
//                         batch (the server is another process), wall = CLOCK_MONOTONIC
//   rounds     (req. 22/23): every (dir, inflight) group runs its four cells in an order
//                         rotated by round; one JSON line per sample (req. 28)
//
// Cell A's generated stub does not expose the wire length. Its response is checked by
// content on every call (500 tasks), and its wire length once, before the rounds, by
// ByteSizeLong() on a decoded response (outside every sample).
#include "rpc_common.h"

#include <sys/resource.h>
#include <sys/syscall.h>

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <thread>
#include <vector>

#include "ak_abi.h"
#include "generated/binding.h"
#include "generated/build.h"
#include "generated/pb_build.h"

using namespace akrpc;

namespace {

const char *const kPush = "/armonik.ffi.shapes.v1.Shapes/Push";

struct Cfg {
  std::string target, transport = "shipped", cells = "ABCD", dirs = "ab";
  std::vector<int> inflight = {1, 8, 16};
  int launch = 0, rounds = 5, calls = 96, warmup = 32, workers = 2;
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

// ---- the shared state of one process --------------------------------------------------
struct World {
  Cfg cfg;
  std::shared_ptr<grpc::Channel> chan;
  std::vector<std::unique_ptr<svcns::Shapes::Stub> > stubs;
  ak_runtime *rt = nullptr;
  ak_client *cl = nullptr;
  size_t expect_a = 0;                        // the P2.2 response length (from the server)
  svcns::ListTasksDetailedResponse pb_req;    // direction b's request, incumbent object
  shapes::ListTasksDetailedResponse fac_req;  // the same, facade object
};

grpc::ChannelArguments channel_args(const std::string &transport) {
  // `shipped`: what packages/cpp's getChannelArguments sets (keepalive 30 s, max idle 5 min,
  // a local subchannel pool; ArmoniK.Api.Common/source/utils/ChannelArguments.cpp). Its
  // retry/timeout service config is NOT applied: the retry policy never fires on a healthy
  // loopback call. `pinned` adds the 4 MiB stream window with BDP off (C31: grpc-core has no
  // connection-window argument, so "pinned" on grpc++ is the stream half only).
  grpc::ChannelArguments a = pinned_channel_args(transport == "pinned");
  a.SetInt(GRPC_ARG_KEEPALIVE_TIME_MS, 30000);
  a.SetInt(GRPC_ARG_MAX_CONNECTION_IDLE_MS, 300000);
  a.SetInt(GRPC_ARG_USE_LOCAL_SUBCHANNEL_POOL, 1);
  return a;
}

// ---- one call of each cell, direction a and b ---------------------------------------
long cell_call(World &w, char cell, char dir, int t, ak_enc_ctx *ec, ak_dec_ctx *dc) {
  static const uint8_t kNoReq[1] = {0};
  if (cell == 'A') {
    grpc::ClientContext ctx;
    svcns::Shapes::Stub &st = *w.stubs[(size_t)t % w.stubs.size()];
    if (dir == 'a') {
      svcns::Empty q;
      svcns::ListTasksDetailedResponse r;
      grpc::Status s = st.Fetch(&ctx, q, &r);
      if (!s.ok()) die("A/a status", (long)s.error_code());
      if (r.tasks_size() != 500) die("A/a content", r.tasks_size());
      return r.tasks_size();
    }
    svcns::Empty r;
    grpc::Status s = st.Push(&ctx, w.pb_req, &r);
    if (!s.ok()) die("A/b status", (long)s.error_code());
    return 1;
  }
  if (cell == 'B' || cell == 'C') {
    const char *path = dir == 'a' ? kFetchPath : kPush;
    std::string pbreq;
    const uint8_t *req = kNoReq;
    size_t reqn = 0;
    if (dir == 'b') {
      if (cell == 'B') {
        w.pb_req.SerializeToString(&pbreq);
        req = (const uint8_t *)pbreq.data();
        reqn = pbreq.size();
      } else {
        intptr_t rc = shapes::ffi::encode_into_list_tasks_detailed_response(ec, w.fac_req, shapes::ffi::tcs_core());
        if (rc < 0 || ak_enc_take(ec, &req, &reqn) != 0) die("C/b encode", (long)rc);
      }
    }
    struct ak_bytes out;
    out.ptr = NULL; out.len = 0; out.owner = NULL;
    int32_t rc = ak_call_unary(w.cl, (const uint8_t *)path, std::strlen(path), req, reqn, &out);
    if (rc != AK_OK) die("core call status", rc);
    long n = 1;
    if (dir == 'a') {
      if (out.len != w.expect_a) die("core/a response length", (long)out.len);
      if (cell == 'B') {
        svcns::ListTasksDetailedResponse m;
        if (!m.ParseFromArray(out.ptr, (int)out.len)) die("B/a decode", 0);
        n = m.tasks_size();
      } else {
        shapes::ListTasksDetailedResponse f;
        int32_t drc = shapes::ffi::decode_with_list_tasks_detailed_response(dc, out.ptr, out.len, &f);
        if (drc != 0) die("C/a decode", drc);
        n = (long)f.tasks.size();
      }
    } else if (out.len != 0) {
      die("core/b response length", (long)out.len);
    }
    ak_bytes_free(&out);
    return n;
  }
  // cell D: grpc++'s transport carrying opaque bytes, the core's codec at the client.
  grpc::internal::RpcMethod method(dir == 'a' ? kFetchPath : kPush, grpc::internal::RpcMethod::NORMAL_RPC);
  grpc::ClientContext ctx;
  grpc::ByteBuffer req, resp;
  if (dir == 'a') {
    grpc::Slice e;
    req = grpc::ByteBuffer(&e, 1);
  } else {
    const uint8_t *p = NULL;
    size_t n = 0;
    intptr_t rc = shapes::ffi::encode_into_list_tasks_detailed_response(ec, w.fac_req, shapes::ffi::tcs_core());
    if (rc < 0 || ak_enc_take(ec, &p, &n) != 0) die("D/b encode", (long)rc);
    grpc::Slice s(p, n);
    req = grpc::ByteBuffer(&s, 1);
  }
  grpc::Status s = grpc::internal::BlockingUnaryCall<grpc::ByteBuffer, grpc::ByteBuffer>(
      w.chan.get(), method, &ctx, req, &resp);
  if (!s.ok()) die("D status", (long)s.error_code());
  if (dir == 'b') {
    if (resp.Length() != 0) die("D/b response length", (long)resp.Length());
    return 1;
  }
  if (resp.Length() != w.expect_a) die("D/a response length", (long)resp.Length());
  std::vector<grpc::Slice> slices;
  if (!resp.Dump(&slices).ok()) die("D/a dump", 0);
  std::string flat;
  const uint8_t *p;
  size_t len;
  if (slices.size() == 1) {
    p = slices[0].begin();
    len = slices[0].size();
  } else {
    // The core's decoder needs one contiguous buffer; protobuf reads the slice list. The
    // concatenation is part of cell D (it is the integration's cost), stated.
    for (size_t i = 0; i < slices.size(); ++i) flat.append((const char *)slices[i].begin(), slices[i].size());
    p = (const uint8_t *)flat.data();
    len = flat.size();
  }
  shapes::ListTasksDetailedResponse f;
  int32_t drc = shapes::ffi::decode_with_list_tasks_detailed_response(dc, p, len, &f);
  if (drc != 0) die("D/a decode", drc);
  return (long)f.tasks.size();
}

// k threads, `total` calls between them, each thread a blocking caller.
long batch(World &w, char cell, char dir, int k, int total) {
  int per = (total + k - 1) / k;
  std::vector<std::thread> ts;
  std::vector<long> acc((size_t)k, 0);
  for (int t = 0; t < k; ++t) {
    ts.push_back(std::thread([&, t]() {
      ak_enc_ctx *ec = ak_enc_ctx_new();
      ak_dec_ctx *dc = ak_dec_ctx_new();
      long n = 0;
      for (int i = 0; i < per; ++i) n += cell_call(w, cell, dir, t, ec, dc);
      ak_enc_ctx_free(ec);
      ak_dec_ctx_free(dc);
      acc[(size_t)t] = n;
    }));
  }
  for (size_t i = 0; i < ts.size(); ++i) ts[i].join();
  long s = 0;
  for (size_t i = 0; i < acc.size(); ++i) s += acc[i];
  return s + (long)per * 0;
}

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
  }
  if (c.target.empty() || (c.transport != "shipped" && c.transport != "pinned") || !w.expect_a) {
    std::fprintf(stderr, "usage: campaign_rpc --target host:port --expect BYTES --transport shipped|pinned ...\n");
    return 2;
  }
  // grpc++
  w.chan = grpc::CreateCustomChannel(c.target, grpc::InsecureChannelCredentials(), channel_args(c.transport));
  int maxk = 1;
  for (int k : c.inflight) maxk = k > maxk ? k : maxk;
  for (int i = 0; i < maxk; ++i) w.stubs.emplace_back(svcns::Shapes::NewStub(w.chan));
  // the core's transport: `shipped` = the stack's defaults (NULL options; packages/cpp pins no
  // window, and ak_client_new's defaults are tonic/hyper's), `pinned` = 4 MiB stream and
  // connection windows, adaptive off, Nagle off, max message 2 MiB (rpc_common.h core_opts).
  w.rt = ak_runtime_new((uint32_t)c.workers);
  std::string uri = "http://" + c.target;
  if (c.transport == "pinned") {
    ak_client_opts o = pinned_core_opts(true);
    w.cl = ak_client_new_opts(w.rt, (const uint8_t *)uri.data(), uri.size(), &o);
  } else {
    w.cl = ak_client_new(w.rt, (const uint8_t *)uri.data(), uri.size());
  }
  if (!w.cl) die("ak_client_new", 0);
  pbbuild::payload_p2_2(&w.pb_req);
  w.fac_req = shapes::build::payload_p2_2();

  // Before any sample: every cell once per direction, and cell A's wire length checked.
  if (c.cells.find('A') != std::string::npos) {
    grpc::ClientContext ctx;
    svcns::Empty q;
    svcns::ListTasksDetailedResponse r;
    if (!w.stubs[0]->Fetch(&ctx, q, &r).ok() || r.ByteSizeLong() != w.expect_a)
      die("A/a wire length (pre-check)", (long)r.ByteSizeLong());
  }
  std::printf("# {\"campaign_rpc\": {\"target\": \"%s\", \"transport\": \"%s\", \"cells\": \"%s\","
              " \"dirs\": \"%s\", \"calls_per_sample\": %d, \"warmup_calls\": %d, \"rounds\": %d,"
              " \"core_workers\": %d, \"expect_bytes\": %zu, \"delivery_B_C\": \"blocking\"}}\n",
              c.target.c_str(), c.transport.c_str(), c.cells.c_str(), c.dirs.c_str(), c.calls,
              c.warmup, c.rounds, c.workers, w.expect_a);
  for (char d : c.dirs)
    for (int k : c.inflight)
      for (char cell : c.cells) batch(w, cell, d, k, c.warmup);  // warm-up, identical per cell

  for (int r = 0; r < c.rounds; ++r) {
    for (char d : c.dirs) {
      for (int k : c.inflight) {
        size_t nc = c.cells.size();
        for (size_t j = 0; j < nc; ++j) {
          char cell = c.cells[(j + (size_t)r) % nc];  // rotated order
          double c0 = rusage_ns(), w0 = wall_ns();
          batch(w, cell, d, k, c.calls);
          double c1 = rusage_ns(), w1 = wall_ns();
          int iters = ((c.calls + k - 1) / k) * k;
          std::printf("{\"slice\":\"cpp\",\"suite\":\"rpc\",\"cell\":\"%c\",\"payload\":\"P2.2\","
                      "\"dir\":\"%c\",\"transport\":\"%s\",\"inflight\":%d,\"launch\":%d,"
                      "\"round\":%d,\"cpu_ns\":%.0f,\"wall_ns\":%.0f,\"iters\":%d}\n",
                      cell, d, c.transport.c_str(), k, c.launch, r, c1 - c0, w1 - w0, iters);
          std::fflush(stdout);
        }
      }
    }
  }
  ak_client_destroy(w.cl);
  ak_runtime_destroy(w.rt);
  return 0;
}
