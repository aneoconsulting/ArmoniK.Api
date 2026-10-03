// design/CAMPAIGN.md section 4.2, the RPC grid's CLIENT: one process pinned by the runner to
// AK_CPU_CLIENT, talking to THE campaign server (WP10, req. 13 as amended: the Rust slice's
// tonic rpc_server, poc/rust/SERVER.md, started by poc/rust/serve.sh) in ANOTHER process
// pinned to AK_CPU_SERVER, over a Unix domain socket (req. 17: grpc++ and the core both dial
// `unix:<path>`). `--transport shipped|pinned` is this client's configuration, against the
// server's socket of the same name (tonic's defaults / 4 MiB windows, adaptive off).
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
//   delivery  (req. 16 as amended 2026-09-28): B, C and E (and framed twins) run in BOTH of
//                        C++'s idiomatic core deliveries: BLOCKING (the plain labels, "(blk)")
//                        and the COMPLETION QUEUE (B-q, C-q-*, E-q-*, Bf-q, Cf-q-*, Ef-q-*,
//                        "(q)"): one ak_queue per cell, the thread that issues a batch drains it
//                        (q_batch); A, D and F use grpc++'s synchronous call, which is what
//                        packages/cpp does (its clients call the generated stubs' blocking
//                        methods), stated.
//   --semantics 1    the queue forms' semantics against the server's test paths, then exit
//   directions (req. 14): a = empty request, P2.2 response, decode only; a+read = the same
//                        call, then every field read (generated traversal, touch.cpp);
//                        b = P2.2 request the server decodes, empty response
//   in flight  (req. 15): 1, 8, 16 -- k host threads, each with one blocking call out
//   transport  (req. 17): --transport shipped|pinned, applied to grpc++ AND to the core
//   channels   (req. 13): one channel (grpc++) or client (the core) PER CELL, opened at start
//                        and warmed by the warm-up before round 1
//   checks     (req. 18): every call: status OK and response length equal to the expected
//                        payload; the first failure aborts the process and leaves no sample
//   samples    (WP9, req. 22a amended): Google Benchmark. One benchmark per (cell, direction,
//                        payload, in-flight k); one iteration = one batch of k calls in flight
//                        (k operations); cpu_time = process CPU (MeasureProcessCPUTime; the
//                        server is another process), real_time = wall (UseRealTime); fixed
//                        iterations, `rounds` repetitions reported raw
//   rounds     (req. 22/23/24): repetitions randomly interleaved across every benchmark of the
//                        process, registration order rotated by launch; warm-up =
//                        --benchmark_min_warmup_time (--warmup-s); the JSON (--gbench-out) is
//                        renamed into place only if every call passed (R-H4)
//   threads    (R-H2, req. 4): the k caller threads are created once, before any timed
//                        window; the header records them, the core's runtime workers and the
//                        process's thread count after warm-up (grpc-core's own threads)
//
//   --count N        (req. 19, a counting build): per call, the crossings of cells B, C, D
//                    and E in each mode and direction, then exit
#include "rpc_common.h"

#include <arpa/inet.h>
#include <dirent.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <sys/socket.h>
#include <fcntl.h>
#include <time.h>
#include <dlfcn.h>
#include <sched.h>
#include <sys/resource.h>
#include <unistd.h>
#include <sys/syscall.h>

#include <algorithm>
#include <atomic>
#include <condition_variable>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <deque>
#include <fstream>
#include <map>
#include <set>
#include <sstream>
#include <cctype>
#include <mutex>
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
#include "bb_bytes.h"
#include "owned_holder.h"
#include "sha256.h"
#include "campaign_grid.grpc.pb.h"
#ifndef AK_COUNTING
#include <benchmark/benchmark.h>
#endif

using namespace akrpc;

// B-2 (2026-09-28): the fill the core arms' encodes use (the header names it). Sparse:
// decision 9's cleared groups and sparse assignment (encode_into_*_zeroed, *_unk_zeroed);
// -DAK_CORE_FILL_TOTAL: the total fill (encode_into_*, *_unk).
#ifndef AK_CORE_FILL_TOTAL
#define AK_FILL(fn) fn##_zeroed
#define AK_CORE_FILL "sparse (decision 9: encode_into_*_zeroed, encode_into_*_unk_zeroed)"
#else
#define AK_FILL(fn) fn
#define AK_CORE_FILL "total (encode_into_*, encode_into_*_unk)"
#endif

namespace {

// WP10 (req. 13 as amended): THE campaign server is the Rust slice's tonic rpc_server, service
// armonik.ffi.campaign.v1.Grid (poc/rust/SERVER.md).
namespace gridns = armonik::ffi::campaign::v1;
const char *const kFetch = "/armonik.ffi.campaign.v1.Grid/Fetch";
const char *const kPush = "/armonik.ffi.campaign.v1.Grid/Push";
const char *const kUpload = "/armonik.ffi.campaign.v1.Grid/Upload";
const char *const kUploadStream = "/armonik.ffi.campaign.v1.Grid/UploadStream";            // 8 B: count
const char *const kUploadStreamCheck = "/armonik.ffi.campaign.v1.Grid/UploadStreamCheck";  // 40 B: + SHA-256
typedef shapes::ListTasksDetailedResponse Fac;
typedef svcns::ListTasksDetailedResponse Pb;
typedef shapes::UploadResultDataMessage Fac5;   // M5, directions c and d
typedef svcns::UploadResultDataMessage Pb5;
const size_t kChunk = 2 * 1024 * 1024;          // ArmoniK's upload chunk (direction d)

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
  bool framed;  // B, C, E: the core's FRAMED send path (ak_client_set_framed), beside the reference
  bool pull = false;  // X-2: C and D decoding with the PULL family (labelled extra cells Cp-*, Dp-*,
                      // directions a and a+read only)
  bool q = false;     // req. 16 as amended 2026-09-28: B, C, E (and framed twins) on the core's
                      // COMPLETION-QUEUE delivery (B-q, C-q-drop, Cf-q-drop, ...), beside the blocking ones
  bool zc = false;    // EXPERIMENT (Rust patch p6-zero-copy): Cf-zc-*, direction d only: the chunk's data
                      // left in the host's memory (ak_enc_set_zc, 64 KiB threshold, a context of its
                      // own) and sent borrowed with ak_call_send_enc_zc; refused on a core without them
  int deferred = 0;   // EXPERIMENT (Rust patch p5-deferred): Cf-enc-* (1) and Cf-encp-* (2), direction d
                      // only: each chunk encoded BY THE TRANSPORT through ak_call_send_deferred (found
                      // with dlsym; a core without it refuses the cell), wait = 1 / wait = 0
  bool cb = false;    // owner 2026-10-01: the CALLBACK delivery (A-cb: grpc++'s callback API; Cf-cb: the
                      // core's ak_call_*_cb), beside the blocking and queue ones; directions c and d
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
      // Every send path that exists runs beside its reference (req. 14, ABI v1 section 9):
      // the core's cells B, C and E have a framed twin (Bf, Cf-*, Ef-*).
      const int twins = (c == 'B' || c == 'C' || c == 'E') ? 2 : 1;
      // Req. 16 as amended: B, C and E run in BOTH of C++'s idiomatic core deliveries,
      // blocking (the plain label) and the completion queue (`-q`), on each send path.
      const int dels = twins;
      if (c == 'C' || c == 'D') {  // X-2: the pull twins, labelled extra cells
        std::string base = std::string(1, c) + "p";
#ifdef AK_NO_UNKNOWN_FIELDS
        out.push_back(Cell{c, kNoUnk, base + "-nounk", false, true});
#else
        out.push_back(Cell{c, kRetain, base + "-retain", false, true});
        out.push_back(Cell{c, kDrop, base + "-drop", false, true});
#endif
      }
      for (int dq = 0; dq < dels; ++dq)
      for (int f = 0; f < twins; ++f) {
        std::string base = std::string(1, c) + (f ? "f" : "") + (dq ? "-q" : "");
        if (c == 'C' || c == 'D' || c == 'E' || c == 'F') {
#ifdef AK_NO_UNKNOWN_FIELDS
          out.push_back(Cell{c, kNoUnk, base + "-nounk", f == 1, false, dq == 1});
#else
          out.push_back(Cell{c, kRetain, base + "-retain", f == 1, false, dq == 1});
          out.push_back(Cell{c, kDrop, base + "-drop", f == 1, false, dq == 1});
#endif
        } else {
          out.push_back(Cell{c, kDefault, base, f == 1, false, dq == 1});
        }
      }
    }
    return out;
  }
  size_t p = 0;
  while (p <= spec.size()) {
    size_t q = spec.find(',', p);
    std::string l = spec.substr(p, q == std::string::npos ? std::string::npos : q - p);
    if (!l.empty()) {
      const bool fr = l.size() > 1 && l[1] == 'f';
      const bool pu = l.size() > 1 && l[1] == 'p';
      std::string rest = l.substr((fr || pu) ? 2 : 1);
      const bool qq = rest.compare(0, 2, "-q") == 0 && (rest.size() == 2 || rest[2] == '-');
      if (qq) rest = rest.substr(2);
      const bool cbk = !qq && rest.compare(0, 3, "-cb") == 0 && (rest.size() == 3 || rest[3] == '-');
      if (cbk) rest = rest.substr(3);
      int df = 0;
      if (rest.compare(0, 5, "-encp") == 0 && (rest.size() == 5 || rest[5] == '-')) { df = 2; rest = rest.substr(5); }
      else if (rest.compare(0, 4, "-enc") == 0 && (rest.size() == 4 || rest[4] == '-')) { df = 1; rest = rest.substr(4); }
      Mode m = rest.empty() ? kDefault
               : rest == "-retain" ? kRetain
               : rest == "-nounk" ? kNoUnk : kDrop;
      bool zc = false;
      if (!df && (rest.compare(0, 4, "-zcp") == 0 || rest.compare(0, 4, "-zcw") == 0) && (rest.size() == 4 || rest[4] == '-')) {
        zc = true; df = rest[3] == 'w' ? 1 : 2; rest = rest.substr(4);  // Cf-zcw (wait = 1), Cf-zcp (wait = 0)
      } else if (!df && rest.compare(0, 3, "-zc") == 0 && (rest.size() == 3 || rest[3] == '-')) { zc = true; rest = rest.substr(3); }
      if (zc || df) m = rest.empty() ? kDefault : rest == "-retain" ? kRetain : rest == "-nounk" ? kNoUnk : kDrop;
      Cell cl{l[0], m, l, fr, pu, qq};
      cl.cb = cbk;
      cl.deferred = df;
      cl.zc = zc;
      out.push_back(cl);
    }
    if (q == std::string::npos) break;
    p = q + 1;
  }
  return out;
}

struct Cfg {
  std::string target, transport = "shipped", cells = "ABCDEF", dirs = "arbcd";
  std::string core_target;  // --core-target URI: the core client's endpoint when it differs from grpc++'s
                            // target (TCP: grpc++ `ipv4:127.0.0.1:P`, the core `http://127.0.0.1:P`)
  std::string plant;     // test only (req. 18 controls): c-len | d-count | d-sha
  std::vector<int> inflight = {1, 8, 16};
  int launch = 0, rounds = 5, calls = 96, workers = 8;  // D14: AK_WORKERS, default 8
  int fail_after = -1;   // test only: abort after this many samples (the gate's R-H4 control)
  double warmup_s = 0.5; // Google Benchmark's min warm-up time per benchmark (req. 24)
  double min_time_s = 0.5; // Google Benchmark's min time per repetition (its iteration control)
  std::string gbout;     // Google Benchmark JSON output (WP9)
  int count = 0;         // --count N
  int alloc_probe = 0;   // --alloc-probe N: allocations >= 1 MiB per call (LD_PRELOAD gen/allocprobe.so)
  int semantics = 0;     // --semantics 1: the queue deliveries' semantics check (no timing), then exit
  std::string payloads;  // --payloads P5.4,16MiB,...: only these (direction, payload) jobs (empty: all)
  int profile = 0;       // --profile N: N batches of the one cell and job, no Google Benchmark (profiling)
  std::string profile_cell;  // --profile-cell LABEL: the cell profiled (default: the first cell)
  std::string perf_ctl;  // --perf-ctl CTL_FIFO,ACK_FIFO: perf's --control fifos, enabled around the loop
  int profile_chunks = 10;  // --profile-chunks C: the loop's per-chunk CPU and wall
  std::string iters;     // --iters [PAYLOAD/]K:N,...: Google Benchmark's fixed Iterations(N) per in-flight
                         // K, or per (payload, K) (P5.4/8:12); no iteration estimation, so --min-time-s
                         // is unused for those benchmarks
};

[[noreturn]] void die(const char *what, long v) {
  std::fprintf(stderr, "CALL CHECK FAILED: %s (%ld) -- the run is aborted, no figure\n", what, v);
  std::fflush(stdout);
  std::_Exit(3);
}

// A grpc++ stream write that failed: the call has ended, and its status says why.
template <typename W>
[[noreturn]] void die_write(W &wr, const char *what, long i) {
  grpc::Status s = wr->Finish();
  std::fprintf(stderr, "stream write %ld failed: gRPC status %d \"%s\"\n", i, (int)s.error_code(),
               s.error_message().c_str());
  die(what, i);
}

int proc_threads() {
  std::ifstream f("/proc/self/status");
  std::string line;
  while (std::getline(f, line))
    if (line.compare(0, 8, "Threads:") == 0) return std::atoi(line.c_str() + 8);
  return -1;
}

// The process's threads by name (/proc/self/task/*/comm), as a JSON object: what each stack
// actually runs (grpc-core's pollers and executor, the core's tokio workers, the callers).
std::string thread_classes() {
  std::map<std::string, int> n;
  if (DIR *d = opendir("/proc/self/task")) {
    while (struct dirent *e = readdir(d)) {
      if (e->d_name[0] == '.') continue;
      std::ifstream f(std::string("/proc/self/task/") + e->d_name + "/comm");
      std::string c;
      std::getline(f, c);
      std::string k;
      for (char ch : c) k += (ch == '"' || ch == '\\') ? '_' : ch;
      ++n[k];
    }
    closedir(d);
  }
  std::string o = "{";
  for (std::map<std::string, int>::const_iterator i = n.begin(); i != n.end(); ++i)
    o += (o.size() > 1 ? ", \"" : "\"") + i->first + "\": " + std::to_string(i->second);
  return o + "}";
}

// Interrupt time on a CPU set, for the kernel's CONFIG_IRQ_TIME_ACCOUNTING=y: softirq and hardirq
// time is then charged to no task (CLOCK_PROCESS_CPUTIME_ID and getrusage miss it), so a loopback
// TCP receive run in softirq inside a sender's write is visible only here. Per CPU of the set:
// /proc/stat's irq and softirq fields (USER_HZ ticks, accumulated from ns) and /proc/softirqs'
// NET_RX and NET_TX counts. Summed over the set.
struct IrqSnap { unsigned long long irq = 0, softirq = 0, net_rx = 0, net_tx = 0; };
std::set<int> cpu_list(const std::string &s) {
  std::set<int> o;
  std::stringstream ss(s);
  std::string part;
  while (std::getline(ss, part, ',')) {
    if (part.empty()) continue;
    const size_t d = part.find('-');
    const int a = std::atoi(part.c_str()), b = d == std::string::npos ? a : std::atoi(part.c_str() + d + 1);
    for (int i = a; i <= b; ++i) o.insert(i);
  }
  return o;
}
std::set<int> self_cpus() {
  std::set<int> o;
  cpu_set_t m;
  CPU_ZERO(&m);
  if (sched_getaffinity(0, sizeof m, &m) == 0)
    for (int i = 0; i < CPU_SETSIZE; ++i)
      if (CPU_ISSET(i, &m)) o.insert(i);
  return o;
}
std::set<int> pid_cpus(long pid) {
  std::ifstream f("/proc/" + std::to_string(pid) + "/status");
  std::string l;
  while (std::getline(f, l))
    if (l.compare(0, 19, "Cpus_allowed_list:\t") == 0) return cpu_list(l.substr(19));
  return std::set<int>();
}
IrqSnap irq_snap(const std::set<int> &cpus) {
  IrqSnap r;
  std::ifstream st("/proc/stat");
  std::string l;
  while (std::getline(st, l)) {
    if (l.compare(0, 3, "cpu") != 0 || l.size() < 4 || !std::isdigit((unsigned char)l[3])) continue;
    std::istringstream is(l.substr(3));
    int cpu;
    unsigned long long user, nice, sys, idle, iow, irq, sirq;
    if (!(is >> cpu >> user >> nice >> sys >> idle >> iow >> irq >> sirq) || !cpus.count(cpu)) continue;
    r.irq += irq;
    r.softirq += sirq;
  }
  std::ifstream si("/proc/softirqs");
  std::getline(si, l);  // header: CPU0 CPU1 ...
  std::vector<int> col;
  {
    std::istringstream is(l);
    std::string h;
    while (is >> h) col.push_back(std::atoi(h.c_str() + 3));
  }
  while (std::getline(si, l)) {
    std::istringstream is(l);
    std::string name;
    is >> name;
    unsigned long long *dst = name == "NET_RX:" ? &r.net_rx : name == "NET_TX:" ? &r.net_tx : nullptr;
    if (!dst) continue;
    unsigned long long v;
    for (size_t i = 0; i < col.size() && (is >> v); ++i)
      if (cpus.count(col[i])) *dst += v;
  }
  return r;
}
std::string irq_delta(const char *name, const IrqSnap &a, const IrqSnap &b) {
  char t[200];
  std::snprintf(t, sizeof t, "\"%s\": {\"irq_ticks\": %llu, \"softirq_ticks\": %llu, \"net_rx\": %llu, \"net_tx\": %llu}", name,
                b.irq - a.irq, b.softirq - a.softirq, b.net_rx - a.net_rx, b.net_tx - a.net_tx);
  return t;
}

// The process's TCP sockets (fds from /proc/self/fd), each with its ports and TCP_NODELAY read back
// with getsockopt on the live socket: what grpc-core and the core's tonic client actually set.
std::string tcp_sockets() {
  std::string o = "[";
  int on = 0, all = 0;
  if (DIR *d = opendir("/proc/self/fd")) {
    while (struct dirent *e = readdir(d)) {
      if (e->d_name[0] == '.') continue;
      const int fd = std::atoi(e->d_name);
      int dom = 0;
      socklen_t l = sizeof dom;
      if (getsockopt(fd, SOL_SOCKET, SO_DOMAIN, &dom, &l) != 0 || (dom != AF_INET && dom != AF_INET6)) continue;
      int ty = 0;
      l = sizeof ty;
      if (getsockopt(fd, SOL_SOCKET, SO_TYPE, &ty, &l) != 0 || ty != SOCK_STREAM) continue;
      int nd = -1;
      l = sizeof nd;
      getsockopt(fd, IPPROTO_TCP, TCP_NODELAY, &nd, &l);
      struct sockaddr_storage la, pa;
      socklen_t ll = sizeof la, pl = sizeof pa;
      int lport = 0, pport = 0;
      // grpc-core dials an AF_INET6 socket (a v4-mapped address) for an ipv4: target
      auto port_of = [](const struct sockaddr_storage &a) {
        return a.ss_family == AF_INET ? (int)ntohs(((const struct sockaddr_in *)&a)->sin_port)
             : a.ss_family == AF_INET6 ? (int)ntohs(((const struct sockaddr_in6 *)&a)->sin6_port) : 0;
      };
      if (getsockname(fd, (struct sockaddr *)&la, &ll) == 0) lport = port_of(la);
      if (getpeername(fd, (struct sockaddr *)&pa, &pl) == 0) pport = port_of(pa);
      char b[120];
      std::snprintf(b, sizeof b, "%s{\"fd\": %d, \"family\": %d, \"local_port\": %d, \"peer_port\": %d, \"nodelay\": %d}", all ? ", " : "", fd, dom == AF_INET6 ? 6 : 4, lport, pport, nd);
      o += b;
      ++all;
      if (nd > 0) ++on;
    }
    closedir(d);
  }
  char t[80];
  std::snprintf(t, sizeof t, "], \"tcp_nodelay_on\": %d, \"tcp_sockets_n\": %d", on, all);
  return o + t;
}

// The CPU facts each stack sizes itself from: the affinity mask (what taskset set) and the
// two sysconf counts (grpc-core's gpr_cpu_num_cores is sysconf(_SC_NPROCESSORS_CONF) in
// v1.80, src/core/util/linux/cpu.cc; its EventEngine reserves Clamp(that, 4, 16) threads,
// posix_engine.h).
std::string cpu_facts() {
  cpu_set_t m;
  CPU_ZERO(&m);
  std::string list;
  int cnt = -1;
  if (sched_getaffinity(0, sizeof(m), &m) == 0) {
    cnt = CPU_COUNT(&m);
    for (int i = 0; i < CPU_SETSIZE; ++i)
      if (CPU_ISSET(i, &m)) list += (list.empty() ? "" : ",") + std::to_string(i);
  }
  return "{\"affinity\": \"" + list + "\", \"affinity_count\": " + std::to_string(cnt) +
         ", \"sysconf_nprocessors_conf\": " + std::to_string(sysconf(_SC_NPROCESSORS_CONF)) +
         ", \"sysconf_nprocessors_onln\": " + std::to_string(sysconf(_SC_NPROCESSORS_ONLN)) +
         ", \"grpcpp_version\": \"" + grpc::Version() + "\"}";
}

// ---- --profile (physical probe step 4a): per-thread CPU from /proc/self/task/*/schedstat ----
// (the first field: nanoseconds on a CPU), classed by thread name; the main thread is "main",
// the other campaign_rpc threads the "callers".
// --profile with AK_SERVER_PID: the server process's threads, schedstat (on-CPU ns, run-queue wait ns)
// per tid, read from /proc (same user; no ptrace)
std::map<long, std::pair<unsigned long long, unsigned long long> > server_times(long pid) {
  std::map<long, std::pair<unsigned long long, unsigned long long> > m;
  if (pid <= 0) return m;
  const std::string dir = "/proc/" + std::to_string(pid) + "/task";
  if (DIR *d = opendir(dir.c_str())) {
    while (struct dirent *e = readdir(d)) {
      if (e->d_name[0] == '.') continue;
      std::ifstream fs(dir + "/" + e->d_name + "/schedstat");
      unsigned long long a = 0, b = 0;
      if (fs >> a >> b) m[std::atol(e->d_name)] = std::make_pair(a, b);
    }
    closedir(d);
  }
  return m;
}
std::map<long, std::pair<unsigned long long, unsigned long long> > g_csw;  // voluntary, involuntary per tid
std::map<long, unsigned long long> g_wait_ns;  // schedstat's second field (ns waiting on a run queue), per tid
std::map<long, std::pair<std::string, unsigned long long> > thread_times() {
  std::map<long, std::pair<std::string, unsigned long long> > m;
  const long self = (long)getpid();
  if (DIR *d = opendir("/proc/self/task")) {
    while (struct dirent *e = readdir(d)) {
      if (e->d_name[0] == '.') continue;
      const std::string base = std::string("/proc/self/task/") + e->d_name;
      std::ifstream fc(base + "/comm"), fs(base + "/schedstat");
      std::string comm;
      std::getline(fc, comm);
      unsigned long long ns = 0, waitns = 0;
      fs >> ns >> waitns;
      g_wait_ns[std::atol(e->d_name)] = waitns;
      const long tid = std::atol(e->d_name);
      {
        std::ifstream st(base + "/status");
        std::string line;
        unsigned long long vv = 0, nv = 0;
        while (std::getline(st, line)) {
          if (line.compare(0, 24, "voluntary_ctxt_switches:") == 0) vv = std::strtoull(line.c_str() + 24, NULL, 10);
          else if (line.compare(0, 27, "nonvoluntary_ctxt_switches:") == 0) nv = std::strtoull(line.c_str() + 27, NULL, 10);
        }
        g_csw[tid] = std::make_pair(vv, nv);
      }
      if (tid == self) comm = "main";
      else if (comm == "campaign_rpc") comm = "caller";
      m[tid] = std::make_pair(comm, ns);
    }
    closedir(d);
  }
  return m;
}
double now_ns(clockid_t id) {
  struct timespec t;
  clock_gettime(id, &t);
  return (double)t.tv_sec * 1e9 + (double)t.tv_nsec;
}

const char *dir_label(char d) {
  return d == 'a' ? "a" : d == 'r' ? "a+read" : d == 'b' ? "b" : d == 'c' ? "c" : "d";
}

// A (direction, payload) pair: a, a+read and b carry P2.2; c carries P5.3 or P5.4 (pi 0/1);
// d streams 4 MiB or 16 MiB (pi 0/1) in 2 MiB chunks.
struct Job {
  char dir;
  int pi;
};
const char *job_payload(const Job &j) {
  if (j.dir == 'c') return j.pi ? "P5.4" : "P5.3";
  if (j.dir == 'd') return j.pi ? "16MiB" : "4MiB";
  return "P2.2";
}

// Direction d's upload (req. 14): M5 messages of 2 MiB of deterministic data (splitmix64, as
// the Rust slice), the ids on the first only; the SHA-256 of every message's wire bytes.
struct Stream {
  std::vector<Pb5> p;
  std::vector<Fac5> f;
  std::vector<std::string> wire;
  uint64_t bytes = 0;
  std::string sha;
};

// ---- one connection per cell (req. 13) ------------------------------------------------
struct Conn {
  std::shared_ptr<grpc::Channel> chan;
  std::vector<std::unique_ptr<gridns::Grid::Stub> > stubs;
  ak_client *cl = nullptr;
  ak_queue *q = nullptr;  // the queue cells' completion queue: ONE PER CELL, drained by the issuing thread
  // H-4 (2026-09-28): the raw methods of cells A (d), D and F, built ONCE with their channel,
  // so every call is a registered call, as the generated stub's methods are (before: an
  // RpcMethod without a channel per call, grpc-core's unregistered path).
  std::unique_ptr<grpc::internal::RpcMethod> m_fetch, m_push, m_upload, m_stream, m_stream_check;
  // A-q: grpc++'s CompletionQueue, one per cell, drained by the issuing thread (as the core's queue)
  std::unique_ptr<grpc::CompletionQueue> gcq;
  // A-cb: the previous batch's contexts and reactors, released at the start of the next batch (a
  // callback may still be on its way out of the library when the waiting caller wakes)
  std::vector<std::shared_ptr<void> > keep;
};

struct World {
  Cfg cfg;
  ak_runtime *rt = nullptr;
  std::vector<Cell> cells;
  std::vector<Conn> conns;   // parallel to cells
  size_t expect_a = 0;       // the P2.2 response length (from the server)
  Pb pb_req;                 // direction b's request, incumbent object
  Fac fac_req;               // the same, facade object
  Pb5 pb_up[2];              // direction c's requests (P5.3, P5.4)
  Fac5 fac_up[2];
  std::string wire_up[2];    // their protobuf wire, the pre-check's reference
  Stream st[2];              // direction d's uploads (4 MiB, 16 MiB)
  std::vector<Job> jobs;
  size_t want_c_len = 0;     // direction c's response length (0; 1 under the c-len plant)
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
  // The :authority of a `unix:` target: gRPC v1.51 and v1.54 send "localhost" (their sockaddr
  // resolver's GetDefaultAuthority); v1.80 sends the socket path percent-encoded
  // (resolver_factory.h's default), which the shared tonic server resets with RST_STREAM
  // PROTOCOL_ERROR on every call (logs/cpp/opt/physical-probe/checks/authority.log). Set to
  // "localhost" so every grpc++ version sends what v1.54 (ArmoniK's) sends.
  a.SetString(GRPC_ARG_DEFAULT_AUTHORITY, "localhost");
  return a;
}

ak_client *core_client_ref(ak_runtime *rt, const std::string &target, const std::string &transport) {
  // the core's transport: `shipped` = the stack's defaults (NULL options; packages/cpp pins no
  // window, and ak_client_new's defaults are tonic/hyper's), `pinned` = 4 MiB stream and
  // connection windows, adaptive off, Nagle off, max message 8 MiB (rpc_common.h core_opts).
  // `shipped` keeps tonic's limits (receive 4 MiB, send unlimited), enforced (D44): every
  // response is at most 540 KB, every request at most 4,194,390 B.
  // The target is `unix:<path>`, which the core dials as a Unix domain socket.
  if (transport == "pinned") {
    ak_client_opts o = pinned_core_opts(true);
    return ak_client_new_opts(rt, (const uint8_t *)target.data(), target.size(), &o);
  }
  return ak_client_new(rt, (const uint8_t *)target.data(), target.size());
}

// A cell's core client, on the reference or the framed send path (ak_client_set_framed: the
// request message sent as one body frame, its 5-byte prefix in the encode context's headroom;
// section 9). The framed path is the core's default (2026-09-28), so the path is set
// explicitly on EVERY core client, 0 for the reference cells (B, C, E and their -q forms), 1
// for the framed ones (Bf, Cf-*, Ef-* and their -q forms), as the Rust grid does (FIX-PLAN WP8
// item 6).
ak_client *core_client(ak_runtime *rt, const std::string &target, const std::string &transport,
                       bool framed = false) {
  ak_client *cl = core_client_ref(rt, target, transport);
  if (cl && ak_client_set_framed(cl, framed ? 1 : 0) != AK_OK) die("ak_client_set_framed", framed ? 1 : 0);
  return cl;
}

// Per caller thread: its encode context, its root-bound decode context, host-gen's encoders.
struct ThreadCtx {
  ak_enc_ctx *ec = nullptr;
  ak_dec_ctx *dc = nullptr;
  ak_dec_ctx *dcr = nullptr;  // retain decodes: its own context, left armed (rule 7), so a drop
                              // decode on `dc` never pays a disarming reset
  ak::Enc *ne = nullptr, *nre = nullptr;
  ak_enc_ctx *zec = nullptr;  // EXPERIMENT p6: cell Cf-zc's own zero-copy encode context (created on first use)
  std::atomic<long> zc_sent{0}, zc_released{0};  // Cf-zc: messages sent borrowing, and released
  std::vector<uint8_t> pbuf;  // H-6: cell B's protobuf request buffer, reused, grown only
  BBFlat bbf;                 // R-2: D and F's response bytes, one slice in place or a reused copy
  ThreadCtx() {
    ec = ak_enc_ctx_new();
    dc = shapes::ffi::dec_ctx_new_for<Fac>();  // decision 11 rule 6
    dcr = shapes::ffi::dec_ctx_new_for<Fac>();
    ne = new ak::Enc(shapes::native::kSites);
#ifndef AK_NO_UNKNOWN_FIELDS
    nre = new ak::Enc(shapes::native_retain::kSites);
#endif
  }
  ~ThreadCtx() {
    ak_enc_ctx_free(ec);
    if (zec) ak_enc_ctx_free(zec);
    shapes::ffi::dec_ctx_free(dc);
    shapes::ffi::dec_ctx_free(dcr);
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
    intptr_t rc = shapes::ffi::AK_FILL(encode_into_list_tasks_detailed_response)(tc.ec, v, shapes::ffi::tcs_core());
#else
    intptr_t rc = m == kRetain
                      ? shapes::ffi::AK_FILL(encode_into_list_tasks_detailed_response_unk)(tc.ec, v, shapes::ffi::tcs_core())
                      : shapes::ffi::AK_FILL(encode_into_list_tasks_detailed_response)(tc.ec, v, shapes::ffi::tcs_core());
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

int32_t codec_decode(char base, Mode m, ThreadCtx &tc, const uint8_t *p, size_t n, Fac *f, bool pull = false) {
  if (core_codec(base) && pull) {  // X-2: the pull family, walked in place
#ifdef AK_NO_UNKNOWN_FIELDS
    (void)m;
    return shapes::ffi::pull_with_list_tasks_detailed_response(tc.dc, p, n, f);
#else
    return m == kRetain ? shapes::ffi::pull_with_list_tasks_detailed_response_unk(tc.dcr, p, n, f)
                        : shapes::ffi::pull_with_list_tasks_detailed_response(tc.dc, p, n, f);
#endif
  }
  if (core_codec(base)) {
#ifdef AK_NO_UNKNOWN_FIELDS
    (void)m;
    return shapes::ffi::decode_with_list_tasks_detailed_response(tc.dc, p, n, f);
#else
    return m == kRetain ? shapes::ffi::decode_with_list_tasks_detailed_response_unk(tc.dcr, p, n, f)
                        : shapes::ffi::decode_with_list_tasks_detailed_response(tc.dc, p, n, f);
#endif
  }
#ifndef AK_NO_UNKNOWN_FIELDS
  if (m == kRetain) return shapes::native_retain::decode_list_tasks_detailed_response(p, n, f);
#endif
  (void)m;
  return shapes::native::decode_list_tasks_detailed_response(p, n, f);
}

// The client-side encoders, per message type and mode, for the request directions b, c, d.
intptr_t core_enc(ak_enc_ctx *ec, const Fac &v, Mode m) {
#ifdef AK_NO_UNKNOWN_FIELDS
  (void)m;
  return shapes::ffi::AK_FILL(encode_into_list_tasks_detailed_response)(ec, v, shapes::ffi::tcs_core());
#else
  return m == kRetain ? shapes::ffi::AK_FILL(encode_into_list_tasks_detailed_response_unk)(ec, v, shapes::ffi::tcs_core())
                      : shapes::ffi::AK_FILL(encode_into_list_tasks_detailed_response)(ec, v, shapes::ffi::tcs_core());
#endif
}
intptr_t core_enc(ak_enc_ctx *ec, const Fac5 &v, Mode m) {
#ifdef AK_NO_UNKNOWN_FIELDS
  (void)m;
  return shapes::ffi::AK_FILL(encode_into_upload_result_data_message)(ec, v, shapes::ffi::tcs_core());
#else
  return m == kRetain ? shapes::ffi::AK_FILL(encode_into_upload_result_data_message_unk)(ec, v, shapes::ffi::tcs_core())
                      : shapes::ffi::AK_FILL(encode_into_upload_result_data_message)(ec, v, shapes::ffi::tcs_core());
#endif
}
ak::Enc *hg_enc(ThreadCtx &tc, const Fac &v, Mode m) {
#ifndef AK_NO_UNKNOWN_FIELDS
  if (m == kRetain) { shapes::native_retain::encode_into_list_tasks_detailed_response(v, tc.nre); return tc.nre; }
#endif
  (void)m;
  shapes::native::encode_into_list_tasks_detailed_response(v, tc.ne);
  return tc.ne;
}
ak::Enc *hg_enc(ThreadCtx &tc, const Fac5 &v, Mode m) {
#ifndef AK_NO_UNKNOWN_FIELDS
  if (m == kRetain) { shapes::native_retain::encode_into_upload_result_data_message(v, tc.nre); return tc.nre; }
#endif
  (void)m;
  shapes::native::encode_into_upload_result_data_message(v, tc.ne);
  return tc.ne;
}

// Cells D and F hand their encoded bytes to grpc++ MOVED, not copied (WP8): D through
// ak_enc_take_owned (the core's buffer, released with ak_bytes_free when grpc++ drops the
// slice), F through ak::Enc::take (host-gen's buffer). The same as the Rust slice's D and F.
grpc::ByteBuffer core_owned_buffer(ThreadCtx &tc, const char *what) {
  ak_bytes *b = akhold::get();  // B-8: a recycled holder
  int32_t rc = ak_enc_take_owned(tc.ec, b);
  if (rc != AK_OK) die(what, rc);
  grpc::Slice sl(const_cast<uint8_t *>(b->ptr), b->len, akhold::free_owned, b);
  return grpc::ByteBuffer(&sl, 1);
}
grpc::ByteBuffer hg_owned_buffer(ak::Enc *e, const char *what) {
  if (e->err != 0) die(what, e->err);
  ak::Enc::Owned *o = e->take();
  grpc::Slice sl(o->v.empty() ? NULL : &o->v[0], o->len, ak::Enc::release, o);
  return grpc::ByteBuffer(&sl, 1);
}
// The request of a D or F cell, as grpc++ will carry it.
template <class V>
grpc::ByteBuffer grpc_request(char cell, Mode m, ThreadCtx &tc, const V &v) {
  if (core_codec(cell)) {
    intptr_t rc = core_enc(tc.ec, v, m);
    if (rc < 0) die("D encode", (long)rc);
    return core_owned_buffer(tc, "D ak_enc_take_owned");
  }
  return hg_owned_buffer(hg_enc(tc, v, m), "F encode");
}

// H-6: a protobuf message serialised into a reused buffer; returns its length.
template <class P>
size_t pb_into(const P &m, std::vector<uint8_t> &buf) {
  const size_t n = m.ByteSizeLong();
  if (buf.size() < n) buf.resize(n);
  m.SerializeWithCachedSizesToArray(buf.data());
  return n;
}

std::string flatten(const grpc::ByteBuffer &bb) {
  std::vector<grpc::Slice> slices;
  if (!bb.Dump(&slices).ok()) die("response dump", 0);
  std::string s;
  for (size_t i = 0; i < slices.size(); ++i) s.append((const char *)slices[i].begin(), slices[i].size());
  return s;
}

// A request of direction b or c on the core's transport (B, C, E and their framed twins):
// B encodes with protobuf and copies (ak_call_unary); C hands the encode context over
// (ak_call_unary_enc: the buffer MOVED); E copies host-gen's bytes (ak_call_unary). The
// response must be `want` bytes (0).
template <class V, class P>
void core_unary_req(const Cell &cl, Conn &cn, ThreadCtx &tc, const char *path, const V &fv, const P &pv,
                    size_t want) {
  struct ak_bytes out;
  out.ptr = NULL; out.len = 0; out.owner = NULL;
  int32_t gs = -1;  // the gRPC status (ABI v1 section 9); non-OK is AK_ERR_RPC_STATUS
  int32_t rc;
  const size_t pl = std::strlen(path);
  if (cl.base == 'B') {
    // H-6 (2026-09-28): ByteSizeLong + SerializeWithCachedSizesToArray into the thread's reused
    // buffer (no fresh string, no zero-fill once it is large enough).
    const size_t qn = pb_into(pv, tc.pbuf);
    rc = ak_call_unary(cn.cl, (const uint8_t *)path, pl, tc.pbuf.data(), qn, &out, &gs);
  } else if (cl.base == 'C') {
    intptr_t e = core_enc(tc.ec, fv, cl.mode);
    if (e < 0) die("C encode", (long)e);
    rc = ak_call_unary_enc(cn.cl, (const uint8_t *)path, pl, tc.ec, &out, &gs);
  } else {
    ak::Enc *e = hg_enc(tc, fv, cl.mode);
    if (e->err != 0) die("E encode", e->err);
    rc = ak_call_unary(cn.cl, (const uint8_t *)path, pl, e->data(), e->size(), &out, &gs);
  }
  if (rc != AK_OK || gs != 0) die(rc == AK_ERR_RPC_STATUS ? "core call gRPC status" : "core call status",
                                  rc == AK_ERR_RPC_STATUS ? gs : rc);
  if (out.len != want) die("core request response length", (long)out.len);
  ak_bytes_free(&out);
}

// Direction d's answer (SERVER.md): UploadStream, 8 bytes, the data byte count (u64 LE);
// UploadStreamCheck, 40 bytes, the count then the SHA-256 of every message as received.
uint64_t le64(const uint8_t *p) {
  uint64_t v = 0;
  for (int i = 7; i >= 0; --i) v = (v << 8) | p[i];
  return v;
}
void check_answer(const Stream &st, const uint8_t *p, size_t n, bool check) {
  if (n != (check ? 40u : 8u)) die("d: the server's answer length", (long)n);
  if (le64(p) != st.bytes) die("d: the server's byte count", (long)le64(p));
  if (check && std::string((const char *)p + 8, 32) != st.sha) die("d: the server's SHA-256 differs from the upload's", 0);
}

// ---- the completion-queue delivery (req. 16 as amended 2026-09-28) -----------------------
// C++'s analogue of grpc++'s CompletionQueue::Next loop: every queue cell has ONE ak_queue
// (per cell, so one cell's completions never reach another's drain), and the thread that
// issues a batch drains it -- ONE drainer thread per batch, the issuing thread itself, no
// thread of its own and no k caller threads. A batch of k calls in flight: the k requests
// are issued back to back (ak_call_unary_q, C: ak_call_unary_enc_q, the request moved as the
// blocking C moves it with ak_call_unary_enc; d: ak_call_open + ak_call_send_q /
// ak_call_send_enc_q, then ak_call_recv_q), then ak_queue_next is called until all k have
// completed, each completion matched to its call by its tag (slot << 8 | operation) and
// checked, its response decoded on the draining thread; a stream's next send is issued from
// the drain when its previous send has completed (at most one pending send per call).
enum { kOpUnary = 1, kOpSend = 2, kOpRecv = 3, kOpStart = 4, kOpWritesDone = 5 };
bool g_check_stream = false;  // --check-stream 1: every d call to UploadStreamCheck (count and SHA-256)
const int kMaxQ = 64;
const uint64_t kQWaitMs = 60000;  // a completion missing for a minute is a failed check

// Cell a's response on the core's transport, checked and decoded (B: protobuf, C/E: their codec).
long core_a_answer(World &w, const Cell &cl, ThreadCtx &tc, const uint8_t *p, size_t n, bool read) {
  if (n != w.expect_a) die("core/a response length", (long)n);
  if (cl.base == 'B') {
    Pb m;
    if (!m.ParseFromArray(p, (int)n)) die("B/a decode", 0);
    return read ? (long)pbtouch::touch(m) : m.tasks_size();
  }
  Fac f;
  int32_t drc = codec_decode(cl.base, cl.mode, tc, p, n, &f, cl.pull);
  if (drc != 0) die("C/E a decode", drc);
  return read ? (long)shapes::touch::touch(f) : (long)f.tasks.size();
}

void q_status(const ak_completion &c, const char *what) {
  if (c.status != AK_OK || c.grpc_status != 0)
    die(what, c.status == AK_ERR_RPC_STATUS ? c.grpc_status : c.status);
}

// One stream message of a queue cell, issued: B copies protobuf's bytes, C moves the encode
// context's output, E copies host-gen's bytes (the blocking forms' request handling).
void q_send(const Cell &cl, ThreadCtx &tc, ak_call *h, const Stream &st, size_t i, ak_queue *q, uint64_t tag) {
  const int32_t last = i + 1 == st.f.size();
  int32_t rc;
  if (cl.base == 'B') {
    const size_t qn = pb_into(st.p[i], tc.pbuf);
    rc = ak_call_send_q(h, tc.pbuf.data(), qn, last, q, tag);
  } else if (cl.base == 'C') {
    intptr_t e = core_enc(tc.ec, st.f[i], cl.mode);
    if (e < 0) die("C-q/d encode", (long)e);
    rc = ak_call_send_enc_q(h, tc.ec, last, q, tag);
  } else {
    ak::Enc *e = hg_enc(tc, st.f[i], cl.mode);
    if (e->err != 0) die("E-q/d encode", e->err);
    rc = ak_call_send_q(h, e->data(), e->size(), last, q, tag);
  }
  if (rc != AK_OK) die("ak_call_send_q (refused)", rc);
}

// One batch of k calls of a queue cell for one job. `check` (d only, the untimed pre-check):
// UploadStreamCheck, the server's SHA-256 verified. Returns a fold.
// EXPERIMENT (Cf-q attribution): AK_Q_SLOT_CTX=1 gives each in-flight slot of a queue cell's d
// batch its own encode context (the blocking cells have one per caller thread; the queue cell's
// single drainer otherwise encodes every stream through one context and one spare ring).
ThreadCtx *q_slot_ctx(int s) {
  static const bool on = std::getenv("AK_Q_SLOT_CTX") && std::string(std::getenv("AK_Q_SLOT_CTX")) == "1";
  static ThreadCtx *slots[16] = {0};
  if (!on || s < 0 || s >= 16) return NULL;
  if (!slots[s]) slots[s] = new ThreadCtx();
  return slots[s];
}
long a_q_batch(World &w, size_t ci, const Job &job, int k, bool check);
long q_batch(World &w, size_t ci, const Job &job, int k, ThreadCtx &tc, bool check = false) {
  // --check-stream reaches the queue cells through Pool::batch too (before 2026-10-01 only the
  // blocking path read g_check_stream: a queue cell's d calls went to UploadStream, the server's
  // byte count checked but not its SHA-256)
  check = check || g_check_stream;
  const Cell &cl = w.cells[ci];
  if (cl.base == 'A') return a_q_batch(w, ci, job, k, check);
  Conn &cn = w.conns[ci];
  if (k < 1 || k > kMaxQ) die("q_batch: in-flight level", k);
  ak_call *h[kMaxQ];
  size_t next[kMaxQ];
  int state[kMaxQ];  // the operation pending on the slot, 0 = none (done)
  long n = 0;
  if (job.dir != 'd') {
    static const uint8_t kNoReq[1] = {0};
    const bool resp = job.dir == 'a' || job.dir == 'r';
    const char *path = resp ? kFetch : job.dir == 'c' ? kUpload : kPush;
    const size_t pl = std::strlen(path);
    for (int s = 0; s < k; ++s) {
      const uint64_t tag = ((uint64_t)s << 8) | kOpUnary;
      if (resp) {
        h[s] = ak_call_unary_q(cn.cl, (const uint8_t *)path, pl, kNoReq, 0, cn.q, tag);
      } else if (cl.base == 'B') {
        const size_t qn = job.dir == 'c' ? pb_into(w.pb_up[job.pi], tc.pbuf) : pb_into(w.pb_req, tc.pbuf);
        h[s] = ak_call_unary_q(cn.cl, (const uint8_t *)path, pl, tc.pbuf.data(), qn, cn.q, tag);
      } else if (cl.base == 'C') {
        intptr_t e = job.dir == 'c' ? core_enc(tc.ec, w.fac_up[job.pi], cl.mode) : core_enc(tc.ec, w.fac_req, cl.mode);
        if (e < 0) die("C-q encode", (long)e);
        h[s] = ak_call_unary_enc_q(cn.cl, (const uint8_t *)path, pl, tc.ec, cn.q, tag);
      } else {
        ak::Enc *e = job.dir == 'c' ? hg_enc(tc, w.fac_up[job.pi], cl.mode) : hg_enc(tc, w.fac_req, cl.mode);
        if (e->err != 0) die("E-q encode", e->err);
        h[s] = ak_call_unary_q(cn.cl, (const uint8_t *)path, pl, e->data(), e->size(), cn.q, tag);
      }
      if (!h[s]) die("ak_call_unary_q (refused)", s);
      state[s] = kOpUnary;
    }
    for (int left = k; left > 0; --left) {
      ak_completion c;
      const int32_t qr = ak_queue_next(cn.q, &c, kQWaitMs);
      if (qr != AK_QUEUE_OK) die("ak_queue_next", qr);
      const int s = (int)(c.tag >> 8);
      if (s >= k || (int)(c.tag & 0xff) != kOpUnary || state[s] != kOpUnary) die("q: a completion matching no pending call", (long)c.tag);
      q_status(c, resp ? "core-q call gRPC status" : "core-q request gRPC status");
      if (resp) n += core_a_answer(w, cl, tc, c.bytes.ptr, c.bytes.len, job.dir == 'r');
      else {
        if (c.bytes.len != (job.dir == 'c' ? w.want_c_len : 0)) die("core-q request response length", (long)c.bytes.len);
        n += 1;
      }
      ak_bytes_free(&c.bytes);
      ak_call_destroy(h[s]);
      state[s] = 0;
    }
    return n;
  }
  const Stream &st = w.st[job.pi];
  const size_t nmsg = st.f.size();
  const char *path = check ? kUploadStreamCheck : kUploadStream;
  for (int s = 0; s < k; ++s) {
    h[s] = ak_call_open(cn.cl, (const uint8_t *)path, std::strlen(path), AK_CALL_CLIENT_STREAM, NULL);
    if (!h[s]) die("ak_call_open", s);
    next[s] = 0;
    ThreadCtx *sc = q_slot_ctx(s);
    q_send(cl, sc ? *sc : tc, h[s], st, 0, cn.q, ((uint64_t)s << 8) | kOpSend);
    state[s] = kOpSend;
  }
  for (int left = k; left > 0;) {
    ak_completion c;
    const int32_t qr = ak_queue_next(cn.q, &c, kQWaitMs);
    if (qr != AK_QUEUE_OK) die("ak_queue_next", qr);
    const int s = (int)(c.tag >> 8), op = (int)(c.tag & 0xff);
    if (s >= k || op != state[s]) die("q/d: a completion matching no pending operation", (long)c.tag);
    if (op == kOpSend) {
      if (c.status != AK_OK || c.bytes.len != 0) die("ak_call_send_q completion", c.status);
      if (++next[s] < nmsg) {
        ThreadCtx *sc = q_slot_ctx(s);
        q_send(cl, sc ? *sc : tc, h[s], st, next[s], cn.q, ((uint64_t)s << 8) | kOpSend);
      } else {
        const int32_t rc = ak_call_recv_q(h[s], cn.q, ((uint64_t)s << 8) | kOpRecv);
        if (rc != AK_OK) die("ak_call_recv_q (refused)", rc);
        state[s] = kOpRecv;
      }
      continue;
    }
    q_status(c, "d-q gRPC status");
    check_answer(st, c.bytes.ptr, c.bytes.len, check);
    ak_bytes_free(&c.bytes);
    ak_call_destroy(h[s]);
    state[s] = 0;
    n += (long)nmsg;
    --left;
  }
  return n;
}

// ---- grpc++'s completion-queue delivery, cell A-q (owner, 2026-10-01: the deliveries compared) ----
// grpc++ 1.80's async API on ONE grpc::CompletionQueue per cell, drained by the issuing thread with
// AsyncNext, the analogue of the core's queue cells: c = the generated stub's AsyncUpload +
// Finish(tag) per call (protobuf request, SerializationTraits); d = ClientAsyncWriterFactory<Pb5>
// (start = true), Write(tag) of each chunk issued when the previous write completed, WritesDone(tag),
// Finish(tag), the raw-bytes answer checked as A's. k calls in flight, each completion matched to
// its call by its tag (slot << 8 | operation).
void *gtag(int s, int op) { return (void *)(intptr_t)(((intptr_t)s << 8) | op); }
bool gnext(grpc::CompletionQueue &cq, int *s, int *op) {
  void *t = nullptr;
  bool ok = false;
  const grpc::CompletionQueue::NextStatus r =
      cq.AsyncNext(&t, &ok, std::chrono::system_clock::now() + std::chrono::milliseconds(kQWaitMs));
  if (r != grpc::CompletionQueue::GOT_EVENT) die("A-q: no completion within the wait", (long)r);
  *s = (int)((intptr_t)t >> 8);
  *op = (int)((intptr_t)t & 0xff);
  return ok;
}
long a_q_batch(World &w, size_t ci, const Job &job, int k, bool check) {
  Conn &cn = w.conns[ci];
  grpc::CompletionQueue &cq = *cn.gcq;
  if (k < 1 || k > kMaxQ) die("A-q: in-flight level", k);
  std::unique_ptr<grpc::ClientContext> ctx[kMaxQ];
  grpc::Status st[kMaxQ];
  int state[kMaxQ];
  if (job.dir == 'c') {
    svcns::Empty rsp[kMaxQ];
    std::unique_ptr<grpc::ClientAsyncResponseReader<svcns::Empty> > rd[kMaxQ];
    for (int s = 0; s < k; ++s) {
      ctx[s].reset(new grpc::ClientContext());
      rd[s] = cn.stubs[0]->AsyncUpload(ctx[s].get(), w.pb_up[job.pi], &cq);
      rd[s]->Finish(&rsp[s], &st[s], gtag(s, kOpUnary));
      state[s] = kOpUnary;
    }
    for (int left = k; left > 0; --left) {
      int s, op;
      const bool ok = gnext(cq, &s, &op);
      if (s >= k || op != kOpUnary || state[s] != kOpUnary) die("A-q/c: a completion matching no pending call", s);
      if (!ok || !st[s].ok()) die("A-q/c status", (long)st[s].error_code());
      if (w.want_c_len != 0) die("A-q/c response length", 0);
      state[s] = 0;
    }
    return k;
  }
  if (job.dir != 'd') die("A-q: directions c and d only", job.dir);
  const Stream &stv = w.st[job.pi];
  const size_t nmsg = stv.f.size();
  const grpc::internal::RpcMethod &method = check ? *cn.m_stream_check : *cn.m_stream;
  grpc::ByteBuffer rsp[kMaxQ];
  size_t next[kMaxQ];
  std::unique_ptr<grpc::ClientAsyncWriter<Pb5> > wr[kMaxQ];
  for (int s = 0; s < k; ++s) {
    ctx[s].reset(new grpc::ClientContext());
    wr[s].reset(grpc::internal::ClientAsyncWriterFactory<Pb5>::Create(cn.chan.get(), &cq, method, ctx[s].get(), &rsp[s],
                                                                       true, gtag(s, kOpStart)));
    state[s] = kOpStart;
    next[s] = 0;
  }
  long n = 0;
  for (int left = k; left > 0;) {
    int s, op;
    const bool ok = gnext(cq, &s, &op);
    if (s >= k || op != state[s]) die("A-q/d: a completion matching no pending operation", s);
    if (!ok && op != kOpRecv) die("A-q/d: the stream failed", op);
    if (op == kOpStart) {
      wr[s]->Write(stv.p[0], gtag(s, kOpSend));
      state[s] = kOpSend;
    } else if (op == kOpSend) {
      if (++next[s] < nmsg) {
        wr[s]->Write(stv.p[next[s]], gtag(s, kOpSend));
      } else {
        wr[s]->WritesDone(gtag(s, kOpWritesDone));
        state[s] = kOpWritesDone;
      }
    } else if (op == kOpWritesDone) {
      wr[s]->Finish(&st[s], gtag(s, kOpRecv));
      state[s] = kOpRecv;
    } else {
      if (!st[s].ok()) die("A-q/d status", (long)st[s].error_code());
      std::string a = flatten(rsp[s]);
      check_answer(stv, (const uint8_t *)a.data(), a.size(), check);
      state[s] = 0;
      n += (long)nmsg;
      --left;
    }
  }
  return n;
}

// ---- the callback deliveries, cells A-cb and Cf-cb (owner, 2026-10-01) ----------------------
// The issuing thread starts the batch's k calls and waits on a condition variable; each call's
// last completion, run on the stack's own thread, signals it (CbWait::done). Nothing else runs on
// the issuing thread while the calls are in flight; no caller thread of the pool is woken.
//   A-cb  grpc++ 1.80's callback API: c = the generated stub's async()->Upload(ctx, req, rsp,
//         std::function) per call; d = a grpc::ClientWriteReactor<Pb5> per call
//         (ClientCallbackWriterFactory<Pb5>, raw-bytes answer as A's): StartWrite(chunk 0),
//         StartCall, each next chunk written from OnWriteDone, StartWritesDone after the last,
//         the answer checked in OnDone. The reactions run on grpc-core's callback threads.
//   Cf-cb the core's callback delivery on the framed send path: c = ak_call_unary_enc_cb (the
//         request encoded on the issuing thread, moved before the call returns); d =
//         ak_call_open, ak_call_send_enc_cb of chunk 0, each next chunk ENCODED AND SENT FROM
//         THE PREVIOUS SEND'S COMPLETION (on a core thread), ak_call_recv_cb after the last, the
//         answer checked in its completion. Each in-flight slot has its own encode context: two
//         streams' completions can run at once on two core workers (Cf-q's single drainer encodes
//         every stream through one context; the blocking Cf, one per caller thread).
struct CbWait {
  std::mutex m;
  std::condition_variable cv;
  int pending = 0;
  long n = 0;
  void done(long v) {
    std::lock_guard<std::mutex> l(m);
    n += v;
    if (--pending == 0) cv.notify_one();
  }
  long wait() {
    std::unique_lock<std::mutex> l(m);
    if (!cv.wait_for(l, std::chrono::milliseconds(kQWaitMs), [&] { return pending == 0; }))
      die("callback delivery: a call did not complete within the wait", pending);
    return n;
  }
};
struct CoreCbSlot {
  const Cell *cl = nullptr;
  World *w = nullptr;
  ak_call *h = nullptr;
  const Stream *st = nullptr;
  size_t next = 0;
  bool check = false;
  ThreadCtx *tc = nullptr;
  CbWait *wt = nullptr;
};
ThreadCtx *cb_slot_ctx(int s) {  // created by the issuing thread on first use (warm-up), kept
  static ThreadCtx *slots[kMaxQ] = {0};
  if (!slots[s]) slots[s] = new ThreadCtx();
  return slots[s];
}
void core_cb_d(void *u, struct ak_completion *c);
void core_cb_send(CoreCbSlot &s) {
  if (core_enc(s.tc->ec, s.st->f[s.next], s.cl->mode) < 0) die("Cf-cb/d encode", (long)s.next);
  const int32_t rc = ak_call_send_enc_cb(s.h, s.tc->ec, s.next + 1 == s.st->f.size(), core_cb_d, &s, kOpSend);
  if (rc != AK_OK) die("ak_call_send_enc_cb (refused)", rc);
}
void core_cb_d(void *u, struct ak_completion *c) {
  CoreCbSlot &s = *(CoreCbSlot *)u;
  if (c->tag == kOpSend) {
    if (c->status != AK_OK || c->bytes.len != 0) die("ak_call_send_enc_cb completion", c->status);
    if (++s.next < s.st->f.size()) {
      core_cb_send(s);
    } else {
      const int32_t rc = ak_call_recv_cb(s.h, core_cb_d, &s, kOpRecv);
      if (rc != AK_OK) die("ak_call_recv_cb (refused)", rc);
    }
    return;
  }
  q_status(*c, "d-cb gRPC status");
  check_answer(*s.st, c->bytes.ptr, c->bytes.len, s.check);
  ak_bytes_free(&c->bytes);
  s.wt->done((long)s.st->f.size());
}
void core_cb_c(void *u, struct ak_completion *c) {
  CoreCbSlot &s = *(CoreCbSlot *)u;
  q_status(*c, "c-cb gRPC status");
  if (c->bytes.len != s.w->want_c_len) die("c-cb response length", (long)c->bytes.len);
  ak_bytes_free(&c->bytes);
  s.wt->done(1);
}
struct AWriter : grpc::ClientWriteReactor<Pb5> {
  const Stream *st = nullptr;
  size_t next = 0;
  bool check = false;
  grpc::ClientContext ctx;
  grpc::ByteBuffer rsp;
  CbWait *wt = nullptr;
  void go(Conn &cn, const grpc::internal::RpcMethod &m) {
    grpc::internal::ClientCallbackWriterFactory<Pb5>::Create(cn.chan.get(), m, &ctx, &rsp, this);
    StartWrite(&st->p[0]);
    StartCall();
  }
  void OnWriteDone(bool ok) override {
    if (!ok) return;  // the stream failed: OnDone carries the status
    if (++next < st->f.size()) StartWrite(&st->p[next]);
    else StartWritesDone();
  }
  void OnDone(const grpc::Status &s) override {
    if (!s.ok()) die("A-cb/d status", (long)s.error_code());
    std::string a = flatten(rsp);
    check_answer(*st, (const uint8_t *)a.data(), a.size(), check);
    wt->done((long)st->f.size());
  }
};
long cb_batch(World &w, size_t ci, const Job &job, int k, ThreadCtx &tc, bool check = false) {
  check = check || g_check_stream;
  const Cell &cl = w.cells[ci];
  Conn &cn = w.conns[ci];
  if (k < 1 || k > kMaxQ) die("cb_batch: in-flight level", k);
  if (job.dir != 'c' && job.dir != 'd') die("callback cells: directions c and d only", job.dir);
  CbWait wt;
  wt.pending = k;
  if (cl.base == 'A') {
    cn.keep.clear();  // the previous batch's contexts and reactors
    if (job.dir == 'c') {
      std::shared_ptr<grpc::ClientContext> ctx[kMaxQ];
      std::shared_ptr<svcns::Empty> rsp[kMaxQ];
      std::shared_ptr<grpc::Status> sts(new grpc::Status[kMaxQ], std::default_delete<grpc::Status[]>());
      for (int s = 0; s < k; ++s) {
        ctx[s].reset(new grpc::ClientContext());
        rsp[s].reset(new svcns::Empty());
        cn.keep.push_back(ctx[s]);
        cn.keep.push_back(rsp[s]);
        grpc::Status *slot = sts.get() + s;
        cn.stubs[0]->async()->Upload(ctx[s].get(), &w.pb_up[job.pi], rsp[s].get(), [slot, &wt](grpc::Status x) {
          *slot = x;
          wt.done(1);
        });
      }
      cn.keep.push_back(sts);
      const long n = wt.wait();
      for (int s = 0; s < k; ++s)
        if (!sts.get()[s].ok()) die("A-cb/c status", (long)sts.get()[s].error_code());
      if (w.want_c_len != 0) die("A-cb/c response length", 0);
      return n;
    }
    const grpc::internal::RpcMethod &method = check ? *cn.m_stream_check : *cn.m_stream;
    for (int s = 0; s < k; ++s) {
      std::shared_ptr<AWriter> r(new AWriter());
      r->st = &w.st[job.pi];
      r->check = check;
      r->wt = &wt;
      cn.keep.push_back(r);
      r->go(cn, method);
    }
    return wt.wait();
  }
  CoreCbSlot slot[kMaxQ];
  if (job.dir == 'c') {
    const size_t pl = std::strlen(kUpload);
    for (int s = 0; s < k; ++s) {
      slot[s].cl = &cl; slot[s].w = &w; slot[s].wt = &wt;
      if (core_enc(tc.ec, w.fac_up[job.pi], cl.mode) < 0) die("Cf-cb/c encode", s);
      slot[s].h = ak_call_unary_enc_cb(cn.cl, (const uint8_t *)kUpload, pl, tc.ec, core_cb_c, &slot[s], kOpUnary);
      if (!slot[s].h) die("ak_call_unary_enc_cb (refused)", s);
    }
  } else {
    const char *path = check ? kUploadStreamCheck : kUploadStream;
    for (int s = 0; s < k; ++s) {
      CoreCbSlot &x = slot[s];
      x.cl = &cl; x.w = &w; x.wt = &wt; x.st = &w.st[job.pi]; x.next = 0; x.check = check; x.tc = cb_slot_ctx(s);
      x.h = ak_call_open(cn.cl, (const uint8_t *)path, std::strlen(path), AK_CALL_CLIENT_STREAM, NULL);
      if (!x.h) die("ak_call_open", s);
      core_cb_send(x);
    }
  }
  const long n = wt.wait();
  for (int s = 0; s < k; ++s) ak_call_destroy(slot[s].h);  // after every completion was delivered
  return n;
}

// EXPERIMENT (Rust patch p5-deferred): ak_call_send_deferred(h, enc, f, user, last, wait), found
// at run time; NULL on a core without it. The callback encodes the chunk it is handed with the
// core codec (the cell's mode) into the context the core hands it, on the core worker that
// writes the call; the core then takes the result framed, as ak_call_send_enc does.
typedef int32_t (*AkEncodeFn)(void *user, ak_enc_ctx *enc);
typedef int32_t (*SendDeferredFn)(ak_call *, ak_enc_ctx *, AkEncodeFn, void *, int32_t last, int32_t wait);
SendDeferredFn g_send_deferred = NULL;
// EXPERIMENT (Rust patch p6-zero-copy): ak_enc_set_zc(enc, min_len), ak_call_send_enc_zc(h, enc, last,
// release, user); NULL on a core without them.
typedef int32_t (*SetZcFn)(ak_enc_ctx *, size_t);
typedef int32_t (*SendZcFn)(ak_call *, ak_enc_ctx *, int32_t last, void (*release)(void *), void *user);
SetZcFn g_set_zc = NULL;
SendZcFn g_send_zc = NULL;
// EXPERIMENT (Rust patch p7-deferred-zc): ak_call_send_deferred_zc(h, enc, f, user, last, wait, release, ruser)
typedef int32_t (*SendDeferredZcFn)(ak_call *, ak_enc_ctx *, AkEncodeFn, void *, int32_t last, int32_t wait,
                                    void (*release)(void *), void *ruser);
SendDeferredZcFn g_send_deferred_zc = NULL;
const size_t kZcMin = 64 * 1024;  // blobs of 64 KiB and more borrowed (the Rust cell's threshold)
void zc_release(void *user) { ((std::atomic<long> *)user)->fetch_add(1); }
// --check-stream 1 (a check run, never timed): every direction d call goes to UploadStreamCheck
// and verifies the server's byte count and SHA-256 of the messages as received.
struct DefJob {
  const Fac5 *v;
  Mode m;
  int32_t fail;  // semantics only: return this (< 0) instead of encoding
};
int32_t def_encode(void *user, ak_enc_ctx *enc) {
  const DefJob *j = (const DefJob *)user;
  if (j->fail < 0) return j->fail;
  const intptr_t e = core_enc(enc, *j->v, j->m);
  return e < 0 ? (int32_t)e : (int32_t)(e > 0x7fffffff ? 0x7fffffff : e);
}

// Direction d, one streamed upload (req. 14): A through grpc++'s typed ClientWriter; B, C, E
// through the core's client streaming (ak_call_open, a send per chunk, ak_call_recv); D and F
// through grpc++'s raw ClientWriter, each chunk handed over moved. Returns the chunk count.
// `check` (the pre-check, never timed) calls UploadStreamCheck and verifies the digest too.
long stream_call(World &w, size_t ci, int pi, int t, ThreadCtx &tc, bool check = false) {
  check = check || g_check_stream;
  const Cell &cl = w.cells[ci];
  Conn &cn = w.conns[ci];
  const Stream &st = w.st[pi];
  const size_t nmsg = st.f.size();
  const char *path = check ? kUploadStreamCheck : kUploadStream;
  if (cl.cb) return cb_batch(w, ci, Job{'d', pi}, 1, tc, check);
  if (cl.q) return q_batch(w, ci, Job{'d', pi}, 1, tc, check);
  if (cl.base == 'A') {
    // grpc++'s ClientWriter with protobuf requests (SerializationTraits, the production path);
    // the answer is raw bytes (SERVER.md), so the response side is a ByteBuffer.
    (void)t;
    const grpc::internal::RpcMethod &method = check ? *cn.m_stream_check : *cn.m_stream;  // H-4
    grpc::ClientContext ctx;
    grpc::ByteBuffer rsp;
    std::unique_ptr<grpc::ClientWriter<Pb5> > wr(
        grpc::internal::ClientWriterFactory<Pb5>::Create(cn.chan.get(), method, &ctx, &rsp));
    for (size_t i = 0; i < nmsg; ++i)
      if (!wr->Write(st.p[i])) die_write(wr, "A/d write", (long)i);
    wr->WritesDone();
    grpc::Status s = wr->Finish();
    if (!s.ok()) die("A/d status", (long)s.error_code());
    std::string a = flatten(rsp);
    check_answer(st, (const uint8_t *)a.data(), a.size(), check);
    return (long)nmsg;
  }
  if (cl.zc && cl.deferred) {  // Cf-zcw / Cf-zcp: the transport encodes the head, the data borrowed
    if (!tc.zec) {
      tc.zec = ak_enc_ctx_new();
      if (!tc.zec || g_set_zc(tc.zec, kZcMin) != AK_OK) die("ak_enc_set_zc", 0);
    }
    for (int spin = 0; tc.zc_released.load() != tc.zc_sent.load(); ++spin) {
      if (spin > 2000) die("Cf-zcp/zcw: a message of the previous call was never released", tc.zc_sent.load() - tc.zc_released.load());
      std::this_thread::sleep_for(std::chrono::microseconds(500));
    }
    ak_call *h = ak_call_open(cn.cl, (const uint8_t *)path, std::strlen(path), AK_CALL_CLIENT_STREAM, NULL);
    if (!h) die("ak_call_open", 0);
    DefJob jobs[16];
    if (nmsg > 16) die("Cf-zcp: too many chunks", (long)nmsg);
    for (size_t i = 0; i < nmsg; ++i) {
      jobs[i].v = &st.f[i];
      jobs[i].m = cl.mode;
      jobs[i].fail = 0;
      tc.zc_sent.fetch_add(1);
      const int32_t rc = g_send_deferred_zc(h, tc.zec, def_encode, &jobs[i], i + 1 == nmsg, cl.deferred == 1 ? 1 : 0,
                                            zc_release, &tc.zc_released);
      if (rc != AK_OK) die("ak_call_send_deferred_zc", rc);
    }
    struct ak_bytes out;
    out.ptr = NULL; out.len = 0; out.owner = NULL;
    int32_t gs = -1;
    const int32_t rc = ak_call_recv(h, &out, &gs);
    if (rc != AK_OK || gs != 0) die(rc == AK_ERR_RPC_STATUS ? "d gRPC status" : "ak_call_recv", rc == AK_ERR_RPC_STATUS ? gs : rc);
    check_answer(st, out.ptr, out.len, check);
    ak_bytes_free(&out);
    ak_call_destroy(h);
    return (long)nmsg;
  }
  if (cl.zc) {  // Cf-zc: each chunk's data borrowed from the host (st.f, alive for the process)
    if (!tc.zec) {
      tc.zec = ak_enc_ctx_new();
      if (!tc.zec || g_set_zc(tc.zec, kZcMin) != AK_OK) die("ak_enc_set_zc", 0);
    }
    // the lifetime contract, checked: every message of this thread's previous call released
    // (the release may run after that call's recv returned: allow it a short while)
    for (int spin = 0; tc.zc_released.load() != tc.zc_sent.load(); ++spin) {
      if (spin > 2000) die("Cf-zc: a message of the previous call was never released", tc.zc_sent.load() - tc.zc_released.load());
      std::this_thread::sleep_for(std::chrono::microseconds(500));
    }
    ak_call *h = ak_call_open(cn.cl, (const uint8_t *)path, std::strlen(path), AK_CALL_CLIENT_STREAM, NULL);
    if (!h) die("ak_call_open", 0);
    for (size_t i = 0; i < nmsg; ++i) {
      if (core_enc(tc.zec, st.f[i], cl.mode) < 0) die("Cf-zc encode", (long)i);
      tc.zc_sent.fetch_add(1);
      const int32_t rc = g_send_zc(h, tc.zec, i + 1 == nmsg, zc_release, &tc.zc_released);
      if (rc != AK_OK) die("ak_call_send_enc_zc", rc);
    }
    struct ak_bytes out;
    out.ptr = NULL; out.len = 0; out.owner = NULL;
    int32_t gs = -1;
    const int32_t rc = ak_call_recv(h, &out, &gs);
    if (rc != AK_OK || gs != 0) die(rc == AK_ERR_RPC_STATUS ? "d gRPC status" : "ak_call_recv", rc == AK_ERR_RPC_STATUS ? gs : rc);
    check_answer(st, out.ptr, out.len, check);
    ak_bytes_free(&out);
    ak_call_destroy(h);
    return (long)nmsg;
  }
  if (cl.deferred) {  // Cf-enc / Cf-encp: the transport encodes each chunk (wait = 1 / 0)
    ak_call *h = ak_call_open(cn.cl, (const uint8_t *)path, std::strlen(path), AK_CALL_CLIENT_STREAM, NULL);
    if (!h) die("ak_call_open", 0);
    DefJob jobs[16];  // alive, with the values and tc.ec untouched, until the recv completes (wait = 0)
    if (nmsg > 16) die("Cf-enc: too many chunks", (long)nmsg);
    for (size_t i = 0; i < nmsg; ++i) {
      jobs[i].v = &st.f[i];
      jobs[i].m = cl.mode;
      jobs[i].fail = 0;
      const int32_t rc = g_send_deferred(h, tc.ec, def_encode, &jobs[i], i + 1 == nmsg, cl.deferred == 1 ? 1 : 0);
      if (rc != AK_OK) die("ak_call_send_deferred", rc);
    }
    struct ak_bytes out;
    out.ptr = NULL; out.len = 0; out.owner = NULL;
    int32_t gs = -1;
    const int32_t rc = ak_call_recv(h, &out, &gs);
    if (rc != AK_OK || gs != 0) die(rc == AK_ERR_RPC_STATUS ? "d gRPC status" : "ak_call_recv", rc == AK_ERR_RPC_STATUS ? gs : rc);
    check_answer(st, out.ptr, out.len, check);
    ak_bytes_free(&out);
    ak_call_destroy(h);
    return (long)nmsg;
  }
  if (!grpc_cell(cl.base)) {
    ak_call *h = ak_call_open(cn.cl, (const uint8_t *)path, std::strlen(path),
                              AK_CALL_CLIENT_STREAM, NULL);
    if (!h) die("ak_call_open", 0);
    for (size_t i = 0; i < nmsg; ++i) {
      const int32_t last = i + 1 == nmsg;
      int32_t rc;
      if (cl.base == 'B') {
        const size_t qn = pb_into(st.p[i], tc.pbuf);  // H-6
        rc = ak_call_send(h, tc.pbuf.data(), qn, last);
      } else if (cl.base == 'C') {
        intptr_t e = core_enc(tc.ec, st.f[i], cl.mode);
        if (e < 0) die("C/d encode", (long)e);
        rc = ak_call_send_enc(h, tc.ec, last);
      } else {
        ak::Enc *e = hg_enc(tc, st.f[i], cl.mode);
        if (e->err != 0) die("E/d encode", e->err);
        rc = ak_call_send(h, e->data(), e->size(), last);
      }
      if (rc != AK_OK) die("ak_call_send", rc);
    }
    struct ak_bytes out;
    out.ptr = NULL; out.len = 0; out.owner = NULL;
    int32_t gs = -1;
    int32_t rc = ak_call_recv(h, &out, &gs);
    if (rc != AK_OK || gs != 0) die(rc == AK_ERR_RPC_STATUS ? "d gRPC status" : "ak_call_recv",
                                    rc == AK_ERR_RPC_STATUS ? gs : rc);
    check_answer(st, out.ptr, out.len, check);
    ak_bytes_free(&out);
    ak_call_destroy(h);
    return (long)nmsg;
  }
  const grpc::internal::RpcMethod &method = check ? *cn.m_stream_check : *cn.m_stream;  // H-4
  grpc::ClientContext ctx;
  grpc::ByteBuffer rsp;
  std::unique_ptr<grpc::ClientWriter<grpc::ByteBuffer> > wr(
      grpc::internal::ClientWriterFactory<grpc::ByteBuffer>::Create(cn.chan.get(), method, &ctx, &rsp));
  for (size_t i = 0; i < nmsg; ++i) {
    grpc::ByteBuffer bb = grpc_request(cl.base, cl.mode, tc, st.f[i]);
    if (!wr->Write(bb)) die_write(wr, "D/F d write", (long)i);
  }
  wr->WritesDone();
  grpc::Status s = wr->Finish();
  if (!s.ok()) die("D/F d status", (long)s.error_code());
  std::string a = flatten(rsp);
  check_answer(st, (const uint8_t *)a.data(), a.size(), check);
  return (long)nmsg;
}

// One call of one cell for one job. Returns a fold (so nothing is dead).
long cell_call(World &w, size_t ci, const Job &job, int t, ThreadCtx &tc) {
  const Cell &cl = w.cells[ci];
  Conn &cn = w.conns[ci];
  const char cell = cl.base;
  const char dir = job.dir;
  if (dir == 'd') return stream_call(w, ci, job.pi, t, tc);
  if (cl.deferred || cl.zc) die("Cf-enc / Cf-encp / Cf-zc: direction d only (stream sends)", dir);
  static const uint8_t kNoReq[1] = {0};
  const bool resp = dir == 'a' || dir == 'r';
  const bool read = dir == 'r';
  const bool up = dir == 'c';
  if (cl.cb) return cb_batch(w, ci, job, 1, tc);  // one call, callback delivery (counts, probes)
  if (cl.q && cell == 'A') return q_batch(w, ci, job, 1, tc);
  if (cell == 'A') {
    grpc::ClientContext ctx;
    gridns::Grid::Stub &st = *cn.stubs[(size_t)t % cn.stubs.size()];
    if (resp) {
      svcns::Empty q;
      Pb r;
      grpc::Status s = st.Fetch(&ctx, q, &r);
      if (!s.ok()) die("A/a status", (long)s.error_code());
      if (r.tasks_size() != 500) die("A/a content", r.tasks_size());
      return read ? (long)pbtouch::touch(r) : r.tasks_size();
    }
    svcns::Empty r;
    grpc::Status s = up ? st.Upload(&ctx, w.pb_up[job.pi], &r) : st.Push(&ctx, w.pb_req, &r);
    if (!s.ok()) die(up ? "A/c status" : "A/b status", (long)s.error_code());
    if (up && w.want_c_len != 0) die("A/c response length", 0);  // the typed stub reads a message, 0 bytes
    return 1;
  }
  if (cl.q) return q_batch(w, ci, job, 1, tc);  // one call on the queue (counts, probes)
  if (!grpc_cell(cell)) {  // B, C, E (and framed twins): the core's transport
    if (!resp) {
      if (up) core_unary_req(cl, cn, tc, kUpload, w.fac_up[job.pi], w.pb_up[job.pi], w.want_c_len);
      else core_unary_req(cl, cn, tc, kPush, w.fac_req, w.pb_req, 0);
      return 1;
    }
    struct ak_bytes out;
    out.ptr = NULL; out.len = 0; out.owner = NULL;
    int32_t gs = -1;  // the gRPC status (ABI v1 section 9); non-OK is AK_ERR_RPC_STATUS
    int32_t rc = ak_call_unary(cn.cl, (const uint8_t *)kFetch, std::strlen(kFetch), kNoReq, 0, &out, &gs);
    if (rc != AK_OK || gs != 0) die(rc == AK_ERR_RPC_STATUS ? "core call gRPC status" : "core call status",
                                    rc == AK_ERR_RPC_STATUS ? gs : rc);
    const long n = core_a_answer(w, cl, tc, out.ptr, out.len, read);
    ak_bytes_free(&out);
    return n;
  }
  // D, F: grpc++'s transport carrying opaque bytes, the generated codec at the client.
  const grpc::internal::RpcMethod &method = resp ? *cn.m_fetch : up ? *cn.m_upload : *cn.m_push;  // H-4
  grpc::ClientContext ctx;
  grpc::ByteBuffer req, rsp;
  if (resp) {
    grpc::Slice e;
    req = grpc::ByteBuffer(&e, 1);
  } else if (up) {
    req = grpc_request(cell, cl.mode, tc, w.fac_up[job.pi]);
  } else {
    req = grpc_request(cell, cl.mode, tc, w.fac_req);
  }
  grpc::Status s = grpc::internal::BlockingUnaryCall<grpc::ByteBuffer, grpc::ByteBuffer>(
      cn.chan.get(), method, &ctx, req, &rsp);
  if (!s.ok()) die("D/F status", (long)s.error_code());
  if (!resp) {
    if (rsp.Length() != (up ? w.want_c_len : 0)) die("D/F request response length", (long)rsp.Length());
    return 1;
  }
  if (rsp.Length() != w.expect_a) die("D/F a response length", (long)rsp.Length());
  // The generated decoders need one contiguous buffer; protobuf reads the slice list. The
  // concatenation is part of cells D and F (it is the integration's cost), stated. R-2: into
  // the thread's reused buffer (one slice is used in place).
  const uint8_t *p = NULL;
  size_t len = 0;
  if (!bb_contig(rsp, tc.bbf, &p, &len)) die("D/F a dump", 0);
  Fac f;
  int32_t drc = codec_decode(cell, cl.mode, tc, p, len, &f, cl.pull);
  if (drc != 0) die("D/F a decode", drc);
  return read ? (long)shapes::touch::touch(f) : (long)f.tasks.size();
}

// R-H2: the caller threads are created ONCE, before any timed window, each with its own
// contexts, and reused by every batch. H-2 (2026-09-28): each thread has its OWN condition
// variable, and a batch wakes only the k threads it needs (before: one notify_all woke every
// thread of the pool, at k = 1 too); the window holds the calls, k hand-offs and one wait.
// --profile only: per-call timestamps on the blocking path (steady clock, ns), set around the loop
std::atomic<bool> g_trace_calls{false};
int64_t mono_ns() {
  return std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now().time_since_epoch()).count();
}
struct Pool {
  struct Seat {
    std::vector<std::pair<int64_t, int64_t> > tr;  // (start, end) of each traced call
    std::mutex m;
    std::condition_variable cv;
    bool go = false, quit = false;
    int per = 0;
    size_t cell = 0, job = 0;  // job: index into World::jobs
    long acc = 0;
  };
  World *w;
  std::vector<std::unique_ptr<Seat> > seats;
  std::vector<std::thread> ts;
  std::mutex dm;
  std::condition_variable done;
  int pending = 0;

  Pool(World &wr, int n) : w(&wr) {
    for (int t = 0; t < n; ++t) seats.emplace_back(new Seat());
    for (int t = 0; t < n; ++t) ts.push_back(std::thread([this, t]() { run(t); }));
  }
  ~Pool() {
    for (size_t i = 0; i < seats.size(); ++i) {
      { std::lock_guard<std::mutex> l(seats[i]->m); seats[i]->quit = true; seats[i]->go = true; }
      seats[i]->cv.notify_one();
    }
    for (size_t i = 0; i < ts.size(); ++i) ts[i].join();
  }
  void run(int t) {
    ThreadCtx tc;
    Seat &s = *seats[(size_t)t];
    for (;;) {
      int myper; size_t c, d;
      {
        std::unique_lock<std::mutex> l(s.m);
        s.cv.wait(l, [&] { return s.go; });
        s.go = false;
        if (s.quit) break;
        myper = s.per; c = s.cell; d = s.job;
      }
      long n = 0;
      const bool tr = g_trace_calls.load(std::memory_order_relaxed);
      for (int i = 0; i < myper; ++i) {
        const int64_t t0 = tr ? mono_ns() : 0;
        n += cell_call(*w, c, w->jobs[d], t, tc);
        if (tr) s.tr.push_back(std::make_pair(t0, mono_ns()));
      }
      std::lock_guard<std::mutex> l(dm);
      s.acc = n;
      if (--pending == 0) done.notify_one();
    }
  }
  ThreadCtx qtc;  // the queue cells' contexts: their batches run on the issuing (benchmark) thread
  long batch(size_t c, size_t j, int kk, int total) {
    // Queue cells (req. 16 as amended): the k calls are issued and drained by THIS thread
    // (q_batch); the caller threads are not woken.
    if (w->cells[c].q) return q_batch(*w, c, w->jobs[j], kk, qtc);
    if (w->cells[c].cb) return cb_batch(*w, c, w->jobs[j], kk, qtc);  // the issuing thread starts and waits
    const bool tr = g_trace_calls.load(std::memory_order_relaxed);
    const int64_t b0 = tr ? mono_ns() : 0;
    const int per = (total + kk - 1) / kk;
    { std::lock_guard<std::mutex> l(dm); pending = kk; }
    for (int i = 0; i < kk; ++i) {
      Seat &s = *seats[(size_t)i];
      { std::lock_guard<std::mutex> l(s.m); s.per = per; s.cell = c; s.job = j; s.go = true; }
      s.cv.notify_one();
    }
    std::unique_lock<std::mutex> l(dm);
    done.wait(l, [&] { return pending == 0; });
    long sum = 0;
    for (int i = 0; i < kk; ++i) sum += seats[(size_t)i]->acc;
    if (tr) btr.push_back(std::make_pair(b0, mono_ns()));
    return sum;
  }
  std::vector<std::pair<int64_t, int64_t> > btr;  // (start, end) of each traced batch
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


#ifdef AK_COUNTING
// --count N (req. 19, amended 2026-09-26/27): per call, every exported entry point the loop
// calls. rpc = the core's transport counters (ak_call_unary, ak_call_unary_enc,
// ak_enc_take_owned, ak_call_open/send/send_enc/recv/destroy, ak_bytes_free); codec = the
// encode/decode contexts' counters (entry points, element loops, reverse calls); host = what
// the binding calls that no counter sees (ak_enc_reset inside encode_into_*, before the
// encode entry point; the one ak_dec_reset_<Root> of a retain decode, before it). Retain:
// no pre-placed buffer, the binding's geometric unk_grow. Cells B, C, D and E with the
// framed twins, directions a, b, c (P5.3, P5.4) and d (4 MiB, 16 MiB); a+read has the
// crossings of a (the read is host code).
int count_cells(World &w, int n) {
  std::printf("# {\"campaign_rpc_counts\": {\"build\": \"%s\", \"calls\": %d, \"retain\": \"every position armed, no pre-placed buffer, unk_grow grows geometrically (decision 11 rule 8)\"}}\n",
              kBuild, n);
  ThreadCtx tc;
  std::vector<Job> jobs;
  jobs.push_back(Job{'a', 0});
  jobs.push_back(Job{'b', 0});
  jobs.push_back(Job{'c', 0});
  jobs.push_back(Job{'c', 1});
  jobs.push_back(Job{'d', 0});
  jobs.push_back(Job{'d', 1});
  for (size_t ci = 0; ci < w.cells.size(); ++ci) {
    const Cell &cl = w.cells[ci];
    if (cl.base != 'B' && cl.base != 'C' && cl.base != 'D' && cl.base != 'E') continue;
    if (cl.pull) continue;  // X-2's pull twins: their crossings are the codec suite's pull rows
    for (size_t ji = 0; ji < jobs.size(); ++ji) {
      const Job &j = jobs[ji];
      const int calls = j.dir == 'd' ? (n > 1 ? n / 2 : 1) : n;
      ak_rpc_counters_reset();
      ak_enc_counters_reset(tc.ec);
      ak_dec_counters_reset(tc.dc);
      ak_dec_counters_reset(tc.dcr);
      shapes::ffi::host_calls_take();
      for (int i = 0; i < calls; ++i) cell_call(w, ci, j, 0, tc);
      struct ak_rpc_counters rc;
      ak_rpc_counters(&rc);
      AkCounters ce, cd, cdr;
      ak_enc_counters(tc.ec, &ce);
      ak_dec_counters(tc.dc, &cd);
      ak_dec_counters(tc.dcr, &cdr);
      uint64_t host = shapes::ffi::host_calls_take();
      uint64_t codec_f = ce.forward + cd.forward + cdr.forward, codec_r = ce.reverse + cd.reverse + cdr.reverse;
      uint64_t fwd = rc.forward + codec_f + host, rev = rc.reverse + codec_r;
      std::printf("  %-10s %-2s %-5s forward/call %8.3f  reverse/call %8.3f   (rpc %.3f/%.3f + codec %.3f/%.3f + host %.3f)\n",
                  cl.label.c_str(), dir_label(j.dir), job_payload(j), fwd / (double)calls, rev / (double)calls,
                  rc.forward / (double)calls, rc.reverse / (double)calls, codec_f / (double)calls,
                  codec_r / (double)calls, host / (double)calls);
    }
  }
  return 0;
}
#endif

// --semantics 1 (req. 16 as amended; no timing): the queue forms of client streaming and of
// ak_call_unary_enc, as C++ drives them, against THE server's test paths (poc/rust
// server.rs: StatusS<n>, SleepS, StallS, EchoS, StatusU<n>), on the reference and the framed
// send path. Every entry returning AK_OK is followed by exactly one completion, a refusal by
// none (a 200 ms wait after each case finds no stray completion).
int q_semantics(World &w) {
  static const std::string pre = "/armonik.ffi.campaign.v1.Grid/";
  int bad = 0, n = 0;
  auto check = [&](bool ok, const std::string &what) {
    std::printf("%s %s\n", ok ? "PASS" : "FAIL", what.c_str());
    ++n;
    if (!ok) ++bad;
  };
#ifdef AK_NO_UNKNOWN_FIELDS
  const Mode m = kNoUnk;
#else
  const Mode m = kDrop;
#endif
  ThreadCtx tc;
  for (int framed = 0; framed < 2; ++framed) {
    const std::string wp = framed ? "[framed, stream q]" : "[reference, stream q]";
    ak_client *cl = core_client(w.rt, w.cfg.core_target, w.cfg.transport, framed == 1);
    ak_queue *q = ak_queue_new();
    if (!cl || !q) die("semantics: client or queue", framed);
    auto open = [&](const std::string &t) {
      const std::string p = t[0] == '/' ? t : pre + t;
      ak_call *h = ak_call_open(cl, (const uint8_t *)p.data(), p.size(), AK_CALL_CLIENT_STREAM, NULL);
      if (!h) die("semantics: ak_call_open", 0);
      return h;
    };
    auto wait = [&](uint64_t ms, ak_completion *c) {
      std::memset(c, 0, sizeof *c);
      return ak_queue_next(q, c, ms) == AK_QUEUE_OK;
    };
    auto stray = [&]() {
      ak_completion c;
      if (!wait(200, &c)) return false;
      ak_bytes_free(&c.bytes);
      return true;
    };
    char buf[400];
    static const uint8_t x[1] = {'x'};
    // status: the server answers 6 (ALREADY_EXISTS) on a stream
    {
      ak_call *h = open("StatusS6");
      ak_completion sa, sb;
      const int32_t a = ak_call_send_q(h, x, 1, 1, q, 11);
      const bool ga = wait(5000, &sa);
      const int32_t b = ak_call_recv_q(h, q, 12);
      const bool gb = wait(5000, &sb);
      ak_call_destroy(h);
      const bool send_ok = ga && sa.tag == 11 && ((sa.status == AK_OK && sa.grpc_status == 0) ||
                                                  (sa.status == AK_ERR_HOST && sa.grpc_status == -1));
      std::snprintf(buf, sizeof(buf), "%s server status 6: send entry %d, completion tag %llu status %d/%d; recv entry %d, completion tag %llu status %d/%d",
                    wp.c_str(), a, (unsigned long long)sa.tag, sa.status, sa.grpc_status, b,
                    (unsigned long long)sb.tag, sb.status, sb.grpc_status);
      check(a == AK_OK && send_ok && b == AK_OK && gb && sb.tag == 12 && sb.status == AK_ERR_RPC_STATUS &&
                sb.grpc_status == 6 && !stray(), buf);
      if (gb) ak_bytes_free(&sb.bytes);
    }
    // bytes: two moved-encode sends (ak_call_send_enc_q) to the checking path, the server's
    // byte count and SHA-256 of the messages as received against the client's
    {
      const Stream &st = w.st[0];
      ak_call *h = open(kUploadStreamCheck);
      bool ok = true;
      for (size_t j = 0; j < st.f.size(); ++j) {
        if (core_enc(tc.ec, st.f[j], m) < 0) die("semantics: encode", (long)j);
        const int32_t rc = ak_call_send_enc_q(h, tc.ec, j + 1 == st.f.size(), q, 20 + j);
        ak_completion d;
        const bool g = wait(5000, &d);
        ok = ok && rc == AK_OK && g && d.tag == 20 + j && d.status == AK_OK && d.grpc_status == 0 && d.bytes.len == 0;
      }
      const int32_t r = ak_call_recv_q(h, q, 30);
      ak_completion d;
      const bool g = wait(5000, &d);
      ak_call_destroy(h);
      const bool verdict = g && d.status == AK_OK && d.grpc_status == 0 && d.bytes.len == 40 &&
                           le64(d.bytes.ptr) == st.bytes && std::string((const char *)d.bytes.ptr + 8, 32) == st.sha;
      std::snprintf(buf, sizeof(buf), "%s two moved-encode sends to the checking path: sends ok %d; recv %d, completion %d/%d, %zu B, server's count and SHA-256 match %d",
                    wp.c_str(), (int)ok, r, g ? d.status : -99, g ? d.grpc_status : -99, g ? d.bytes.len : 0, (int)verdict);
      check(ok && r == AK_OK && d.tag == 30 && verdict && !stray(), buf);
      if (g) ak_bytes_free(&d.bytes);
    }
    // cancel a pending recv (the server sleeps 3 s after the stream)
    {
      ak_call *h = open("SleepS");
      ak_completion sa, early, sb, none;
      const int32_t a = ak_call_send_q(h, x, 1, 1, q, 40);
      const bool ga = wait(5000, &sa);
      const int32_t b = ak_call_recv_q(h, q, 41);
      const bool ge = wait(200, &early);
      ak_call_cancel(h);
      const bool gb = wait(5000, &sb);
      const int32_t again = ak_call_recv_q(h, q, 42);
      const bool gn = wait(200, &none);
      ak_call_destroy(h);
      std::snprintf(buf, sizeof(buf), "%s cancel a pending recv: completion before the cancel %d; after it tag %llu status %d/%d (CANCELLED = 1); a second recv %d, a completion after it %d",
                    wp.c_str(), (int)ge, (unsigned long long)sb.tag, sb.status, sb.grpc_status, again, (int)gn);
      check(a == AK_OK && ga && sa.status == AK_OK && b == AK_OK && !ge && gb && sb.tag == 41 &&
                sb.status == AK_ERR_RPC_STATUS && sb.grpc_status == 1 && again == AK_ERR_INVALID_STATE && !gn, buf);
      if (gb) ak_bytes_free(&sb.bytes);
    }
    // cancel a pending send (the server stalls 3 s before reading), and a second send while one is pending
    {
      ak_call *h = open("StallS");
      std::vector<uint8_t> big((size_t)1 << 20, 1);
      long pending = -1;
      int sent = 0;
      for (uint64_t i = 0; i < 64; ++i) {
        if (ak_call_send_q(h, big.data(), big.size(), 0, q, 100 + i) != AK_OK) break;
        ak_completion d;
        if (!wait(300, &d)) { pending = (long)(100 + i); break; }
        if (d.status != AK_OK) break;
        ++sent;
      }
      const int32_t busy = ak_call_send_q(h, x, 1, 0, q, 200);
      ak_call_cancel(h);
      ak_completion sp, sr;
      const bool gp = wait(5000, &sp);
      const int32_t r = ak_call_recv_q(h, q, 201);
      const bool gr = wait(5000, &sr);
      ak_call_destroy(h);
      std::snprintf(buf, sizeof(buf), "%s cancel a pending send (%d 1 MiB sends accepted, then #%ld pending): a second send while pending %d; the pending send's completion tag %llu status %d/%d; recv %d, completion %d/%d",
                    wp.c_str(), sent, pending, busy, (unsigned long long)sp.tag, sp.status, sp.grpc_status, r,
                    gr ? sr.status : -99, gr ? sr.grpc_status : -99);
      check(pending >= 0 && busy == AK_ERR_INVALID_STATE && gp && (long)sp.tag == pending && sp.status == AK_ERR_HOST &&
                sp.grpc_status == -1 && r == AK_OK && gr && sr.tag == 201 && sr.status == AK_ERR_RPC_STATUS &&
                sr.grpc_status == 1 && !stray(), buf);
      if (gp) ak_bytes_free(&sp.bytes);
      if (gr) ak_bytes_free(&sr.bytes);
    }
    // misuse: a send after last (refused, no completion); a blocking recv after this delivery's recv
    {
      ak_call *h = open("EchoS");
      ak_completion sa, d1;
      const int32_t a = ak_call_send_q(h, x, 1, 1, q, 50);
      const bool ga = wait(5000, &sa);
      const int32_t b = ak_call_send_q(h, x, 1, 1, q, 51);
      const int32_t r1 = ak_call_recv_q(h, q, 52);
      const bool g1 = wait(5000, &d1);
      struct ak_bytes out;
      out.ptr = NULL; out.len = 0; out.owner = NULL;
      int32_t gs = -99;
      const int32_t r2 = ak_call_recv(h, &out, &gs);
      const bool gn = stray();
      ak_call_destroy(h);
      std::snprintf(buf, sizeof(buf), "%s misuse: a send after last %d (no completion); recv %d completion %d/%d; a blocking recv after it %d, grpc_status untouched (%d); stray completion %d",
                    wp.c_str(), b, r1, g1 ? d1.status : -99, g1 ? d1.grpc_status : -99, r2, gs, (int)gn);
      check(a == AK_OK && ga && sa.tag == 50 && b == AK_ERR_INVALID_STATE && r1 == AK_OK && g1 && d1.tag == 52 &&
                d1.status == AK_OK && d1.grpc_status == 0 && r2 == AK_ERR_INVALID_STATE && gs == -99 && !gn, buf);
      if (ga) ak_bytes_free(&sa.bytes);
      if (g1) ak_bytes_free(&d1.bytes);
    }
    // ak_call_unary_enc_q: a chosen status, then OK (the request moved out of the context)
    {
      ak_completion d9, d0;
      if (core_enc(tc.ec, w.fac_req, m) < 0) die("semantics: encode P2.2", 0);
      const std::string p9 = pre + "StatusU9";
      ak_call *h = ak_call_unary_enc_q(cl, (const uint8_t *)p9.data(), p9.size(), tc.ec, q, 60);
      const bool g9 = h && wait(5000, &d9);
      if (h) ak_call_destroy(h);
      if (core_enc(tc.ec, w.fac_req, m) < 0) die("semantics: encode P2.2", 0);
      ak_call *h2 = ak_call_unary_enc_q(cl, (const uint8_t *)kPush, std::strlen(kPush), tc.ec, q, 61);
      const bool g0 = h2 && wait(5000, &d0);
      if (h2) ak_call_destroy(h2);
      ak_call *hn = ak_call_unary_enc_q(cl, (const uint8_t *)kPush, std::strlen(kPush), tc.ec, NULL, 62);
      std::snprintf(buf, sizeof(buf), "[%s] ak_call_unary_enc_q: server status 9 -> %d/%d; Push -> %d/%d, %zu B; a NULL queue -> %s",
                    framed ? "framed" : "reference", g9 ? d9.status : -99, g9 ? d9.grpc_status : -99,
                    g0 ? d0.status : -99, g0 ? d0.grpc_status : -99, g0 ? d0.bytes.len : 0, hn ? "a handle" : "NULL");
      check(g9 && d9.tag == 60 && d9.status == AK_ERR_RPC_STATUS && d9.grpc_status == 9 && g0 && d0.tag == 61 &&
                d0.status == AK_OK && d0.grpc_status == 0 && d0.bytes.len == 0 && !hn && !stray(), buf);
      if (g9) ak_bytes_free(&d9.bytes);
      if (g0) ak_bytes_free(&d0.bytes);
    }
    // The CALLBACK delivery (ak_call_*_cb, cell Cf-cb; owner 2026-10-01) on the same send path:
    // every entry returning AK_OK is followed by exactly one completion, run on a core thread; a
    // refusal by none. The completions are collected into a box the checking thread waits on.
    {
      struct CbBox { std::mutex m; std::condition_variable cv; std::deque<ak_completion> d; };
      static CbBox box;  // static: a late completion of a failed case still has a home
      ak_completion_cb cbf = [](void *u, struct ak_completion *c) {
        CbBox &b = *(CbBox *)u;
        std::lock_guard<std::mutex> l(b.m);
        b.d.push_back(*c);
        b.cv.notify_one();
      };
      auto cwait = [&](uint64_t ms, ak_completion *c) {
        std::unique_lock<std::mutex> l(box.m);
        if (!box.cv.wait_for(l, std::chrono::milliseconds(ms), [&] { return !box.d.empty(); })) return false;
        *c = box.d.front();
        box.d.pop_front();
        return true;
      };
      auto cstray = [&]() {
        ak_completion c;
        if (!cwait(200, &c)) return false;
        ak_bytes_free(&c.bytes);
        return true;
      };
      const std::string wc = framed ? "[framed, stream cb]" : "[reference, stream cb]";
      {  // status 6 on a stream
        ak_call *h = open("StatusS6");
        ak_completion sa, sb;
        const int32_t a = ak_call_send_cb(h, x, 1, 1, cbf, &box, 71);
        const bool ga = cwait(5000, &sa);
        const int32_t b = ak_call_recv_cb(h, cbf, &box, 72);
        const bool gb = cwait(5000, &sb);
        ak_call_destroy(h);
        const bool send_ok = ga && sa.tag == 71 && ((sa.status == AK_OK && sa.grpc_status == 0) ||
                                                    (sa.status == AK_ERR_HOST && sa.grpc_status == -1));
        std::snprintf(buf, sizeof(buf), "%s server status 6: send entry %d, completion tag %llu status %d/%d; recv entry %d, completion tag %llu status %d/%d",
                      wc.c_str(), a, (unsigned long long)sa.tag, sa.status, sa.grpc_status, b,
                      (unsigned long long)sb.tag, sb.status, sb.grpc_status);
        check(a == AK_OK && send_ok && b == AK_OK && gb && sb.tag == 72 && sb.status == AK_ERR_RPC_STATUS &&
                  sb.grpc_status == 6 && !cstray(), buf);
        if (ga) ak_bytes_free(&sa.bytes);
        if (gb) ak_bytes_free(&sb.bytes);
      }
      {  // moved-encode sends to the checking path, the server's count and SHA-256
        const Stream &st = w.st[0];
        ak_call *h = open(kUploadStreamCheck);
        bool ok = true;
        for (size_t j = 0; j < st.f.size(); ++j) {
          if (core_enc(tc.ec, st.f[j], m) < 0) die("semantics: encode", (long)j);
          const int32_t rc = ak_call_send_enc_cb(h, tc.ec, j + 1 == st.f.size(), cbf, &box, 80 + j);
          ak_completion d;
          const bool g = cwait(5000, &d);
          ok = ok && rc == AK_OK && g && d.tag == 80 + j && d.status == AK_OK && d.grpc_status == 0 && d.bytes.len == 0;
          if (g) ak_bytes_free(&d.bytes);
        }
        const int32_t r = ak_call_recv_cb(h, cbf, &box, 90);
        ak_completion d;
        const bool g = cwait(5000, &d);
        ak_call_destroy(h);
        const bool verdict = g && d.status == AK_OK && d.grpc_status == 0 && d.bytes.len == 40 &&
                             le64(d.bytes.ptr) == st.bytes && std::string((const char *)d.bytes.ptr + 8, 32) == st.sha;
        std::snprintf(buf, sizeof(buf), "%s moved-encode sends to the checking path: sends ok %d; recv %d, completion %d/%d, %zu B, server's count and SHA-256 match %d",
                      wc.c_str(), (int)ok, r, g ? d.status : -99, g ? d.grpc_status : -99, g ? d.bytes.len : 0, (int)verdict);
        check(ok && r == AK_OK && g && d.tag == 90 && verdict && !cstray(), buf);
        if (g) ak_bytes_free(&d.bytes);
      }
      {  // cancel a pending recv (the server sleeps 3 s after the stream)
        ak_call *h = open("SleepS");
        ak_completion sa, early, sb;
        const int32_t a = ak_call_send_cb(h, x, 1, 1, cbf, &box, 100);
        const bool ga = cwait(5000, &sa);
        const int32_t b = ak_call_recv_cb(h, cbf, &box, 101);
        const bool ge = cwait(200, &early);
        ak_call_cancel(h);
        const bool gb = cwait(5000, &sb);
        const int32_t again = ak_call_recv_cb(h, cbf, &box, 102);
        const bool gn = cstray();
        ak_call_destroy(h);
        std::snprintf(buf, sizeof(buf), "%s cancel a pending recv: completion before the cancel %d; after it tag %llu status %d/%d (CANCELLED = 1); a second recv %d, a completion after it %d",
                      wc.c_str(), (int)ge, (unsigned long long)sb.tag, sb.status, sb.grpc_status, again, (int)gn);
        check(a == AK_OK && ga && sa.status == AK_OK && b == AK_OK && !ge && gb && sb.tag == 101 &&
                  sb.status == AK_ERR_RPC_STATUS && sb.grpc_status == 1 && again == AK_ERR_INVALID_STATE && !gn, buf);
        if (ga) ak_bytes_free(&sa.bytes);
        if (gb) ak_bytes_free(&sb.bytes);
      }
      {  // misuse: a send after last (refused, no completion); a second recv refused
        ak_call *h = open("EchoS");
        ak_completion sa, d1;
        const int32_t a = ak_call_send_cb(h, x, 1, 1, cbf, &box, 110);
        const bool ga = cwait(5000, &sa);
        const int32_t b = ak_call_send_cb(h, x, 1, 1, cbf, &box, 111);
        const int32_t r1 = ak_call_recv_cb(h, cbf, &box, 112);
        const bool g1 = cwait(5000, &d1);
        const int32_t r2 = ak_call_recv_cb(h, cbf, &box, 113);
        const bool gn = cstray();
        ak_call_destroy(h);
        std::snprintf(buf, sizeof(buf), "%s misuse: a send after last %d (no completion); recv %d completion %d/%d; a second recv %d; stray completion %d",
                      wc.c_str(), b, r1, g1 ? d1.status : -99, g1 ? d1.grpc_status : -99, r2, (int)gn);
        check(a == AK_OK && ga && sa.tag == 110 && b == AK_ERR_INVALID_STATE && r1 == AK_OK && g1 && d1.tag == 112 &&
                  d1.status == AK_OK && d1.grpc_status == 0 && r2 == AK_ERR_INVALID_STATE && !gn, buf);
        if (ga) ak_bytes_free(&sa.bytes);
        if (g1) ak_bytes_free(&d1.bytes);
      }
      {  // ak_call_unary_enc_cb: a chosen status, then OK (the request moved out of the context)
        ak_completion d9, d0;
        if (core_enc(tc.ec, w.fac_req, m) < 0) die("semantics: encode P2.2", 0);
        const std::string p9 = pre + "StatusU9";
        ak_call *h = ak_call_unary_enc_cb(cl, (const uint8_t *)p9.data(), p9.size(), tc.ec, cbf, &box, 120);
        const bool g9 = h && cwait(5000, &d9);
        if (h) ak_call_destroy(h);
        if (core_enc(tc.ec, w.fac_req, m) < 0) die("semantics: encode P2.2", 0);
        ak_call *h2 = ak_call_unary_enc_cb(cl, (const uint8_t *)kPush, std::strlen(kPush), tc.ec, cbf, &box, 121);
        const bool g0 = h2 && cwait(5000, &d0);
        if (h2) ak_call_destroy(h2);
        std::snprintf(buf, sizeof(buf), "[%s] ak_call_unary_enc_cb: server status 9 -> %d/%d; Push -> %d/%d, %zu B",
                      framed ? "framed" : "reference", g9 ? d9.status : -99, g9 ? d9.grpc_status : -99,
                      g0 ? d0.status : -99, g0 ? d0.grpc_status : -99, g0 ? d0.bytes.len : 0);
        check(g9 && d9.tag == 120 && d9.status == AK_ERR_RPC_STATUS && d9.grpc_status == 9 && g0 && d0.tag == 121 &&
                  d0.status == AK_OK && d0.grpc_status == 0 && d0.bytes.len == 0 && !cstray(), buf);
        if (g9) ak_bytes_free(&d9.bytes);
        if (g0) ak_bytes_free(&d0.bytes);
      }
    }
    ak_queue_shutdown(q);
    ak_completion z;
    check(ak_queue_next(q, &z, 0) == AK_QUEUE_SHUTDOWN, wp + " the queue drains to AK_QUEUE_SHUTDOWN (nothing left)");
    ak_queue_destroy(q);
    ak_client_destroy(cl);
  }
  // EXPERIMENT (p5-deferred): ak_call_send_deferred, when the core exports it.
  SendDeferredFn sd = (SendDeferredFn)dlsym(RTLD_DEFAULT, "ak_call_send_deferred");
  if (!sd) {
    std::printf("SKIP deferred send: this core does not export ak_call_send_deferred\n");
  } else {
    char buf[400];
    ak_client *fr = core_client(w.rt, w.cfg.core_target, w.cfg.transport, true);
    ak_client *rf = core_client(w.rt, w.cfg.core_target, w.cfg.transport, false);
    if (!fr || !rf) die("semantics: deferred clients", 0);
    auto open_on = [&](ak_client *cl, const char *p) {
      ak_call *h = ak_call_open(cl, (const uint8_t *)p, std::strlen(p), AK_CALL_CLIENT_STREAM, NULL);
      if (!h) die("semantics: ak_call_open", 0);
      return h;
    };
    // the server's count and SHA-256 of every message as received, both waits, both payloads
    for (int wt = 1; wt >= 0; --wt)
      for (int pi = 0; pi < 2; ++pi) {
        const Stream &st = w.st[pi];
        ak_call *h = open_on(fr, kUploadStreamCheck);
        DefJob jobs[16];
        bool ok = true;
        for (size_t j = 0; j < st.f.size(); ++j) {
          jobs[j].v = &st.f[j]; jobs[j].m = m; jobs[j].fail = 0;
          ok = ok && sd(h, tc.ec, def_encode, &jobs[j], j + 1 == st.f.size(), wt) == AK_OK;
        }
        struct ak_bytes out; out.ptr = NULL; out.len = 0; out.owner = NULL;
        int32_t gs = -1;
        const int32_t r = ak_call_recv(h, &out, &gs);
        const bool verdict = r == AK_OK && gs == 0 && out.len == 40 && le64(out.ptr) == st.bytes &&
                             std::string((const char *)out.ptr + 8, 32) == st.sha;
        std::snprintf(buf, sizeof(buf), "[framed, deferred wait=%d] %zu deferred sends (%s) to the checking path: sends ok %d; recv %d/%d, %zu B, the server's count and SHA-256 match %d",
                      wt, st.f.size(), pi ? "16 MiB" : "4 MiB", (int)ok, r, gs, out.len, (int)verdict);
        check(ok && verdict, buf);
        ak_bytes_free(&out);
        ak_call_destroy(h);
      }
    // an encode that fails: wait = 1 returns its code; wait = 0 is queued, and the call does not
    // complete as a full upload
    for (int wt = 1; wt >= 0; --wt) {
      const Stream &st = w.st[0];
      ak_call *h = open_on(fr, kUploadStreamCheck);
      DefJob j0; j0.v = &st.f[0]; j0.m = m; j0.fail = -77;
      const int32_t a = sd(h, tc.ec, def_encode, &j0, 1, wt);
      struct ak_bytes out; out.ptr = NULL; out.len = 0; out.owner = NULL;
      int32_t gs = -1;
      const int32_t r = ak_call_recv(h, &out, &gs);
      const bool full = r == AK_OK && out.len >= 8 && le64(out.ptr) == st.bytes;
      std::snprintf(buf, sizeof(buf), "[framed, deferred wait=%d] an encode returning -77: send %d (want %d); recv %d/%d, %zu B, a full upload reported %d",
                    wt, a, wt ? -77 : AK_OK, r, gs, out.len, (int)full);
      check(a == (wt ? -77 : AK_OK) && !full, buf);
      ak_bytes_free(&out);
      ak_call_destroy(h);
    }
    // misuse: a send after last; a deferred send on the reference path (framed streams only)
    {
      const Stream &st = w.st[0];
      ak_call *h = open_on(fr, kUploadStreamCheck);
      DefJob jobs[2];
      for (int j = 0; j < 2; ++j) { jobs[j].v = &st.f[j]; jobs[j].m = m; jobs[j].fail = 0; }
      const int32_t a = sd(h, tc.ec, def_encode, &jobs[0], 1, 1);
      const int32_t b = sd(h, tc.ec, def_encode, &jobs[1], 1, 1);
      struct ak_bytes out; out.ptr = NULL; out.len = 0; out.owner = NULL;
      int32_t gs = -1;
      ak_call_recv(h, &out, &gs);
      ak_bytes_free(&out);
      ak_call_destroy(h);
      ak_call *g = open_on(rf, kUploadStreamCheck);
      const int32_t c2 = sd(g, tc.ec, def_encode, &jobs[0], 1, 1);
      ak_call_cancel(g);
      ak_call_destroy(g);
      std::snprintf(buf, sizeof(buf), "[deferred misuse] send after last %d (want %d); on the reference path %d (want %d)",
                    b, AK_ERR_INVALID_STATE, c2, AK_ERR_INVALID_STATE);
      check(a == AK_OK && b == AK_ERR_INVALID_STATE && c2 == AK_ERR_INVALID_STATE, buf);
    }
    ak_client_destroy(fr);
    ak_client_destroy(rf);
  }
  // EXPERIMENT (p6-zero-copy): ak_enc_set_zc + ak_call_send_enc_zc, when the core exports them.
  SetZcFn szc = (SetZcFn)dlsym(RTLD_DEFAULT, "ak_enc_set_zc");
  SendZcFn snd = (SendZcFn)dlsym(RTLD_DEFAULT, "ak_call_send_enc_zc");
  if (!szc || !snd) {
    std::printf("SKIP zero-copy send: this core does not export ak_enc_set_zc / ak_call_send_enc_zc\n");
  } else {
    char buf[400];
    ak_client *fr = core_client(w.rt, w.cfg.core_target, w.cfg.transport, true);
    ak_client *rf = core_client(w.rt, w.cfg.core_target, w.cfg.transport, false);
    ak_enc_ctx *ze = ak_enc_ctx_new();
    if (!fr || !rf || !ze || szc(ze, kZcMin) != AK_OK) die("semantics: zero-copy setup", 0);
    auto open_on = [&](ak_client *cl, const std::string &t) {
      const std::string p = t[0] == '/' ? t : pre + t;
      ak_call *h = ak_call_open(cl, (const uint8_t *)p.data(), p.size(), AK_CALL_CLIENT_STREAM, NULL);
      if (!h) die("semantics: ak_call_open", 0);
      return h;
    };
    auto released = [&](std::atomic<long> &r, long want) {  // wait up to 3 s for the releases
      for (int i = 0; i < 3000 && r.load() < want; ++i) std::this_thread::sleep_for(std::chrono::milliseconds(1));
      std::this_thread::sleep_for(std::chrono::milliseconds(20));
      return r.load();
    };
    // the server's count and SHA-256 of every message as received; one release per message
    for (int pi = 0; pi < 2; ++pi) {
      const Stream &st = w.st[pi];
      std::atomic<long> rel{0};
      ak_call *h = open_on(fr, kUploadStreamCheck);
      bool ok = true;
      for (size_t j = 0; j < st.f.size(); ++j) {
        ok = ok && core_enc(ze, st.f[j], m) >= 0 && snd(h, ze, j + 1 == st.f.size(), zc_release, &rel) == AK_OK;
      }
      struct ak_bytes out; out.ptr = NULL; out.len = 0; out.owner = NULL;
      int32_t gs = -1;
      const int32_t r = ak_call_recv(h, &out, &gs);
      const bool verdict = r == AK_OK && gs == 0 && out.len == 40 && le64(out.ptr) == st.bytes &&
                           std::string((const char *)out.ptr + 8, 32) == st.sha;
      ak_bytes_free(&out);
      ak_call_destroy(h);
      const long got = released(rel, (long)st.f.size());
      std::snprintf(buf, sizeof(buf), "[framed, zero-copy] %zu borrowed sends (%s) to the checking path: sends ok %d; recv %d/%d, the server's count and SHA-256 match %d; releases %ld (want %zu)",
                    st.f.size(), pi ? "16 MiB" : "4 MiB", (int)ok, r, gs, (int)verdict, got, st.f.size());
      check(ok && verdict && got == (long)st.f.size(), buf);
    }
    // a cancelled call releases: one message sent (not last), the call cancelled while it waits
    {
      const Stream &st = w.st[0];
      std::atomic<long> rel{0};
      ak_call *h = open_on(fr, kUploadStreamCheck);
      const bool ok = core_enc(ze, st.f[0], m) >= 0 && snd(h, ze, 0, zc_release, &rel) == AK_OK;
      ak_call_cancel(h);
      struct ak_bytes out; out.ptr = NULL; out.len = 0; out.owner = NULL;
      int32_t gs = -1;
      const int32_t r = ak_call_recv(h, &out, &gs);
      ak_bytes_free(&out);
      ak_call_destroy(h);
      const long got = released(rel, 1);
      std::snprintf(buf, sizeof(buf), "[framed, zero-copy] cancel after one borrowed send: send ok %d; recv %d/%d (want CANCELLED 1); releases %ld (want 1)",
                    (int)ok, r, gs, got);
      check(ok && gs == 1 && got == 1, buf);
    }
    // a failed call releases: the server answers status 6 on the stream
    {
      const Stream &st = w.st[0];
      std::atomic<long> rel{0};
      ak_call *h = open_on(fr, "StatusS6");
      bool ok = true;
      long sent = 0;
      for (size_t j = 0; j < st.f.size(); ++j) {
        if (core_enc(ze, st.f[j], m) < 0) { ok = false; break; }
        const int32_t rc = snd(h, ze, j + 1 == st.f.size(), zc_release, &rel);
        if (rc == AK_OK || rc == AK_ERR_HOST) ++sent;  // AK_ERR_HOST: the call had already ended
        ok = ok && (rc == AK_OK || rc == AK_ERR_HOST);
      }
      struct ak_bytes out; out.ptr = NULL; out.len = 0; out.owner = NULL;
      int32_t gs = -1;
      const int32_t r = ak_call_recv(h, &out, &gs);
      ak_bytes_free(&out);
      ak_call_destroy(h);
      const long got = released(rel, sent);
      std::snprintf(buf, sizeof(buf), "[framed, zero-copy] server status 6: %ld borrowed sends accepted or refused as ended, ok %d; recv %d/%d (want 6); releases %ld (want %ld)",
                    sent, (int)ok, r, gs, got, sent);
      check(ok && gs == 6 && got == sent, buf);
    }
    // misuse: the reference path refuses, and takes nothing (no release)
    {
      const Stream &st = w.st[0];
      std::atomic<long> rel{0};
      ak_call *h = open_on(rf, kUploadStreamCheck);
      core_enc(ze, st.f[0], m);
      const int32_t a = snd(h, ze, 1, zc_release, &rel);
      ak_call_cancel(h);
      ak_call_destroy(h);
      ak_enc_reset(ze);
      const long got = released(rel, 1);
      std::snprintf(buf, sizeof(buf), "[zero-copy misuse] on the reference path %d (want %d); releases %ld (the send took nothing)",
                    a, AK_ERR_INVALID_STATE, got);
      check(a == AK_ERR_INVALID_STATE, buf);
    }
    // EXPERIMENT (p7-deferred-zc): ak_call_send_deferred_zc, when the core exports it
    SendDeferredZcFn sdz = (SendDeferredZcFn)dlsym(RTLD_DEFAULT, "ak_call_send_deferred_zc");
    if (!sdz) {
      std::printf("SKIP deferred zero-copy send: this core does not export ak_call_send_deferred_zc\n");
    } else {
      for (int wt = 1; wt >= 0; --wt)
        for (int pi = 0; pi < 2; ++pi) {
          const Stream &st = w.st[pi];
          std::atomic<long> rel{0};
          ak_call *h = open_on(fr, kUploadStreamCheck);
          DefJob jobs[16];
          bool ok = true;
          for (size_t j = 0; j < st.f.size(); ++j) {
            jobs[j].v = &st.f[j]; jobs[j].m = m; jobs[j].fail = 0;
            ok = ok && sdz(h, ze, def_encode, &jobs[j], j + 1 == st.f.size(), wt, zc_release, &rel) == AK_OK;
          }
          struct ak_bytes out; out.ptr = NULL; out.len = 0; out.owner = NULL;
          int32_t gs = -1;
          const int32_t r = ak_call_recv(h, &out, &gs);
          const bool verdict = r == AK_OK && gs == 0 && out.len == 40 && le64(out.ptr) == st.bytes &&
                               std::string((const char *)out.ptr + 8, 32) == st.sha;
          ak_bytes_free(&out);
          ak_call_destroy(h);
          const long got = released(rel, (long)st.f.size());
          std::snprintf(buf, sizeof(buf), "[framed, deferred zero-copy wait=%d] %zu sends (%s) to the checking path: sends ok %d; recv %d/%d, the server's count and SHA-256 match %d; releases %ld (want %zu)",
                        wt, st.f.size(), pi ? "16 MiB" : "4 MiB", (int)ok, r, gs, (int)verdict, got, st.f.size());
          check(ok && verdict && got == (long)st.f.size(), buf);
        }
      // cancel after one encoded (wait = 1) borrowed message: released once
      {
        const Stream &st = w.st[0];
        std::atomic<long> rel{0};
        ak_call *h = open_on(fr, kUploadStreamCheck);
        DefJob j0; j0.v = &st.f[0]; j0.m = m; j0.fail = 0;
        const int32_t a = sdz(h, ze, def_encode, &j0, 0, 1, zc_release, &rel);
        ak_call_cancel(h);
        struct ak_bytes out; out.ptr = NULL; out.len = 0; out.owner = NULL;
        int32_t gs = -1;
        const int32_t r = ak_call_recv(h, &out, &gs);
        ak_bytes_free(&out);
        ak_call_destroy(h);
        const long got = released(rel, 1);
        std::snprintf(buf, sizeof(buf), "[framed, deferred zero-copy] cancel after one encoded send (wait=1): send %d; recv %d/%d (want CANCELLED 1); releases %ld (want 1)",
                      a, r, gs, got);
        check(a == AK_OK && gs == 1 && got == 1, buf);
      }
      // a failed call (server status 6): every encoded message released
      {
        const Stream &st = w.st[0];
        std::atomic<long> rel{0};
        ak_call *h = open_on(fr, "StatusS6");
        DefJob jobs[2];
        long enc_ok = 0;
        for (size_t j = 0; j < st.f.size(); ++j) {
          jobs[j].v = &st.f[j]; jobs[j].m = m; jobs[j].fail = 0;
          const int32_t rc = sdz(h, ze, def_encode, &jobs[j], j + 1 == st.f.size(), 1, zc_release, &rel);
          if (rc == AK_OK) ++enc_ok;
        }
        struct ak_bytes out; out.ptr = NULL; out.len = 0; out.owner = NULL;
        int32_t gs = -1;
        const int32_t r = ak_call_recv(h, &out, &gs);
        ak_bytes_free(&out);
        ak_call_destroy(h);
        const long got = released(rel, enc_ok);
        std::snprintf(buf, sizeof(buf), "[framed, deferred zero-copy] server status 6: %ld sends encoded (wait=1); recv %d/%d (want 6); releases %ld (want %ld)",
                      enc_ok, r, gs, got, enc_ok);
        check(gs == 6 && got == enc_ok, buf);
      }
      // a failing encode: no borrowed message, no release; the reference path refuses
      {
        const Stream &st = w.st[0];
        std::atomic<long> rel{0};
        ak_call *h = open_on(fr, kUploadStreamCheck);
        DefJob j0; j0.v = &st.f[0]; j0.m = m; j0.fail = -77;
        const int32_t a = sdz(h, ze, def_encode, &j0, 1, 1, zc_release, &rel);
        struct ak_bytes out; out.ptr = NULL; out.len = 0; out.owner = NULL;
        int32_t gs = -1;
        ak_call_recv(h, &out, &gs);
        ak_bytes_free(&out);
        ak_call_destroy(h);
        ak_call *g = open_on(rf, kUploadStreamCheck);
        DefJob j1; j1.v = &st.f[0]; j1.m = m; j1.fail = 0;
        const int32_t b = sdz(g, ze, def_encode, &j1, 1, 1, zc_release, &rel);
        ak_call_cancel(g);
        ak_call_destroy(g);
        ak_enc_reset(ze);
        const long got = released(rel, 1);
        std::snprintf(buf, sizeof(buf), "[deferred zero-copy misuse] a failing encode returns %d (want -77); the reference path %d (want %d); releases %ld (want 0)",
                      a, b, AK_ERR_INVALID_STATE, got);
        check(a == -77 && b == AK_ERR_INVALID_STATE && got == 0, buf);
      }
    }
    ak_enc_ctx_free(ze);
    ak_client_destroy(fr);
    ak_client_destroy(rf);
  }
  std::printf("# {\"campaign_rpc_semantics\": {\"build\": \"%s\", \"checks\": %d, \"failed\": %d}}\n", kBuild, n, bad);
  return bad ? 1 : 0;
}

// Direction d's upload of `chunks` x 2 MiB (req. 14): deterministic data (splitmix64 from a
// seed of the chunk count, as the Rust slice), the ids on the first message only.
void make_stream(Stream *st, int chunks) {
  uint64_t seed = 0x5EED0000ull + (uint64_t)chunks;
  aksha::Sha256 h;
  for (int i = 0; i < chunks; ++i) {
    std::string data;
    data.resize(kChunk);
    for (size_t o = 0; o < kChunk; o += 8) {
      seed += 0x9E3779B97F4A7C15ull;
      uint64_t z = seed;
      z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ull;
      z = (z ^ (z >> 27)) * 0x94D049BB133111EBull;
      z ^= z >> 31;
      for (int b = 0; b < 8; ++b) data[o + b] = (char)(uint8_t)(z >> (8 * b));
    }
    Pb5 p;
    Fac5 f;
    shapes::UploadResultData &u = f.upload.emplace();
    if (i == 0) {
      p.mutable_upload()->set_session_id("session-u2");
      p.mutable_upload()->set_result_id("result-u2");
      u.session_id = "session-u2";
      u.result_id = "result-u2";
    }
    p.mutable_upload()->set_data_chunk(data);
    u.data_chunk = data;
    std::string wire;
    p.SerializeToString(&wire);
    h.update((const uint8_t *)wire.data(), wire.size());
    st->p.push_back(p);
    st->f.push_back(f);
    st->wire.push_back(wire);
  }
  st->bytes = (uint64_t)chunks * kChunk;
  uint8_t d[32];
  h.final(d);
  st->sha.assign((const char *)d, 32);
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
    else if (a == "--core-target") c.core_target = v;
    else if (a == "--transport") c.transport = v;
    else if (a == "--cells") c.cells = v;
    else if (a == "--dirs") c.dirs = v;
    else if (a == "--inflight") c.inflight = parse_list(v);
    else if (a == "--launch") c.launch = std::atoi(v);
    else if (a == "--rounds") c.rounds = std::atoi(v);
    else if (a == "--calls") c.calls = std::atoi(v);
    else if (a == "--workers") c.workers = std::atoi(v);
    else if (a == "--expect") w.expect_a = (size_t)std::atoll(v);
    else if (a == "--fail-after") c.fail_after = std::atoi(v);
    else if (a == "--warmup-s") c.warmup_s = std::atof(v);
    else if (a == "--min-time-s") c.min_time_s = std::atof(v);
    else if (a == "--gbench-out") c.gbout = v;
    else if (a == "--count") c.count = std::atoi(v);
    else if (a == "--alloc-probe") c.alloc_probe = std::atoi(v);
    else if (a == "--plant") c.plant = v;
    else if (a == "--semantics") c.semantics = std::atoi(v);
    else if (a == "--payloads") c.payloads = v;
    else if (a == "--iters") c.iters = v;
    else if (a == "--profile") c.profile = std::atoi(v);
    else if (a == "--check-stream") g_check_stream = std::atoi(v) != 0;
    else if (a == "--profile-cell") c.profile_cell = v;
    else if (a == "--perf-ctl") c.perf_ctl = v;
    else if (a == "--profile-chunks") c.profile_chunks = std::atoi(v);
    else { std::fprintf(stderr, "unknown option %s\n", a.c_str()); return 2; }
  }
  if (c.core_target.empty()) c.core_target = c.target;
  if (c.target.empty() || (c.transport != "shipped" && c.transport != "pinned") || !w.expect_a) {
    std::fprintf(stderr, "usage: campaign_rpc --target unix:PATH --expect BYTES --transport shipped|pinned ...\n");
    return 2;
  }
  // D14 (2026-10-03): the core runtime's worker count is AK_WORKERS (campaign.machine: the set's
  // 8 threads), overriding --workers; default 8
  if (const char *wv = std::getenv("AK_WORKERS")) c.workers = std::atoi(wv);
  w.rt = ak_runtime_new((uint32_t)c.workers);
  pbbuild::payload_p2_2(&w.pb_req);
  w.fac_req = shapes::build::payload_p2_2();
  // Directions c and d (req. 14, required 2026-09-27).
  pbbuild::payload_p5_3(&w.pb_up[0]);
  pbbuild::payload_p5_4(&w.pb_up[1]);
  w.fac_up[0] = shapes::build::payload_p5_3();
  w.fac_up[1] = shapes::build::payload_p5_4();
  for (int i = 0; i < 2; ++i) w.pb_up[i].SerializeToString(&w.wire_up[i]);
  if (!aksha::sha256_selftest()) die("SHA-256 self-test (FIPS 180-2 vectors)", 0);
  make_stream(&w.st[0], 2);
  make_stream(&w.st[1], 8);
  // Req. 18 controls (the gate's): each must abort the run with no sample.
  if (c.plant == "c-len") w.want_c_len = 1;
  else if (c.plant == "d-count") { w.st[0].bytes += 1; w.st[1].bytes += 1; }
  else if (c.plant == "d-sha") { w.st[0].sha[0] ^= 1; w.st[1].sha[0] ^= 1; }
  else if (!c.plant.empty()) die("unknown --plant", 0);
  if (c.semantics) {
    const int r = q_semantics(w);
    ak_runtime_destroy(w.rt);
    std::fflush(stdout);
    return r;
  }
  for (char d : c.dirs) {
    if (d == 'c' || d == 'd') { w.jobs.push_back(Job{d, 0}); w.jobs.push_back(Job{d, 1}); }
    else if (d == 'a' || d == 'r' || d == 'b') w.jobs.push_back(Job{d, 0});
    else die("unknown direction", d);
  }
  if (!c.payloads.empty()) {  // --payloads: keep the (direction, payload) jobs named
    std::vector<Job> keep;
    const std::string want = "," + c.payloads + ",";
    for (size_t i = 0; i < w.jobs.size(); ++i)
      if (want.find(std::string(",") + job_payload(w.jobs[i]) + ",") != std::string::npos) keep.push_back(w.jobs[i]);
    if (keep.empty()) die("--payloads selects no job", 0);
    w.jobs = keep;
  }
  std::map<std::string, long> fixed_iters;  // --iters [PAYLOAD/]K:N, keyed "K" or "PAYLOAD/K"
  for (size_t p = 0; !c.iters.empty() && p <= c.iters.size();) {
    size_t q = c.iters.find(',', p);
    std::string t = c.iters.substr(p, q == std::string::npos ? std::string::npos : q - p);
    size_t colon = t.find(':');
    if (colon == std::string::npos || colon == 0 || std::atol(t.c_str() + colon + 1) <= 0) die("--iters [PAYLOAD/]K:N,...", 0);
    fixed_iters[t.substr(0, colon)] = std::atol(t.c_str() + colon + 1);
    if (q == std::string::npos) break;
    p = q + 1;
  }

  w.cells = parse_cells(c.cells);
  for (size_t i = 0; i < w.cells.size(); ++i) {
    const Cell &cl = w.cells[i];
    bool coded = cl.base >= 'C' && cl.base <= 'F';
    if (std::string("ABCDEF").find(cl.base) == std::string::npos || coded != (cl.mode != kDefault)
        || (cl.framed && cl.base != 'B' && cl.base != 'C' && cl.base != 'E')
        || (cl.pull && cl.base != 'C' && cl.base != 'D')
        || (cl.q && (cl.pull || (cl.base != 'A' && cl.base != 'B' && cl.base != 'C' && cl.base != 'E')))
        || (cl.cb && (cl.pull || cl.q || !(cl.base == 'A' || (cl.base == 'C' && cl.framed))))
        || ((cl.deferred || cl.zc) && (cl.base != 'C' || !cl.framed || cl.q || cl.pull))
#ifdef AK_NO_UNKNOWN_FIELDS
        || cl.mode == kRetain || cl.mode == kDrop
#else
        || cl.mode == kNoUnk
#endif
        )
      die("unknown cell label", (long)i);
    if (cl.zc && cl.deferred && !g_send_deferred_zc) {
      g_set_zc = (SetZcFn)dlsym(RTLD_DEFAULT, "ak_enc_set_zc");
      g_send_deferred_zc = (SendDeferredZcFn)dlsym(RTLD_DEFAULT, "ak_call_send_deferred_zc");
      if (!g_set_zc || !g_send_deferred_zc) {
        std::fprintf(stderr, "REFUSED: cell %s needs a core that exports ak_enc_set_zc and ak_call_send_deferred_zc (patch p7-deferred-zc); this one does not\n",
                     cl.label.c_str());
        return 2;
      }
    }
    if (cl.deferred && !cl.zc && !g_send_deferred) {
      g_send_deferred = (SendDeferredFn)dlsym(RTLD_DEFAULT, "ak_call_send_deferred");
      if (!g_send_deferred) {
        std::fprintf(stderr, "REFUSED: cell %s needs a core that exports ak_call_send_deferred (patch p5-deferred); this one does not\n",
                     cl.label.c_str());
        return 2;
      }
    }
    if (cl.zc && !cl.deferred && !g_send_zc) {
      g_set_zc = (SetZcFn)dlsym(RTLD_DEFAULT, "ak_enc_set_zc");
      g_send_zc = (SendZcFn)dlsym(RTLD_DEFAULT, "ak_call_send_enc_zc");
      if (!g_set_zc || !g_send_zc) {
        std::fprintf(stderr, "REFUSED: cell %s needs a core that exports ak_enc_set_zc and ak_call_send_enc_zc (patch p6-zero-copy); this one does not\n",
                     cl.label.c_str());
        return 2;
      }
    }
    if (cl.cb || (cl.q && cl.base == 'A')) {
      for (size_t j = 0; j < w.jobs.size(); ++j)
        if (w.jobs[j].dir != 'c' && w.jobs[j].dir != 'd') {
          std::fprintf(stderr, "REFUSED: cell %s runs directions c and d only (--dirs c, d or cd)\n", cl.label.c_str());
          return 2;
        }
    }
    if (cl.deferred || cl.zc) {
      for (size_t j = 0; j < w.jobs.size(); ++j)
        if (w.jobs[j].dir != 'd') {
          std::fprintf(stderr, "REFUSED: cell %s runs direction d only (--dirs d)\n", cl.label.c_str());
          return 2;
        }
    }
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
      for (int s = 0; s < maxk; ++s) cn.stubs.emplace_back(gridns::Grid::NewStub(cn.chan));
      typedef grpc::internal::RpcMethod RM;
      cn.m_fetch.reset(new RM(kFetch, RM::NORMAL_RPC, cn.chan));
      cn.m_push.reset(new RM(kPush, RM::NORMAL_RPC, cn.chan));
      cn.m_upload.reset(new RM(kUpload, RM::NORMAL_RPC, cn.chan));
      cn.m_stream.reset(new RM(kUploadStream, RM::CLIENT_STREAMING, cn.chan));
      cn.m_stream_check.reset(new RM(kUploadStreamCheck, RM::CLIENT_STREAMING, cn.chan));
      if (w.cells[i].q) cn.gcq.reset(new grpc::CompletionQueue());
    } else {
      cn.cl = core_client(w.rt, c.core_target, c.transport, w.cells[i].framed);
      if (!cn.cl) die("ak_client_new", (long)i);
      if (w.cells[i].q && !(cn.q = ak_queue_new())) die("ak_queue_new", (long)i);
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
      ak_client *pc = grpc_cell(cl.base) ? core_client(w.rt, c.core_target, c.transport) : w.conns[i].cl;
      struct ak_bytes out;
      out.ptr = NULL; out.len = 0; out.owner = NULL;
      static const uint8_t kNone[1] = {0};
      if (ak_call_unary(pc, (const uint8_t *)kFetch, std::strlen(kFetch), kNone, 0, &out, NULL) != AK_OK ||
          out.len != w.expect_a)
        die("pre-check fetch", (long)out.len);
      std::string wire((const char *)out.ptr, out.len);
      ak_bytes_free(&out);
      if (grpc_cell(cl.base)) ak_client_destroy(pc);
      Fac f;
      int32_t drc = codec_decode(cl.base, cl.mode, tc, (const uint8_t *)wire.data(), wire.size(), &f, cl.pull);
      if (drc != 0) die("pre-check decode", drc);
      const uint8_t *q = NULL;
      size_t qn = 0;
      if (codec_encode(cl.base, cl.mode, tc, f, &q, &qn) != 0) die("pre-check re-encode", (long)i);
      std::string ours((const char *)q, qn);
      Pb inc, back;
      if (!inc.ParseFromString(wire) || !back.ParseFromString(ours)) die("pre-check incumbent parse", 0);
      if (det(inc) != det(back)) die("pre-check: the re-encode differs from the incumbent's", (long)i);
      if ((long)f.tasks.size() != inc.tasks_size()) die("pre-check task count", (long)f.tasks.size());
      // Directions c and d: every request message this cell's codec sends is byte-identical
      // to protobuf's (M5 has one canonical form), before any call.
      for (int pi = 0; pi < 2; ++pi) {
        if (core_codec(cl.base)) {
          const uint8_t *b = NULL; size_t bn = 0;
          if (core_enc(tc.ec, w.fac_up[pi], cl.mode) < 0 || ak_enc_take(tc.ec, &b, &bn) != AK_OK ||
              std::string((const char *)b, bn) != w.wire_up[pi])
            die("pre-check: c request differs from protobuf's", (long)i);
          for (size_t k = 0; k < w.st[pi].f.size(); ++k)
            if (core_enc(tc.ec, w.st[pi].f[k], cl.mode) < 0 || ak_enc_take(tc.ec, &b, &bn) != AK_OK ||
                std::string((const char *)b, bn) != w.st[pi].wire[k])
              die("pre-check: d chunk differs from protobuf's", (long)k);
        } else {
          ak::Enc *e = hg_enc(tc, w.fac_up[pi], cl.mode);
          if (e->err || std::string((const char *)e->data(), e->size()) != w.wire_up[pi])
            die("pre-check: c request differs from protobuf's", (long)i);
          for (size_t k = 0; k < w.st[pi].f.size(); ++k) {
            e = hg_enc(tc, w.st[pi].f[k], cl.mode);
            if (e->err || std::string((const char *)e->data(), e->size()) != w.st[pi].wire[k])
              die("pre-check: d chunk differs from protobuf's", (long)k);
          }
        }
      }
    }
  }
  // Direction d's digest (req. 18, SERVER.md): one UploadStreamCheck per cell and payload,
  // before any benchmark, the server's count and SHA-256 of the messages as received against
  // the client's own. Never timed (the timed d calls UploadStream, whose count is checked).
  if (std::string(c.dirs).find('d') != std::string::npos) {
    ThreadCtx tc;
    for (size_t i = 0; i < w.cells.size(); ++i)
      if (!w.cells[i].pull)  // the pull twins run a and a+read only
        for (int pi = 0; pi < 2; ++pi) stream_call(w, i, pi, 0, tc, true);
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
  if (c.alloc_probe > 0) {
    // R-1 / HG-5 probe: the big allocations (>= AK_PROBE_MIN, default 1 MiB) per call of every
    // cell in directions c and d, counted by the preloaded gen/allocprobe.so. Nothing is timed.
    typedef unsigned long (*probe_fn)(void);
    probe_fn pf = (probe_fn)dlsym(RTLD_DEFAULT, "akprobe_big_allocs");
    if (!pf) { std::fprintf(stderr, "--alloc-probe needs LD_PRELOAD=gen/allocprobe.so\n"); return 2; }
    ThreadCtx tc;
    for (size_t ci = 0; ci < w.cells.size(); ++ci)
      for (size_t ji = 0; ji < w.jobs.size(); ++ji) {
        const Job &j = w.jobs[ji];
        if (j.dir != 'c' && j.dir != 'd') continue;
        cell_call(w, ci, j, 0, tc);  // one warm call first: steady state only
        unsigned long b0 = pf();
        for (int i = 0; i < c.alloc_probe; ++i) cell_call(w, ci, j, 0, tc);
        unsigned long b1 = pf();
        std::printf("  %-10s %-2s %-5s big allocations per call %6.2f\n", w.cells[ci].label.c_str(),
                    dir_label(j.dir), job_payload(j), (double)(b1 - b0) / c.alloc_probe);
      }
    return 0;
  }
  if (c.count > 0) { std::fprintf(stderr, "--count needs a counting build\n"); return 2; }
#endif

  // Directions c and d run at 1 and 8 in flight only (req. 14).
  auto job_runs = [&](const Job &j, int k) { return (j.dir != 'c' && j.dir != 'd') || k == 1 || k == 8; };
  Pool pool(w, maxk);  // R-H2: the caller threads, created before any timed window
#ifdef AK_COUNTING
  (void)job_runs;
  std::fprintf(stderr, "a counting build does not time\n");
  return 2;
#else
  if (c.profile > 0) {
    // Physical probe step 4a: ONE cell, ONE job, one k, c.profile batches on the pre-created
    // threads (the benchmark's own path, Pool::batch), no Google Benchmark. perf is enabled only
    // around the loop (--perf-ctl), a marker write to /dev/null brackets it for strace, and the
    // loop reports process CPU and wall per chunk, per-thread CPU by class and getrusage deltas.
    size_t pc = 0;
    for (size_t i = 0; i < w.cells.size(); ++i)
      if (w.cells[i].label == c.profile_cell) pc = i;
    if (!c.profile_cell.empty() && w.cells[pc].label != c.profile_cell) die("--profile-cell names no cell", 0);
    if (w.jobs.size() != 1 || c.inflight.size() != 1) die("--profile needs one job (--dirs/--payloads) and one k", 0);
    const int k = c.inflight[0];
    for (int i = 0; i < 5; ++i) pool.batch(pc, 0, k, k);  // warm: steady state only
    const std::string tcp_socks = tcp_sockets();  // after the warm batches: every connection open
    // --perf-ctl CTL,ACK[;CTL,ACK...]: several perf sessions (the client's, one attached to the
    // server) enabled and disabled together around the loop
    std::vector<std::pair<int, int> > ctls;
    for (size_t p0 = 0; !c.perf_ctl.empty() && p0 <= c.perf_ctl.size();) {
      const size_t semi = c.perf_ctl.find(';', p0);
      const std::string one = c.perf_ctl.substr(p0, semi == std::string::npos ? std::string::npos : semi - p0);
      const size_t comma = one.find(',');
      const int ctl = open(one.substr(0, comma).c_str(), O_WRONLY);
      const int ack = comma != std::string::npos ? open(one.substr(comma + 1).c_str(), O_RDONLY) : -1;
      if (ctl < 0) die("--perf-ctl: open", errno);
      ctls.push_back(std::make_pair(ctl, ack));
      if (semi == std::string::npos) break;
      p0 = semi + 1;
    }
    auto perf_cmd = [&](const char *cmd) {
      for (size_t i = 0; i < ctls.size(); ++i) {
        if (write(ctls[i].first, cmd, std::strlen(cmd)) < 0) die("--perf-ctl: write", errno);
        char buf[16];
        if (ctls[i].second >= 0 && read(ctls[i].second, buf, sizeof(buf)) <= 0) die("--perf-ctl: ack", errno);
      }
    };
    const int devnull = open("/dev/null", O_WRONLY);
    const int chunks = c.profile_chunks > 0 && c.profile_chunks <= c.profile ? c.profile_chunks : 1;
    std::vector<double> ccpu, cwall;
    std::vector<int> cbat;
    // With LD_PRELOAD=gen/allocprobe.so: the allocations of at least AK_PROBE_MIN bytes (1 MiB)
    // made during the loop, process-wide (a separate process from the timed ones: the probe wraps malloc).
    typedef unsigned long (*probe_fn)(void);
    probe_fn big = (probe_fn)dlsym(RTLD_DEFAULT, "akprobe_big_allocs");
    const unsigned long big0 = big ? big() : 0;
    std::map<long, std::pair<std::string, unsigned long long> > t0 = thread_times();
    const std::map<long, unsigned long long> w0 = g_wait_ns;
    const std::map<long, std::pair<unsigned long long, unsigned long long> > csw0 = g_csw;
    const long spid = std::getenv("AK_SERVER_PID") ? std::atol(std::getenv("AK_SERVER_PID")) : 0;
    const std::map<long, std::pair<unsigned long long, unsigned long long> > s0 = server_times(spid);
    const double loop0 = now_ns(CLOCK_MONOTONIC);
    for (size_t i = 0; i < pool.seats.size(); ++i) pool.seats[i]->tr.clear();
    pool.btr.clear();
    g_trace_calls.store(true);
    struct rusage r0, r1;
    getrusage(RUSAGE_SELF, &r0);
    const std::set<int> ccpus = self_cpus(), scpus = pid_cpus(spid);
    if (write(devnull, "AK_PROFILE_BEGIN", 16) < 0) die("marker", 0);
    perf_cmd("enable");
    const IrqSnap ci0 = irq_snap(ccpus), si0 = irq_snap(scpus);
    long h = 0;
    int done = 0;
    for (int ch = 0; ch < chunks; ++ch) {
      const int nb = c.profile / chunks + (ch < c.profile % chunks ? 1 : 0);
      const double p0 = now_ns(CLOCK_PROCESS_CPUTIME_ID), w0 = now_ns(CLOCK_MONOTONIC);
      for (int b = 0; b < nb; ++b) h += pool.batch(pc, 0, k, k);
      ccpu.push_back(now_ns(CLOCK_PROCESS_CPUTIME_ID) - p0);
      cwall.push_back(now_ns(CLOCK_MONOTONIC) - w0);
      cbat.push_back(nb);
      done += nb;
    }
    const IrqSnap ci1 = irq_snap(ccpus), si1 = irq_snap(scpus);
    perf_cmd("disable");
    g_trace_calls.store(false);
    if (write(devnull, "AK_PROFILE_END", 14) < 0) die("marker", 0);
    getrusage(RUSAGE_SELF, &r1);
    const long big_allocs = big ? (long)(big() - big0) : -1;
    const double loopw = now_ns(CLOCK_MONOTONIC) - loop0;
    const std::map<long, std::pair<unsigned long long, unsigned long long> > s1 = server_times(spid);
    double scpu = 0, swait = 0, smax = 0;
    int sbusy = 0;
    for (std::map<long, std::pair<unsigned long long, unsigned long long> >::const_iterator i = s1.begin(); i != s1.end(); ++i) {
      std::map<long, std::pair<unsigned long long, unsigned long long> >::const_iterator j = s0.find(i->first);
      const double c = (double)(i->second.first - (j == s0.end() ? 0ULL : j->second.first));
      scpu += c;
      swait += (double)(i->second.second - (j == s0.end() ? 0ULL : j->second.second));
      if (c > smax) smax = c;
      if (c > 0.05 * loopw) ++sbusy;
    }
    char sb[300];
    std::snprintf(sb, sizeof(sb), "\"server\": {\"pid\": %ld, \"threads\": %zu, \"cpu_ns\": %.0f, \"wait_ns\": %.0f, \"max_thread_ns\": %.0f,"
                  " \"threads_over_5pct\": %d, \"loop_wall_ns\": %.0f}", spid, s1.size(), scpu, swait, smax, sbusy, loopw);
    std::map<long, std::pair<std::string, unsigned long long> > t1 = thread_times();
    std::map<std::string, std::pair<double, double> > clcsw;  // voluntary / involuntary switches by class
    for (std::map<long, std::pair<std::string, unsigned long long> >::const_iterator i = t1.begin(); i != t1.end(); ++i) {
      std::map<long, std::pair<unsigned long long, unsigned long long> >::const_iterator a = g_csw.find(i->first), b = csw0.find(i->first);
      if (a == g_csw.end()) continue;
      clcsw[i->second.first].first += (double)(a->second.first - (b == csw0.end() ? 0ULL : b->second.first));
      clcsw[i->second.first].second += (double)(a->second.second - (b == csw0.end() ? 0ULL : b->second.second));
    }
    std::map<std::string, double> clsw;  // run-queue wait (schedstat field 2) by class
    for (std::map<long, std::pair<std::string, unsigned long long> >::const_iterator i = t1.begin(); i != t1.end(); ++i) {
      std::map<long, unsigned long long>::const_iterator a = g_wait_ns.find(i->first), b = w0.find(i->first);
      clsw[i->second.first] += (double)((a == g_wait_ns.end() ? 0ULL : a->second) - (b == w0.end() ? 0ULL : b->second));
    }
    // the batch accounting (blocking cells): each batch's wall against its calls' (the calls are the
    // seats' traced intervals, matched to batches by time)
    std::vector<std::pair<int64_t, int64_t> > tcalls;
    for (size_t i = 0; i < pool.seats.size(); ++i) tcalls.insert(tcalls.end(), pool.seats[i]->tr.begin(), pool.seats[i]->tr.end());
    std::sort(tcalls.begin(), tcalls.end());
    std::vector<double> bw, cd, lead, lag, spanv;
    size_t ci2 = 0;
    for (size_t b = 0; b < pool.btr.size(); ++b) {
      const int64_t bs = pool.btr[b].first, be = pool.btr[b].second;
      int64_t fs = 0, le = 0, ls = 0;
      int nn = 0;
      while (ci2 < tcalls.size() && tcalls[ci2].first < be) {
        const int64_t s0 = tcalls[ci2].first, e0 = tcalls[ci2].second;
        if (s0 >= bs) {
          if (!nn || s0 < fs) fs = s0;
          if (!nn || e0 > le) le = e0;
          if (!nn || s0 > ls) ls = s0;
          cd.push_back((double)(e0 - s0));
          ++nn;
        }
        ++ci2;
      }
      if (!nn) continue;
      bw.push_back((double)(be - bs));
      lead.push_back((double)(ls - bs));  // the batch start to its LAST call's start (dispatch)
      lag.push_back((double)(be - le));   // the last call's end to the batch's end (completion hand-off)
      spanv.push_back((double)(le - fs));
    }
    auto med = [](std::vector<double> v) { if (v.empty()) return -1.0; std::sort(v.begin(), v.end()); return v[v.size() / 2]; };
    auto pct = [](std::vector<double> v, double p) { if (v.empty()) return -1.0; std::sort(v.begin(), v.end()); return v[(size_t)(p * (v.size() - 1))]; };
    char tb[600];
    std::snprintf(tb, sizeof(tb), "\"batch_trace\": {\"batches\": %zu, \"calls\": %zu, \"batch_wall_ns_median\": %.0f, \"call_ns_median\": %.0f,"
                  " \"call_ns_p10\": %.0f, \"call_ns_p90\": %.0f, \"calls_span_ns_median\": %.0f, \"dispatch_last_start_ns_median\": %.0f,"
                  " \"completion_lag_ns_median\": %.0f}", bw.size(), cd.size(), med(bw), med(cd), pct(cd, 0.1), pct(cd, 0.9), med(spanv),
                  med(lead), med(lag));
    std::map<std::string, double> cls;
    std::map<std::string, int> cnt;
    for (std::map<long, std::pair<std::string, unsigned long long> >::const_iterator i = t1.begin(); i != t1.end(); ++i) {
      std::map<long, std::pair<std::string, unsigned long long> >::const_iterator j = t0.find(i->first);
      cls[i->second.first] += (double)(i->second.second - (j == t0.end() ? 0ULL : j->second.second));
      ++cnt[i->second.first];
    }
    const double calls = (double)done * k;
    std::string o = "{\"profile\": {\"cell\": \"" + w.cells[pc].label + "\", \"payload\": \"" + job_payload(w.jobs[0]) +
                    "\", \"dir\": \"" + dir_label(w.jobs[0].dir) + "\", \"k\": " + std::to_string(k) +
                    ", \"batches\": " + std::to_string(done) + ", \"calls\": " + std::to_string((long)calls) +
                    ", \"cells_open\": \"" + c.cells + "\", \"chunks\": [";
    for (size_t i = 0; i < ccpu.size(); ++i) {
      char b[160];
      std::snprintf(b, sizeof(b), "%s{\"batches\": %d, \"cpu_ns\": %.0f, \"wall_ns\": %.0f}", i ? ", " : "", cbat[i], ccpu[i], cwall[i]);
      o += b;
    }
    o += "], " + std::string(tb) + ", " + std::string(sb) + ", \"irq_time\": {\"user_hz\": " + std::to_string(sysconf(_SC_CLK_TCK)) + ", " + irq_delta("client_cpus", ci0, ci1) + ", " +
         irq_delta("server_cpus", si0, si1) + "}, \"tcp_sockets\": " + tcp_socks + ", \"core_target\": \"" + c.core_target +
         "\", \"cpu_at_end\": " + cpu_facts() + ", \"thread_wait_ns\": {";
    {
      bool f1 = true;
      for (std::map<std::string, double>::const_iterator i = clsw.begin(); i != clsw.end(); ++i) {
        char bb[120];
        std::snprintf(bb, sizeof(bb), "%s\"%s\": %.0f", f1 ? "" : ", ", i->first.c_str(), i->second);
        o += bb;
        f1 = false;
      }
    }
    o += "}, \"thread_csw\": {";
    {
      bool f1 = true;
      for (std::map<std::string, std::pair<double, double> >::const_iterator i = clcsw.begin(); i != clcsw.end(); ++i) {
        char bb[160];
        std::snprintf(bb, sizeof(bb), "%s\"%s\": [%.0f, %.0f]", f1 ? "" : ", ", i->first.c_str(), i->second.first, i->second.second);
        o += bb;
        f1 = false;
      }
    }
    o += "}, \"thread_cpu_ns\": {";
    bool first = true;
    for (std::map<std::string, double>::const_iterator i = cls.begin(); i != cls.end(); ++i) {
      char b[160];
      std::snprintf(b, sizeof(b), "%s\"%s\": {\"threads\": %d, \"ns\": %.0f}", first ? "" : ", ", i->first.c_str(), cnt[i->first], i->second);
      o += b;
      first = false;
    }
    char b[400];
    std::snprintf(b, sizeof(b), "}, \"big_allocs\": %ld, \"rusage\": {\"nvcsw\": %ld, \"nivcsw\": %ld, \"minflt\": %ld, \"majflt\": %ld, \"utime_us\": %ld, \"stime_us\": %ld}, \"fold\": %ld}}",
                  big_allocs, r1.ru_nvcsw - r0.ru_nvcsw, r1.ru_nivcsw - r0.ru_nivcsw, r1.ru_minflt - r0.ru_minflt, r1.ru_majflt - r0.ru_majflt,
                  (long)((r1.ru_utime.tv_sec - r0.ru_utime.tv_sec) * 1000000L + (r1.ru_utime.tv_usec - r0.ru_utime.tv_usec)),
                  (long)((r1.ru_stime.tv_sec - r0.ru_stime.tv_sec) * 1000000L + (r1.ru_stime.tv_usec - r0.ru_stime.tv_usec)), h);
    o += b;
    std::printf("%s\n", o.c_str());
    std::fflush(stdout);
    return 0;
  }
  if (c.gbout.empty()) {  // a usage error, never a call check (the gate's controls grep for those)
    std::fprintf(stderr, "usage: --gbench-out FILE is required (the samples are Google Benchmark's)\n");
    return 2;
  }
  std::printf("# {\"campaign_rpc\": {\"build\": \"%s\", \"target\": \"%s\", \"transport\": \"%s\", \"cells\": \"%s\","
              " \"dirs\": \"%s\", \"min_time_s_per_repetition\": %.3f, \"rounds\": %d, \"launch\": %d,"
              " \"expect_bytes\": %zu, \"delivery\": \"B, C, E: the core's blocking ak_call_unary (C: ak_call_unary_enc,"
              " the encode context moved); d: ak_call_open + ak_call_send (C: ak_call_send_enc) + ak_call_recv."
              " A, D, F: grpc++'s synchronous call and ClientWriter (packages/cpp's idiom); D and F hand their bytes"
              " over moved (ak_enc_take_owned, ak::Enc::take). Req. 16 as amended 2026-09-28: C++'s core reference is BOTH"
              " blocking (B, C, E: labelled (blk)) and the completion queue (B-q, C-q-*, E-q-* and framed twins: (q)):"
              " ak_call_unary_q (C: ak_call_unary_enc_q, the request moved), d: ak_call_open + ak_call_send_q (C:"
              " ak_call_send_enc_q) + ak_call_recv_q\", \"queue_drainer\": \"one ak_queue PER queue cell; ONE drainer"
              " per batch, the thread that issues it (the benchmark thread; no thread of its own, no k caller threads):"
              " the k calls of a batch issued back to back, then ak_queue_next until all k completed, each completion"
              " matched to its call by tag (slot << 8 | operation) and checked, a response decoded on the draining"
              " thread; a stream's next send issued from the drain once its previous send completed (at most one"
              " pending send per call), its recv_q after the last; grpc++'s CompletionQueue::Next idiom\", \"send_paths\": \"set explicitly on every core client at"
              " open (ak_client_set_framed): 1 = framed, the core's default since 2026-09-28 (Bf, Cf-*, Ef-*, and -q),"
              " 0 = the reference path, tonic's codec (B, C-*, E-*, and -q)\","
              " \"directions_c_d\": \"c: P5.3, P5.4 unary upload, empty response; d: 4 MiB and 16 MiB in 2 MiB M5"
              " chunks (ids on the first), the server's byte count checked on every call and its SHA-256 of the"
              " messages as received once per cell and payload before any benchmark (UploadStreamCheck); both at 1 and 8"
              " in flight\", \"channels\": \"one per cell per benchmark process, opened"
              " before any benchmark, warmed by the framework's warm-up\","
              " \"sampler\": \"Google Benchmark %s (WP9, req. 22a amended): one benchmark per (cell, direction, payload,"
              " in-flight k); one iteration = one batch of k calls in flight, one per pre-created caller thread (k"
              " operations; per-iteration cost includes one condition-variable hand-off to the k threads, where the"
              " earlier sampler had one per sample); iterations chosen by the framework (--benchmark_min_time per"
              " repetition, so every benchmark gets the same time rather than the same call count); repetitions = rounds,"
              " every one reported raw; cpu_time = process CPU (MeasureProcessCPUTime), real_time = wall (UseRealTime);"
              " warm-up = --benchmark_min_warmup_time %.3f s per benchmark, before its first repetition; order ="
              " --benchmark_enable_random_interleaving (repetitions of every benchmark in random order, unseeded;"
              " order_pos is the position in Google Benchmark's output) plus registration order rotated by launch; a"
              " failed call check aborts the process (exit 3) and the JSON is only renamed into place on success\","
              " \"threads\": {\"caller_threads\": %d, \"core_runtime_workers\": %d,"
              " \"process_threads_before_benchmarks\": %d, \"grpcpp\": \"grpc-core sizes its own pollers and executor"
              " (no application setting); they are counted in the process totals\"},"
              " \"core_encode_fill\": \"" AK_CORE_FILL "\","
              " \"harness\": \"H-2: one condition variable per caller thread, a batch wakes only its k threads; H-4: the raw methods of A (d), D and F built once per channel (registered calls, as the generated stub's); H-6: cell B serialises with ByteSizeLong + SerializeWithCachedSizesToArray into a reused per-thread buffer\","
              " \"precheck\": \"C, D, E, F in each mode: decode, re-encode, equal to the incumbent's deterministic"
              " re-serialisation; every c/d request message byte-identical to protobuf's\","
              " \"payloads\": \"%s\", \"fixed_iters\": \"%s\", \"rusage\": \"getrusage(RUSAGE_SELF) around each"
              " repetition's timed loop: counters ru_nvcsw, ru_nivcsw, ru_minflt, ru_majflt (repetition totals)\","
              " \"cpu\": %s, \"thread_classes_before_benchmarks\": %s, \"core_target\": \"%s\", \"tcp_sockets\": %s}}\n",
              kBuild, c.target.c_str(), c.transport.c_str(), c.cells.c_str(), c.dirs.c_str(), c.min_time_s,
              c.rounds, c.launch, w.expect_a, AK_GBENCH_VERSION, c.warmup_s, maxk, c.workers, proc_threads(),
              c.payloads.empty() ? "all" : c.payloads.c_str(), c.iters.empty() ? "none" : c.iters.c_str(),
              cpu_facts().c_str(), thread_classes().c_str(), c.core_target.c_str(), tcp_sockets().c_str());
  std::fflush(stdout);

  // WP9: the samples are Google Benchmark's. One benchmark per (cell, job, k), named
  // "cell|payload|content|dir|mode|inflight=k,transport=..,send_path=..,build=.." for
  // gen/gbench_to_jsonl.py; registration order rotated by launch.
  struct Reg { std::string name; size_t cell, job; int k; };
  std::vector<Reg> regs;
  for (size_t ji = 0; ji < w.jobs.size(); ++ji)
    for (int k : c.inflight) {
      if (!job_runs(w.jobs[ji], k)) continue;
      for (size_t ci = 0; ci < w.cells.size(); ++ci) {
        const Cell &cl = w.cells[ci];
        if (cl.pull && w.jobs[ji].dir != 'a' && w.jobs[ji].dir != 'r') continue;  // X-2: a, a+read only
        char tags[240];
        std::snprintf(tags, sizeof(tags), "inflight=%d,transport=%s,send_path=%s,build=%s%s%s", k, c.transport.c_str(),
                      cl.framed ? "framed" : "reference", kBuild, cl.pull ? ",decode=pull" : "",
                      cl.cb ? ",delivery=callback" : cl.q ? ",delivery=queue" : grpc_cell(cl.base) ? "" : cl.zc && cl.deferred == 1 ? ",delivery=blocking,send=deferred-zc-wait"
                      : cl.zc && cl.deferred == 2 ? ",delivery=blocking,send=deferred-zc-nowait" : cl.deferred == 1 ? ",delivery=blocking,send=deferred-wait"
                      : cl.deferred == 2 ? ",delivery=blocking,send=deferred-nowait"
                      : cl.zc ? ",delivery=blocking,send=zero-copy" : ",delivery=blocking");
        regs.push_back(Reg{cl.label + "|" + job_payload(w.jobs[ji]) + "|-|" + dir_label(w.jobs[ji].dir) + "|" +
                               mode_name(cl.mode) + "|" + tags,
                           ci, ji, k});
      }
    }
  static int g_done = 0;
  const int fail_after = c.fail_after;
  size_t nr = regs.size(), rot = nr ? ((size_t)c.launch * 7919u) % nr : 0;
  for (size_t q = 0; q < nr; ++q) {
    const Reg r = regs[(q + rot) % nr];
    Pool *pp = &pool;
    benchmark::internal::Benchmark *bm =
        benchmark::RegisterBenchmark(r.name.c_str(), [pp, r, fail_after](benchmark::State &st) {
      long h = 0;
      // Process-wide getrusage around the timed loop (outside Google Benchmark's timers, which
      // start at the loop's first iteration and stop at its end): context switches and page
      // faults of every thread of the client, as user counters of the repetition (totals; per
      // call = total / (iterations x k)).
      struct rusage r0, r1;
      getrusage(RUSAGE_SELF, &r0);
      for (auto _ : st) h += pp->batch(r.cell, r.job, r.k, r.k);  // one batch: k calls in flight
      getrusage(RUSAGE_SELF, &r1);
      benchmark::DoNotOptimize(h);
      st.SetItemsProcessed(st.iterations() * r.k);
      st.counters["ru_nvcsw"] = (double)(r1.ru_nvcsw - r0.ru_nvcsw);
      st.counters["ru_nivcsw"] = (double)(r1.ru_nivcsw - r0.ru_nivcsw);
      st.counters["ru_minflt"] = (double)(r1.ru_minflt - r0.ru_minflt);
      st.counters["ru_majflt"] = (double)(r1.ru_majflt - r0.ru_majflt);
      // --fail-after N (the gate's R-H4 control): abort after N measured repetitions.
      if (++g_done == fail_after) die("--fail-after (test control)", g_done);
    });
    bm->Repetitions(c.rounds)->Unit(benchmark::kNanosecond)
      ->MeasureProcessCPUTime()->UseRealTime()->ReportAggregatesOnly(false);
    std::map<std::string, long>::const_iterator fi =
        fixed_iters.find(std::string(job_payload(w.jobs[r.job])) + "/" + std::to_string(r.k));
    if (fi == fixed_iters.end()) fi = fixed_iters.find(std::to_string(r.k));
    if (fi != fixed_iters.end()) bm->Iterations(fi->second);  // --iters: no estimation
  }
  char wu[64];
  std::snprintf(wu, sizeof(wu), "--benchmark_min_warmup_time=%.6f", c.warmup_s);
  char mt[64];
  std::snprintf(mt, sizeof(mt), "--benchmark_min_time=%.6fs", c.min_time_s);
  std::string part = c.gbout + ".part";
  std::vector<std::string> args = {"campaign_rpc", "--benchmark_out=" + part, "--benchmark_out_format=json",
                                   "--benchmark_enable_random_interleaving=true", wu, mt,
                                   "--benchmark_format=console"};
  std::vector<char *> av;
  for (size_t q = 0; q < args.size(); ++q) av.push_back(&args[q][0]);
  int ac = (int)av.size();
  benchmark::Initialize(&ac, av.data());
  benchmark::RunSpecifiedBenchmarks();
  benchmark::Shutdown();
  // Only a run in which every call passed its check reaches here (die -> _Exit(3)); the file
  // is renamed into place only now, so an aborted run leaves no sample file (R-H4).
  if (std::rename(part.c_str(), c.gbout.c_str()) != 0) die("rename the Google Benchmark output", 0);
  std::printf("# {\"campaign_rpc_end\": {\"benchmarks\": %zu, \"process_threads\": %d, \"thread_classes\": %s}}\n", nr,
              proc_threads(), thread_classes().c_str());
  std::fflush(stdout);
#endif
  for (size_t i = 0; i < w.conns.size(); ++i) {
    if (w.conns[i].cl) ak_client_destroy(w.conns[i].cl);
    if (w.conns[i].q) {  // every call naming it has completed (each batch drains its k)
      ak_queue_shutdown(w.conns[i].q);
      ak_queue_destroy(w.conns[i].q);
    }
  }
  ak_runtime_destroy(w.rt);
  return 0;
}
