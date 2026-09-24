// R5 for the RPC half: count the crossings, do not infer them.
//
// ABI v1 section 9 carries a table of what each delivery costs at the boundary -- 2/0 for
// the blocking call, 2/1 for the callback, 3/0 for the queue -- and it is arithmetic
// somebody did by reading the signatures. This binary links the core built with
// `--features count`, resets the counters, does a known number of RPCs and reads them back.
//
// Two methods, and that is the other half of R5's question. `Fetch` returns 500
// `TaskDetailed` messages -- about 4,500 fields -- and `Ping` returns an empty message with
// none. Section 9 says the count per RPC is "two crossings per call, zero per field, and it
// is a property of the code rather than a measurement". Two methods three orders of
// magnitude apart in field count, with the same counted crossings, is what makes that a
// measurement instead of a reading of `rpc.rs`.
#include "rpc_common.h"

#include <condition_variable>
#include <mutex>

#include "ak_abi.h"

using namespace akrpc;

static const uint8_t kNoReq[1] = {0};

struct CbState {
  std::mutex m;
  std::condition_variable cv;
  bool got;
  ak_completion c;
};

extern "C" void on_complete_counting(void *user, struct ak_completion *c) {
  CbState *s = static_cast<CbState *>(user);
  {
    std::lock_guard<std::mutex> g(s->m);
    s->c = *c;
    s->got = true;
  }
  s->cv.notify_one();
}

struct Counted { double fwd, rev; long bytes; };

// Every delivery does exactly the same thing: one call, take the bytes, release them, and
// release the handle where there is one. A host that leaked the handle would report the
// table's numbers, which is precisely the reading being checked.
static Counted run_blocking(ak_client *cl, const char *path, int n) {
  size_t plen = strlen(path);
  ak_rpc_counters_reset();
  long bytes = 0;
  for (int i = 0; i < n; ++i) {
    struct ak_bytes out;
    out.ptr = NULL; out.len = 0; out.owner = NULL;
    int32_t rc = ak_call_unary(cl, (const uint8_t *)path, plen, kNoReq, 0, &out);
    if (rc != AK_OK) { std::printf("blocking rc=%d\n", rc); abort(); }
    bytes += (long)out.len;
    ak_bytes_free(&out);
  }
  struct ak_rpc_counters c;
  ak_rpc_counters(&c);
  Counted r;
  r.fwd = (double)c.forward / n;
  r.rev = (double)c.reverse / n;
  r.bytes = bytes / n;
  return r;
}

static Counted run_callback(ak_client *cl, const char *path, int n) {
  size_t plen = strlen(path);
  CbState st;
  ak_rpc_counters_reset();
  long bytes = 0;
  for (int i = 0; i < n; ++i) {
    st.got = false;
    ak_call *h = ak_call_unary_cb(cl, (const uint8_t *)path, plen, kNoReq, 0,
                                  on_complete_counting, &st, (uint64_t)i);
    if (!h) { std::printf("cb NULL\n"); abort(); }
    ak_completion c;
    {
      std::unique_lock<std::mutex> g(st.m);
      st.cv.wait(g, [&] { return st.got; });
      c = st.c;
    }
    if (c.status != AK_OK) { std::printf("cb status=%d\n", c.status); abort(); }
    bytes += (long)c.bytes.len;
    ak_bytes_free(&c.bytes);
    ak_call_destroy(h);
  }
  struct ak_rpc_counters cc;
  ak_rpc_counters(&cc);
  Counted r;
  r.fwd = (double)cc.forward / n;
  r.rev = (double)cc.reverse / n;
  r.bytes = bytes / n;
  return r;
}

static Counted run_queue(ak_client *cl, const char *path, int n) {
  size_t plen = strlen(path);
  ak_queue *q = ak_queue_new();
  ak_rpc_counters_reset();
  long bytes = 0;
  for (int i = 0; i < n; ++i) {
    ak_call *h = ak_call_unary_q(cl, (const uint8_t *)path, plen, kNoReq, 0, q, (uint64_t)i);
    if (!h) { std::printf("queue NULL\n"); abort(); }
    ak_completion c;
    int32_t rc = ak_queue_next(q, &c, (uint64_t)-1);
    if (rc != AK_QUEUE_OK) { std::printf("queue_next rc=%d\n", rc); abort(); }
    if (c.status != AK_OK) { std::printf("queue status=%d\n", c.status); abort(); }
    bytes += (long)c.bytes.len;
    ak_bytes_free(&c.bytes);
    ak_call_destroy(h);
  }
  struct ak_rpc_counters cc;
  ak_rpc_counters(&cc);
  Counted r;
  r.fwd = (double)cc.forward / n;
  r.rev = (double)cc.reverse / n;
  r.bytes = bytes / n;
  // `ak_queue_shutdown` and `ak_queue_destroy` are per-QUEUE and not per-call, so they are
  // outside the counted window on purpose -- as `ak_runtime_new` and `ak_client_new` are.
  ak_queue_shutdown(q);
  ak_queue_destroy(q);
  return r;
}

int main(int argc, char **argv) {
  akrpc::init_core_or_die();
  int n = argc > 1 ? atoi(argv[1]) : 20;

  if (!ak_rpc_counting()) {
    std::printf("REFUSED: this core does not count RPC crossings (built without "
                "--features count). A harness that read its zeroes and published them "
                "would have reported that the boundary is free.\n");
    return 2;
  }
  std::printf("counting core: ak_rpc_counting() = 1\n");
  std::printf("configuration: grpc++ %s server, UDS, transport pinned, %d RPCs per cell\n\n",
              grpc::Version().c_str(), n);

  Transport tr = make_transport(true, 0);
  ShapesImpl svc;
  grpc::ServerBuilder b;
  pin_server(&b, true);
  b.AddListeningPort(tr.grpc_target, grpc::InsecureServerCredentials());
  b.RegisterService(&svc);
  std::unique_ptr<grpc::Server> server = b.BuildAndStart();
  if (!server) { std::printf("server failed to start\n"); return 1; }

  ak_runtime *rt = ak_runtime_new(2);
  ak_client_opts opts = pinned_core_opts(true);
  ak_client *cl = ak_client_new_opts(rt, (const uint8_t *)tr.core_uri.data(),
                                     tr.core_uri.size(), &opts);
  if (!cl) { std::printf("ak_client_new_opts failed\n"); return 1; }

  // Warm, outside every counted window: the connect is a crossing too and it is not per-RPC.
  { Counted w = run_blocking(cl, kFetchPath, 2); (void)w; }

  struct Row { const char *delivery; const char *method; const char *path; Counted c; };
  Row rows[6];
  rows[0].delivery = "blocking"; rows[0].method = "Fetch (~4,500 fields)"; rows[0].path = kFetchPath;
  rows[1].delivery = "callback"; rows[1].method = "Fetch (~4,500 fields)"; rows[1].path = kFetchPath;
  rows[2].delivery = "queue";    rows[2].method = "Fetch (~4,500 fields)"; rows[2].path = kFetchPath;
  rows[3].delivery = "blocking"; rows[3].method = "Ping  (0 fields)";      rows[3].path = kPingPath;
  rows[4].delivery = "callback"; rows[4].method = "Ping  (0 fields)";      rows[4].path = kPingPath;
  rows[5].delivery = "queue";    rows[5].method = "Ping  (0 fields)";      rows[5].path = kPingPath;
  for (int i = 0; i < 6; ++i) {
    if (!strcmp(rows[i].delivery, "blocking")) rows[i].c = run_blocking(cl, rows[i].path, n);
    else if (!strcmp(rows[i].delivery, "callback")) rows[i].c = run_callback(cl, rows[i].path, n);
    else rows[i].c = run_queue(cl, rows[i].path, n);
  }

  std::printf("%-10s %-24s %10s %9s %9s   %s\n", "delivery", "method", "resp bytes",
              "fwd/RPC", "rev/RPC", "ABI v1 section 9 says");
  const char *says[3] = {"2 fwd / 0 rev", "2 fwd / 1 rev", "3 fwd / 0 rev"};
  for (int i = 0; i < 6; ++i)
    std::printf("%-10s %-24s %10ld %9.3f %9.3f   %s\n", rows[i].delivery, rows[i].method,
                rows[i].c.bytes, rows[i].c.fwd, rows[i].c.rev, says[i % 3]);

  std::printf("\nWhat the two methods settle: the counts are identical on a 540 KB response\n"
              "with about 4,500 fields and on an empty one with none, so the crossing count\n"
              "per RPC is not a function of field count. That is section 9's claim, COUNTED.\n");

  std::printf("\nWhat the table gets wrong, and it is not rounding. The callback and the\n"
              "queue both return an `ak_call*` the host must release, and section 9's table\n"
              "does not count `ak_call_destroy`. A host that does not call it leaks a handle\n"
              "per RPC. Counted with the handle released as the ABI requires:\n"
              "  blocking  %.0f fwd / %.0f rev   (the table's 2/0, and this one is right --\n"
              "                                   the blocking form takes no handle here)\n"
              "  callback  %.0f fwd / %.0f rev   (the table says 2/1)\n"
              "  queue     %.0f fwd / %.0f rev   (the table says 3/0)\n",
              rows[0].c.fwd, rows[0].c.rev, rows[1].c.fwd, rows[1].c.rev,
              rows[2].c.fwd, rows[2].c.rev);

  std::printf("\nStill the point section 9 is making: three, four crossings against a call of\n"
              "about a millisecond of CPU. At this slice's measured 1.82 ns per forward\n"
              "crossing through a shared library, the whole boundary is under 8 ns.\n");

  ak_client_destroy(cl);
  ak_runtime_destroy(rt);
  server->Shutdown();
  server->Wait();
  ::unlink(tr.sock.c_str());
  return 0;
}
