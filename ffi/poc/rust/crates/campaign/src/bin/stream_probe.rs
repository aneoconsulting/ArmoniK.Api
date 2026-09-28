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
//! list), AK_PROBE_SIZES (4MiB,16MiB), AK_PROBE_ROUNDS, AK_PROBE_CALLS, AK_PROBE_WARM, AK_OUT.
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
}

impl Caller {
    fn new(call: &Call) -> Caller {
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
fn core_split(conn: &Conn, chunks: usize) -> Call {
    use campaign::generated::roots::R_UploadResultDataMessage as M5;
    use campaign::Ops;
    use ak_abi::*;
    let cc = match conn {
        Conn::Core(cc) => cc.clone(),
        _ => panic!("a split core cell needs a core connection"),
    };
    let pl = grid::stream_payload(chunks);
    let sl = grid::slots(1);
    let want = (chunks * grid::CHUNK) as u64;
    Call::Blocking(Arc::new(move |_i| unsafe {
        let path = grid::STREAM;
        let h = ak_call_open(cc.raw(), path.as_ptr(), path.len(), AK_CALL_CLIENT_STREAM, std::ptr::null());
        if h.is_null() {
            return Err("ak_call_open NULL".into());
        }
        for j in 0..chunks {
            let t0 = campaign::thread_cpu_ns();
            if let Err(e) = M5::f_encode(&sl[0].ctx, &pl.f[j], true) {
                return Err(format!("encode {e}"));
            }
            let (t1, w1) = (campaign::thread_cpu_ns(), wall_ns());
            let rc = ak_call_send_enc(h, sl[0].ctx.enc, (j + 1 == chunks) as i32);
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
fn core_split_cb(conn: &Conn, chunks: usize) -> Call {
    use campaign::generated::roots::R_UploadResultDataMessage as M5;
    use campaign::Ops;
    use ak_abi::*;
    let (cc, rt) = match conn {
        Conn::CoreCb(cc, rt) => (cc.clone(), rt.clone()),
        _ => panic!("a split callback cell needs a callback connection"),
    };
    let pl = grid::stream_payload(chunks);
    let sl = grid::slots(1);
    let want = (chunks * grid::CHUNK) as u64;
    Call::Async(rt, Arc::new(move |_i| {
        let cc = cc.clone();
        Box::pin(async move {
            let path = grid::STREAM;
            let h = unsafe { ak_call_open(cc.raw(), path.as_ptr(), path.len(), AK_CALL_CLIENT_STREAM, std::ptr::null()) } as usize;
            if h == 0 {
                return Err("ak_call_open NULL".to_string());
            }
            for j in 0..chunks {
                let t0 = campaign::thread_cpu_ns();
                M5::f_encode(&sl[0].ctx, &pl.f[j], true).map_err(|e| format!("encode {e}"))?;
                let (t1, w1) = (campaign::thread_cpu_ns(), wall_ns());
                let (ud, rx) = bridge();
                let rc = unsafe { ak_call_send_enc_cb(h as *mut ak_call, sl[0].ctx.enc, (j + 1 == chunks) as i32, on_done, ud as *mut std::ffi::c_void, 0) };
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

/// The `Df-chan` call (see main).
fn df_chan(conn: &Conn, chunks: usize) -> Call {
    use campaign::generated::roots::R_UploadResultDataMessage as M5;
    use campaign::Ops;
    let (rt, ch) = match conn {
        Conn::Tonic(rt, ch) => (rt.clone(), ch.clone()),
        _ => panic!("Df-chan needs a tonic connection"),
    };
    let pl = grid::stream_payload(chunks);
    let sl = grid::slots(1);
    let want = (chunks * grid::CHUNK) as u64;
    Call::Blocking(Arc::new(move |_i| {
        let (tx, rx) = tokio::sync::mpsc::channel::<bytes::Bytes>(1);
        let ch = ch.clone();
        let h = rt.spawn(async move {
            rpc::client_streaming_framed(ch, http::uri::PathAndQuery::from_static(grid::STREAM), rpc::ReceiverStream::new(rx), None).await
        });
        for j in 0..chunks {
            let t0 = campaign::thread_cpu_ns();
            M5::f_encode(&sl[0].ctx, &pl.f[j], true).map_err(|e| format!("encode {e}"))?;
            let b = campaign::ffi_owned_body(sl[0].ctx.enc).map_err(|rc| format!("ak_enc_take_owned {rc}"))?;
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
    let cells: Vec<&'static str> = env("AK_PROBE_CELLS", "A,D,Df,C,Cf,C-cb,Cf-cb,B,Bf".to_string())
        .split(',').map(|s| match s { "Df-chan" => "Df-chan", "Cf-split" => "Cf-split", "C-split" => "C-split",
                                       "Cf-cb-split" => "Cf-cb-split", "C-cb-split" => "C-cb-split", s => grid::cell_of(s) }).collect();
    let sizes: Vec<(&'static str, usize)> = env("AK_PROBE_SIZES", "16MiB,4MiB".to_string()).split(',')
        .map(|s| *grid::D_PAYLOADS.iter().find(|(l, _)| *l == s).unwrap_or_else(|| panic!("size {s}"))).collect();
    let rounds: usize = env("AK_PROBE_ROUNDS", 15);
    let calls: usize = env("AK_PROBE_CALLS", 8);
    let warm: usize = env("AK_PROBE_WARM", 4);
    let out: String = env("AK_OUT", "stream-probe.jsonl".to_string());

    let conns: Vec<Conn> = cells.iter().map(|c| Conn::open(match *c { "Df-chan" => grid::cell_of("Df"), "Cf-split" => grid::cell_of("Cf"), "C-split" => grid::cell_of("C"),
                                                                    "Cf-cb-split" => grid::cell_of("Cf-cb"), "C-cb-split" => grid::cell_of("C-cb"), c => c }, &target, pinned)).collect();
    let mut work = Vec::new();
    for (ci, &cell) in cells.iter().enumerate() {
        for &(label, chunks) in &sizes {
            let call = if cell == "Df-chan" {
                df_chan(&conns[ci], chunks)
            } else if cell.ends_with("-cb-split") {
                core_split_cb(&conns[ci], chunks)
            } else if cell.ends_with("-split") {
                core_split(&conns[ci], chunks)
            } else {
                grid::call_of_d(cell, &conns[ci], chunks, grid::slots(1), (chunks * grid::CHUNK) as u64, false)
            };
            let caller = Caller::new(&call);
            caller.run(warm).unwrap_or_else(|e| panic!("warm-up {cell} {label}: {e}"));
            work.push((cell, label, caller));
        }
    }
    let mut f = std::fs::File::create(&out).unwrap();
    writeln!(f, "# stream probe: transport {transport}, cells {cells:?}, sizes {:?}, rounds {rounds}, calls per round {calls}, warm {warm}, allocation shim {}",
             sizes.iter().map(|s| s.0).collect::<Vec<_>>(), if alloc_counts().is_some() { "loaded" } else { "absent" }).unwrap();
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
    for (r, i) in order {
        {
            let (cell, label, caller) = &work[i];
            let t0 = if procs { threads() } else { BTreeMap::new() };
            let a0 = alloc_counts();
            let s0 = split();
            let (c0, w0) = (campaign::process_clock_ns(), Instant::now());
            let per_call = caller.run(calls).unwrap_or_else(|e| panic!("ABORT (requirement 18): {cell} {label}: {e}"));
            let (cpu, wall) = (campaign::process_clock_ns() - c0, w0.elapsed().as_nanos() as u64);
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
            let per = calls as f64;
            let mut o = serde_json::json!({
                "round": r, "cell": cell, "size": label, "calls": calls, "transport": transport,
                "cpu_ns": cpu as f64 / per, "wall_ns": wall as f64 / per, "cpu_calls": per_call,
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
}
