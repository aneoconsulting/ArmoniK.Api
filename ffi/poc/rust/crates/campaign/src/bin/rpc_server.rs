//! CAMPAIGN.md 4.2: the RPC grid's server, a SEPARATE process (requirement 13), pinned by the
//! caller to `AK_CPU_SERVER`. FIX-PLAN WP10 (requirement 13 as amended at 9f6d579fa): THE one
//! RPC server of every slice, started through `poc/rust/serve.sh`; its interface is
//! `poc/rust/SERVER.md`.
//!
//!   rpc_server --socket-shipped PATH --socket-pinned PATH --ready-file PATH
//!       one process, two Unix sockets: the shipped and the pinned server configuration
//!   rpc_server --transport shipped|pinned --socket PATH --ready-file PATH
//!       one socket, one configuration
//!
//!   --tcp PORT (optional, with either form above): also serve on TCP 127.0.0.1:PORT (0 = any
//!       free port), the pinned configuration, TCP_NODELAY on every accepted socket
//!
//! The ready file is written once every socket is bound: `<config> <path>` per line, and
//! `tcp 127.0.0.1:<port>` when --tcp is given.
//! Worker threads: AK_SERVER_THREADS (default 4, the campaign's SERVER set size), one tokio
//! multi-thread runtime shared by both sockets; recorded in its log.

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let arg = |k: &str| a.iter().position(|x| x == k).map(|i| a[i + 1].clone());
    let ready = arg("--ready-file").expect("--ready-file");
    let mut socks: Vec<(String, bool)> = Vec::new();
    if let Some(p) = arg("--socket-shipped") {
        socks.push((p, false));
    }
    if let Some(p) = arg("--socket-pinned") {
        socks.push((p, true));
    }
    if socks.is_empty() {
        let transport = arg("--transport").unwrap_or_else(|| "shipped".into());
        let pinned = match transport.as_str() {
            "pinned" => true,
            "shipped" => false,
            t => panic!("transport {t}"),
        };
        socks.push((arg("--socket").expect("--socket"), pinned));
    }
    let bytes = campaign::server::p22_response();
    let threads: usize = std::env::var("AK_SERVER_THREADS").ok().and_then(|v| v.parse().ok()).unwrap_or(4);
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(threads).enable_all().build().unwrap();
    let n = bytes.len();
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let mut tasks = Vec::new();
    for (p, pinned) in &socks {
        let tx = tx.clone();
        tasks.push(rt.spawn(campaign::server::serve_uds(p.into(), *pinned, bytes.clone(), move || {
            let _ = tx.send(());
        })));
    }
    for _ in &socks {
        rx.recv().expect("a socket did not bind");
    }
    let mut tcp_port = None;
    if let Some(port) = arg("--tcp") {
        let port: u16 = port.parse().expect("--tcp PORT");
        let (ptx, prx) = std::sync::mpsc::channel::<u16>();
        tasks.push(rt.spawn(campaign::server::serve_tcp(port, bytes.clone(), move |p| {
            let _ = ptx.send(p);
        })));
        tcp_port = Some(prx.recv().expect("the TCP listener did not bind"));
    }
    let mut lines: String = socks.iter().map(|(p, pinned)| format!("{} {p}\n", if *pinned { "pinned" } else { "shipped" })).collect();
    if let Some(p) = tcp_port {
        lines.push_str(&format!("tcp 127.0.0.1:{p}\n"));
    }
    let tmp = format!("{ready}.tmp");
    std::fs::write(&tmp, &lines).unwrap();
    std::fs::rename(&tmp, &ready).unwrap();
    for (p, pinned) in &socks {
        eprintln!("# rpc_server: {} on unix:{p}, P2.2 {n} B pre-serialised, tokio multi-thread {threads} workers (shared), receive limit {} B, pid {}",
                  if *pinned { "pinned" } else { "shipped" }, campaign::server::SERVER_MAX_RECV, std::process::id());
    }
    if let Some(p) = tcp_port {
        eprintln!("# rpc_server: pinned configuration on tcp 127.0.0.1:{p} (TCP_NODELAY set on every accepted socket), P2.2 {n} B pre-serialised, tokio multi-thread {threads} workers (shared), receive limit {} B, pid {}",
                  campaign::server::SERVER_MAX_RECV, std::process::id());
    }
    rt.block_on(async move {
        for t in tasks {
            t.await.unwrap();
        }
    });
}
