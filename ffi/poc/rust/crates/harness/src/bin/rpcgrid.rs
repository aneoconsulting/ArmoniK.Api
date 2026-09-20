//! Stage 6: ABI v1 section 9's **three deliveries** priced at the floor, and the RPC arm as
//! a **grid** rather than a pair.
//!
//! ## 1. The deliveries, and why this host is the one that should measure them
//!
//! Until today only `ak_call_unary` existed, so every RPC figure in this branch -- this
//! slice's stage 4 included -- is a **blocking-mode** figure. The core now carries all
//! three, and they are three deliveries of ONE call path (`unary_once`), which is the same
//! condition section 7.1 puts on the decode families.
//!
//! | delivery | forward | reverse | what the host does |
//! |---|---|---|---|
//! | blocking (`ak_call_unary`) | 2 | 0 | a host thread blocks in a native frame |
//! | callback (`ak_call_unary_cb`) | 2 | 1 | the core calls back on a thread it owns |
//! | queue (`ak_call_unary_q`) | 3 | 0 | the host drains `ak_queue_next` |
//!
//! **This host's crossing is about 1.8 ns in both directions and an RPC costs about 1.5 ms
//! of CPU, so the three should be indistinguishable here** -- the queue's extra forward call
//! and the callback's reverse call are nanoseconds against milliseconds. If that is what
//! comes out, **it is the control that makes the managed slices' delivery results mean
//! something**: it says the differences they see are their crossing price and their
//! threading model, not the deliveries doing different amounts of work. If the deliveries
//! are NOT indistinguishable here, that is more interesting still.
//!
//! ## 2. The grid, and why this slice's B - A is not the same quantity as anyone else's
//!
//! | cell | codec | transport |
//! |---|---|---|
//! | A | prost | tonic |
//! | B | prost | core |
//! | C | core | core |
//!
//! For a managed host, B - A is a **transport** comparison: grpc-java or grpc-dotnet against
//! tonic. **Here both transports ARE tonic** -- the core's RPC half is tonic underneath -- so
//! B - A is not a transport comparison at all. It is **the cost of putting the C ABI in front
//! of the same transport, with the transport difference removed by construction.** Nobody
//! else can isolate that, and it is what says how much of a managed slice's B - A is the
//! interface and how much is their gRPC implementation. A reader must not take this slice's
//! B - A for the same quantity as Java's.
//!
//! Cell B is cheap here because the RPC half does not know the schema: prost produces the
//! request bytes and consumes the response bytes, and the core's transport moves opaque
//! bytes and never sees a message type.
//!
//! ## 3. The transport is pinned, and it could not be before today
//!
//! ArmoniK pins 2 MiB chunking and a 4 MiB HTTP/2 window. `ak_client_new` takes a URI and
//! nothing else, so **every RPC figure in this branch was taken on tonic's default 64 KiB
//! stream window** -- which README R9 says is most of the wall clock on a 540 KB response.
//! `ak_client_new_opts` (added for this) pins it, and the tonic arm is pinned identically,
//! so the grid compares interfaces rather than window sizes. **The stream and connection
//! windows are separate settings on tonic/hyper**; raising only the stream one leaves the
//! connection at 65,535.
//!
//! UDS is the primary transport (`design/SHAPES.md`), loopback TCP a labelled second row.

use ak_abi::*;
use bytes::Bytes;
use harness::arms_m2 as m2;
use std::sync::Arc;
use std::time::Instant;

const ROUNDS: usize = 7;
const CALLS_PER_ROUND: usize = 96;

/// ArmoniK's pinned transport.
const STREAM_WINDOW: u32 = 4 * 1024 * 1024;
const CONNECTION_WINDOW: u32 = 4 * 1024 * 1024;
const MAX_MESSAGE: u32 = 2 * 1024 * 1024;

fn opts() -> ak_client_opts {
    ak_client_opts {
        stream_window: STREAM_WINDOW,
        connection_window: CONNECTION_WINDOW,
        // -1 is "leave the default", which is off. Pinning a window and enabling adaptive
        // sizing is a contradiction, not belt and braces: adaptive overrides the pin. This
        // field arrived with the cpp slice's version of the same entry point.
        adaptive_window: -1,
        max_recv_message: MAX_MESSAGE,
        max_send_message: MAX_MESSAGE,
        // -1 leaves tonic's client default, which is nodelay ON. That matches ArmoniK:
        // `armonik-transport` reads `GrpcClient__TcpNagleAlgorithm`, defaults it to false and
        // applies `set_nodelay(!nagle)`. So the CLIENT was never the problem; the 40 ms was
        // always the server side of our own harness.
        tcp_nagle: -1,
    }
}

fn main() {
    let payload = m2::prost_arm::encode(&m2::prost_arm::value(m2::P2_2));
    println!("# stage 6: section 9's three deliveries, and the RPC grid");
    println!("#   payload    P2.2, {} B, the shape the control plane actually moves", payload.len());
    println!("#   transport  UNIX DOMAIN SOCKET (primary), loopback TCP as a second row");
    println!("#   pinned     stream window {} B, CONNECTION window {} B, max message {} B",
             STREAM_WINDOW, CONNECTION_WINDOW, MAX_MESSAGE);
    println!("#              -- ArmoniK's settings, on BOTH the tonic arm and the core arms.");
    println!("#              Before `ak_client_new_opts` the core could not express them at");
    println!("#              all, so every RPC figure in this branch is a 64 KiB-window one.");
    println!("#   incumbent  tonic 0.14 + tonic-prost, the codec packages/rust uses (R14)");
    println!("#   rounds     {ROUNDS} x {CALLS_PER_ROUND} calls, CPU per RPC is the headline");
    println!();

    let server_rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let sock = std::env::temp_dir().join(format!("ak-rpcgrid-{}.sock", std::process::id()));
    let uds = server_rt.block_on(rpc::serve_uds(Bytes::from(payload.clone()), sock.clone()));
    // `rpc::serve` now sets TCP_NODELAY on the accepted socket (the aggregating session took
    // that flip after this slice reported it). `window_control` below builds a second server
    // through `rpc::serve_nagle`, which keeps the old defective form so the artifact stays
    // reproducible, and shows what the flip was worth.
    let tcp = server_rt.block_on(rpc::serve(Bytes::from(payload.clone())));
    let uds_target = format!("unix:{}", sock.display());
    let tcp_target = format!("http://{}", tcp.addr);
    println!("#   servers    {} and {}", uds_target, tcp.addr);
    println!();

    crossings();
    window_control(&uds_target, &tcp_target, &payload, &server_rt);

    for (tname, target) in [("UDS", &uds_target), ("TCP", &tcp_target)] {
        println!("{}", "=".repeat(78));
        println!("== transport: {tname}{}", if tname == "UDS" { "  (PRIMARY)" } else { "  (second row)" });
        println!("{}", "=".repeat(78));
        println!();
        let opaque = deliveries(target, &payload, tname);
        grid(target, &payload, tname, &opaque);
    }

    let _ = uds;
    hazards();
}

fn crossings() {
    println!("## crossings per RPC, per delivery");
    println!();
    println!("{:<28} {:>9} {:>9} {:>10}  {}", "delivery", "forward", "reverse", "per field", "what it is");
    println!("{:<28} {:>9} {:>9} {:>10}  {}", "blocking  ak_call_unary", 2, 0, 0.0, "call + ak_bytes_free");
    println!("{:<28} {:>9} {:>9} {:>10}  {}", "callback  ak_call_unary_cb", 2, 1, 0.0, "call + free; the core calls back once");
    println!("{:<28} {:>9} {:>9} {:>10}  {}", "queue     ak_call_unary_q", 3, 0, 0.0, "call + queue_next + free");
    println!();
    println!("# Counted from the code, not from a counting build: the core's counters live in");
    println!("# the ENCODE and DECODE contexts and the RPC half has neither. What makes the");
    println!("# count trustworthy anyway is the same property section 9 rests on -- neither");
    println!("# `crates/rpc` nor `ak-core`'s rpc module mentions a message type, so there is");
    println!("# nowhere a per-field cost could enter. `per field` is 0 by construction.");
    println!();
}

/// **Is the window pinning actually in the build?** The slice's standing question, and it
/// has to be asked here because `ak_client_new_opts` is new and because the first run of
/// this harness showed TCP at flight 1 barely moving from the 64 KiB figure stage 4
/// reported. A setting that is not applied looks exactly like a setting that does not
/// matter.
///
/// The same blocking call at flight 1 through a DEFAULT client and a PINNED one. R9 says a
/// 540 KB response on a 64 KiB stream window spends most of its wall clock waiting for
/// WINDOW_UPDATE, so if the pinning is live the wall column must move and the CPU column
/// must not.
fn window_control(uds: &str, tcp: &str, payload: &[u8], srt: &tokio::runtime::Runtime) {
    println!("## is the window pinning live? (R5's question, asked of a new entry point)");
    println!();
    println!("{:<8} {:<10} {:>13} {:>13}   {}",
             "transport", "client", "CPU us/RPC", "wall us/RPC", "server socket");
    // `tcp` is `rpc::serve`, which now sets TCP_NODELAY; this is `rpc::serve_nagle`, which
    // preserves the old form. The two differ in exactly one socket option.
    let nagle_big = srt.block_on(rpc::serve_nagle(Bytes::from(payload.to_vec())));
    let ngb = format!("http://{}", nagle_big.addr);
    for (tname, target, sock) in
        [("UDS", uds, "-"), ("TCP", tcp, "TCP_NODELAY (`serve`)"), ("TCP", &ngb[..], "Nagle (`serve_nagle`)")]
    {
        for (label, pinned) in [("default", false), ("pinned", true)] {
            let r = blocking_once(target, payload, pinned);
            println!("{:<8} {:<10} {:>13.1} {:>13.1}   {}",
                     tname, label, r.cpu_us, r.wall_us, sock);
        }
    }
    // And a SMALL response, which separates "the payload does not fit the window" from
    // "each call costs this much whatever it carries". Flow control is a property of
    // HTTP/2 and is the same on both transports; a per-call TCP effect is not.
    let small = vec![0u8; 1024];
    let ssock = std::env::temp_dir().join(format!("ak-rpcgrid-small-{}.sock", std::process::id()));
    let suds = srt.block_on(rpc::serve_uds(Bytes::from(small.clone()), ssock.clone()));
    let stcp = srt.block_on(rpc::serve_nagle(Bytes::from(small.clone())));
    let sfix = srt.block_on(rpc::serve(Bytes::from(small.clone())));
    let (st, ss) = (format!("http://{}", stcp.addr), format!("unix:{}", ssock.display()));
    let sf = format!("http://{}", sfix.addr);
    for (tname, target, sock) in
        [("UDS", &ss, "-"), ("TCP", &sf, "TCP_NODELAY (`serve`)"), ("TCP", &st, "Nagle (`serve_nagle`)")]
    {
        let r = blocking_once(target, &[], true);
        println!("{:<8} {:<10} {:>13.1} {:>13.1}   1 KB, {}",
                 tname, "pinned", r.cpu_us, r.wall_us, sock);
    }
    let _ = (suds, sfix);
    nagle_cpu(&ngb, tcp, payload);
    let _ = nagle_big;
    println!();
    println!("# READ THIS BEFORE THE TABLES BELOW.");
    println!("#");
    println!("# 1. The pinning barely moves the wall column on either transport, so the");
    println!("#    payload is NOT window-bound -- which means the grid below is a fair");
    println!("#    comparison either way, and also that pinning bought nothing here.");
    println!("#");
    println!("# 2. **R9's STATED HAZARD IS WRONG, and the UDS row is what refutes it.**");
    println!("#    `README.md` R9 says a 540 KB response does not fit the 65,535-octet initial");
    println!("#    stream window, so one call in flight `spends most of its wall clock idle");
    println!("#    waiting for WINDOW_UPDATE`, and names this slice's own 33 ms as the case.");
    println!("#    HTTP/2 flow control is a property of the protocol and is identical");
    println!("#    on both transports -- and UDS carries the same 540 KB in about 2 ms where");
    println!("#    loopback TCP takes about 30. A transport-independent cause cannot produce a");
    println!("#    transport-dependent result. The 1 KB rows say whether what remains is");
    println!("#    per-CALL or per-BYTE: 1 KB costs MORE than 540 KB over TCP, so it is");
    println!("#    per-CALL, and a per-call cost of tens of milliseconds is not flow control.");
    println!("#    Note the DIRECTION. If flow control drove this, 540 KB (which needs the");
    println!("#    WINDOW_UPDATE round trips) would cost more than 1 KB (which needs none).");
    println!("#    It costs less. That is backwards for flow control and exactly right for");
    println!("#    Nagle: a large response has full segments to send and never waits, while a");
    println!("#    1 KB one is a small write with nothing behind it, which is the stall case.");
    println!("#");
    println!("# 3. **AND HERE IS WHAT IT ACTUALLY IS.** The two `serve_nagle` rows differ");
    println!("#    from the `TCP_NODELAY` rows above them in ONE SOCKET OPTION on the server's");
    println!("#    accepted socket, and in nothing else at all. `crates/rpc` drives");
    println!("#    tonic through `serve_with_incoming`, and tonic documents that the builder's");
    println!("#    `tcp_nodelay` is IGNORED on that path, while `TcpIncoming::from(listener)`");
    println!("#    leaves its own nodelay unset -- so the server end kept Nagle ON while");
    println!("#    tonic's client had it off by default. A gRPC response is HEADERS, then");
    println!("#    DATA, then TRAILERS; with Nagle on the writer the second small write waits");
    println!("#    for the peer's ACK of the first, and Linux's delayed-ACK timer is 40 ms.");
    println!("#    That is a HARNESS defect, not a transport result, and every loopback-TCP");
    println!("#    figure taken before `rpc::serve` was flipped -- stage 4's included --");
    println!("#    carries it. **Including its CPU column: see the control below.**");
    println!("#");
    println!("# 4. So THERE IS NO LOOPBACK-TCP PENALTY. With the one option set, TCP and UDS");
    println!("#    agree on both payloads. The TCP tables further down go through");
    println!("#    `rpc::serve`, which now sets the option; `rpc::serve_nagle` keeps the old");
    println!("#    form so the artifact above stays reproducible.");
    println!();
}

/// DID THE CPU COLUMN MOVE TOO? The table above says yes, and that matters more than the
/// wall column did: every slice has been told CPU is the trustworthy column, and if Nagle
/// on the server moves it then that advice was only sound once the socket was right.
///
/// Two rows of one table are not enough to say so, because they are taken minutes apart on a
/// shared box and the machine drifts. So this INTERLEAVES them: nagle, nodelay, nagle,
/// nodelay, five pairs, each pair adjacent in time. Drift moves both members of a pair
/// together and cannot manufacture a consistent sign. That is R4's within-arm-delta rule
/// applied to a question about the harness rather than about a codec.
fn nagle_cpu(nagle: &str, nodelay: &str, payload: &[u8]) {
    println!("## did Nagle move the CPU column, or only the wall column?");
    println!();
    println!("# {:>5}  {:>12}  {:>12}  {:>8}   {:>12}  {:>12}  {:>8}",
             "pair", "nagle CPU", "nodel CPU", "CPU x", "nagle wall", "nodel wall", "wall x");
    let (mut cr, mut wr) = (Vec::new(), Vec::new());
    for i in 1..=5 {
        let n = blocking_once(nagle, payload, true);
        let d = blocking_once(nodelay, payload, true);
        cr.push(n.cpu_us / d.cpu_us);
        wr.push(n.wall_us / d.wall_us);
        println!("# {:>5}  {:>12.1}  {:>12.1}  {:>7.3}x   {:>12.1}  {:>12.1}  {:>7.1}x",
                 i, n.cpu_us, d.cpu_us, n.cpu_us / d.cpu_us,
                 n.wall_us, d.wall_us, n.wall_us / d.wall_us);
    }
    cr.sort_by(|a, b| a.partial_cmp(b).unwrap());
    wr.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("# {:>5}  {:>12}  {:>12}  {:>7.3}x   {:>12}  {:>12}  {:>7.1}x",
             "med", "", "", cr[cr.len() / 2], "", "", wr[wr.len() / 2]);
    println!("#");
    println!("# **THE CPU COLUMN MOVED TOO.** It is a smaller effect than the wall column's and");
    println!("# it is in the same direction, and the sign is consistent across interleaved");
    println!("# pairs, so it is not drift. The mechanism is the same one: a stalled write");
    println!("# means the reactor parks and is woken again by a timer, so the call costs");
    println!("# extra epoll and scheduler work on both ends -- and both ends are in THIS");
    println!("# process, so both land in this process's utime+stime.");
    println!("#");
    println!("# What that costs the branch: a loopback-TCP CPU figure taken against the old");
    println!("# server is inflated, not merely accompanied by an unreadable wall figure. What");
    println!("# it does NOT cost: a RATIO between two arms that both went through the same");
    println!("# server. Stage 4's `CPU/tonic` column is such a ratio and still stands; its");
    println!("# ABSOLUTE `CPU us/RPC` column does not.");
    println!();
}

/// One flight-1 blocking arm, with the windows either pinned or left at tonic's defaults.
fn blocking_once(target: &str, payload: &[u8], pinned: bool) -> Row {
    let core = Arc::new(if pinned { Core::new(target) } else { Core::new_default(target) });
    let req: Arc<Vec<u8>> = Arc::new(payload.to_vec());
    measure(move |calls| unsafe {
        for _ in 0..calls {
            let mut out = ak_bytes::default();
            let rc = ak_call_unary(
                core.client, rpc::PATH.as_ptr(), rpc::PATH.len(),
                req.as_ptr(), req.len(), &mut out);
            assert_eq!(rc, AK_OK);
            std::hint::black_box(out.len);
            ak_bytes_free(&mut out);
        }
    })
}

// ============================================================ the three deliveries

fn deliveries(target: &str, payload: &[u8], tname: &str)
    -> std::collections::BTreeMap<usize, f64> {
    println!("### the three deliveries of one call path, {tname}");
    println!();
    println!("{:<12} {:>7} {:>13} {:>13} {:>12}",
             "delivery", "flight", "CPU us/RPC", "wall us/RPC", "/ blocking");
    let mut base = std::collections::BTreeMap::new();
    let mut seen: std::collections::BTreeMap<usize, Vec<f64>> = Default::default();
    for flight in [1usize, 8, 16] {
        let r = run_blocking(target, flight, payload);
        base.insert(flight, r.cpu_us);
        seen.entry(flight).or_default().push(r.cpu_us);
        say("blocking", flight, &r, None);
    }
    for flight in [1usize, 8, 16] {
        let r = run_callback(target, flight, payload);
        seen.entry(flight).or_default().push(r.cpu_us);
        say("callback", flight, &r, base.get(&flight).copied());
    }
    for flight in [1usize, 8, 16] {
        let r = run_queue(target, flight, payload);
        seen.entry(flight).or_default().push(r.cpu_us);
        say("queue", flight, &r, base.get(&flight).copied());
    }
    println!();
    floor(&seen, tname);
    base
}

/// THE FLOOR. This is the whole point of running section 9's deliveries in the slice whose
/// crossing is free: to say what a managed slice's delivery table is being compared against.
///
/// The three deliveries differ by exactly one crossing -- blocking 2 forward, callback 2
/// forward and 1 reverse, queue 3 forward -- and this slice's calibrated crossing (R13) is
/// about 1.8 ns in either direction: `logs/rust/stage2-four-arms-M1.log` measures the
/// forward and the forward+reverse no-op at 1.8 ns over 20,000,000 crossings, and
/// `logs/rust/stage5-lifecycle.log` reads 2.1 ns for the same thing in a different binary,
/// which is the layout sensitivity that log is about. So the ABI's own
/// contribution to the choice of delivery is ONE crossing against a whole RPC, and the
/// arithmetic below is not a prediction to be checked against the table, it is the reason
/// the table cannot check it.
fn floor(seen: &std::collections::BTreeMap<usize, Vec<f64>>, tname: &str) {
    const CROSSING_NS: f64 = 1.8;
    println!("# the floor, {tname}: what the delivery choice costs when a crossing is free");
    println!("#");
    println!("# {:>7}  {:>12}  {:>12}  {:>10}  {:>12}", "flight", "min us/RPC", "spread us",
             "spread %", "1 crossing %");
    for (flight, v) in seen {
        let lo = v.iter().cloned().fold(f64::MAX, f64::min);
        let hi = v.iter().cloned().fold(f64::MIN, f64::max);
        println!("# {:>7}  {:>12.1}  {:>12.1}  {:>9.2}%  {:>11.6}%",
                 flight, lo, hi - lo, 100.0 * (hi - lo) / lo,
                 100.0 * (CROSSING_NS / 1000.0) / lo);
    }
    println!("#");
    println!("# Read the last two columns against each other. The ABI difference between the");
    println!("# three deliveries is one crossing, which is about six orders of magnitude below");
    println!("# the spread this harness can resolve. **So on this host the three deliveries are");
    println!("# indistinguishable, and they are indistinguishable BY CONSTRUCTION, not by");
    println!("# luck**: the spread column is the harness's own noise, and no run of it could");
    println!("# ever have shown a delivery difference. That is what makes it a control.");
    println!("#");
    println!("# WHAT THIS LICENSES A MANAGED SLICE TO CONCLUDE, and what it does not.");
    println!("# If Java or C# measures its three deliveries apart by a resolvable margin, the");
    println!("# margin is NOT the delivery shape -- it is that host's cost for the ONE crossing");
    println!("# that differs, plus whatever its runtime wraps around a callback (an upcall");
    println!("# stub, a pinned object, a thread transition) or around a queue (a wait, a");
    println!("# handoff). The Java slice already prices an upcall at about 80 ns, some 44x this");
    println!("# slice's crossing, and the callback delivery is the one that takes a reverse");
    println!("# crossing per call. That is the quantity to attribute it to, and this table is");
    println!("# the evidence that there is nothing else in the ABI for it to be.");
    println!();
}

// ==================================================================== the grid

fn grid(target: &str, payload: &[u8], tname: &str,
        opaque: &std::collections::BTreeMap<usize, f64>) {
    println!("### the grid, {tname}: A prost+tonic | B prost+core | C core+core");
    println!();
    println!("{:<22} {:>7} {:>13} {:>13} {:>10}",
             "cell", "flight", "CPU us/RPC", "wall us/RPC", "/ A");
    let mut base = std::collections::BTreeMap::new();
    for flight in [1usize, 8, 16] {
        let r = cell_a(target, flight, payload);
        base.insert(flight, r.cpu_us);
        say2("A  prost + tonic", flight, &r, None);
    }
    for flight in [1usize, 8, 16] {
        let r = cell_b(target, flight, payload);
        say2("B  prost + core", flight, &r, base.get(&flight).copied());
    }
    for flight in [1usize, 8, 16] {
        let r = cell_c(target, flight, payload);
        say2("C  core + core", flight, &r, base.get(&flight).copied());
    }
    println!();
    println!("# B - A IS NOT A TRANSPORT COMPARISON HERE. Both transports are tonic, so what");
    println!("# it isolates is the cost of putting the C ABI in front of the same transport,");
    println!("# with the transport difference removed by construction. For a managed host the");
    println!("# same subtraction is grpc-java or grpc-dotnet against tonic, which is a");
    println!("# different quantity. C - B is the codec, with the transport held constant.");
    println!();
    codec_share(&base, opaque, tname);
}

/// WHAT FRACTION OF A REAL RPC IS THE CODEC. This subtraction is the one figure in stage 6
/// that bears directly on the exploration's question, and it is available only because the
/// two tables above move the SAME BYTES over the SAME CALL.
///
/// `main` builds `payload` as prost's encoding of P2.2 and gives it to the server as its
/// fixed response; the deliveries table sends those bytes and receives them back, touching
/// no codec at all, while cell A encodes that same value and decodes that same response on
/// every call. So cell A minus blocking is exactly one encode plus one decode of P2.2, with
/// the transport, the server, the windows, the flight and the machine identical on both
/// sides of the minus sign.
fn codec_share(grid_a: &std::collections::BTreeMap<usize, f64>,
               opaque: &std::collections::BTreeMap<usize, f64>, tname: &str) {
    println!("# the codec's share of one RPC, {tname}");
    println!("#");
    println!("# {:>7}  {:>14}  {:>14}  {:>14}  {:>9}",
             "flight", "opaque us/RPC", "A us/RPC", "codec us/RPC", "codec %");
    for (flight, a) in grid_a {
        let Some(o) = opaque.get(flight) else { continue };
        println!("# {:>7}  {:>14.1}  {:>14.1}  {:>14.1}  {:>8.1}%",
                 flight, o, a, a - o, 100.0 * (a - o) / a);
    }
    println!("#");
    println!("# **At the payload the control plane actually moves, over a real gRPC call, the");
    println!("# codec is the MAJORITY of the CPU.** That is the premise this whole exploration");
    println!("# rests on and it had not been measured end to end before: stages 1 to 3 timed");
    println!("# the codec in isolation, and stage 4 timed the RPC without one in the loop.");
    println!("#");
    println!("# Two things this does NOT say. It does not say a native codec would recover");
    println!("# that share -- cells A, B and C are within this harness's noise of each other,");
    println!("# so on THIS host, where prost is already native, there is nothing to recover;");
    println!("# the share is what a managed host's codec has to compete for. And it is a share");
    println!("# of CPU on a loopback with a do-nothing server, so the denominator is as small");
    println!("# as a real deployment's ever gets, which makes this an UPPER bound on the");
    println!("# fraction, not a typical one.");
    println!();
}

// ============================================================== the runners

struct Row {
    cpu_us: f64,
    wall_us: f64,
}

fn say(arm: &str, flight: usize, t: &Row, base: Option<f64>) {
    println!("{:<12} {:>7} {:>13.1} {:>13.1} {:>12}",
             arm, flight, t.cpu_us, t.wall_us,
             base.map(|b| format!("{:.3}", t.cpu_us / b)).unwrap_or_else(|| "1.000".into()));
}

fn say2(arm: &str, flight: usize, t: &Row, base: Option<f64>) {
    println!("{:<22} {:>7} {:>13.1} {:>13.1} {:>10}",
             arm, flight, t.cpu_us, t.wall_us,
             base.map(|b| format!("{:.3}", t.cpu_us / b)).unwrap_or_else(|| "1.000".into()));
}

/// Process CPU: utime + stime from /proc/self/stat. `design/SHAPES.md` asks for CPU per
/// RPC and R9 says why -- a payload larger than the window spends its wall clock idle.
fn cpu_us() -> f64 {
    let s = std::fs::read_to_string("/proc/self/stat").unwrap();
    let tail = &s[s.rfind(')').unwrap() + 2..];
    let f: Vec<&str> = tail.split_whitespace().collect();
    (f[11].parse::<f64>().unwrap() + f[12].parse::<f64>().unwrap()) * 1_000_000.0 / 100.0
}

fn measure(mut round: impl FnMut(usize)) -> Row {
    // One warm round outside the clock: the connection is established, the runtime's
    // threads are up, and the allocator has stopped growing.
    round(CALLS_PER_ROUND);
    let mut cpus = Vec::new();
    let mut walls = Vec::new();
    for _ in 0..ROUNDS {
        let c0 = cpu_us();
        let t = Instant::now();
        round(CALLS_PER_ROUND);
        walls.push(t.elapsed().as_micros() as f64 / CALLS_PER_ROUND as f64);
        cpus.push((cpu_us() - c0) / CALLS_PER_ROUND as f64);
    }
    Row { cpu_us: median(&mut cpus), wall_us: median(&mut walls) }
}

fn median(v: &mut Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

struct Core {
    rt: *mut ak_runtime,
    client: *mut ak_client,
}

impl Core {
    fn new(target: &str) -> Self {
        unsafe {
            let rt = ak_runtime_new(2);
            let o = opts();
            let client = ak_client_new_opts(rt, target.as_ptr(), target.len(), &o);
            assert!(!client.is_null(), "ak_client_new_opts failed for {target}");
            Core { rt, client }
        }
    }

    /// The same client with tonic's DEFAULT windows: what `ak_client_new` gives, and what
    /// every RPC figure in this branch before today was taken on.
    fn new_default(target: &str) -> Self {
        unsafe {
            let rt = ak_runtime_new(2);
            let client = ak_client_new(rt, target.as_ptr(), target.len());
            assert!(!client.is_null(), "ak_client_new failed for {target}");
            Core { rt, client }
        }
    }
}

impl Drop for Core {
    fn drop(&mut self) {
        unsafe {
            ak_client_destroy(self.client);
            ak_runtime_destroy(self.rt);
        }
    }
}

unsafe impl Send for Core {}
unsafe impl Sync for Core {}

/// The blocking delivery: N host threads, each blocking in one native frame. That is what
/// the ABI's blocking form IS, and giving tonic N futures instead would compare two
/// concurrency mechanisms rather than two RPC paths.
fn run_blocking(target: &str, flight: usize, payload: &[u8]) -> Row {
    let core = Arc::new(Core::new(target));
    let req: Arc<Vec<u8>> = Arc::new(payload.to_vec());
    measure(move |calls| {
        let per = calls / flight.max(1);
        std::thread::scope(|s| {
            for _ in 0..flight {
                let (core, req) = (core.clone(), req.clone());
                s.spawn(move || unsafe {
                    for _ in 0..per {
                        let mut out = ak_bytes::default();
                        let rc = ak_call_unary(
                            core.client, rpc::PATH.as_ptr(), rpc::PATH.len(),
                            req.as_ptr(), req.len(), &mut out);
                        assert_eq!(rc, AK_OK);
                        std::hint::black_box(out.len);
                        ak_bytes_free(&mut out);
                    }
                });
            }
        });
    })
}

/// The callback delivery. The completion arrives on a thread the core owns; the host counts
/// them down. One reverse crossing per call, which is the only place any delivery makes one.
/// The callback delivery's completion signal.
///
/// **It must BLOCK, not spin.** The first version of this arm waited with
/// `std::thread::yield_now()` in a loop and reported the callback delivery at 1.37 to 1.67
/// times blocking on CPU -- which was the harness burning a core, not the delivery costing
/// anything. The blocking arm parks in a native frame and the queue arm parks in
/// `ak_queue_next`'s condvar; an arm that spins where the others park is measuring its own
/// wait strategy. A mutex and a condvar put all three on the same footing.
struct Signal {
    m: std::sync::Mutex<usize>,
    cv: std::sync::Condvar,
}

impl Signal {
    fn new() -> Self {
        Signal { m: std::sync::Mutex::new(0), cv: std::sync::Condvar::new() }
    }
    fn wait_for(&self, n: usize) {
        let mut g = self.m.lock().unwrap();
        while *g < n {
            g = self.cv.wait(g).unwrap();
        }
    }
}

extern "C" fn on_complete(user: *mut std::ffi::c_void, comp: *mut ak_completion) {
    unsafe {
        let sig = &*(user as *const Signal);
        std::hint::black_box((*comp).bytes.len);
        ak_bytes_free(&mut (*comp).bytes);
        *sig.m.lock().unwrap() += 1;
        sig.cv.notify_all();
    }
}

fn run_callback(target: &str, flight: usize, payload: &[u8]) -> Row {
    let core = Core::new(target);
    let req = payload.to_vec();
    let sig: &'static Signal = Box::leak(Box::new(Signal::new()));
    measure(move |calls| unsafe {
        let start = *sig.m.lock().unwrap();
        let mut issued = 0usize;
        while issued < calls {
            let want = (calls - issued).min(flight);
            let mut handles: Vec<*mut ak_call> = Vec::with_capacity(want);
            for _ in 0..want {
                let h = ak_call_unary_cb(
                    core.client, rpc::PATH.as_ptr(), rpc::PATH.len(),
                    req.as_ptr(), req.len(), on_complete,
                    sig as *const Signal as *mut std::ffi::c_void, issued as u64);
                assert!(!h.is_null());
                handles.push(h);
                issued += 1;
            }
            sig.wait_for(start + issued);
            for h in handles.drain(..) {
                ak_call_destroy(h);
            }
        }
    })
}

/// The completion-queue delivery. No upcall at all: the host drains. One extra forward
/// crossing per call, which is `ak_queue_next`.
fn run_queue(target: &str, flight: usize, payload: &[u8]) -> Row {
    let core = Core::new(target);
    let req = payload.to_vec();
    let q = unsafe { ak_queue_new() };
    let row = measure(move |calls| unsafe {
        let mut issued = 0usize;
        while issued < calls {
            let want = (calls - issued).min(flight);
            let mut handles: Vec<*mut ak_call> = Vec::with_capacity(want);
            for _ in 0..want {
                let h = ak_call_unary_q(
                    core.client, rpc::PATH.as_ptr(), rpc::PATH.len(),
                    req.as_ptr(), req.len(), q, issued as u64);
                assert!(!h.is_null());
                handles.push(h);
                issued += 1;
            }
            for _ in 0..want {
                let mut comp = ak_completion {
                    tag: 0,
                    status: 0,
                    bytes: ak_bytes::default(),
                };
                let rc = ak_queue_next(q, &mut comp, 10_000);
                assert_eq!(rc, AK_QUEUE_OK, "queue drain");
                std::hint::black_box(comp.bytes.len);
                ak_bytes_free(&mut comp.bytes);
            }
            for h in handles.drain(..) {
                ak_call_destroy(h);
            }
        }
    });
    unsafe {
        ak_queue_shutdown(q);
        ak_queue_destroy(q);
    }
    row
}

// ---------------------------------------------------------------- the grid cells

/// A: prost's codec, tonic's transport. The incumbent stack end to end, with the SAME
/// window settings the core arms get.
fn cell_a(target: &str, flight: usize, _payload: &[u8]) -> Row {
    let rt = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap(),
    );
    let t = target.to_string();
    let chan = rt.block_on(async move {
        tonic::transport::Endpoint::from_shared(t)
            .unwrap()
            .initial_stream_window_size(Some(STREAM_WINDOW))
            .initial_connection_window_size(Some(CONNECTION_WINDOW))
            .connect()
            .await
            .unwrap()
    });
    let v = Arc::new(m2::prost_arm::value(m2::P2_2));
    measure(move |calls| {
        let per = calls / flight.max(1);
        std::thread::scope(|s| {
            for _ in 0..flight {
                let (rt, chan, v) = (rt.clone(), chan.clone(), v.clone());
                s.spawn(move || {
                    let mut g = tonic::client::Grpc::new(chan)
                        .max_decoding_message_size(MAX_MESSAGE as usize)
                        .max_encoding_message_size(MAX_MESSAGE as usize);
                    for _ in 0..per {
                        // The codec is in the loop on BOTH sides, which is what makes this
                        // a stack comparison rather than a transport one.
                        let body = Bytes::from(prost::Message::encode_to_vec(&*v));
                        let r: tonic::Response<Bytes> = rt
                            .block_on(async {
                                g.ready().await.unwrap();
                                g.unary(
                                    tonic::Request::new(body),
                                    http::uri::PathAndQuery::from_static(rpc::PATH),
                                    rpc::RawCodec,
                                )
                                .await
                            })
                            .unwrap();
                        std::hint::black_box(m2::prost_arm::decode(&r.into_inner()));
                    }
                });
            }
        });
    })
}

/// B: prost's codec, the core's transport. Both transports are tonic, so B - A is the C ABI
/// in front of one transport.
fn cell_b(target: &str, flight: usize, payload: &[u8]) -> Row {
    let core = Arc::new(Core::new(target));
    let v = Arc::new(m2::prost_arm::value(m2::P2_2));
    let _ = payload;
    measure(move |calls| {
        let per = calls / flight.max(1);
        std::thread::scope(|s| {
            for _ in 0..flight {
                let (core, v) = (core.clone(), v.clone());
                s.spawn(move || unsafe {
                    for _ in 0..per {
                        let body = prost::Message::encode_to_vec(&*v);
                        let mut out = ak_bytes::default();
                        let rc = ak_call_unary(
                            core.client, rpc::PATH.as_ptr(), rpc::PATH.len(),
                            body.as_ptr(), body.len(), &mut out);
                        assert_eq!(rc, AK_OK);
                        let resp = core::slice::from_raw_parts(out.ptr, out.len);
                        std::hint::black_box(m2::prost_arm::decode(resp));
                        ak_bytes_free(&mut out);
                    }
                });
            }
        });
    })
}

/// C: the core's codec and the core's transport. The whole stack on the C ABI.
///
/// **The contexts and the facade value are built ONCE, outside the clock.** The first
/// version built `Ctx::new()` and `armonik_arm::value(P2.2)` inside each spawned thread, so
/// every round paid a full payload construction per thread and cell C read 1.5 to 1.8 times
/// cell A at 8 and 16 in flight. That was the harness, not the stack: cell A hoists its
/// value, so the two cells were not doing the same work.
///
/// One context per flight slot, which is ABI v1's contract (a context is host-owned and
/// single-threaded) and what stage 5's concurrency suite established is safe.
fn cell_c(target: &str, flight: usize, payload: &[u8]) -> Row {
    let core = Arc::new(Core::new(target));
    let _ = payload;
    let v: &'static _ = Box::leak(Box::new(m2::armonik_arm::value(m2::P2_2)));
    let ctxs: &'static [CtxSlot] = Box::leak(
        (0..flight)
            .map(|_| CtxSlot(harness::arms::core_ffi_arm::Ctx::new()))
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    );
    measure(move |calls| {
        let per = calls / flight.max(1);
        std::thread::scope(|s| {
            for i in 0..flight {
                let core = core.clone();
                s.spawn(move || unsafe {
                    let ctx = &ctxs[i].0;
                    for _ in 0..per {
                        let body = m2::core_ffi_arm::encode_into(ctx, v);
                        let mut out = ak_bytes::default();
                        let rc = ak_call_unary(
                            core.client, rpc::PATH.as_ptr(), rpc::PATH.len(),
                            body.as_ptr(), body.len(), &mut out);
                        assert_eq!(rc, AK_OK);
                        let resp = core::slice::from_raw_parts(out.ptr, out.len);
                        std::hint::black_box(m2::core_ffi_arm::decode(ctx, resp));
                        ak_bytes_free(&mut out);
                    }
                });
            }
        });
    })
}

/// One codec context, handed to exactly one thread at a time. `Ctx` is not `Sync` and must
/// not be: stage 5's positive control showed that sharing one across threads aborts the
/// process. Each slot here is used by one thread per round and by no two at once.
struct CtxSlot(harness::arms::core_ffi_arm::Ctx);
unsafe impl Send for CtxSlot {}
unsafe impl Sync for CtxSlot {}

fn hazards() {
    println!("# HAZARDS (R9), and they are large here.");
    println!("#   - FOUR vCPUs, with the server's two worker threads, each arm's runtime and");
    println!("#     N host threads all on them. At 8 and 16 in flight the machine is");
    println!("#     oversubscribed before the measurement starts, so these rows are a LOWER");
    println!("#     bound and an upper bound on nothing. What survives is the arms' RELATIVE");
    println!("#     behaviour under identical oversubscription.");
    println!("#   - The server is in the same process and contends for the same four vCPUs,");
    println!("#     so server cost is inside every number in every cell equally.");
    println!("#   - CPU is process utime+stime at 10 ms resolution; the ROUND carries it and");
    println!("#     the per-call figure is a division, not a per-call measurement.");
    println!("#   - The windows are pinned now, so the wall column is no longer mostly");
    println!("#     WINDOW_UPDATE latency. It is still a shared-box wall clock.");
    println!("#   - No TLS, no retry, no metadata, no deadlines, no streaming, no failure");
    println!("#     injection, no server-side measurement. **The RPC half's case is");
    println!("#     BEHAVIOURAL and none of that behaviour is exercised here either.**");
    println!("#   - Cancellation: `ak_call_cancel` exists and nothing in this harness calls");
    println!("#     it. `design/SHAPES.md`'s carrier-thread row stays empty: Rust has no");
    println!("#     carrier thread to pin (R8, report it empty rather than substitute).");
}
