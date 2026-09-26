//! CAMPAIGN.md 4.2: the RPC grid's server, a SEPARATE process (requirement 13), pinned by the
//! runner to `AK_CPU_SERVER`. ONE server per launch, serving every cell of both builds'
//! clients (requirement 13 as amended 2026-09-26, R-H33).
//!
//!   rpc_server --transport shipped|pinned --socket PATH --ready-file PATH
//!
//! What it serves: `campaign::server` (Fetch = pre-serialised P2.2, Push = prost decode).
//! Worker threads: AK_SERVER_THREADS (default 4, the campaign's SERVER set size), recorded
//! in its log and in every client header.

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let arg = |k: &str| a.iter().position(|x| x == k).map(|i| a[i + 1].clone());
    let transport = arg("--transport").unwrap_or_else(|| "shipped".into());
    let socket = std::path::PathBuf::from(arg("--socket").expect("--socket"));
    let ready = arg("--ready-file").expect("--ready-file");
    let pinned = match transport.as_str() {
        "pinned" => true,
        "shipped" => false,
        t => panic!("transport {t}"),
    };
    let bytes = campaign::server::p22_response();
    let threads: usize = std::env::var("AK_SERVER_THREADS").ok().and_then(|v| v.parse().ok()).unwrap_or(4);
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(threads).enable_all().build().unwrap();
    let n = bytes.len();
    let sock = socket.clone();
    rt.block_on(campaign::server::serve_uds(socket, pinned, bytes, move || {
        let tmp = format!("{ready}.tmp");
        std::fs::write(&tmp, format!("{}\n{n}\n", sock.display())).unwrap();
        std::fs::rename(&tmp, &ready).unwrap();
        eprintln!("# rpc_server: {transport} on unix:{}, P2.2 {n} B pre-serialised, tokio multi-thread {threads} workers, pid {}",
                  sock.display(), std::process::id());
    }));
}
