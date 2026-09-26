//! CAMPAIGN.md 4.2: the RPC grid's client (requirements 12-18 as amended 2026-09-26). One
//! process per launch and build, pinned by the runner to `AK_CPU_CLIENT`; the server is
//! another process (`rpc_server`), ONE per launch, serving both builds' clients.
//!
//!   rpc_client --socket PATH --transport shipped|pinned --launch N --rounds R --calls C
//!              --warmup W --server-warm S --out FILE [--plant]
//!
//! Cells (`campaign::grid`): A prost+tonic, B prost+core, C core-ffi+core, D core-ffi+tonic,
//! E core-native+core, F core-native+tonic; C to F per unknown-field mode of this build.
//! Directions `a`, `a+read`, `b`; in flight 1, 8, 16. B, C and E: k host threads from a pool
//! created before the warm-up and reused (blocking `ak_call_unary`, requirement 16); A, D
//! and F: k tokio tasks (tonic's idiomatic async client, requirement 16 as amended).
//! Before round 1 of the first cell the server is warmed by S checked calls from each client
//! transport; each cell opens ONE channel for the launch and warms it by W calls per
//! (dir, in-flight) before that combination's rounds.
//! Every call is checked (requirement 18): status OK and the response length equal to the
//! expected one; C, D, E and F also require their decode to succeed. The first failure
//! aborts the process with no output (samples are written only at the end). `--plant`
//! expects a wrong length, so the run must abort (the runner's control).
//! CPU: getrusage(RUSAGE_SELF) of this process per round, wall beside it (requirement 21).

use campaign::grid::{self, Call, Conn, CELLS, DIRS};
use campaign::process_cpu_ns;
use std::io::Write;
use std::sync::Arc;
use std::time::Instant;

/// The k callers of one (cell, dir, in-flight k). Blocking cells: k OS threads created once,
/// before the warm-up and outside every timed window, and reused (R-H2); a batch is one job
/// per thread. Async cells: k tasks spawned on the cell's runtime per batch (spawning a task
/// is the idiomatic client's own per-request cost). The first error of any caller is
/// returned; a panic comes back as an error, never a hang.
enum Callers {
    Threads {
        jobs: Vec<std::sync::mpsc::Sender<usize>>,
        done: std::sync::mpsc::Receiver<Result<(), String>>,
        threads: Vec<std::thread::JoinHandle<()>>,
    },
    Tasks {
        rt: Arc<tokio::runtime::Runtime>,
        f: Arc<dyn Fn(usize) -> grid::Fut + Send + Sync>,
        k: usize,
    },
}

impl Callers {
    fn new(call: &Call, k: usize) -> Callers {
        match call {
            Call::Blocking(f) => {
                let (dtx, done) = std::sync::mpsc::channel();
                let mut jobs = Vec::with_capacity(k);
                let mut threads = Vec::with_capacity(k);
                for i in 0..k {
                    let (jtx, jrx) = std::sync::mpsc::channel::<usize>();
                    let (f, dtx) = (f.clone(), dtx.clone());
                    threads.push(std::thread::spawn(move || {
                        for per in jrx {
                            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                for _ in 0..per {
                                    f(i)?;
                                }
                                Ok(())
                            }))
                            .unwrap_or_else(|_| Err("a client thread panicked".to_string()));
                            if dtx.send(r).is_err() {
                                break;
                            }
                        }
                    }));
                    jobs.push(jtx);
                }
                Callers::Threads { jobs, done, threads }
            }
            Call::Async(rt, f) => Callers::Tasks { rt: rt.clone(), f: f.clone(), k },
        }
    }

    fn batch(&self, per: usize) -> Result<(), String> {
        match self {
            Callers::Threads { jobs, done, .. } => {
                for j in jobs {
                    j.send(per).map_err(|_| "a client thread is gone".to_string())?;
                }
                let mut first = Ok(());
                for _ in 0..jobs.len() {
                    let r = done.recv().map_err(|_| "a client thread is gone".to_string())?;
                    if first.is_ok() {
                        first = r;
                    }
                }
                first
            }
            Callers::Tasks { rt, f, k } => rt.block_on(async {
                let hs: Vec<_> = (0..*k).map(|i| {
                    let f = f.clone();
                    tokio::spawn(async move {
                        for _ in 0..per {
                            f(i).await?;
                        }
                        Ok::<(), String>(())
                    })
                }).collect();
                let mut first = Ok(());
                for h in hs {
                    let r = h.await.unwrap_or_else(|_| Err("a client task panicked".to_string()));
                    if first.is_ok() {
                        first = r;
                    }
                }
                first
            }),
        }
    }
}

impl Drop for Callers {
    fn drop(&mut self) {
        if let Callers::Threads { jobs, threads, .. } = self {
            jobs.clear();
            for t in threads.drain(..) {
                let _ = t.join();
            }
        }
    }
}

fn abort(msg: String) -> ! {
    eprintln!("ABORT (requirement 18): {msg}");
    std::process::exit(3);
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let arg = |k: &str| a.iter().position(|x| x == k).map(|i| a[i + 1].clone());
    let socket = arg("--socket").expect("--socket");
    let transport = arg("--transport").unwrap_or_else(|| "shipped".into());
    let pinned = transport == "pinned";
    let launch: usize = arg("--launch").map(|v| v.parse().unwrap()).unwrap_or(1);
    let rounds: usize = arg("--rounds").map(|v| v.parse().unwrap()).unwrap_or(5);
    let calls: usize = arg("--calls").map(|v| v.parse().unwrap()).unwrap_or(96);
    let warm: usize = arg("--warmup").map(|v| v.parse().unwrap()).unwrap_or(64);
    let server_warm: usize = arg("--server-warm").map(|v| v.parse().unwrap()).unwrap_or(64);
    let out = arg("--out").expect("--out");
    let plant = a.iter().any(|x| x == "--plant");
    let target = format!("unix:{socket}");
    let p22 = prost::Message::encode_to_vec(&harness::arms_m2::prost_arm::value(harness::arms_m2::P2_2)).len() as u64;
    let want_a = if plant { p22 + 1 } else { p22 };
    assert!(harness::generated::binding::ak_init_once() >= 0);

    // Requirement 13 as amended: the server warmed from each client transport first.
    if let Err(e) = grid::warm_server(&target, pinned, server_warm, want_a) {
        abort(format!("server warm-up: {e}"));
    }
    // Requirement 22 as amended (R-H23): the cell order is a seeded random permutation per
    // launch (seed = launch); directions and in-flight counts stay nested inside a cell.
    let mut order: Vec<&str> = CELLS.to_vec();
    campaign::shuffle(&mut order, launch as u64);
    let mut lines = Vec::new();
    for cell in &order {
        // ONE channel per cell per launch (requirement 13 as amended, R-H33).
        let conn = Conn::open(cell, &target, pinned);
        for &dir in DIRS {
            for k in [1usize, 8, 16] {
                let per = calls.div_ceil(k);
                let callers = Callers::new(&grid::call_of(cell, &conn, dir, grid::slots(k), want_a), k);
                // Warm-up, identical for every cell (requirement 24): the channel and the
                // callers exercised, allocator grown.
                if let Err(e) = callers.batch(warm.div_ceil(k)) {
                    abort(format!("cell {cell} dir {dir} inflight {k} warm-up: {e}"));
                }
                for r in 1..=rounds {
                    let (c0, t0) = (process_cpu_ns(), Instant::now());
                    if let Err(e) = callers.batch(per) {
                        abort(format!("cell {cell} dir {dir} inflight {k} round {r}: {e}"));
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
    let server_threads = std::env::var("AK_SERVER_THREADS").unwrap_or_else(|_| "4".into());
    let mut f = std::fs::File::create(&out).unwrap();
    for h in campaign::header("rpc", &[
        ("transport", format!("{transport}: {}", if pinned {
            "4 MiB stream + connection windows, adaptive off, on tonic, the core client and the server (Nagle does not apply to a Unix socket)"
        } else {
            "tonic endpoint defaults, ak_client_new, server defaults"
        })),
        ("link", format!("Unix domain socket {socket} (requirement 17 as amended, R-H28); server = rpc_server, a separate process, one per launch for every cell of both builds, pre-serialised P2.2")),
        ("cells", "A prost+tonic, B prost+core, C core-ffi+core, D core-ffi+tonic, E core-native+core, F core-native+tonic; C-F per unknown-field mode (-retain: every decision 11 position armed and u-group encode; -drop: nothing armed; -nounk: the build with unknown-field support compiled out); callback/queue deliveries not run in this suite".into()),
        ("delivery", "B, C, E: the core's blocking ak_call_unary from k host threads (a pool created before the warm-up, reused); A, D, F: tonic's idiomatic async unary call from k tokio tasks (packages/rust's shape)".into()),
        ("directions", "a = Fetch then decode; a+read = Fetch, decode, read every field; b = encode then Push (the server decodes with prost)".into()),
        ("build", if cfg!(feature = "unknown-fields") { "unknown-fields (retain/drop)".into() } else { "NO-UNKNOWN (unknown-field support compiled out; facade without unknown_fields)".to_string() }),
        ("launch", launch.to_string()),
        ("cell order", format!("{} (seeded random permutation, seed = launch)", order.join(","))),
        ("rounds", rounds.to_string()),
        ("calls per round", format!("{calls} rounded up to a multiple of in-flight")),
        ("warm-up", format!("server: {server_warm} checked Fetch calls from each client transport (tonic, core) before the first cell; then {warm} calls per (cell, dir, in-flight) before its rounds; one channel per cell per launch")),
        ("clocks", "cpu_ns = getrusage(RUSAGE_SELF) utime+stime of the client per round (process CPU); wall_ns = monotonic".into()),
        ("worker threads", format!("client: tokio multi-thread {} workers per A/D/F cell runtime; ak_runtime_new({}) per B/C/E core client; k = 1/8/16 caller threads (B/C/E) or tasks (A/D/F); server: tokio multi-thread {server_threads} workers (AK_SERVER_THREADS)", grid::TOKIO_WORKERS, grid::CORE_WORKERS)),
    ]) {
        writeln!(f, "{h}").unwrap();
    }
    for l in lines {
        writeln!(f, "{l}").unwrap();
    }
}
