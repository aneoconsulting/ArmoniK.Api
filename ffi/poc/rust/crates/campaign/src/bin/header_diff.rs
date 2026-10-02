//! T1 option 3's header question, answered with evidence: the request headers the SERVER
//! receives from the reference send path (tonic's `Grpc::unary` + the raw-bytes codec) and
//! from the framed path (`rpc::unary_framed`), over both client transports -- tonic's
//! Channel in the harness (cells D / Df) and the core's client (cells B / Bf, switched with
//! `ak_client_set_framed`) -- for both methods (Fetch: empty request; Push: P2.2). An
//! in-process server on a Unix socket (AK_CHECK_TRANSPORT=tcp: TCP loopback, pinned) records every request (`server::CAPTURE`). Prints
//! each request's headers and, per (transport, method), the diff reference -> framed.
//! Exit status 1 if any pair differs.
//!
//!   header_diff [--pinned]

use campaign::grid::{self, Conn};
use std::collections::BTreeSet;

fn main() {
    // AK_CHECK_TRANSPORT=tcp: the server on TCP loopback, whose configuration is always the
    // pinned one, so the clients are pinned too.
    let tcp = std::env::var("AK_CHECK_TRANSPORT").as_deref() == Ok("tcp");
    let pinned = tcp || std::env::args().any(|a| a == "--pinned");
    assert!(harness::generated::binding::ak_init_once() >= 0);
    let dir = std::env::temp_dir().join(format!("ak-header-diff-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let sock = dir.join("grid.sock");
    let (_server, target, _tcp) = campaign::server::spawn_check_server(sock.clone(), pinned);
    let want_a = campaign::server::p22_response().len() as u64;
    let d_ref = grid::CELLS.iter().copied().find(|c| c.starts_with("D-")).unwrap();
    let d_fr = grid::CELLS.iter().copied().find(|c| c.starts_with("Df-")).unwrap();
    let pairs = [("tonic Channel (harness)", d_ref, d_fr), ("core client", "B", "Bf")];
    println!("# T1 option 3: request headers as the server receives them (transport {}), reference vs framed send path", if tcp { "tcp, pinned" } else if pinned { "pinned" } else { "shipped" });
    let mut differ = 0;
    for (what, r, f) in pairs {
        for dir in ["a", "b", "d"] {
            let mut got = Vec::new();
            for cell in [r, f] {
                let conn = Conn::open(cell, &target, pinned);
                // d: U2-stream's client-streamed upload (4 MiB, 2 chunks).
                let call = if dir == "d" {
                    grid::call_of_d(cell, &conn, 2, grid::slots(1), (2 * grid::CHUNK) as u64, false)
                } else {
                    grid::call_of(cell, &conn, dir, grid::slots(1), want_a)
                };
                campaign::server::FRAMES.lock().unwrap().clear();
                *campaign::server::CAPTURE.lock().unwrap() = Some(Vec::new());
                call.once(0).unwrap_or_else(|e| panic!("{cell} {dir}: {e}"));
                let reqs = campaign::server::CAPTURE.lock().unwrap().take().unwrap();
                assert_eq!(reqs.len(), 1, "{cell} {dir}: one request expected");
                println!("\n## {what}, direction {dir}, cell {cell} ({})", if grid::framed(cell) { "framed" } else { "reference" });
                for l in &reqs[0] {
                    println!("  {l}");
                }
                // The body as the server received it: the evidence that the path ran.
                let fr = campaign::server::FRAMES.lock().unwrap().first().map(|x| x.lock().unwrap().clone()).unwrap_or_default();
                if fr.len() > 12 {
                    let tot: usize = fr.iter().sum();
                    println!("  (request body DATA frames as received: {} frames, {} bytes; first {:?}, last {:?})", fr.len(), tot, &fr[..4], &fr[fr.len() - 3..]);
                } else {
                    println!("  (request body DATA frames as received, bytes: {:?})", fr);
                }
                got.push(reqs[0].clone());
            }
            let a: BTreeSet<_> = got[0].iter().collect();
            let b: BTreeSet<_> = got[1].iter().collect();
            let only_r: Vec<_> = a.difference(&b).collect();
            let only_f: Vec<_> = b.difference(&a).collect();
            let order = got[0] == got[1];
            println!("\n=> {what}, direction {dir}: {} (only reference: {:?}; only framed: {:?}; same order: {})",
                if only_r.is_empty() && only_f.is_empty() { "IDENTICAL header sets" } else { "DIFFERENT" }, only_r, only_f, order);
            if !(only_r.is_empty() && only_f.is_empty()) {
                differ += 1;
            }
        }
    }
    // Status handling: a request the server refuses (Push of bytes that are not a
    // ListTasksDetailedResponse -> INVALID_ARGUMENT, sent trailers-only) must come back as
    // the same error on both paths; the send limit refuses before sending, with tonic's
    // encode_item message; a path the grid does not name gets the same outcome on both paths
    // (this server answers every path but Push with the Fetch response).
    println!("\n## status handling (tonic Channel)");
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    let ch = grid::tonic_channel(&rt, &target, pinned);
    let bad = bytes::Bytes::from_static(b"\xff\xff\xff");
    let push = http::uri::PathAndQuery::from_static(grid::PUSH);
    let (r_ref, r_fr, r_lim, r_404_ref, r_404_fr) = rt.block_on(async {
        let mut g = tonic::client::Grpc::new(ch.clone());
        g.ready().await.unwrap();
        let r_ref = g.unary(tonic::Request::new(bad.clone()), push.clone(), rpc::RawCodec).await.map(|r| r.into_inner());
        let r_fr = rpc::unary_framed(ch.clone(), push.clone(), bad.clone(), None).await;
        let r_lim = rpc::unary_framed(ch.clone(), push.clone(), bad.clone(), Some(2)).await;
        let nope = http::uri::PathAndQuery::from_static("/armonik.ffi.campaign.v1.Grid/Nope");
        let mut g = tonic::client::Grpc::new(ch.clone());
        g.ready().await.unwrap();
        let r_404_ref = g.unary(tonic::Request::new(bad.clone()), nope.clone(), rpc::RawCodec).await.map(|r| r.into_inner());
        let r_404_fr = rpc::unary_framed(ch.clone(), nope, bad.clone(), None).await;
        (r_ref, r_fr, r_lim, r_404_ref, r_404_fr)
    });
    let show = |r: &Result<bytes::Bytes, tonic::Status>| match r {
        Ok(b) => format!("Ok({} B)", b.len()),
        Err(s) => format!("Err({:?}, {:?})", s.code(), s.message()),
    };
    let same = |a: &Result<bytes::Bytes, tonic::Status>, b: &Result<bytes::Bytes, tonic::Status>| match (a, b) {
        (Err(x), Err(y)) => x.code() == y.code() && x.message() == y.message(),
        _ => false,
    };
    println!("  refused Push   reference {}  framed {}  => {}", show(&r_ref), show(&r_fr), if same(&r_ref, &r_fr) { "SAME" } else { "DIFFERENT" });
    let same_any = |a: &Result<bytes::Bytes, tonic::Status>, b: &Result<bytes::Bytes, tonic::Status>| match (a, b) {
        (Ok(x), Ok(y)) => x == y,
        _ => same(a, b),
    };
    println!("  other path     reference {}  framed {}  => {}", show(&r_404_ref), show(&r_404_fr),
        if same_any(&r_404_ref, &r_404_fr) { "SAME" } else { "DIFFERENT" });
    println!("  send limit 2 B framed {} (expected OutOfRange, not sent)", show(&r_lim));
    if !same(&r_ref, &r_fr) || !same_any(&r_404_ref, &r_404_fr)
        || r_lim.as_ref().err().map(|s| s.code()) != Some(tonic::Code::OutOfRange) {
        differ += 1;
    }
    let _ = std::fs::remove_dir_all(&dir);
    std::process::exit(if differ == 0 { 0 } else { 1 });
}
