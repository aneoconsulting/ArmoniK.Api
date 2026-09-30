//! The stream probe (coordinator unit 2026-09-28, "explore the streaming issues"): direction d
//! only, chosen cells in ONE client process against the shared server (serve.sh), k = 1.
//! CONTAINER INSTRUMENTATION, never part of a campaign run.
//!
//! Per (cell, size), rounds interleaved across the cells (the order rotates each round): per
//! round `calls` calls, each checked (requirement 18: status and the server's byte count),
//! and around the round:
//!   - process CPU and wall,
//!   - per-thread CPU (clock_gettime on each thread's CPU clock) and minor faults
//!     (/proc/self/task/*/stat), summed by thread class: `main` (this thread), `caller` (the
//!     blocking cells' caller thread), `cell-rt` (the cells' own tokio runtimes: A/D/F and
//!     -cb), `core-rt` (the core's runtime, tokio's default name `tokio-rt-worker`), `other`,
//!   - allocation calls from the LD_PRELOAD shim gen/probe/allocprobe.c when it is loaded
//!     (all, >= 1 MiB, their bytes), else absent.
//! One JSON line per (round, cell, size), values per call.
//!
//! Environment: AK_RPC_SOCKET, AK_RPC_TRANSPORT (pinned|shipped), AK_PROBE_CELLS (comma
//! list), AK_PROBE_SIZES (4MiB,16MiB), AK_PROBE_ROUNDS, AK_PROBE_CALLS, AK_PROBE_WARM, AK_OUT;
//! the runtime probe's knobs: AK_HOST_WORKERS / AK_CORE_WORKERS (grid.rs), AK_CHAN_DEPTH
//! (Df-chan's mpsc depth, default 1), AK_CORE_CHAN_DEPTH (read only by a core built with the
//! experiment patch of logs/rust/opt/runtime-probe; printed here as seen). Every round also
//! records getrusage(RUSAGE_SELF) deltas: voluntary / involuntary context switches and minor
//! faults of the whole process (`ru_nvcsw`, `ru_nivcsw`, `ru_minflt`), per call.
//!
//! Added for the attribution unit (2026-09-30): AK_PROBE_SIZES also takes `P5.3` / `P5.4`
//! (direction c, the unary upload); AK_PROBE_K = k in flight (default 1: the callers above,
//! unchanged; k > 1: one batch = k calls in flight, as the grid's criterion iteration, k caller
//! threads or k tasks spawned per batch; `cpu_calls` then holds per-batch CPU / k); AK_PERF_CTL =
//! `CTL_FIFO,ACK_FIFO` of `perf stat|record --control fifo:CTL,ACK -D -1`: the probe sends
//! `enable` before its first timed round and `disable` after its last, so perf counts the
//! timed rounds only (warm-up and connection setup excluded).
use campaign::grid::{self, Call, Conn};
use std::collections::BTreeMap;
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::time::Instant;

fn env<T: std::str::FromStr>(k: &str, d: T) -> T {
    std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d)
}

/// One thread's CPU time in ns, through its per-thread CPU clock (Linux's
/// MAKE_THREAD_CPUCLOCK(tid, CPUCLOCK_SCHED)); None if the thread is gone.
fn thread_cpu_ns(tid: i32) -> Option<u64> {
    let clk: libc::clockid_t = ((!tid) << 3) | 6;
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    if unsafe { libc::clock_gettime(clk, &mut ts) } != 0 {
        return None;
    }
    Some(ts.tv_sec as u64 * 1_000_000_000 + ts.tv_nsec as u64)
}

/// Per thread: class, cpu ns, minor faults, user and system ticks, voluntary and involuntary
/// context switches.
#[derive(Default, Clone)]
struct Th {
    class: String,
    cpu: u64,
    minflt: u64,
    ut: u64,
    st: u64,
    vcs: u64,
    ics: u64,
    syscr: u64,
    syscw: u64,
    rchar: u64,
    wchar: u64,
}

fn threads() -> BTreeMap<i32, Th> {
    let mut m = BTreeMap::new();
    let me = unsafe { libc::syscall(libc::SYS_gettid) } as i32;
    for e in std::fs::read_dir("/proc/self/task").unwrap().flatten() {
        let tid: i32 = match e.file_name().to_string_lossy().parse() {
            Ok(t) => t,
            Err(_) => continue,
        };
        let comm = std::fs::read_to_string(e.path().join("comm")).unwrap_or_default().trim().to_string();
        let stat = std::fs::read_to_string(e.path().join("stat")).unwrap_or_default();
        // fields after the ')' of comm: state is field 3, minflt field 10
        // fields after the ')' of comm, from field 3 (state): minflt 10, utime 14, stime 15
        let f: Vec<u64> = stat.rsplit_once(')').map_or(Vec::new(), |(_, r)| r.split_whitespace().map(|v| v.parse().unwrap_or(0)).collect());
        let at = |n: usize| f.get(n - 3).copied().unwrap_or(0);
        let status = std::fs::read_to_string(e.path().join("status")).unwrap_or_default();
        let cs = |k: &str| status.lines().find_map(|l| l.strip_prefix(k)).and_then(|v| v.trim().parse().ok()).unwrap_or(0u64);
        let io = std::fs::read_to_string(e.path().join("io")).unwrap_or_default();
        let iv = |k: &str| io.lines().find_map(|l| l.strip_prefix(k)).and_then(|v| v.trim().parse().ok()).unwrap_or(0u64);
        let class = if tid == me {
            "main"
        } else if comm == "caller" {
            "caller"
        } else if comm.starts_with("cell-rt") {
            "cell-rt"
        } else if comm.starts_with("tokio-rt-worker") || comm.starts_with("tokio-runtime-w") {
            "core-rt"
        } else if comm.starts_with("ak-reactor") {
            "core-reactor"
        } else {
            "other"
        };
        if let Some(cpu) = thread_cpu_ns(tid) {
            m.insert(tid, Th { class: class.to_string(), cpu, minflt: at(10), ut: at(14), st: at(15),
                               vcs: cs("voluntary_ctxt_switches:"), ics: cs("nonvoluntary_ctxt_switches:"),
                               syscr: iv("syscr:"), syscw: iv("syscw:"), rchar: iv("rchar:"), wchar: iv("wchar:") });
        }
    }
    m
}

type Counts = unsafe extern "C" fn(*mut u64);
fn alloc_counts() -> Option<[u64; 4]> {
    static F: std::sync::OnceLock<Option<Counts>> = std::sync::OnceLock::new();
    let f = F.get_or_init(|| {
        let p = unsafe { libc::dlsym(libc::RTLD_DEFAULT, b"akp_counts\0".as_ptr() as *const libc::c_char) };
        if p.is_null() { None } else { Some(unsafe { std::mem::transmute::<*mut libc::c_void, Counts>(p) }) }
    });
    f.map(|f| {
        let mut o = [0u64; 4];
        unsafe { f(o.as_mut_ptr()) };
        o
    })
}

/// k = 1 callers as the grid bench makes them: a named caller thread for a blocking cell, a
/// task spawned on the cell's runtime (and awaited from this thread) for an async one.
enum Caller {
    Thread(std::sync::mpsc::Sender<usize>, std::sync::mpsc::Receiver<Result<Vec<u64>, String>>),
    Task(Arc<tokio::runtime::Runtime>, Arc<dyn Fn(usize) -> grid::Fut + Send + Sync>),
    /// k > 1: k caller threads, one call each per batch.
    Threads(Vec<std::sync::mpsc::Sender<()>>, std::sync::mpsc::Receiver<Result<(), String>>),
    /// k > 1: k tasks spawned per batch.
    Tasks(Arc<tokio::runtime::Runtime>, Arc<dyn Fn(usize) -> grid::Fut + Send + Sync>, usize),
}

impl Caller {
    fn new(call: &Call, k: usize) -> Caller {
        if k > 1 {
            return match call {
                Call::Blocking(f) => {
                    let (dtx, drx) = std::sync::mpsc::channel();
                    let jobs = (0..k).map(|i| {
                        let (jtx, jrx) = std::sync::mpsc::channel::<()>();
                        let (f, dtx) = (f.clone(), dtx.clone());
                        std::thread::Builder::new().name("caller".into()).spawn(move || {
                            for () in jrx {
                                if dtx.send(f(i)).is_err() {
                                    break;
                                }
                            }
                        }).unwrap();
                        jtx
                    }).collect();
                    Caller::Threads(jobs, drx)
                }
                Call::Async(rt, f) => Caller::Tasks(rt.clone(), f.clone(), k),
            };
        }
        match call {
            Call::Blocking(f) => {
                let (jtx, jrx) = std::sync::mpsc::channel::<usize>();
                let (dtx, drx) = std::sync::mpsc::channel();
                let f = f.clone();
                std::thread::Builder::new().name("caller".into()).spawn(move || {
                    for n in jrx {
                        let mut r = Ok(Vec::with_capacity(n));
                        for _ in 0..n {
                            let c0 = campaign::process_clock_ns();
                            if let Err(e) = f(0) {
                                r = Err(e);
                                break;
                            }
                            if let Ok(v) = r.as_mut() {
                                v.push(campaign::process_clock_ns() - c0);
                            }
                        }
                        if dtx.send(r).is_err() {
                            break;
                        }
                    }
                }).unwrap();
                Caller::Thread(jtx, drx)
            }
            Call::Async(rt, f) => Caller::Task(rt.clone(), f.clone()),
        }
    }
    /// `n` calls; the process CPU of each, read on the thread that runs it.
    fn run(&self, n: usize) -> Result<Vec<u64>, String> {
        match self {
            Caller::Thread(j, d) => {
                j.send(n).map_err(|_| "caller gone".to_string())?;
                d.recv().map_err(|_| "caller gone".to_string())?
            }
            Caller::Task(rt, f) => {
                let f = f.clone();
                rt.block_on(async move {
                    tokio::spawn(async move {
                        let mut v = Vec::with_capacity(n);
                        for _ in 0..n {
                            let c0 = campaign::process_clock_ns();
                            f(0).await?;
                            v.push(campaign::process_clock_ns() - c0);
                        }
                        Ok::<Vec<u64>, String>(v)
                    }).await.unwrap_or_else(|_| Err("task panicked".into()))
                })
            }
            Caller::Threads(jobs, done) => {
                let k = jobs.len() as u64;
                let mut v = Vec::with_capacity(n);
                for _ in 0..n {
                    let c0 = campaign::process_clock_ns();
                    for j in jobs {
                        j.send(()).map_err(|_| "caller gone".to_string())?;
                    }
                    let mut first = Ok(());
                    for _ in 0..jobs.len() {
                        let r = done.recv().map_err(|_| "caller gone".to_string())?;
                        if first.is_ok() {
                            first = r;
                        }
                    }
                    first?;
                    v.push((campaign::process_clock_ns() - c0) / k);
                }
                Ok(v)
            }
            Caller::Tasks(rt, f, k) => {
                let (f, k) = (f.clone(), *k);
                rt.block_on(async move {
                    let mut v = Vec::with_capacity(n);
                    for _ in 0..n {
                        let c0 = campaign::process_clock_ns();
                        let hs: Vec<_> = (0..k).map(|i| tokio::spawn(f(i))).collect();
                        let mut first = Ok(());
                        for h in hs {
                            let r = h.await.unwrap_or_else(|_| Err("task panicked".into()));
                            if first.is_ok() {
                                first = r;
                            }
                        }
                        first?;
                        v.push((campaign::process_clock_ns() - c0) / k as u64);
                    }
                    Ok(v)
                })
            }
        }
    }
}

/// Per-chunk split of the host thread's work in the split-timed cells (`Df-chan`, `Cf-split`,
/// `C-split`): the caller thread's CPU in the encode, its CPU and wall time inside the send
/// (the hand-off: blocking_send / ak_call_send_enc), and the wall time of the final recv.
static ENC_CPU: AtomicU64 = AtomicU64::new(0);
static SEND_CPU: AtomicU64 = AtomicU64::new(0);
static SEND_WALL: AtomicU64 = AtomicU64::new(0);
static RECV_WALL: AtomicU64 = AtomicU64::new(0);
fn split() -> [u64; 4] {
    [ENC_CPU.load(Relaxed), SEND_CPU.load(Relaxed), SEND_WALL.load(Relaxed), RECV_WALL.load(Relaxed)]
}
fn add(a: &AtomicU64, t0: u64, t1: u64) {
    a.fetch_add(t1.saturating_sub(t0), Relaxed);
}
fn wall_ns() -> u64 {
    static T0: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    T0.get_or_init(Instant::now).elapsed().as_nanos() as u64
}

/// `Cf-split` / `C-split` (probe only): cell Cf's / C's call (core-ffi retain encode, then
/// ak_call_send_enc per chunk, ak_call_recv) through the cell's own core client, the same
/// entries grid.rs's core_stream calls, split-timed per chunk.
fn core_split(conn: &Conn, chunks: usize, k: usize) -> Call {
    use campaign::generated::roots::R_UploadResultDataMessage as M5;
    use campaign::Ops;
    use ak_abi::*;
    let cc = match conn {
        Conn::Core(cc) => cc.clone(),
        _ => panic!("a split core cell needs a core connection"),
    };
    let pl = grid::stream_payload(chunks);
    let sl = grid::slots(k);
    let want = (chunks * grid::CHUNK) as u64;
    Call::Blocking(Arc::new(move |i| unsafe {
        let path = grid::STREAM;
        let h = ak_call_open(cc.raw(), path.as_ptr(), path.len(), AK_CALL_CLIENT_STREAM, std::ptr::null());
        if h.is_null() {
            return Err("ak_call_open NULL".into());
        }
        for j in 0..chunks {
            let t0 = campaign::thread_cpu_ns();
            if let Err(e) = M5::f_encode(&sl[i].ctx, &pl.f[j], true) {
                return Err(format!("encode {e}"));
            }
            let (t1, w1) = (campaign::thread_cpu_ns(), wall_ns());
            let rc = ak_call_send_enc(h, sl[i].ctx.enc, (j + 1 == chunks) as i32);
            let (t2, w2) = (campaign::thread_cpu_ns(), wall_ns());
            add(&ENC_CPU, t0, t1);
            add(&SEND_CPU, t1, t2);
            add(&SEND_WALL, w1, w2);
            if rc != AK_OK {
                return Err(format!("ak_call_send_enc {rc}"));
            }
        }
        let mut out = ak_bytes::default();
        let mut gs = -1;
        let w0 = wall_ns();
        let rc = ak_call_recv(h, &mut out, &mut gs);
        add(&RECV_WALL, w0, wall_ns());
        let r = if rc != AK_OK {
            Err(format!("ak_call_recv {rc} {gs}"))
        } else {
            grid::stream_response(std::slice::from_raw_parts(out.ptr, out.len), want, None)
        };
        ak_bytes_free(&mut out);
        ak_call_destroy(h);
        r
    }))
}

/// The callback bridge of the async split cells: the completion into a oneshot.
struct Comp(ak_abi::ak_completion);
unsafe impl Send for Comp {}
extern "C" fn on_done(user: *mut std::ffi::c_void, comp: *mut ak_abi::ak_completion) {
    let tx = unsafe { Box::from_raw(user as *mut tokio::sync::oneshot::Sender<Comp>) };
    let _ = tx.send(Comp(unsafe { *comp }));
}
fn bridge() -> (usize, tokio::sync::oneshot::Receiver<Comp>) {
    let (tx, rx) = tokio::sync::oneshot::channel::<Comp>();
    (Box::into_raw(Box::new(tx)) as usize, rx)
}

/// `Cf-cb-split` / `C-cb-split` (probe only): cell Cf-cb's / C-cb's call (the callback
/// forms, each completion into a oneshot awaited by the cell's task), split-timed per chunk
/// as the blocking split cells: the task thread's CPU in the encode and inside the send
/// entry, and the wall time from the send entry to the completion's arrival (the blocking
/// cells' send wall), then the response.
fn core_split_cb(conn: &Conn, chunks: usize, k: usize) -> Call {
    use campaign::generated::roots::R_UploadResultDataMessage as M5;
    use campaign::Ops;
    use ak_abi::*;
    let (cc, rt) = match conn {
        Conn::CoreCb(cc, rt) => (cc.clone(), rt.clone()),
        _ => panic!("a split callback cell needs a callback connection"),
    };
    let pl = grid::stream_payload(chunks);
    let sl = grid::slots(k);
    let want = (chunks * grid::CHUNK) as u64;
    Call::Async(rt, Arc::new(move |i| {
        let cc = cc.clone();
        Box::pin(async move {
            let path = grid::STREAM;
            let h = unsafe { ak_call_open(cc.raw(), path.as_ptr(), path.len(), AK_CALL_CLIENT_STREAM, std::ptr::null()) } as usize;
            if h == 0 {
                return Err("ak_call_open NULL".to_string());
            }
            for j in 0..chunks {
                let t0 = campaign::thread_cpu_ns();
                M5::f_encode(&sl[i].ctx, &pl.f[j], true).map_err(|e| format!("encode {e}"))?;
                let (t1, w1) = (campaign::thread_cpu_ns(), wall_ns());
                let (ud, rx) = bridge();
                let rc = unsafe { ak_call_send_enc_cb(h as *mut ak_call, sl[i].ctx.enc, (j + 1 == chunks) as i32, on_done, ud as *mut std::ffi::c_void, 0) };
                let t2 = campaign::thread_cpu_ns();
                if rc != AK_OK {
                    return Err(format!("ak_call_send_enc_cb {rc}"));
                }
                let c = rx.await.map_err(|_| "no completion".to_string())?.0;
                add(&ENC_CPU, t0, t1);
                add(&SEND_CPU, t1, t2);
                add(&SEND_WALL, w1, wall_ns());
                if c.status != AK_OK {
                    return Err(format!("send completion {}", c.status));
                }
            }
            let w0 = wall_ns();
            let (ud, rx) = bridge();
            let rc = unsafe { ak_call_recv_cb(h as *mut ak_call, on_done, ud as *mut std::ffi::c_void, 0) };
            if rc != AK_OK {
                return Err(format!("ak_call_recv_cb {rc}"));
            }
            let mut c = rx.await.map_err(|_| "no completion".to_string())?.0;
            add(&RECV_WALL, w0, wall_ns());
            let r = if c.status != AK_OK {
                Err(format!("recv completion {} {}", c.status, c.grpc_status))
            } else {
                grid::stream_response(unsafe { std::slice::from_raw_parts(c.bytes.ptr, c.bytes.len) }, want, None)
            };
            unsafe {
                ak_bytes_free(&mut c.bytes);
                ak_call_destroy(h as *mut ak_call);
            }
            r
        }) as grid::Fut
    }))
}

/// The process's voluntary and involuntary context switches and minor faults (getrusage).
fn rusage() -> [u64; 3] {
    let mut u: libc::rusage = unsafe { std::mem::zeroed() };
    unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut u) };
    [u.ru_nvcsw as u64, u.ru_nivcsw as u64, u.ru_minflt as u64]
}

/// Df-chan's channel depth (AK_CHAN_DEPTH, default 1 = the core's own depth).
fn chan_depth() -> usize {
    env("AK_CHAN_DEPTH", 1usize).max(1)
}

/// The `Df-chan` call (see main).
fn df_chan(conn: &Conn, chunks: usize, k: usize) -> Call {
    use campaign::generated::roots::R_UploadResultDataMessage as M5;
    use campaign::Ops;
    let (rt, ch) = match conn {
        Conn::Tonic(rt, ch) => (rt.clone(), ch.clone()),
        _ => panic!("Df-chan needs a tonic connection"),
    };
    // The body's task runs on the cell's runtime while this host thread blocks in
    // blocking_send: a current-thread runtime runs its tasks only inside a block_on, so the
    // second send would wait forever. Refused, not worked around (the alternative, a thread
    // parked in block_on, is a one-worker runtime by another name).
    assert!(grid::host_workers().is_some(), "Df-chan cannot run on a current-thread host runtime (AK_HOST_WORKERS=ct): nothing drives the body while the host thread blocks in blocking_send");
    let depth = chan_depth();
    let pl = grid::stream_payload(chunks);
    let sl = grid::slots(k);
    let want = (chunks * grid::CHUNK) as u64;
    Call::Blocking(Arc::new(move |i| {
        let (tx, rx) = tokio::sync::mpsc::channel::<bytes::Bytes>(depth);
        let ch = ch.clone();
        let h = rt.spawn(async move {
            rpc::client_streaming_framed(ch, http::uri::PathAndQuery::from_static(grid::STREAM), rpc::ReceiverStream::new(rx), None).await
        });
        for j in 0..chunks {
            let t0 = campaign::thread_cpu_ns();
            M5::f_encode(&sl[i].ctx, &pl.f[j], true).map_err(|e| format!("encode {e}"))?;
            let b = campaign::ffi_owned_body(sl[i].ctx.enc).map_err(|rc| format!("ak_enc_take_owned {rc}"))?;
            let (t1, w1) = (campaign::thread_cpu_ns(), wall_ns());
            let sent = tx.blocking_send(b);
            let (t2, w2) = (campaign::thread_cpu_ns(), wall_ns());
            add(&ENC_CPU, t0, t1);
            add(&SEND_CPU, t1, t2);
            add(&SEND_WALL, w1, w2);
            sent.map_err(|_| "the call ended".to_string())?;
        }
        drop(tx);
        let w0 = wall_ns();
        let resp = rt.block_on(h).map_err(|e| e.to_string())?.map_err(|s| s.to_string())?;
        add(&RECV_WALL, w0, wall_ns());
        grid::stream_response(&resp, want, None)
    }))
}

/// `Ff-1f` (probe only, attribution 2026-09-30): cell Ff's connection and codec (core-native,
/// retain, encoded lazily as the body asks, as Ff) but each message as ONE body frame: the
/// encoder keeps FRAME_HEAD bytes of headroom, `take_framed` writes the prefix in place, and
/// the body is rpc's preframed stream (the core transport's own framing). Its own encoders.
fn ff_one_frame(conn: &Conn, chunks: usize, k: usize) -> Call {
    use campaign::generated::roots::R_UploadResultDataMessage as M5;
    use campaign::Ops;
    let (rt, ch) = match conn {
        Conn::Tonic(rt, ch) => (rt.clone(), ch.clone()),
        _ => panic!("Ff-1f needs a tonic connection"),
    };
    let pl = grid::stream_payload(chunks);
    let encs: &'static [std::sync::Mutex<ak_rt::Enc>] = Box::leak((0..k).map(|_| {
        let mut e = ak_rt::Enc::new(facade::generated::core_native::SITES);
        e.head = ak_rt::enc::FRAME_HEAD;
        std::sync::Mutex::new(e)
    }).collect::<Vec<_>>().into_boxed_slice());
    let want = (chunks * grid::CHUNK) as u64;
    // AK_PROBE_CHECK=1: the server's check path (byte count and SHA-256 of every message as
    // received), for the correctness run of this cell.
    let check = std::env::var("AK_PROBE_CHECK").map_or(false, |v| v == "1");
    let (path, sha) = if check { (grid::STREAM_CHECK, Some(&pl.sha256)) } else { (grid::STREAM, None) };
    Call::Async(rt, Arc::new(move |i| {
        let ch = ch.clone();
        let enc = move |j: usize| {
            let mut e = encs[i].lock().unwrap();
            M5::n_encode(&pl.f[j], &mut e, true);
            e.take_framed()
        };
        let msgs = tokio_stream::StreamExt::map(tokio_stream::iter(0..chunks), enc);
        Box::pin(async move {
            let resp = rpc::client_streaming_preframed_cfg(ch, http::uri::PathAndQuery::from_static(path), msgs, &rpc::CallCfg::default())
                .await.map_err(|e| e.to_string())?;
            grid::stream_response(&resp, want, sha)
        }) as grid::Fut
    }))
}

/// `Df-1f` (probe only, attribution 2026-09-30): cell Df (core-ffi encode, retain, lazily as
/// the body asks, the harness's tonic Channel) with each message as ONE body frame: the owned
/// buffer is taken WITH its 5-byte prefix by `ak_enc_take_owned_framed`, an entry that exists
/// only in a core built with logs/rust/opt/patches/p2-take-framed (found with dlsym; the cell
/// refuses to run without it), and sent through rpc's preframed stream.
fn df_one_frame(conn: &Conn, chunks: usize, k: usize) -> Call {
    use campaign::generated::roots::R_UploadResultDataMessage as M5;
    use campaign::Ops;
    type TakeFramed = unsafe extern "C" fn(*mut ak_abi::ak_enc_ctx, *mut ak_abi::ak_bytes) -> i32;
    let p = unsafe { libc::dlsym(libc::RTLD_DEFAULT, b"ak_enc_take_owned_framed\0".as_ptr() as *const libc::c_char) };
    assert!(!p.is_null(), "Df-1f needs a core with ak_enc_take_owned_framed (patch p2-take-framed)");
    let take: TakeFramed = unsafe { std::mem::transmute::<*mut libc::c_void, TakeFramed>(p) };
    let (rt, ch) = match conn {
        Conn::Tonic(rt, ch) => (rt.clone(), ch.clone()),
        _ => panic!("Df-1f needs a tonic connection"),
    };
    let pl = grid::stream_payload(chunks);
    let sl = grid::slots(k);
    let want = (chunks * grid::CHUNK) as u64;
    let check = std::env::var("AK_PROBE_CHECK").map_or(false, |v| v == "1");
    let (path, sha) = if check { (grid::STREAM_CHECK, Some(&pl.sha256)) } else { (grid::STREAM, None) };
    Call::Async(rt, Arc::new(move |i| {
        let ch = ch.clone();
        let slot: &'static grid::Slot = &sl[i];
        let enc = move |j: usize| {
            if M5::f_encode(&slot.ctx, &pl.f[j], true).is_err() {
                return bytes::Bytes::new();
            }
            let mut b = ak_abi::ak_bytes { ptr: std::ptr::null(), len: 0, owner: std::ptr::null_mut() };
            if unsafe { take(slot.ctx.enc, &mut b) } != ak_abi::AK_OK {
                return bytes::Bytes::new();
            }
            campaign::owned_bytes(b)
        };
        let msgs = tokio_stream::StreamExt::map(tokio_stream::iter(0..chunks), enc);
        Box::pin(async move {
            let resp = rpc::client_streaming_preframed_cfg(ch, http::uri::PathAndQuery::from_static(path), msgs, &rpc::CallCfg::default())
                .await.map_err(|e| e.to_string())?;
            grid::stream_response(&resp, want, sha)
        }) as grid::Fut
    }))
}

/// `Cn-1rt` (Goal 2, harness only): the core linked as Rust crates, one tokio runtime. The
/// core's transport crate (`rpc`) and the core-native codec called directly on the host's
/// runtime (the cell's own, AK_HOST_WORKERS workers), no C ABI and no second runtime; built as
/// the core builds a stream: per call, the transport's future spawned as its own task reading
/// an mpsc(1) (rpc::client_streaming_preframed_cfg over rpc::ReceiverStream), the caller's task
/// encoding each chunk (retain, FRAME_HEAD headroom, take_framed: one body frame, the spare
/// ring) and awaiting the send. Direction c: the same encode, rpc::unary_preframed_cfg.
/// Harness's tonic Channel with the pinned windows (as the core's pinned client).
fn cn_1rt(conn: &Conn, label: &'static str, chunks: usize, k: usize) -> Call {
    use campaign::generated::roots::R_UploadResultDataMessage as M5;
    use campaign::Ops;
    let (rt, ch) = match conn {
        Conn::Tonic(rt, ch) => (rt.clone(), ch.clone()),
        _ => panic!("Cn-1rt needs a tonic connection"),
    };
    let encs: &'static [std::sync::Mutex<ak_rt::Enc>] = Box::leak((0..k).map(|_| {
        let mut e = ak_rt::Enc::new(facade::generated::core_native::SITES);
        e.head = ak_rt::enc::FRAME_HEAD;
        std::sync::Mutex::new(e)
    }).collect::<Vec<_>>().into_boxed_slice());
    let check = std::env::var("AK_PROBE_CHECK").map_or(false, |v| v == "1");
    if chunks == 0 {
        let (f_val, _, _) = grid::m5_values(label);
        return Call::Async(rt, Arc::new(move |i| {
            let ch = ch.clone();
            let body = {
                let mut e = encs[i].lock().unwrap();
                M5::n_encode(f_val, &mut e, true);
                e.take_framed()
            };
            Box::pin(async move {
                let resp = rpc::unary_preframed_cfg(ch, http::uri::PathAndQuery::from_static(grid::UPLOAD), body, &rpc::CallCfg::default())
                    .await.map_err(|e| e.to_string())?;
                if resp.is_empty() { Ok(()) } else { Err(format!("Cn-1rt upload response {} B, expected 0", resp.len())) }
            }) as grid::Fut
        }));
    }
    let pl = grid::stream_payload(chunks);
    let want = (chunks * grid::CHUNK) as u64;
    let (path, sha) = if check { (grid::STREAM_CHECK, Some(&pl.sha256)) } else { (grid::STREAM, None) };
    Call::Async(rt, Arc::new(move |i| {
        let ch = ch.clone();
        Box::pin(async move {
            let (tx, rx) = tokio::sync::mpsc::channel::<bytes::Bytes>(1);
            let h = tokio::spawn(async move {
                rpc::client_streaming_preframed_cfg(ch, http::uri::PathAndQuery::from_static(path), rpc::ReceiverStream::new(rx), &rpc::CallCfg::default()).await
            });
            for j in 0..chunks {
                let b = {
                    let mut e = encs[i].lock().unwrap();
                    M5::n_encode(&pl.f[j], &mut e, true);
                    e.take_framed()
                };
                if tx.send(b).await.is_err() {
                    break;
                }
            }
            drop(tx);
            let resp = h.await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
            grid::stream_response(&resp, want, sha)
        }) as grid::Fut
    }))
}

/// `Cf-enc` / `Cf-encp` (Goal 1 item 2, patch p5-deferred): cell Cf's call with each chunk
/// ENCODED BY THE TRANSPORT on the core worker that writes it (`ak_call_send_deferred`, found
/// with dlsym): the caller thread hands over the value and an encode callback. Cf-enc waits
/// for each encode (wait = 1); Cf-encp returns once queued (wait = 0), so the worker encodes
/// chunk n+1 while the caller has already moved on.
fn core_deferred(conn: &Conn, chunks: usize, k: usize, wait: bool) -> Call {
    use ak_abi::*;
    type SendDeferred = unsafe extern "C" fn(*mut ak_call, *mut ak_enc_ctx, unsafe extern "C" fn(*mut libc::c_void, *mut ak_enc_ctx) -> i32, *mut libc::c_void, i32, i32) -> i32;
    let p = unsafe { libc::dlsym(libc::RTLD_DEFAULT, b"ak_call_send_deferred\0".as_ptr() as *const libc::c_char) };
    assert!(!p.is_null(), "Cf-enc needs a core with ak_call_send_deferred (patch p5-deferred)");
    let send: SendDeferred = unsafe { std::mem::transmute::<*mut libc::c_void, SendDeferred>(p) };
    let cc = match conn {
        Conn::Core(cc) => cc.clone(),
        _ => panic!("Cf-enc needs a core connection"),
    };
    struct Job {
        slot: &'static grid::Slot,
        val: &'static facade::UploadResultDataMessage,
    }
    unsafe extern "C" fn enc_cb(user: *mut libc::c_void, _enc: *mut ak_abi::ak_enc_ctx) -> i32 {
        use campaign::generated::roots::R_UploadResultDataMessage as M5;
        use campaign::Ops;
        let j = &*(user as *const Job);
        match M5::f_encode(&j.slot.ctx, j.val, true) {
            Ok(n) => n as i32,
            Err(e) => e,
        }
    }
    let pl = grid::stream_payload(chunks);
    let sl = grid::slots(k);
    let want = (chunks * grid::CHUNK) as u64;
    let check = std::env::var("AK_PROBE_CHECK").map_or(false, |v| v == "1");
    let (path, sha) = if check { (grid::STREAM_CHECK, Some(&pl.sha256)) } else { (grid::STREAM, None) };
    Call::Blocking(Arc::new(move |i| unsafe {
        let jobs: Vec<Job> = (0..chunks).map(|j| Job { slot: &sl[i], val: &pl.f[j] }).collect();
        let h = ak_call_open(cc.raw(), path.as_ptr(), path.len(), AK_CALL_CLIENT_STREAM, std::ptr::null());
        if h.is_null() {
            return Err("ak_call_open NULL".into());
        }
        for j in 0..chunks {
            let rc = send(h, sl[i].ctx.enc, enc_cb, &jobs[j] as *const Job as *mut libc::c_void, (j + 1 == chunks) as i32, wait as i32);
            if rc != AK_OK {
                ak_call_destroy(h);
                return Err(format!("ak_call_send_deferred rc {rc}"));
            }
        }
        let mut out = ak_bytes::default();
        let mut gs = -1;
        let rc = ak_call_recv(h, &mut out, &mut gs);
        let r = if rc != AK_OK {
            Err(format!("ak_call_recv {rc} {gs}"))
        } else {
            grid::stream_response(std::slice::from_raw_parts(out.ptr, out.len), want, sha)
        };
        ak_bytes_free(&mut out);
        ak_call_destroy(h);
        drop(jobs);
        r
    }))
}

fn main() {
    assert!(harness::generated::binding::ak_init_once() >= 0);
    let socket: String = std::env::var("AK_RPC_SOCKET").expect("AK_RPC_SOCKET");
    let transport: String = env("AK_RPC_TRANSPORT", "pinned".to_string());
    let pinned = transport == "pinned";
    let target = format!("unix:{socket}");
    // `Df-chan` (probe only): cell Df's connection (the harness's tonic Channel on the cell's
    // runtime) and framed body, fed as the core feeds its own: a host thread encodes each
    // chunk (core-ffi, retain, moved out by ak_enc_take_owned) and blocking_sends it into an
    // mpsc(1) whose ReceiverStream is the body's message stream; the call runs as a task on
    // the cell's runtime, its result awaited with block_on from the host thread.
    // `A2` (probe only): a second, independent instance of cell A (its own runtime and
    // channel), so an A/A gap in the same process is measured beside every cell's gap to A.
    let cells: Vec<&'static str> = env("AK_PROBE_CELLS", "A,D,Df,C,Cf,C-cb,Cf-cb,B,Bf".to_string())
        .split(',').map(|s| match s { "Df-chan" => "Df-chan", "Cf-split" => "Cf-split", "C-split" => "C-split",
                                       "Cf-cb-split" => "Cf-cb-split", "C-cb-split" => "C-cb-split", "A2" => "A2", "Ff-1f" => "Ff-1f", "Df-1f" => "Df-1f", "Cf-cb-1rt" => "Cf-cb-1rt", "Cn-1rt" => "Cn-1rt", "Cf-enc" => "Cf-enc", "Cf-encp" => "Cf-encp", s => grid::cell_of(s) }).collect();
    // The grid cell each probe-only name runs on.
    let base = |c: &'static str| -> &'static str {
        match c { "Df-chan" => grid::cell_of("Df"), "Cf-split" => grid::cell_of("Cf"), "C-split" => grid::cell_of("C"),
                  "Cf-cb-split" => grid::cell_of("Cf-cb"), "C-cb-split" => grid::cell_of("C-cb"), "A2" => "A", "Ff-1f" => grid::cell_of("Ff"), "Df-1f" => grid::cell_of("Df"), "Cf-cb-1rt" => grid::cell_of("Cf-cb"), "Cn-1rt" => grid::cell_of("Ff"), "Cf-enc" | "Cf-encp" => grid::cell_of("Cf"), c => c }
    };
    // (label, chunks): direction d's sizes, or direction c's payload with chunks = 0.
    let sizes: Vec<(&'static str, usize)> = env("AK_PROBE_SIZES", "16MiB,4MiB".to_string()).split(',')
        .map(|s| grid::D_PAYLOADS.iter().copied().find(|(l, _)| *l == s)
            .or_else(|| grid::C_PAYLOADS.iter().copied().find(|p| *p == s).map(|p| (p, 0)))
            .unwrap_or_else(|| panic!("size {s}"))).collect();
    let k: usize = env("AK_PROBE_K", 1usize).max(1);
    let perf_ctl: Option<(String, String)> = std::env::var("AK_PERF_CTL").ok().map(|v| {
        let (a, b) = v.split_once(',').expect("AK_PERF_CTL=CTL_FIFO,ACK_FIFO");
        (a.to_string(), b.to_string())
    });
    let rounds: usize = env("AK_PROBE_ROUNDS", 15);
    let calls: usize = env("AK_PROBE_CALLS", 8);
    let warm: usize = env("AK_PROBE_WARM", 4);
    let out: String = env("AK_OUT", "stream-probe.jsonl".to_string());

    // `Cf-cb-1rt` (Goal 2, patch p3-exec-slot): cell Cf-cb with the core's runtime HOSTED on
    // the cell's own runtime: the core's tasks (its calls', hyper's connection task, tonic's
    // buffer worker) run on the host's workers, the core keeping one `ak-reactor` thread for
    // its I/O and timer drivers; delivery as Cf-cb (callback into a oneshot).
    let conns: Vec<Conn> = cells.iter().map(|c| match *c {
        "Cf-cb-1rt" => {
            let rt = grid::host_runtime();
            let cc = grid::CoreClient::new_hosted(&target, pinned, rt.clone());
            assert_eq!(unsafe { ak_abi::ak_client_set_framed(cc.raw(), 1) }, ak_abi::AK_OK);
            Conn::CoreCb(Arc::new(cc), rt)
        }
        c => Conn::open(base(c), &target, pinned),
    }).collect();
    let mut work = Vec::new();
    for (ci, &cell) in cells.iter().enumerate() {
        for &(label, chunks) in &sizes {
            let call = if cell == "Cn-1rt" {
                cn_1rt(&conns[ci], label, chunks, k)
            } else if chunks == 0 {
                assert!(!cell.ends_with("-split") && !["Df-chan", "Ff-1f", "Df-1f", "Cf-enc", "Cf-encp"].contains(&cell), "{cell}: direction d only");
                grid::call_of_c(base(cell), &conns[ci], label, grid::slots(k), 0)
            } else if cell == "Df-chan" {
                df_chan(&conns[ci], chunks, k)
            } else if cell == "Ff-1f" {
                ff_one_frame(&conns[ci], chunks, k)
            } else if cell == "Df-1f" {
                df_one_frame(&conns[ci], chunks, k)
            } else if cell == "Cf-enc" || cell == "Cf-encp" {
                core_deferred(&conns[ci], chunks, k, cell == "Cf-enc")
            } else if cell.ends_with("-cb-split") {
                core_split_cb(&conns[ci], chunks, k)
            } else if cell.ends_with("-split") {
                core_split(&conns[ci], chunks, k)
            } else {
                grid::call_of_d(base(cell), &conns[ci], chunks, grid::slots(k), (chunks * grid::CHUNK) as u64, std::env::var("AK_PROBE_CHECK").map_or(false, |v| v == "1"))
            };
            let caller = Caller::new(&call, k);
            caller.run(warm).unwrap_or_else(|e| panic!("warm-up {cell} {label}: {e}"));
            work.push((cell, label, caller));
        }
    }
    // AK_PIN_CALLER / AK_PIN_CORE_RT / AK_PIN_CELL_RT (attribution only): after the warm-up,
    // pin every thread of that class (caller threads, the core runtime's workers, the cells'
    // own runtime workers) to the given CPU list with sched_setaffinity; the process stays on
    // AK_CPU_CLIENT otherwise.
    let mut pins = Vec::new();
    for (var, class) in [("AK_PIN_CALLER", "caller"), ("AK_PIN_CORE_RT", "core-rt"), ("AK_PIN_CELL_RT", "cell-rt")] {
        if let Ok(list) = std::env::var(var) {
            let mut set: libc::cpu_set_t = unsafe { std::mem::zeroed() };
            for part in list.split(',') {
                let (a, b) = part.split_once('-').unwrap_or((part, part));
                for c in a.parse::<usize>().unwrap()..=b.parse::<usize>().unwrap() {
                    unsafe { libc::CPU_SET(c, &mut set) };
                }
            }
            let mut n = 0;
            for (tid, t) in threads() {
                if t.class == class {
                    assert_eq!(unsafe { libc::sched_setaffinity(tid, std::mem::size_of::<libc::cpu_set_t>(), &set) }, 0);
                    n += 1;
                }
            }
            pins.push(format!("{class} ({n} threads) -> {list}"));
        }
    }
    let mut f = std::fs::File::create(&out).unwrap();
    if !pins.is_empty() {
        writeln!(f, "# pinned after the warm-up: {}", pins.join("; ")).unwrap();
    }
    writeln!(f, "# stream probe: transport {transport}, cells {cells:?}, sizes {:?}, k {k} (a round = `calls` batches of k), rounds {rounds}, calls per round {calls}, warm {warm}, allocation shim {}, perf control {}",
             sizes.iter().map(|s| s.0).collect::<Vec<_>>(), if alloc_counts().is_some() { "loaded" } else { "absent" },
             if perf_ctl.is_some() { "on (perf counts the timed rounds only)" } else { "off" }).unwrap();
    writeln!(f, "# runtimes: host (cell-rt) {} (AK_HOST_WORKERS), core ak_runtime_new({}) (AK_CORE_WORKERS); Df-chan mpsc depth {} (AK_CHAN_DEPTH); AK_CORE_CHAN_DEPTH={} (honoured only by the patched core, which prints its depth on stderr)",
             grid::host_rt_label(), grid::core_workers(), chan_depth(), std::env::var("AK_CORE_CHAN_DEPTH").unwrap_or_else(|_| "unset".into())).unwrap();
    // The threads that exist once every cell is open and warm, by class (the runtime
    // settings are in effect: e.g. AK_HOST_WORKERS=1 gives one cell-rt thread per host runtime).
    let mut tc: BTreeMap<String, usize> = BTreeMap::new();
    for t in threads().values() {
        *tc.entry(t.class.clone()).or_default() += 1;
    }
    writeln!(f, "# threads by class after the warm-up: {tc:?} (host runtimes {}, core runtimes {})",
             conns.iter().filter(|c| !matches!(c, Conn::Core(_))).count(), conns.iter().filter(|c| !matches!(c, Conn::Tonic(..))).count()).unwrap();
    let n = work.len();
    // AK_PROBE_ORDER=rotate (default): every round visits every (cell, size), the order
    // rotating; =block: all rounds of one (cell, size) back to back, as a criterion benchmark
    // runs one benchmark at a time. AK_PROBE_PROC=0: no /proc reads around the rounds.
    let block = env("AK_PROBE_ORDER", "rotate".to_string()) == "block";
    let procs = env("AK_PROBE_PROC", 1u32) == 1;
    let order: Vec<(usize, usize)> = if block {
        (0..n).flat_map(|i| (0..rounds).map(move |r| (r, i))).collect()
    } else {
        (0..rounds).flat_map(|r| (0..n).map(move |i| (r, (i + r) % n))).collect()
    };
    writeln!(f, "# order {}, /proc reads {}", if block { "block" } else { "rotate" }, procs).unwrap();
    let perf = |cmd: &str| {
        if let Some((ctl, ack)) = &perf_ctl {
            use std::io::{BufRead, Write as _};
            let mut c = std::fs::OpenOptions::new().write(true).open(ctl).expect("perf control fifo");
            c.write_all(format!("{cmd}\n").as_bytes()).unwrap();
            drop(c);
            let mut l = String::new();
            std::io::BufReader::new(std::fs::File::open(ack).expect("perf ack fifo")).read_line(&mut l).unwrap();
        }
    };
    // AK_PERF_CELL: perf counts only that cell's timed rounds (every other cell runs as usual).
    let perf_cell: Option<&'static str> = std::env::var("AK_PERF_CELL").ok().map(|c| match c.as_str() {
        "Df-chan" | "Cf-split" | "C-split" | "Cf-cb-split" | "C-cb-split" | "A2" | "Ff-1f" | "Df-1f" | "Cf-cb-1rt" | "Cn-1rt" | "Cf-enc" | "Cf-encp" => Box::leak(c.into_boxed_str()) as &'static str,
        s => grid::cell_of(s),
    });
    let mut perf_on = false;
    for (r, i) in order {
        {
            let (cell, label, caller) = &work[i];
            let want = perf_cell.as_deref().map_or(true, |c| c == *cell);
            if want != perf_on {
                perf(if want { "enable" } else { "disable" });
                perf_on = want;
            }
            let t0 = if procs { threads() } else { BTreeMap::new() };
            let a0 = alloc_counts();
            let s0 = split();
            let u0 = rusage();
            let (c0, w0) = (campaign::process_clock_ns(), Instant::now());
            let per_call = caller.run(calls).unwrap_or_else(|e| panic!("ABORT (requirement 18): {cell} {label}: {e}"));
            let (cpu, wall) = (campaign::process_clock_ns() - c0, w0.elapsed().as_nanos() as u64);
            let u1 = rusage();
            let a1 = alloc_counts();
            let s1 = split();
            let t1 = if procs { threads() } else { BTreeMap::new() };
            let mut by: BTreeMap<String, [u64; 10]> = BTreeMap::new();
            for (tid, b) in &t1 {
                let a = t0.get(tid).cloned().unwrap_or_default();
                let e = by.entry(b.class.clone()).or_default();
                for (i, (x, y)) in [(b.cpu, a.cpu), (b.minflt, a.minflt), (b.ut, a.ut), (b.st, a.st), (b.vcs, a.vcs), (b.ics, a.ics),
                                    (b.syscr, a.syscr), (b.syscw, a.syscw), (b.rchar, a.rchar), (b.wchar, a.wchar)].into_iter().enumerate() {
                    e[i] += x.saturating_sub(y);
                }
            }
            let per = (calls * k) as f64;
            let mut o = serde_json::json!({
                "round": r, "cell": cell, "size": label, "calls": calls * k, "k": k, "transport": transport,
                "cpu_ns": cpu as f64 / per, "wall_ns": wall as f64 / per, "cpu_calls": per_call,
                "ru_nvcsw": (u1[0] - u0[0]) as f64 / per, "ru_nivcsw": (u1[1] - u0[1]) as f64 / per, "ru_minflt": (u1[2] - u0[2]) as f64 / per,
            });
            if s1 != s0 {
                for (i, k) in ["host_encode_cpu", "host_send_cpu", "host_send_wall", "host_recv_wall"].iter().enumerate() {
                    o[*k] = ((s1[i] - s0[i]) as f64 / per).into();
                }
            }
            for (class, v) in &by {
                for (i, k) in ["cpu", "minflt", "uticks", "sticks", "vcs", "ics", "syscr", "syscw", "rchar", "wchar"].iter().enumerate() {
                    o[format!("{k}_{class}")] = (v[i] as f64 / per).into();
                }
            }
            if let (Some(a0), Some(a1)) = (a0, a1) {
                o["allocs"] = ((a1[0] - a0[0]) as f64 / per).into();
                o["allocs_big"] = ((a1[1] - a0[1]) as f64 / per).into();
                o["alloc_big_bytes"] = ((a1[2] - a0[2]) as f64 / per).into();
                o["frees"] = ((a1[3] - a0[3]) as f64 / per).into();
            }
            writeln!(f, "{o}").unwrap();
        }
    }
    if perf_on {
        perf("disable");
    }
}
