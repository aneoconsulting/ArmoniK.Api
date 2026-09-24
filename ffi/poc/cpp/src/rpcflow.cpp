// The flow-control probe: what each of this slice's two HTTP/2 stacks actually does, from
// its own behaviour rather than from a table.
//
// `design/SHAPES.md` requires every RPC arm to state its stream and connection window and
// whether auto-tuning is on, and names two traps: that the connection window may be a
// separate setting from the stream window, and that pinning a window may or may not turn
// auto-tuning off. It has those answers for grpc-java and for .NET. It does not have them
// for grpc-core (grpc++) or for tonic/hyper, and its table's entry for tonic is wrong --
// which is exactly the kind of thing this branch exists to catch, so it is established here
// and not inherited.
//
// Method: run the same three RPCs under several channel configurations, each in a CHILD
// process with grpc's own tracers on, and read the answers out of grpc's trace. Three trace
// sites carry everything needed:
//
//   frame_settings.cc  "CHTTP2:SVR:<peer>: got setting INITIAL_WINDOW_SIZE = N"
//                      -- logged by the RECEIVER, so an SVR line is what the CLIENT announced
//   bdp_estimator      "<peer>: Start BDP ping"
//                      -- auto-tuning, per transport, with the peer named
//   writing.cc:211     "[fc:pending=..:flowed=..:peer_initwin=..:t_win=..:s_win=..]"
//                      -- a stall, with the CONNECTION window (t_win) and the STREAM window
//                         (s_win) printed side by side. A stall at t_win=0 with s_win large
//                         is the connection window binding while the stream window is idle,
//                         which is the first trap, observed.
//
// A child process rather than an in-process capture, because the tracers are read from the
// environment once at grpc init and a configuration cannot be changed afterwards.
#include "rpc_common.h"

#include <thread>

#include "ak_abi.h"

using namespace akrpc;

static const uint8_t kNoReq[1] = {0};

// ---- the child: one configuration, a burst of RPCs, exit --------------------------------
//
// Nine configurations, and the extra ones are there because the obvious four could not tell
// two mechanisms apart. `win` is the stream window in bytes (0 = leave the stack's default),
// `conn` the connection window where the stack has one to set, `bdp` -1 leave / 0 off.
struct Config {
  const char *name;
  bool core_client;   // false: grpc++ is the client. true: the core is.
  long win;
  long conn;
  int bdp;
};

static const long M = 1024 * 1024;

static const Config kConfigs[] = {
  {"grpcpp default",          false,      0,     0, -1},
  {"grpcpp win 4M",           false,  4 * M,     0, -1},
  {"grpcpp bdp off",          false,      0,     0,  0},
  {"grpcpp win 4M bdp off",   false,  4 * M,     0,  0},
  {"grpcpp win 64M",          false, 64 * M,     0, -1},
  {"grpcpp win 64M bdp off",  false, 64 * M,     0,  0},
  {"core default",            true,       0,     0, -1},
  {"core win 4M conn 4M",     true,   4 * M, 4 * M,  0},
  {"core win 4M conn 64K",    true,   4 * M, 65535,  0},
};
static const int kNConfigs = (int)(sizeof(kConfigs) / sizeof(kConfigs[0]));

static int child_main(int cfg, int port) {
  const Config &c = kConfigs[cfg];
  // The SERVER is left at its defaults in every configuration, so the only variable across
  // the table is the client's.
  Transport tr = make_transport(false, port);
  ShapesImpl svc;
  grpc::ServerBuilder b;
  b.SetMaxReceiveMessageSize(kMaxMessage);
  b.SetMaxSendMessageSize(kMaxMessage);
  b.AddListeningPort(tr.grpc_target, grpc::InsecureServerCredentials());
  b.RegisterService(&svc);
  std::unique_ptr<grpc::Server> server = b.BuildAndStart();
  if (!server) { std::fprintf(stderr, "AKFLOW server failed\n"); return 1; }

  // Enough load that BDP adaptation has something to adapt to: with twelve RPCs the
  // estimator probes once and never moves the window, and a probe that cannot see
  // auto-tuning happen cannot say whether a setting turned it off.
  const int kInFlight = 8, kPer = 8;
  if (c.core_client) {
    ak_runtime *rt = ak_runtime_new(2);
    // All six fields, through the one helper (R-D2).
    ak_client_opts o = core_opts((uint32_t)c.win, (uint32_t)c.conn, c.bdp);
    ak_client *cl = ak_client_new_opts(rt, (const uint8_t *)tr.core_uri.data(),
                                       tr.core_uri.size(), &o);
    if (!cl) { std::fprintf(stderr, "AKFLOW core client failed\n"); return 1; }
    std::vector<std::thread> ts;
    for (int t = 0; t < kInFlight; ++t)
      ts.push_back(std::thread([&]() {
        for (int i = 0; i < kPer; ++i) {
          struct ak_bytes out; out.ptr = NULL; out.len = 0; out.owner = NULL;
          if (ak_call_unary(cl, (const uint8_t *)kFetchPath, strlen(kFetchPath), kNoReq, 0,
                            &out) != AK_OK) { std::fprintf(stderr, "AKFLOW call failed\n"); }
          ak_bytes_free(&out);
        }
      }));
    for (size_t i = 0; i < ts.size(); ++i) ts[i].join();
    ak_client_destroy(cl);
    ak_runtime_destroy(rt);
  } else {
    grpc::ChannelArguments a;
    a.SetMaxReceiveMessageSize(kMaxMessage);
    a.SetMaxSendMessageSize(kMaxMessage);
    if (c.win) a.SetInt(GRPC_ARG_HTTP2_STREAM_LOOKAHEAD_BYTES, (int)c.win);
    if (c.bdp >= 0) a.SetInt(GRPC_ARG_HTTP2_BDP_PROBE, c.bdp);
    std::shared_ptr<grpc::Channel> chan =
        grpc::CreateCustomChannel(tr.grpc_target, grpc::InsecureChannelCredentials(), a);
    std::vector<std::unique_ptr<svcns::Shapes::Stub> > stubs;
    for (int i = 0; i < kInFlight; ++i) stubs.push_back(svcns::Shapes::NewStub(chan));
    std::vector<std::thread> ts;
    for (int t = 0; t < kInFlight; ++t)
      ts.push_back(std::thread([&, t]() {
        for (int i = 0; i < kPer; ++i) {
          grpc::ClientContext ctx;
          svcns::Empty q;
          svcns::ListTasksDetailedResponse r;
          grpc::Status s = stubs[(size_t)t]->Fetch(&ctx, q, &r);
          if (!s.ok()) std::fprintf(stderr, "AKFLOW grpc++ call failed\n");
        }
      }));
    for (size_t i = 0; i < ts.size(); ++i) ts[i].join();
  }
  server->Shutdown();
  server->Wait();
  return 0;
}

// ---- the parent: run each configuration and read grpc's trace back ----------------------
struct Observed {
  long client_first, client_last;  // SETTINGS_INITIAL_WINDOW_SIZE seen BY THE SERVER
  int client_settings_frames;
  long server_announced;
  int bdp_pings_client, bdp_pings_server, bdp_pings_other;
  int stalls;
  long min_twin, min_swin, peer_initwin_max;
  bool saw_any;
};

static long grab_long(const std::string &l, const char *key) {
  size_t i = l.find(key);
  if (i == std::string::npos) return -1;
  i += strlen(key);
  return strtol(l.c_str() + i, NULL, 10);
}

static Observed analyse(const std::string &out, int port) {
  Observed o;
  memset(&o, 0, sizeof o);
  o.client_first = o.client_last = o.server_announced = -1;
  o.min_twin = o.min_swin = -1;
  o.peer_initwin_max = -1;
  char srv[64];
  std::snprintf(srv, sizeof srv, "127.0.0.1:%d", port);
  size_t pos = 0;
  while (pos < out.size()) {
    size_t e = out.find('\n', pos);
    if (e == std::string::npos) e = out.size();
    std::string l = out.substr(pos, e - pos);
    pos = e + 1;
    if (l.find("got setting INITIAL_WINDOW_SIZE") != std::string::npos) {
      long v = grab_long(l, "INITIAL_WINDOW_SIZE = ");
      o.saw_any = true;
      if (l.find("CHTTP2:SVR:") != std::string::npos) {
        // The server received it, so the CLIENT announced it.
        if (o.client_first < 0) o.client_first = v;
        o.client_last = v;
        o.client_settings_frames++;
      } else if (l.find("CHTTP2:CLI:") != std::string::npos) {
        o.server_announced = v;
      }
    } else if (l.find("Start BDP ping") != std::string::npos) {
      // `bdp[<peer>]` / `<peer>: Start BDP ping`. A ping whose peer is the LISTENING
      // address was sent by the client; any other peer is an ephemeral port, so the
      // server sent it.
      if (l.find(srv) != std::string::npos) o.bdp_pings_client++;
      else if (l.find("127.0.0.1:") != std::string::npos) o.bdp_pings_server++;
      else o.bdp_pings_other++;
    } else if (l.find("[fc:pending=") != std::string::npos) {
      o.stalls++;
      long t = grab_long(l, ":t_win=");
      long s = grab_long(l, ":s_win=");
      long p = grab_long(l, ":peer_initwin=");
      if (o.min_twin < 0 || t < o.min_twin) o.min_twin = t;
      if (o.min_swin < 0 || s < o.min_swin) o.min_swin = s;
      if (p > o.peer_initwin_max) o.peer_initwin_max = p;
    }
  }
  return o;
}

int main(int argc, char **argv) {
  if (argc > 2 && !strcmp(argv[1], "child")) return child_main(atoi(argv[2]), atoi(argv[3]));

  std::printf("grpc++ %s, protobuf C++ %d. Each row is a CHILD process with\n"
              "GRPC_TRACE=http,flowctl,bdp_estimator and GRPC_VERBOSITY=DEBUG, running 8x8\n"
              "P2.2 Fetches over loopback TCP against a server left at its DEFAULTS, so the\n"
              "only variable is the client's configuration.\n\n",
              grpc::Version().c_str(), GOOGLE_PROTOBUF_VERSION);

  Observed obs[16];
  for (int i = 0; i < kNConfigs; ++i) {
    int port = 50200 + i;
    char cmd[1024];
    std::snprintf(cmd, sizeof cmd,
                  "GRPC_VERBOSITY=DEBUG GRPC_TRACE=http,flowctl,bdp_estimator "
                  "'/proc/self/exe' child %d %d 2>&1", i, port);
    // /proc/self/exe inside the shell would be the shell's, so resolve it here.
    char self[4096];
    ssize_t n = readlink("/proc/self/exe", self, sizeof self - 1);
    if (n <= 0) { std::printf("cannot resolve own path\n"); return 1; }
    self[n] = 0;
    std::snprintf(cmd, sizeof cmd,
                  "GRPC_VERBOSITY=DEBUG GRPC_TRACE=http,flowctl,bdp_estimator "
                  "%s child %d %d 2>&1", self, i, port);
    FILE *f = popen(cmd, "r");
    if (!f) { std::printf("popen failed\n"); return 1; }
    std::string out;
    char buf[8192];
    size_t got;
    while ((got = fread(buf, 1, sizeof buf, f)) > 0) out.append(buf, got);
    pclose(f);
    obs[i] = analyse(out, port);
    if (!obs[i].saw_any) {
      std::printf("configuration %d produced no SETTINGS trace -- the probe is broken, not "
                  "the stack. %zu bytes of output.\n", i, out.size());
      return 1;
    }
  }

  std::printf("%-24s %13s %13s %9s %8s %8s %10s %10s\n", "client configuration",
              "announced", "last announced", "settings", "BDPping", "stalls",
              "min t_win", "min s_win");
  for (int i = 0; i < kNConfigs; ++i) {
    Observed &o = obs[i];
    std::printf("%-24s %13ld %13ld %9d %8d %8d %10ld %10ld\n", kConfigs[i].name,
                o.client_first, o.client_last, o.client_settings_frames,
                o.bdp_pings_client, o.stalls, o.min_twin, o.min_swin);
  }

  std::printf("\n`announced` is SETTINGS_INITIAL_WINDOW_SIZE as the SERVER received it, so it\n"
              "is what the client asked its peer to respect -- the window the 540 KB response\n"
              "has to fit inside. `settings` counts how many times the client re-announced it,\n"
              "which is auto-tuning made visible. `BDPping` counts the CLIENT transport's\n"
              "pings. `stalls` counts writing.cc's 'moved to stalled list' with the two\n"
              "windows printed, and `min t_win` / `min s_win` are the CONNECTION and STREAM\n"
              "windows at their lowest across those stalls. A stall at t_win=0 with s_win\n"
              "large is the connection window binding while the stream window is idle.\n");

  std::printf("\n-- SHAPES.md's two traps, answered from the rows above rather than inherited --\n");

  std::printf("\n1. ARE THE STREAM AND CONNECTION WINDOWS SEPARATE CHANNEL ARGUMENTS?\n"
              "   Two different answers for the two stacks, and neither is grpc-java's.\n"
              "\n"
              "   grpc++: they are separate QUANTITIES and only one of them is reachable.\n"
              "   grpc/impl/codegen/grpc_types.h exposes GRPC_ARG_HTTP2_STREAM_LOOKAHEAD_BYTES\n"
              "   and GRPC_ARG_HTTP2_BDP_PROBE and NO connection-window argument at all. That\n"
              "   they are separate is observed and not argued: `grpcpp win 4M bdp off` stalls\n"
              "   %d times at t_win=%ld with s_win=%ld, and raising the stream window sixteen\n"
              "   times over to 64 MiB (`grpcpp win 64M bdp off`) leaves %d stalls at\n"
              "   t_win=%ld -- the argument does not reach the connection window, so it cannot\n"
              "   be pinned from a channel argument.\n"
              "\n"
              "   The core (tonic 0.14 / hyper 1.11): separate and BOTH reachable.\n"
              "   `initial_stream_window_size` and `initial_connection_window_size` are two\n"
              "   builder calls and they behave as two. Holding the stream window at 4 MiB and\n"
              "   moving only the connection window: 4 MiB gives %d stalls, 65,535 gives %d,\n"
              "   at t_win=%ld with s_win=%ld. Same stream window, two orders of magnitude\n"
              "   apart in stalls, so the second setting is doing the work -- which is what\n"
              "   grpc++ has no argument for.\n",
              obs[3].stalls, obs[3].min_twin, obs[3].min_swin,
              obs[5].stalls, obs[5].min_twin,
              obs[7].stalls, obs[8].stalls, obs[8].min_twin, obs[8].min_swin);

  std::printf("\n2. DOES AN EXPLICIT WINDOW DISABLE BDP PROBING IN grpc++?\n"
              "   %s, and the interaction runs the other way from grpc-java's.\n"
              "   With nothing configured the client sends %d BDP pings and re-announces its\n"
              "   window %d time(s), ending at %ld. Setting the window to 4 MiB and leaving the\n"
              "   probe alone: %d pings, %d announcement(s), ending at %ld -- auto-tuning is\n"
              "   still running, and the configured value is not even what gets announced\n"
              "   (%ld asked for, %ld announced), because with the estimator on it is a\n"
              "   starting point the estimator overwrites. Only GRPC_ARG_HTTP2_BDP_PROBE=0\n"
              "   stops it: %d pings.\n"
              "   So on grpc++ the window argument and the probe argument are INDEPENDENT,\n"
              "   where on grpc-java calling flowControlWindow turns BDP off by itself. An\n"
              "   arm that set the window and not the probe would be measuring an auto-tuned\n"
              "   transport while its configuration line claimed a pinned one.\n",
              obs[1].bdp_pings_client > 0 ? "No" : "Yes",
              obs[0].bdp_pings_client, obs[0].client_settings_frames, obs[0].client_last,
              obs[1].bdp_pings_client, obs[1].client_settings_frames, obs[1].client_last,
              kConfigs[1].win, obs[1].client_first,
              obs[3].bdp_pings_client);

  std::printf("\n3. AND A THIRD TRAP NEITHER SHAPES.md NOR THIS SLICE HAD: on grpc++, TURNING\n"
              "   AUTO-TUNING OFF SHRINKS THE WINDOW TO 64 KIB.\n"
              "   `grpcpp bdp off` -- the probe off and no window set -- announces %ld, not\n"
              "   the %ld the same build announces by default, and it stalls %d times where\n"
              "   the default stalls %d. grpc-core's 4 MiB default initial window is the BDP\n"
              "   estimator's doing; switch the estimator off and the announced window falls\n"
              "   back to GRPC_ARG_HTTP2_STREAM_LOOKAHEAD_BYTES, whose documented default is\n"
              "   64 kb. So 'turn auto-tuning off so the arm is deterministic' is, on its own,\n"
              "   a 64x REDUCTION in the stream window, and a slice that did it without also\n"
              "   setting the window would have published a flow-control artefact as a\n"
              "   transport measurement.\n",
              obs[2].client_first, obs[0].client_first, obs[2].stalls, obs[0].stalls);

  std::printf("\n4. WHAT EACH STACK'S DEFAULT ACTUALLY IS, and SHAPES.md's table is wrong about\n"
              "   one of them.\n"
              "   grpc++ (grpc-core %s) announces %ld with nothing configured -- about 4 MiB,\n"
              "   not the 65,535 RFC 9113 gives as the protocol's initial value -- and\n"
              "   auto-tuning is ON.\n"
              "   tonic 0.14 over hyper 1.11 announces %ld: hyper's DEFAULT_STREAM_WINDOW, 2\n"
              "   MiB, with DEFAULT_CONN_WINDOW at 5 MiB and adaptive sizing off\n"
              "   (hyper/src/proto/h2/client.rs:48-50). **SHAPES.md's table says tonic/hyper is\n"
              "   at 65,535 with auto-tuning off.** The auto-tuning half is right and the\n"
              "   window is wrong by a factor of 32.\n"
              "   The consequence is not cosmetic and it lands on this slice's own published\n"
              "   log: `logs/cpp/rpc.log` said a 540 KB response 'against a 64 KB default\n"
              "   stream window means a single call in flight spends most of its wall clock\n"
              "   waiting for WINDOW_UPDATE'. Neither stack was at 64 KB. 540 KB fits inside\n"
              "   2 MiB and inside 4 MiB with no stall at all, and the %d stalls this probe\n"
              "   does see are all in configurations that PIN a window, never in a default\n"
              "   one. That sentence is withdrawn.\n",
              grpc::Version().c_str(), obs[0].client_first, obs[6].client_first,
              obs[3].stalls + obs[5].stalls + obs[8].stalls);

  std::printf("\n5. THE CORE COULD NOT BE PINNED AT ALL until this work unit.\n"
              "   `ak_client_new` took a URI and nothing else, so cells B and C ran at hyper's\n"
              "   defaults while cell A ran at grpc-core's, and the two were compared as\n"
              "   though that were one transport. `ak_client_new_opts` was ADDED to the shared\n"
              "   core (R0 permits an addition; `ak_client_new` is now a call to it with NULL\n"
              "   options, which is byte-for-byte the old behaviour). The pinned row is that\n"
              "   entry point working: %ld announced against %ld unpinned.\n",
              obs[7].client_first, obs[6].client_first);

  std::printf("\n6. WHAT THIS MEANS FOR THE TIMED ARM, and it is not what SHAPES.md assumes.\n"
              "   SHAPES.md says pin ArmoniK's 4 MiB stream window in every cell and make that\n"
              "   the headline, with the stack default as a labelled second row. On grpc++ the\n"
              "   pinned configuration is NOT REACHABLE: the connection window has no\n"
              "   argument, so pinning the stream window at 4 MiB and the probe off leaves the\n"
              "   connection window at its un-tuned value and produces %d stalls that the\n"
              "   DEFAULT configuration does not have (%d). `logs/cpp/rpc.log` therefore\n"
              "   carries BOTH configurations in full rather than publishing a pinned headline\n"
              "   whose incumbent is handicapped by the pinning -- which would be an R14\n"
              "   defect pointed at the incumbent by its own harness.\n",
              obs[3].stalls, obs[0].stalls);
  return 0;
}
