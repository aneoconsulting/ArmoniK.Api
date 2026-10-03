//! CAMPAIGN.md 4.2: the RPC grid on **criterion** (requirement 22a as amended 2026-09-27,
//! FIX-PLAN WP9), the codec suite's framework and its process-CPU Measurement. One process
//! per launch and build, pinned by the runner to `AK_CPU_CLIENT`; the server is another
//! process (`rpc_server`), ONE per launch, serving both builds (started by the runner).
//!
//! Configuration (environment; criterion's own argument parser is not used):
//!   AK_RPC_SOCKET, AK_RPC_TRANSPORT (shipped|pinned over the Unix socket; armonik over TCP
//!   AK_RPC_TCP = 127.0.0.1:PORT, CAMPAIGN 4.0 as amended), AK_LAUNCH, AK_OUT, CRITERION_HOME
//!   AK_SAMPLES (rounds, >= 10), AK_WARMUP_MS, AK_MEASURE_MS, AK_NRESAMPLES   criterion's
//!   AK_RPC_SERVER_WARMUP  checked calls from each client transport before the first benchmark
//!   AK_RPC_PLANT=warm     the runner's control: the warm-up expects a wrong length -> exit 3, no output
//!   AK_RPC_PLANT=bench    the same, inside the first criterion benchmark -> panic, no output
//!   AK_RPC_WARM_CELLS / AK_RPC_WARM_DIR   the warm-up's (and so a plant's) cells and direction
//!   AK_RPC_CELLS / AK_RPC_DIRS / AK_RPC_INFLIGHT   narrowed runs only
//!   AK_RPC_PAYLOADS   narrowed runs only: the payloads of directions c and d (P5.3, P5.4, 4MiB,
//!                     16MiB); a and b always run P2.2
//!
//! Every exported sample also carries the process's getrusage(RUSAGE_SELF) deltas over the
//! sample's iterations (ru_nvcsw, ru_nivcsw, ru_minflt: totals for the sample, read outside the
//! clock reads, two syscalls per sample).
//!
//! Cells (`campaign::grid`): A prost+tonic, B prost+core, C core-ffi+core, D core-ffi+tonic,
//! E core-native+core, F core-native+tonic, and the framed twins Bf-Ff; C-F per
//! unknown-field mode of this build. Directions a, a+read, b (k = 1, 8, 16), c and d (the
//! labelled extra uploads, k = 1, 8). The tokio runtimes, the core clients, the channels
//! and the k callers (threads for B/C/E, tasks for A/D/F) are all built OUTSIDE the
//! measured closure; one criterion iteration is one batch of k calls in flight.

use campaign::grid::{self, Call, Conn, CELLS, DIRS};
use campaign::{process_clock_ns, ProcessCpu};
use criterion::{Criterion, SamplingMode, Throughput};
use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn env<T: std::str::FromStr>(k: &str, d: T) -> T {
    std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d)
}

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


/// The process's voluntary and involuntary context switches and minor faults (getrusage).
fn rusage() -> [u64; 3] {
    let mut u: libc::rusage = unsafe { std::mem::zeroed() };
    unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut u) };
    [u.ru_nvcsw as u64, u.ru_nivcsw as u64, u.ru_minflt as u64]
}

fn abort(msg: String) -> ! {
    eprintln!("ABORT (requirement 18): {msg}");
    std::process::exit(3);
}

/// One criterion benchmark of the grid.
struct Spec {
    id: String,
    cell: &'static str,
    dir: &'static str,
    payload: &'static str,
    k: usize,
    /// Direction d's chunk count, or 0.
    chunks: usize,
}

fn main() {
    // CAMPAIGN req 25 (D9): the allocator mode, checked before anything is timed.
    let (alloc, alloc_read) = campaign::alloc_check();
    // CAMPAIGN section 4.0 (D18): the grid, and the h2 variant of the core this process mapped.
    let grid_sel = campaign::campaign_grid();
    let (h2, core_so) = campaign::h2_check();
    let socket: String = std::env::var("AK_RPC_SOCKET").unwrap_or_default();
    let transport: String = env("AK_RPC_TRANSPORT", "shipped".to_string());
    let pinned = transport == "pinned";
    let launch: usize = env("AK_LAUNCH", 1);
    let out: String = std::env::var("AK_OUT").expect("AK_OUT");
    let home = std::env::var("CRITERION_HOME").expect("CRITERION_HOME (the runner sets it per launch)");
    let samples: usize = env("AK_SAMPLES", 10).max(10);
    // Requirement 24 as amended (8c02e7c58, owner 2026-10-03): two warm-ups, by direction.
    // Short (AK_WARMUP_MS) for a, a+read, b; long (AK_WARMUP_LONG_MS) for c and d.
    let warm_ms: u64 = env("AK_WARMUP_MS", 1500);
    let warm_long_ms: u64 = env("AK_WARMUP_LONG_MS", 5000);
    let meas_ms: u64 = env("AK_MEASURE_MS", 2000);
    let server_warm: usize = env("AK_RPC_SERVER_WARMUP", 64);
    let plant: String = env("AK_RPC_PLANT", String::new());
    let (plant_warm, plant_bench) = (plant == "warm", plant == "bench");
    // CAMPAIGN 4.0 as amended (b58543f7b): the core grid's one configuration, `armonik`, over TCP
    // 127.0.0.1 (D10): cell A through armonik_transport::connect, the core cells through the
    // core's current client (ak_client_new), Nagle off on every socket, read back below.
    let armonik = transport == "armonik";
    let tcp: String = std::env::var("AK_RPC_TCP").unwrap_or_default();
    if grid_sel == "core" && !armonik {
        eprintln!("REFUSED: the core grid runs one transport configuration, armonik (AK_RPC_TRANSPORT={transport}); shipped and pinned are full-grid extras");
        std::process::exit(6);
    }
    if armonik && tcp.is_empty() {
        eprintln!("REFUSED: the armonik transport needs AK_RPC_TCP (127.0.0.1:PORT)");
        std::process::exit(6);
    }
    grid::ARMONIK_CHANNEL.store(armonik, std::sync::atomic::Ordering::Relaxed);
    let target = if armonik { format!("http://{tcp}") } else { format!("unix:{socket}") };
    let p22 = prost::Message::encode_to_vec(&harness::arms_m2::prost_arm::value(harness::arms_m2::P2_2)).len() as u64;
    let bad = |on: bool| on as u64;
    assert!(harness::generated::binding::ak_init_once() >= 0);

    // Requirement 13 as amended: the server warmed from each client transport first
    // (AK_RPC_WARM_CELLS: stems; default A and B). The runner's plant names one send path.
    let wc: String = env("AK_RPC_WARM_CELLS", "A,B".to_string());
    let warm_cells: Vec<&str> = wc.split(',').map(grid::cell_of).collect();
    let warm_dir: String = env("AK_RPC_WARM_DIR", "a".to_string());
    if let Err(e) = match warm_dir.as_str() {
        "c" => grid::warm_with_c(&warm_cells, &target, pinned, server_warm, bad(plant_warm)),
        "d" => grid::warm_with_d(&warm_cells, &target, pinned, server_warm.div_ceil(4), bad(plant_warm)),
        _ => grid::warm_with(&warm_cells, &target, pinned, server_warm, p22 + bad(plant_warm)),
    } {
        abort(format!("server warm-up: {e}"));
    }

    // Every benchmark, then one seeded random order over all of them (requirement 22).
    let narrow = |k: &str| std::env::var(k).ok().map(|v| v.split(',').map(String::from).collect::<Vec<_>>());
    let cells_n = narrow("AK_RPC_CELLS").map(|v| v.iter().map(|s| grid::cell_of(s)).collect::<Vec<_>>());
    let dirs_n = narrow("AK_RPC_DIRS");
    let ks_n: Option<Vec<usize>> = narrow("AK_RPC_INFLIGHT").map(|v| v.iter().map(|x| x.parse().unwrap()).collect());
    let pays_n = narrow("AK_RPC_PAYLOADS");
    let pay_on = |p: &str| pays_n.as_ref().map_or(true, |v| v.iter().any(|x| x == p));
    let dir_on = |d: &str| dirs_n.as_ref().map_or(true, |v| v.iter().any(|x| x == d));
    let k_on = |k: usize| ks_n.as_ref().map_or(true, |v| v.contains(&k));
    let mut specs = Vec::new();
    for &cell in CELLS.iter().filter(|c| cells_n.as_ref().map_or(true, |v| v.contains(c))) {
        for &dir in DIRS.iter().filter(|d| dir_on(d)) {
            for k in [1usize, 8, 16].into_iter().filter(|k| k_on(*k)) {
                specs.push(Spec { id: format!("{cell}/{dir}/P2.2/k{k}"), cell, dir, payload: "P2.2", k, chunks: 0 });
            }
        }
        if dir_on("c") {
            for &pid in grid::C_PAYLOADS.iter().filter(|p| pay_on(p)) {
                for &k in grid::C_INFLIGHT.iter().filter(|k| k_on(**k)) {
                    specs.push(Spec { id: format!("{cell}/c/{pid}/k{k}"), cell, dir: "c", payload: pid, k, chunks: 0 });
                }
            }
        }
        if dir_on("d") {
            for &(label, chunks) in grid::D_PAYLOADS.iter().filter(|(l, _)| pay_on(l)) {
                for &k in grid::D_INFLIGHT.iter().filter(|k| k_on(**k)) {
                    specs.push(Spec { id: format!("{cell}/d/{label}/k{k}"), cell, dir: "d", payload: label, k, chunks });
                }
            }
        }
    }
    if grid_sel == "core" {
        specs.retain(|s| campaign::core_rpc_spec(s.cell, s.dir, s.payload, s.k, alloc, h2));
        if specs.is_empty() {
            eprintln!("REFUSED: the core grid has no benchmark for this build / allocator / h2 / narrowing; no sample is taken");
            std::process::exit(6);
        }
    }
    campaign::shuffle(&mut specs, launch as u64);
    // One channel per cell for the whole process, opened before the first benchmark.
    let mut conns: std::collections::HashMap<&str, Conn> = Default::default();
    for s in &specs {
        conns.entry(s.cell).or_insert_with(|| Conn::open(s.cell, &target, pinned));
    }

    // CAMPAIGN 4.0 as amended: Nagle off on every client socket, read back on the live sockets
    // of this process before any timing (refuses on Nagle on).
    let nd_before = if armonik { campaign::nodelay_readback(&tcp) } else { 0 };
    let mut c = Criterion::default()
        .with_measurement(ProcessCpu)
        .sample_size(samples)
        .warm_up_time(Duration::from_millis(warm_ms))
        .measurement_time(Duration::from_millis(meas_ms))
        .nresamples(env("AK_NRESAMPLES", 100_000))
        .without_plots();
    // Per criterion call of the routine, per benchmark: (iterations, cpu, wall, getrusage
    // deltas [voluntary, involuntary context switches, minor faults]).
    let walls: Vec<Rc<RefCell<Vec<(u64, u64, u64, [u64; 3])>>>> = specs.iter().map(|_| Rc::new(RefCell::new(Vec::new()))).collect();
    {
        let mut g = c.benchmark_group("rpc");
        g.sampling_mode(SamplingMode::Flat);
        for (i, s) in specs.iter().enumerate() {
            let conn = &conns[s.cell];
            let sl = grid::slots(s.k);
            let call: Call = match s.dir {
                "c" => grid::call_of_c(s.cell, conn, s.payload, sl, bad(plant_bench)),
                "d" => grid::call_of_d(s.cell, conn, s.chunks, sl, (s.chunks * grid::CHUNK) as u64 + bad(plant_bench), false),
                d => grid::call_of(s.cell, conn, d, sl, p22 + bad(plant_bench)),
            };
            let callers = Callers::new(&call, s.k);
            g.throughput(Throughput::Elements(s.k as u64));
            let w = walls[i].clone();
            let id = s.id.clone();
            // criterion reads the group's config at each bench_function: per-benchmark warm-up
            g.warm_up_time(Duration::from_millis(if matches!(s.dir, "c" | "d") { warm_long_ms } else { warm_ms }));
            g.bench_function(format!("{i:05}"), |b| b.iter_custom(|iters| {
                let u0 = rusage();
                let (c0, t0) = (process_clock_ns(), Instant::now());
                for _ in 0..iters {
                    if let Err(e) = callers.batch(1) {
                        // Requirement 18: the benchmark fails, and with it the process.
                        panic!("ABORT (requirement 18): {id}: {e}");
                    }
                }
                let (cpu, wall) = (process_clock_ns() - c0, t0.elapsed().as_nanos() as u64);
                let u1 = rusage();
                w.borrow_mut().push((iters, cpu, wall, [u1[0] - u0[0], u1[1] - u0[1], u1[2] - u0[2]]));
                cpu
            }));
            drop(callers);
        }
        g.finish();
    }

    let nd_after = if armonik { campaign::nodelay_readback(&tcp) } else { 0 };
    let server_threads = std::env::var("AK_SERVER_THREADS").unwrap_or_else(|_| grid::workers_default().to_string());
    let mut f = std::fs::File::create(&out).unwrap();
    for h in campaign::header("rpc", &[
        ("transport", format!("{transport}: {}", if armonik {
            format!("CAMPAIGN 4.0 as amended (b58543f7b): the core grid's one configuration, TCP 127.0.0.1 (D10). Cell A (and D, F under the full grid): packages/rust/armonik-transport `connect`, CALLED DIRECTLY, ClientConfigArgs::default() + the endpoint, so {:?}: hyper-util HttpConnector, nodelay true (tcp_nagle_algorithm false), connect timeout 60 s, no request timeout, no rate limit, no TCP keepalive, no HTTP/2 keepalive, keep_alive_while_idle false, no header-list limit, no user agent, wrapped in hyper-rustls https_or_http (plain http here, TLS never negotiated), http1 + http2 enabled on the connector, tonic Endpoint defaults otherwise (hyper's h2 client windows: 2 MiB stream, 5 MiB connection; adaptive off). Core cells (Bf-cb, Cf-cb, Ef-cb): the core's current client, ak_client_new (no ak_client_opts): tonic Endpoint::from_shared + connect(), tonic's own HttpConnector, TCP_NODELAY true (tonic's default), the same hyper h2 windows, adaptive off. Differences from armonik-transport: no connect timeout (armonik 60 s), tonic's connector instead of the hyper-rustls wrapper (plain TCP either way), HTTP/1 not enabled on the connector (both speak HTTP/2 prior knowledge); everything else equal. Server: rpc_server's TCP listener, the pinned server configuration (4 MiB stream and connection windows, adaptive off), TCP_NODELAY set and read back on every accepted socket (the server log). Nagle read back on this process's live TCP sockets to {tcp}: {nd_before} sockets before the first benchmark and {nd_after} after the last, every one TCP_NODELAY = 1 (a 0 refuses the run)", grid::armonik_config(&target))
        } else if pinned {
            "4 MiB stream + connection windows, adaptive off, on tonic, the core client and the server (Nagle does not apply to a Unix socket)".to_string()
        } else {
            "tonic endpoint defaults, ak_client_new, server defaults".to_string()
        })),
        ("link", if armonik { format!("TCP 127.0.0.1 ({tcp}), CAMPAIGN req 17 as amended (D10); server = rpc_server, a separate process, one per launch, pre-serialised P2.2; receive limit {} B", campaign::server::SERVER_MAX_RECV) } else { format!("Unix domain socket {socket} (requirement 17 as amended, R-H28); server = rpc_server, a separate process, one per launch for every cell of both builds, pre-serialised P2.2; receive limit {} B (tonic default 4 MiB; P5.4 of direction c is 4,194,390 B)", campaign::server::SERVER_MAX_RECV) }),
        ("cells", "A prost+tonic, B prost+core, C core-ffi+core, D core-ffi+tonic, E core-native+core, F core-native+tonic; C-F per unknown-field mode (-retain: every decision 11 position armed and u-group encode; -drop: nothing armed; -nounk: the build with unknown-field support compiled out); Bf, Cf, Df, Ef, Ff = the same cells on the FRAMED send path (optimisation T1 option 3, labelled extra cells: rpc::unary_framed, the request message sent as a 5-byte prefix frame and the caller's Bytes, no copy into tonic's buffer; B/C/E via ak_client_set_framed, D/F in the harness; request headers as tonic's Grpc::unary builds them, response via Status::from_header_map + Streaming::new_response; compression off on both paths); -cb = the same B, C, E cells (and framed twins) through the core's CALLBACK delivery bridged to async Rust with a tokio oneshot (CAMPAIGN req 16 as amended 2026-09-28: for Rust the REFERENCE core-transport cells; the blocking B, C, E cells are the labelled row, kept for cross-host comparability); queue deliveries not run in this suite".into()),
        ("delivery", "B-cb, C-cb, E-cb (the reference core cells): ak_call_unary_cb / ak_call_unary_enc_cb (C direction b and c: the context's buffer moved), and for direction d ak_call_open + ak_call_send_cb / ak_call_send_enc_cb (one oneshot per send completion) + ak_call_recv_cb, each completion sent by the core's callback into a tokio oneshot awaited by one of k tokio tasks on the cell's own runtime (AK_HOST_WORKERS workers, as A/D/F); B, C, E (labelled, blocking): the core's blocking ak_call_unary / ak_call_unary_enc / ak_call_send / ak_call_recv from k host threads (a pool created before the warm-up, reused); A, D, F: tonic's idiomatic async unary call from k tokio tasks (packages/rust's shape)".into()),
        ("cell C request", "direction b: the core encode context's output MOVED into the call (ak_call_unary_enc, optimisation R2; since T1 through ak_rt::Enc::take, the buffer recycled through a spare slot); a and a+read: empty request through ak_call_unary. Cell D direction b: the output moved to the host as an owned buffer (ak_enc_take_owned, T1 ffi) wrapped by Bytes::from_owner and released with ak_bytes_free when tonic drops it; cell F direction b: core-native's Enc::take (T1)".into()),
        ("core-ffi encode fill", campaign::FFI_ENCODE_FILL.into()),
        ("client limits", "ak_client_opts max_send_message = 0 and max_recv_message = 0 on both transports, i.e. tonic's defaults (unlimited send, 4 MiB receive), enforced by the core since D44 and the same on the tonic cells (Grpc defaults): what packages/rust's client sets (neither). Largest response received: P2.2, 540,422 B; largest request message: P5.4, 4,194,390 B (direction c); stream messages 2 MiB + 57 B (direction d); the server accepts 8 MiB".into()),
        ("directions", "a = Fetch then decode; a+read = Fetch, decode, read every field; b = encode then Push (the server decodes with prost); c = U1-unary, a labelled extra direction: encode P5.3 (1 MB) or P5.4 (4 MB), M5, and Upload it (the server decodes it with prost, answers empty), k = 1 and 8 only, every cell; d = U2-stream, a labelled extra direction: a client-streamed upload (CAMPAIGN req 14, 2 MiB chunks, M5 per chunk with the ids on the first message only) of 4 MiB (2 chunks) or 16 MiB (8 chunks), k = 1 and 8, every cell, a third of the calls per round; the server (tonic client streaming) decodes each message with prost and answers the data byte count, checked; B/C/E through ak_call_open / ak_call_send (C: ak_call_send_enc, the context's buffer moved) / ak_call_recv, blocking; A tonic client streaming with prost; D/F tonic client streaming with the raw codec (or framed)".into()),
        ("build", if cfg!(feature = "unknown-fields") { "unknown-fields (retain/drop)".into() } else { "NO-UNKNOWN (unknown-field support compiled out; facade without unknown_fields)".to_string() }),
        ("launch", launch.to_string()),
        ("order", format!("benchmarks registered in a seeded random permutation of every (cell, dir, payload, k) (seed = launch); criterion runs them in registration order; first: {}", specs.iter().take(6).map(|s| s.id.as_str()).collect::<Vec<_>>().join(", "))),
        ("engine", format!("criterion 0.5 (CAMPAIGN req 22a as amended 2026-09-27, WP9): one criterion benchmark per (cell, dir, payload, in-flight k), SamplingMode::Flat, {samples} samples (= rounds, criterion's floor 10), warm-up {warm_ms} ms (directions a, a+read, b; AK_WARMUP_MS) or {warm_long_ms} ms (directions c, d; AK_WARMUP_LONG_MS) and measurement {meas_ms} ms per benchmark (AK_MEASURE_MS); ONE ITERATION = ONE BATCH OF k CALLS IN FLIGHT, counted as k operations (Throughput::Elements(k); `iters` in a row = calls = criterion iterations x k, `batches` = criterion iterations); raw samples exported from criterion's sample.json, none dropped")),
        ("warm-up", format!("server: {server_warm} checked Fetch calls from each client transport (tonic, core) before the first benchmark (AK_RPC_SERVER_WARMUP); then each benchmark's warm-up is criterion's own ({warm_ms} ms for a, a+read, b; {warm_long_ms} ms for c, d; every call checked); every cell's channel opened once, before the first benchmark, and shared by all its benchmarks (one channel per cell per benchmark process)")),
        ("grid", format!("AK_CAMPAIGN_GRID={grid_sel} (CAMPAIGN section 4.0, D18; core | full). core: cells A, Bf-cb, Cf-cb-retain, Ef-cb-retain (the framed core cells with Rust's idiomatic delivery, the callback bridged to async, req 16 as amended), full build; a+read and b at P2.2, c at P5.4, d at 16 MiB; k = 1 and 8; the pinned allocator pass: A and Cf-cb-retain on c and d at k = 1; h2-batch: Cf-cb-retain on c and d at k = 1 and 8, labelled h2 = h2-batch. Every row is labelled row = core | extra")),
        ("extras left out", if grid_sel == "core" { campaign::RPC_EXTRAS.to_string() } else { "none (full grid: extras run, labelled row = extra)".to_string() }),
        ("h2", format!("{h2} (AK_H2; checked against the core this process mapped: {core_so}); tonic's own h2 (cells A, D, F and the tonic side of every cell) is stock h2 0.4.19 in every process")),
        ("alloc", campaign::alloc_header(alloc, alloc_read)),
        ("clocks", "cpu_ns = process CPU per sample, CLOCK_PROCESS_CPUTIME_ID (criterion Measurement ProcessCpu, the codec suite's); wall_ns = monotonic, measured around the same iterations by the benchmark's own routine (iter_custom) and matched to criterion's samples: criterion keeps one quantity, so wall is a column beside it".into()),
        ("checks", "every call checked (requirement 18): a failed check PANICS inside the benchmark (criterion has no stop-on-error), which aborts the process before any output is written; the runner then discards the launch's output".into()),
        ("worker threads", format!("client: tokio {} per A/D/F and -cb cell runtime (AK_HOST_WORKERS; mtN = multi-thread N workers, ct = current-thread; default AK_WORKERS = mt{}); ak_runtime_new({}) per B/C/E core client (AK_CORE_WORKERS; default AK_WORKERS = {}); k = 1/8/16 caller threads (B/C/E) or tasks (A/D/F); server: tokio multi-thread {server_threads} workers (AK_SERVER_THREADS; default AK_WORKERS). D14 (owner, 2026-10-03): every pool is AK_WORKERS workers, default 8", grid::host_rt_label(), grid::workers_default(), grid::core_workers(), grid::workers_default())),
    ]) {
        writeln!(f, "{h}").unwrap();
    }
    let build = if cfg!(feature = "unknown-fields") { "full" } else { "no-unknown" };
    let mut rows = 0usize;
    for (i, s) in specs.iter().enumerate() {
        let p = std::path::Path::new(&home).join("rpc").join(format!("{i:05}")).join("new/sample.json");
        let js: serde_json::Value = serde_json::from_slice(&std::fs::read(&p)
            .unwrap_or_else(|e| panic!("criterion sample file {}: {e}", p.display()))).unwrap();
        let iters = js["iters"].as_array().unwrap();
        let times = js["times"].as_array().unwrap();
        // The measured samples are the routine's LAST `n` calls (criterion's warm-up calls
        // come first); each must match criterion's own (iterations, cpu) exactly.
        let w = walls[i].borrow();
        let n = iters.len();
        assert!(w.len() >= n, "{}: {} routine calls for {n} samples", s.id, w.len());
        let tail = &w[w.len() - n..];
        for (r, ((it, t), &(wi, wc, ww, ru))) in iters.iter().zip(times).zip(tail).enumerate() {
            let (it, t) = (it.as_f64().unwrap() as u64, t.as_f64().unwrap() as u64);
            assert!(it == wi && t == wc, "{}: sample {r} (iters {it}, cpu {t}) does not match the routine's ({wi}, {wc})", s.id);
            let mut o = serde_json::json!({
                "slice": "rust", "suite": "rpc", "cell": s.cell, "payload": s.payload, "dir": s.dir,
                "transport": transport, "inflight": s.k, "launch": launch, "round": r + 1,
                "cpu_ns": t, "wall_ns": ww, "iters": it * s.k as u64, "batches": it,
                "build": build, "send_path": if grid::framed(s.cell) { "framed" } else { "reference" },
                "ru_nvcsw": ru[0], "ru_nivcsw": ru[1], "ru_minflt": ru[2], "minflt": ru[2], "alloc": alloc,
                "grid": grid_sel, "h2": h2,
                "row": if campaign::core_rpc_spec(s.cell, s.dir, s.payload, s.k, alloc, h2) { "core" } else { "extra" },
            });
            o["delivery"] = grid::delivery(s.cell).into();
            if let Some(m) = grid::mode_of(s.cell) {
                o["unknown_mode"] = match m { "nounk" => "no-unknown", x => x }.into();
            }
            writeln!(f, "{o}").unwrap();
            rows += 1;
        }
    }
    eprintln!("# wrote {rows} sample rows to {out}");
}
