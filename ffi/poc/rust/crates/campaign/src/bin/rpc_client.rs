//! CAMPAIGN.md 4.2: the RPC grid's client, pinned by the runner to `AK_CPU_CLIENT`; the
//! server is `rpc_server`, another process on `AK_CPU_SERVER` (requirement 13).
//!
//!   rpc_client --port N --transport shipped|pinned --launch L --rounds R --calls C
//!              --warmup W --out FILE [--plant]
//!
//! Cells (requirement 12), directions (14), in-flight 1/8/16 (15):
//!   A  prost through tonic's codec calls (`Message::encode` into the EncodeBuf, `decode`
//!      from the DecodeBuf), tonic's transport
//!   B  prost, the core's transport, BLOCKING delivery (`ak_call_unary`, requirement 16)
//!   C  the core's codec through the C ABI (generated binding), the core's transport, blocking
//!   D  the core's codec through the C ABI, tonic's transport (raw-bytes codec)
//!   (a) empty request, P2.2 response; (b) P2.2 request the server decodes, empty response.
//! In flight k = k host threads, each making calls back to back (the blocking shape; the
//! tonic cells block on a shared runtime per call, the same shape).
//! Every call is checked (requirement 18): status OK and the response length equal to the
//! expected one; cells C and D also require the core's decode to succeed. The first failure
//! aborts the process with no output at all (samples are kept in memory and written only at
//! the end). `--plant` expects a wrong length, so the run must abort (the runner's control).
//! CPU: getrusage(RUSAGE_SELF) of this process per round, wall beside it (requirement 21).
//! Transport (17): `shipped` = what packages/rust configures (tonic's endpoint defaults, TCP
//! nodelay since `GrpcClient__TcpNagleAlgorithm` defaults to false; `ak_client_new`), `pinned`
//! = 4 MiB stream and connection windows, adaptive off, Nagle off, on both transports.

use ak_abi::*;
use bytes::Bytes;
use campaign::process_cpu_ns;
use harness::arms_m2 as m2;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};

const FETCH: &str = "/armonik.ffi.campaign.v1.Grid/Fetch";
const PUSH: &str = "/armonik.ffi.campaign.v1.Grid/Push";
const WIN: u32 = 4 * 1024 * 1024;

// ---- cell A's codec: tonic-prost's encoder and decoder bodies, with the request held in an
// Arc (tonic takes the request by value; cloning a P2.2 graph per call would be work
// production does not do) and the response's wire length recorded for requirement 18.
struct PCodec<Q, S> {
    len: Arc<AtomicU64>,
    _p: std::marker::PhantomData<(Q, S)>,
}
struct PEnc<Q>(std::marker::PhantomData<Q>);
struct PDec<S>(Arc<AtomicU64>, std::marker::PhantomData<S>);
impl<Q: prost::Message + Send + Sync + 'static> Encoder for PEnc<Q> {
    type Item = Arc<Q>;
    type Error = tonic::Status;
    fn encode(&mut self, item: Arc<Q>, dst: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        item.encode(dst).expect("Message only errors if not enough space");
        Ok(())
    }
}
impl<S: prost::Message + Default + Send + 'static> Decoder for PDec<S> {
    type Item = S;
    type Error = tonic::Status;
    fn decode(&mut self, src: &mut DecodeBuf<'_>) -> Result<Option<S>, Self::Error> {
        use bytes::Buf;
        self.0.store(src.remaining() as u64, Ordering::Relaxed);
        S::decode(src).map(Some).map_err(|e| tonic::Status::internal(e.to_string()))
    }
}
impl<Q, S> Codec for PCodec<Q, S>
where
    Q: prost::Message + Send + Sync + 'static,
    S: prost::Message + Default + Send + 'static,
{
    type Encode = Arc<Q>;
    type Decode = S;
    type Encoder = PEnc<Q>;
    type Decoder = PDec<S>;
    fn encoder(&mut self) -> PEnc<Q> {
        PEnc(std::marker::PhantomData)
    }
    fn decoder(&mut self) -> PDec<S> {
        PDec(self.len.clone(), std::marker::PhantomData)
    }
}

struct CoreClient {
    rt: *mut ak_runtime,
    client: *mut ak_client,
}
unsafe impl Send for CoreClient {}
unsafe impl Sync for CoreClient {}
impl CoreClient {
    fn new(target: &str, pinned: bool) -> Self {
        unsafe {
            let rt = ak_runtime_new(2);
            let client = if pinned {
                let o = ak_client_opts {
                    stream_window: WIN,
                    connection_window: WIN,
                    adaptive_window: 0,
                    max_recv_message: 0,
                    max_send_message: 0,
                    tcp_nagle: 0,
                };
                ak_client_new_opts(rt, target.as_ptr(), target.len(), &o)
            } else {
                ak_client_new(rt, target.as_ptr(), target.len())
            };
            assert!(!client.is_null(), "core client for {target}");
            CoreClient { rt, client }
        }
    }
    /// One blocking call; the response bytes are handed to `f` and freed after.
    fn call<T>(&self, path: &str, req: &[u8], f: impl FnOnce(&[u8]) -> Result<T, String>) -> Result<T, String> {
        unsafe {
            let mut out = ak_bytes::default();
            let rc = ak_call_unary(self.client, path.as_ptr(), path.len(), req.as_ptr(), req.len(), &mut out);
            if rc != AK_OK {
                return Err(format!("ak_call_unary rc {rc}"));
            }
            let r = f(if out.len == 0 { &[] } else { std::slice::from_raw_parts(out.ptr, out.len) });
            ak_bytes_free(&mut out);
            r
        }
    }
}
impl Drop for CoreClient {
    fn drop(&mut self) {
        unsafe {
            ak_client_destroy(self.client);
            ak_runtime_destroy(self.rt);
        }
    }
}

struct Slot(harness::arms::core_ffi_arm::Ctx);
unsafe impl Send for Slot {}
unsafe impl Sync for Slot {}

type CallFn = Arc<dyn Fn(usize) -> Result<(), String> + Send + Sync>;

fn tonic_channel(rt: &tokio::runtime::Runtime, target: &str, pinned: bool) -> tonic::transport::Channel {
    let t = target.to_string();
    rt.block_on(async move {
        let mut e = tonic::transport::Endpoint::from_shared(t).unwrap().tcp_nodelay(true);
        if pinned {
            e = e
                .initial_stream_window_size(Some(WIN))
                .initial_connection_window_size(Some(WIN))
                .http2_adaptive_window(false);
        }
        e.connect().await.unwrap()
    })
}

/// The call function of one (cell, direction) at in-flight k: thread i uses slot i.
fn make(cell: &str, dir: &'static str, k: usize, target: &str, pinned: bool, want_a: u64) -> CallFn {
    let want = if dir == "a" { want_a } else { 0 };
    let p_val = Arc::new(m2::prost_arm::value(m2::P2_2));
    let f_val: &'static _ = Box::leak(Box::new(m2::armonik_arm::value(m2::P2_2)));
    let slots: &'static [Slot] = Box::leak((0..k).map(|_| Slot(harness::arms::core_ffi_arm::Ctx::new()))
        .collect::<Vec<_>>().into_boxed_slice());
    let path = if dir == "a" { FETCH } else { PUSH };
    match cell {
        "A" => {
            let rt = Arc::new(tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap());
            let ch = tonic_channel(&rt, target, pinned);
            let empty = Arc::new(shapes_prost::shapes::Empty {});
            Arc::new(move |_i| {
                let len = Arc::new(AtomicU64::new(u64::MAX));
                let mut g = tonic::client::Grpc::new(ch.clone());
                let pq = http::uri::PathAndQuery::from_static(path);
                let r = rt.block_on(async {
                    g.ready().await.map_err(|e| e.to_string())?;
                    if dir == "a" {
                        let c = PCodec::<shapes_prost::shapes::Empty, shapes_prost::shapes::ListTasksDetailedResponse> { len: len.clone(), _p: Default::default() };
                        g.unary(tonic::Request::new(empty.clone()), pq, c).await.map(|r| { std::hint::black_box(r.into_inner()); }).map_err(|s| s.to_string())
                    } else {
                        let c = PCodec::<shapes_prost::shapes::ListTasksDetailedResponse, shapes_prost::shapes::Empty> { len: len.clone(), _p: Default::default() };
                        g.unary(tonic::Request::new(p_val.clone()), pq, c).await.map(|r| { std::hint::black_box(r.into_inner()); }).map_err(|s| s.to_string())
                    }
                });
                r?;
                let got = len.load(Ordering::Relaxed);
                if got != want { return Err(format!("cell A response {got} B, expected {want}")); }
                Ok(())
            })
        }
        "B" => {
            let cc = Arc::new(CoreClient::new(target, pinned));
            Arc::new(move |_i| {
                let body = if dir == "a" { Vec::new() } else { prost::Message::encode_to_vec(&*p_val) };
                cc.call(path, &body, |resp| {
                    if resp.len() as u64 != want { return Err(format!("cell B response {} B, expected {want}", resp.len())); }
                    if dir == "a" {
                        let v = <shapes_prost::shapes::ListTasksDetailedResponse as prost::Message>::decode(resp).map_err(|e| e.to_string())?;
                        std::hint::black_box(v);
                    }
                    Ok(())
                })
            })
        }
        "C" => {
            let cc = Arc::new(CoreClient::new(target, pinned));
            Arc::new(move |i| {
                let ctx = &slots[i].0;
                let body: &[u8] = if dir == "a" { &[] } else { m2::core_ffi_arm::encode_into(ctx, f_val) };
                cc.call(path, body, |resp| {
                    if resp.len() as u64 != want { return Err(format!("cell C response {} B, expected {want}", resp.len())); }
                    if dir == "a" {
                        let v = harness::generated::binding::decode_with_list_tasks_detailed_response(ctx.dec, resp)
                            .map_err(|e| format!("core-ffi decode {e}"))?;
                        std::hint::black_box(v);
                    }
                    Ok(())
                })
            })
        }
        "D" => {
            let rt = Arc::new(tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap());
            let ch = tonic_channel(&rt, target, pinned);
            Arc::new(move |i| {
                let ctx = &slots[i].0;
                let body = if dir == "a" { Bytes::new() } else { Bytes::copy_from_slice(m2::core_ffi_arm::encode_into(ctx, f_val)) };
                let mut g = tonic::client::Grpc::new(ch.clone());
                let pq = http::uri::PathAndQuery::from_static(path);
                let resp: Bytes = rt.block_on(async {
                    g.ready().await.map_err(|e| e.to_string())?;
                    g.unary(tonic::Request::new(body), pq, rpc::RawCodec).await.map(|r| r.into_inner()).map_err(|s| s.to_string())
                })?;
                if resp.len() as u64 != want { return Err(format!("cell D response {} B, expected {want}", resp.len())); }
                if dir == "a" {
                    let v = harness::generated::binding::decode_with_list_tasks_detailed_response(ctx.dec, &resp)
                        .map_err(|e| format!("core-ffi decode {e}"))?;
                    std::hint::black_box(v);
                }
                Ok(())
            })
        }
        c => panic!("cell {c}"),
    }
}

/// k threads, `per` calls each; the first error of any thread is returned.
fn batch(f: &CallFn, k: usize, per: usize) -> Result<(), String> {
    std::thread::scope(|s| {
        let hs: Vec<_> = (0..k).map(|i| {
            let f = f.clone();
            s.spawn(move || -> Result<(), String> {
                for _ in 0..per {
                    f(i)?;
                }
                Ok(())
            })
        }).collect();
        for h in hs {
            h.join().map_err(|_| "a client thread panicked".to_string())??;
        }
        Ok(())
    })
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let arg = |k: &str| a.iter().position(|x| x == k).map(|i| a[i + 1].clone());
    let port: u16 = arg("--port").expect("--port").parse().unwrap();
    let transport = arg("--transport").unwrap_or_else(|| "shipped".into());
    let pinned = transport == "pinned";
    let launch: usize = arg("--launch").map(|v| v.parse().unwrap()).unwrap_or(1);
    let rounds: usize = arg("--rounds").map(|v| v.parse().unwrap()).unwrap_or(5);
    let calls: usize = arg("--calls").map(|v| v.parse().unwrap()).unwrap_or(96);
    let warm: usize = arg("--warmup").map(|v| v.parse().unwrap()).unwrap_or(64);
    let out = arg("--out").expect("--out");
    let plant = a.iter().any(|x| x == "--plant");
    let target = format!("http://127.0.0.1:{port}");
    let p22 = prost::Message::encode_to_vec(&m2::prost_arm::value(m2::P2_2)).len() as u64;
    let want_a = if plant { p22 + 1 } else { p22 };
    assert!(harness::generated::binding::ak_init_once() >= 0);

    let cells = ["A", "B", "C", "D"];
    let order: Vec<&str> = (0..4).map(|i| cells[(i + launch - 1) % 4]).collect();
    let mut lines = Vec::new();
    for cell in &order {
        for dir in ["a", "b"] {
            for k in [1usize, 8, 16] {
                let per = calls.div_ceil(k);
                let f = make(cell, dir, k, &target, pinned, want_a);
                // Warm-up, identical for every cell (requirement 24): connection up,
                // runtime threads started, allocator grown.
                if let Err(e) = batch(&f, k, warm.div_ceil(k)) {
                    eprintln!("ABORT (requirement 18): cell {cell} dir {dir} inflight {k} warm-up: {e}");
                    std::process::exit(3);
                }
                for r in 1..=rounds {
                    let (c0, t0) = (process_cpu_ns(), Instant::now());
                    if let Err(e) = batch(&f, k, per) {
                        eprintln!("ABORT (requirement 18): cell {cell} dir {dir} inflight {k} round {r}: {e}");
                        std::process::exit(3);
                    }
                    let (wall, cpu) = (t0.elapsed().as_nanos() as u64, process_cpu_ns() - c0);
                    lines.push(serde_json::json!({
                        "slice": "rust", "suite": "rpc", "cell": cell, "payload": "P2.2", "dir": dir,
                        "transport": transport, "inflight": k, "launch": launch, "round": r,
                        "cpu_ns": cpu, "wall_ns": wall, "iters": per * k,
                    }).to_string());
                }
            }
        }
    }
    let mut f = std::fs::File::create(&out).unwrap();
    for h in campaign::header("rpc", &[
        ("transport", format!("{transport}: {}", if pinned {
            "4 MiB stream + connection windows, adaptive off, Nagle off, both transports and the server"
        } else {
            "tonic endpoint defaults with TCP nodelay (packages/rust's GrpcClient__TcpNagleAlgorithm=false), ak_client_new, server defaults"
        })),
        ("link", "loopback TCP; server = rpc_server, a separate process (requirement 13), pre-serialised P2.2".into()),
        ("cells", "A prost+tonic, B prost+core (blocking), C core+core (blocking), D core+tonic; callback/queue deliveries not run in this suite".into()),
        ("launch", launch.to_string()),
        ("cell order", order.join(",")),
        ("rounds", rounds.to_string()),
        ("calls per round", format!("{calls} rounded up to a multiple of in-flight")),
        ("warm-up", format!("{warm} calls per (cell, dir, in-flight) before round 1")),
        ("clocks", "cpu_ns = getrusage(RUSAGE_SELF) utime+stime of the client per round; wall_ns = monotonic".into()),
        ("runtimes", "tonic cells: tokio multi-thread, 2 workers; core cells: ak_runtime_new(2)".into()),
    ]) {
        writeln!(f, "{h}").unwrap();
    }
    for l in lines {
        writeln!(f, "{l}").unwrap();
    }
}
