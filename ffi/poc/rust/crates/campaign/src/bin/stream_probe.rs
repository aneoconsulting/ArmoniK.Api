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
                               vcs: cs("voluntary_ctxt_switches:"), ics: cs("nonvoluntary_ctxt_switches:") });
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
    Thread(std::sync::mpsc::Sender<usize>, std::sync::mpsc::Receiver<Result<(), String>>),
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
                        let mut r = Ok(());
                        for _ in 0..n {
                            if let Err(e) = f(0) {
                                r = Err(e);
                                break;
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
    fn run(&self, n: usize) -> Result<(), String> {
        match self {
            Caller::Thread(j, d) => {
                j.send(n).map_err(|_| "caller gone".to_string())?;
                d.recv().map_err(|_| "caller gone".to_string())?
            }
            Caller::Task(rt, f) => {
                let f = f.clone();
                rt.block_on(async move {
                    tokio::spawn(async move {
                        for _ in 0..n {
                            f(0).await?;
                        }
                        Ok::<(), String>(())
                    }).await.unwrap_or_else(|_| Err("task panicked".into()))
                })
            }
        }
    }
}

fn main() {
    assert!(harness::generated::binding::ak_init_once() >= 0);
    let socket: String = std::env::var("AK_RPC_SOCKET").expect("AK_RPC_SOCKET");
    let transport: String = env("AK_RPC_TRANSPORT", "pinned".to_string());
    let pinned = transport == "pinned";
    let target = format!("unix:{socket}");
    let cells: Vec<&'static str> = env("AK_PROBE_CELLS", "A,D,Df,C,Cf,C-cb,Cf-cb,B,Bf".to_string())
        .split(',').map(|s| grid::cell_of(s)).collect();
    let sizes: Vec<(&'static str, usize)> = env("AK_PROBE_SIZES", "16MiB,4MiB".to_string()).split(',')
        .map(|s| *grid::D_PAYLOADS.iter().find(|(l, _)| *l == s).unwrap_or_else(|| panic!("size {s}"))).collect();
    let rounds: usize = env("AK_PROBE_ROUNDS", 15);
    let calls: usize = env("AK_PROBE_CALLS", 8);
    let warm: usize = env("AK_PROBE_WARM", 4);
    let out: String = env("AK_OUT", "stream-probe.jsonl".to_string());

    let conns: Vec<Conn> = cells.iter().map(|c| Conn::open(c, &target, pinned)).collect();
    let mut work = Vec::new();
    for (ci, &cell) in cells.iter().enumerate() {
        for &(label, chunks) in &sizes {
            let call = grid::call_of_d(cell, &conns[ci], chunks, grid::slots(1), (chunks * grid::CHUNK) as u64, false);
            let caller = Caller::new(&call);
            caller.run(warm).unwrap_or_else(|e| panic!("warm-up {cell} {label}: {e}"));
            work.push((cell, label, caller));
        }
    }
    let mut f = std::fs::File::create(&out).unwrap();
    writeln!(f, "# stream probe: transport {transport}, cells {cells:?}, sizes {:?}, rounds {rounds}, calls per round {calls}, warm {warm}, allocation shim {}",
             sizes.iter().map(|s| s.0).collect::<Vec<_>>(), if alloc_counts().is_some() { "loaded" } else { "absent" }).unwrap();
    let n = work.len();
    for r in 0..rounds {
        for i in 0..n {
            let (cell, label, caller) = &work[(i + r) % n];
            let t0 = threads();
            let a0 = alloc_counts();
            let (c0, w0) = (campaign::process_clock_ns(), Instant::now());
            caller.run(calls).unwrap_or_else(|e| panic!("ABORT (requirement 18): {cell} {label}: {e}"));
            let (cpu, wall) = (campaign::process_clock_ns() - c0, w0.elapsed().as_nanos() as u64);
            let a1 = alloc_counts();
            let t1 = threads();
            let mut by: BTreeMap<String, [u64; 6]> = BTreeMap::new();
            for (tid, b) in &t1 {
                let a = t0.get(tid).cloned().unwrap_or_default();
                let e = by.entry(b.class.clone()).or_default();
                for (i, (x, y)) in [(b.cpu, a.cpu), (b.minflt, a.minflt), (b.ut, a.ut), (b.st, a.st), (b.vcs, a.vcs), (b.ics, a.ics)].into_iter().enumerate() {
                    e[i] += x.saturating_sub(y);
                }
            }
            let per = calls as f64;
            let mut o = serde_json::json!({
                "round": r, "cell": cell, "size": label, "calls": calls, "transport": transport,
                "cpu_ns": cpu as f64 / per, "wall_ns": wall as f64 / per,
            });
            for (class, v) in &by {
                for (i, k) in ["cpu", "minflt", "uticks", "sticks", "vcs", "ics"].iter().enumerate() {
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
        eprintln!("round {r} done");
    }
}
