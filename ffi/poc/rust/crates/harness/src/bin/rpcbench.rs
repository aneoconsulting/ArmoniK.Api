//! Stage 4: the RPC arm. One unary RPC carrying P2.2 over loopback, against tonic.
//!
//! `design/SHAPES.md` asks for three things and this answers two of them. The third is
//! "whether the language's idiomatic wait can be satisfied without pinning a carrier
//! thread", and **Rust cannot test what that requirement exists for**: there is no carrier
//! thread to pin, because Rust has no virtual threads and a blocking call from a host thread
//! blocks that thread and nothing else. README R8's rule applies -- a slice that cannot run
//! an arm says so rather than substituting a different comparison -- so that row is reported
//! empty and not filled.
//!
//! Both arms drive N HOST THREADS, each performing one blocking unary call. That is on
//! purpose: the ABI's blocking form is a host thread blocking in a native frame, and giving
//! tonic N futures on a runtime instead would compare two concurrency mechanisms rather than
//! two RPC paths. A sync tonic user does exactly what the tonic arm does here.

use ak_abi::{ak_bytes, ak_bytes_free, ak_call_unary, ak_client_new, ak_runtime_new};
use bytes::Bytes;
use harness::arms_m2 as m2;
use std::sync::Arc;
use std::time::Instant;

const ROUNDS: usize = 9;
const CALLS_PER_ROUND: usize = 200;

fn main() {
    let payload = m2::prost_arm::encode(&m2::prost_arm::value(m2::P2_2));
    println!("# stage 4: the RPC arm");
    println!("#   payload   P2.2, {} B, the shape the control plane actually moves", payload.len());
    println!("#   transport loopback TCP, h2, no TLS");
    println!("#   incumbent tonic 0.14 + tonic-prost, the codec packages/rust uses");
    println!("#   arm       core-ffi-rust: ak_call_unary through the C ABI, opaque bytes");
    println!("#   both arms drive N host threads, each blocking in one unary call");
    println!();

    let server_rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let srv = server_rt.block_on(rpc::serve(Bytes::from(payload.clone())));
    let uri = format!("http://{}", srv.addr);
    println!("#   server    {} (2 worker threads, answers with a fixed body)", srv.addr);
    println!();

    // ---- crossing count per RPC, which is the number the fallback recommendation rests on.
    println!("## crossings per RPC");
    println!();
    println!("{:<26} {:>10} {:>10} {}", "layer", "per RPC", "per field", "what it is");
    println!("{:<26} {:>10} {:>10} {}", "RPC (ak_call_unary)", 1, "-", "one call in, opaque bytes");
    println!("{:<26} {:>10} {:>10} {}", "RPC (ak_bytes_free)", 1, "-", "the host releases the response");
    println!("{:<26} {:>10} {:>10} {}", "  RPC TOTAL", 2, 0.0, "NOT a function of field count");
    println!();
    println!("# The codec's crossings are separate and already counted (stage 3): 10.02 per");
    println!("# element on encode and 7.00 on decode for this message. The RPC half adds two");
    println!("# per call and nothing else, because nothing in crates/rpc or in ak-core's rpc");
    println!("# module mentions a message type: it dispatches on a path string and moves");
    println!("# opaque bytes. That is the claim ABI v1 section 9 makes and the one the");
    println!("# 'adopt the RPC layer, generate the codec' fallback rests on.");
    println!();

    // ---- CPU per RPC at 1, 8 and 16 in flight.
    println!("## CPU per RPC, and allocation");
    println!();
    println!("{:<16} {:>6} {:>13} {:>13} {:>10} {:>12}",
             "arm", "flight", "CPU us/RPC", "wall us/RPC", "CPU/tonic", "bytes/RPC");
    let mut base = std::collections::BTreeMap::new();
    for flight in [1usize, 8, 16] {
        let t = tonic_arm(&uri, flight, &payload);
        base.insert(flight, t.cpu_us);
        row("tonic", flight, &t, None);
    }
    for flight in [1usize, 8, 16] {
        let t = ffi_arm(&uri, flight, &payload);
        row("core-ffi-rust", flight, &t, base.get(&flight).copied());
    }

    println!();
    println!("# HAZARDS (R9), and they are large here.");
    println!("#   - FOUR vCPUs in a container, with the server's two worker threads, the");
    println!("#     client's runtime and N host threads all on them. At 8 and 16 in flight");
    println!("#     the machine is oversubscribed before the measurement starts, so these");
    println!("#     rows are a LOWER BOUND on what the two paths can do and an upper bound on");
    println!("#     nothing. What they can still show is the two arms' RELATIVE behaviour");
    println!("#     under identical oversubscription, which is why the ratio column exists");
    println!("#     and the absolute column is there only to be reproducible.");
    println!("#   - Loopback TCP, no TLS, no retry, no metadata, no deadlines. The RPC half's");
    println!("#     case is behavioural (one retry set, one backoff, one TLS configuration)");
    println!("#     and NONE of that is exercised: this measures the call path only.");
    println!("#   - The server is the same process. Client and server contend for the same");
    println!("#     four vCPUs, so server cost is inside every number.");
    println!("#   - CPU is process utime+stime from /proc/self/stat at 10 ms resolution, so a");
    println!("#     round has to be long enough to carry it; the ROUND is long enough and the");
    println!("#     per-call figure is a division, not a per-call measurement.");
    println!("#   - CPU and WALL differ by an order of magnitude at flight 1 and that is h2");
    println!("#     flow control, not the RPC path: a 540 KB response exceeds the default");
    println!("#     64 KB stream window, so a single call in flight spends most of its time");
    println!("#     idle waiting for WINDOW_UPDATE. With 8 in flight the stalls overlap. Read");
    println!("#     the CPU column for what the paths cost and the wall column only as a");
    println!("#     reminder that this payload does not fit a default window.");
    println!("#   - `bytes/RPC` counts the response body the host ends up holding, not total");
    println!("#     allocation: nothing here instruments the allocator.");
    println!();
    println!("# NOT MEASURED, and not substituted for (README R8):");
    println!("#   - whether an idiomatic wait pins a carrier thread. Rust has no carrier");
    println!("#     thread to pin, so the requirement has no Rust test. It is a JVM question");
    println!("#     and the Java slice is where it is answered.");
    println!("#   - the callback and completion-queue delivery modes of ABI v1 section 9.");
    println!("#   - streaming, TLS, a real network, failure injection, the server side.");
    println!("#   - cancellation: section 9 gives the blocking call a handle so it can be");
    println!("#     cancelled, and ak_call_unary here takes none.");
}

/// Process CPU time in microseconds: utime + stime from /proc/self/stat.
///
/// `design/SHAPES.md` asks for CPU per RPC, not wall-clock per RPC, and on this payload the
/// difference is the whole measurement: a 540 KB response exceeds h2's default 64 KB stream
/// window, so with ONE call in flight the connection spends most of each call idle waiting
/// for WINDOW_UPDATE round trips. Wall-clock at flight 1 is 33 ms and CPU is a fraction of
/// it; with 8 in flight the stalls overlap and wall-clock collapses. Reporting wall-clock
/// alone would have said the RPC path costs 33 ms per call, which is a statement about h2
/// flow control and about nothing this branch is deciding.
fn cpu_us() -> f64 {
    let s = std::fs::read_to_string("/proc/self/stat").unwrap();
    // Field 2 is the comm and may contain spaces; everything after the last ')' is fixed.
    let tail = &s[s.rfind(')').unwrap() + 2..];
    let f: Vec<&str> = tail.split_whitespace().collect();
    let ticks: f64 = f[11].parse::<f64>().unwrap() + f[12].parse::<f64>().unwrap();
    ticks * 1_000_000.0 / 100.0
}

struct Row {
    cpu_us: f64,
    wall_us: f64,
    bytes: usize,
}

fn row(arm: &str, flight: usize, t: &Row, base: Option<f64>) {
    println!("{:<16} {:>6} {:>13.1} {:>13.1} {:>10} {:>12}",
             arm, flight, t.cpu_us, t.wall_us,
             base.map(|b| format!("{:.3}", t.cpu_us / b)).unwrap_or_else(|| "1.000".into()),
             t.bytes);
}

fn tonic_arm(uri: &str, flight: usize, _payload: &[u8]) -> Row {
    let rt = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap(),
    );
    let chan = rt
        .block_on(async { tonic::transport::Endpoint::from_shared(uri.to_string()).unwrap().connect().await.unwrap() });
    let mut times = Vec::new();
    let mut cpus = Vec::new();
    let mut bytes = 0usize;
    for _ in 0..ROUNDS {
        let c0 = cpu_us();
        let t = Instant::now();
        let mut hs = Vec::new();
        for _ in 0..flight {
            let rt = rt.clone();
            let chan = chan.clone();
            hs.push(std::thread::spawn(move || {
                let mut g = tonic::client::Grpc::new(chan);
                let mut n = 0usize;
                for _ in 0..CALLS_PER_ROUND / flight.max(1) {
                    let r: tonic::Response<Bytes> = rt
                        .block_on(async {
                            g.ready().await.unwrap();
                            g.unary(
                                tonic::Request::new(Bytes::from_static(b"")),
                                http::uri::PathAndQuery::from_static(rpc::PATH),
                                rpc::RawCodec,
                            )
                            .await
                        })
                        .unwrap();
                    n += r.into_inner().len();
                }
                n
            }));
        }
        let got: usize = hs.into_iter().map(|h| h.join().unwrap()).sum();
        let calls = (CALLS_PER_ROUND / flight.max(1)) * flight;
        times.push(t.elapsed().as_nanos() as f64 / calls as f64 / 1000.0);
        cpus.push((cpu_us() - c0) / calls as f64);
        bytes = got / calls;
    }
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    cpus.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Row { cpu_us: cpus[cpus.len() / 2], wall_us: times[times.len() / 2], bytes }
}

fn ffi_arm(uri: &str, flight: usize, _payload: &[u8]) -> Row {
    let mut times = Vec::new();
    let mut cpus = Vec::new();
    let mut bytes = 0usize;
    unsafe {
        let rt = ak_runtime_new(2);
        let client = ak_client_new(rt, uri.as_ptr(), uri.len());
        assert!(!client.is_null(), "core client did not connect");
        let client = client as usize;
        for _ in 0..ROUNDS {
            let c0 = cpu_us();
            let t = Instant::now();
            let mut hs = Vec::new();
            for _ in 0..flight {
                hs.push(std::thread::spawn(move || {
                    let c = client as *mut ak_abi::ak_client;
                    let mut n = 0usize;
                    for _ in 0..CALLS_PER_ROUND / flight.max(1) {
                        let mut out = ak_bytes::default();
                        let rc = ak_call_unary(
                            c,
                            rpc::PATH.as_ptr(),
                            rpc::PATH.len(),
                            [].as_ptr(),
                            0,
                            &mut out,
                        );
                        assert_eq!(rc, 0, "ak_call_unary failed");
                        n += out.len;
                        ak_bytes_free(&mut out);
                    }
                    n
                }));
            }
            let got: usize = hs.into_iter().map(|h| h.join().unwrap()).sum();
            let calls = (CALLS_PER_ROUND / flight.max(1)) * flight;
            times.push(t.elapsed().as_nanos() as f64 / calls as f64 / 1000.0);
            cpus.push((cpu_us() - c0) / calls as f64);
            bytes = got / calls;
        }
        ak_client_destroy(client as *mut ak_abi::ak_client);
        ak_runtime_destroy(rt);
    }
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    cpus.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Row { cpu_us: cpus[cpus.len() / 2], wall_us: times[times.len() / 2], bytes }
}

use ak_abi::{ak_client_destroy, ak_runtime_destroy};
