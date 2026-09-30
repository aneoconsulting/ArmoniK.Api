//! Properties of the write-coalescing experiment (patch p4-h2-coalesce, 2026-09-30) on the core's
//! transport, against the shared server (serve.sh; AK_RPC_SOCKET_SHIPPED and AK_RPC_SOCKET_PINNED),
//! every call checked on the server's UploadStreamCheck path (byte count and SHA-256 of every
//! message as received). Run with the patched core loaded (LD_LIBRARY_PATH) and AK_H2_COALESCE=N;
//! with N = 1 it checks h2 as shipped. Prints one line per check, then BURST CHECK PASSED or FAILED.
//!   1. flow control: 16 MiB streams on the shipped socket (the server's default 65,535-byte stream
//!      and connection windows, far below N x 16 KiB, so every burst is cut by the window and
//!      continued after WINDOW_UPDATE frames) and on the pinned one (4 MiB windows);
//!   2. multiplexing: 8 concurrent 16 MiB streams on ONE core client (one connection), each
//!      checked; per-stream wall time (min, median, max) printed;
//!   3. cancel mid-burst: ak_call_cancel after 1 and after 4 of 8 chunks (the call's RST_STREAM
//!      while its data is queued), the recv must report CANCELLED, then a full checked stream on
//!      the same client must pass (the connection's framing and windows survived), 10 times each.
use ak_abi::*;
use campaign::generated::roots::R_UploadResultDataMessage as M5;
use campaign::grid;
use campaign::Ops;
use std::time::Instant;

fn stream(cc: &grid::CoreClient, slot: &grid::Slot, chunks: usize, cancel_after: Option<usize>) -> Result<(), String> {
    let pl = grid::stream_payload(chunks);
    let path = grid::STREAM_CHECK;
    unsafe {
        let h = ak_call_open(cc.raw(), path.as_ptr(), path.len(), AK_CALL_CLIENT_STREAM, std::ptr::null());
        if h.is_null() {
            return Err("ak_call_open NULL".into());
        }
        for j in 0..chunks {
            if cancel_after == Some(j) {
                ak_call_cancel(h);
                break;
            }
            M5::f_encode(&slot.ctx, &pl.f[j], true).map_err(|e| format!("encode {e}"))?;
            let rc = ak_call_send_enc(h, slot.ctx.enc, (j + 1 == chunks) as i32);
            if rc != AK_OK {
                ak_call_destroy(h);
                return Err(format!("send {j}: rc {rc}"));
            }
        }
        let mut out = ak_bytes::default();
        let mut gs = -1;
        let rc = ak_call_recv(h, &mut out, &mut gs);
        let r = match cancel_after {
            Some(_) => {
                if rc == AK_ERR_RPC_STATUS && gs == 1 {
                    Ok(())
                } else {
                    Err(format!("cancelled stream: recv rc {rc}, grpc status {gs} (want CANCELLED 1)"))
                }
            }
            None if rc != AK_OK => Err(format!("recv rc {rc} status {gs}")),
            None => grid::stream_response(std::slice::from_raw_parts(out.ptr, out.len), (chunks * grid::CHUNK) as u64, Some(&pl.sha256)),
        };
        ak_bytes_free(&mut out);
        ak_call_destroy(h);
        r
    }
}

fn main() {
    assert!(harness::generated::binding::ak_init_once() >= 0);
    let n = std::env::var("AK_H2_COALESCE").unwrap_or_else(|_| "unset (1)".into());
    println!("# burst check: AK_H2_COALESCE={n}; core {}", std::env::var("LD_LIBRARY_PATH").unwrap_or_else(|_| "(runpath)".into()));
    let mut bad = 0;
    let mut say = |ok: bool, what: String| {
        if !ok {
            bad += 1;
        }
        println!("{} {what}", if ok { "ok  " } else { "FAIL" });
    };
    for (label, var, pinned) in [("shipped", "AK_RPC_SOCKET_SHIPPED", false), ("pinned", "AK_RPC_SOCKET_PINNED", true)] {
        let target = format!("unix:{}", std::env::var(var).expect(var));
        let cc = grid::CoreClient::new(&target, pinned);
        assert_eq!(unsafe { ak_client_set_framed(cc.raw(), 1) }, AK_OK);
        let sl = grid::slots(8);
        // 1. flow control
        for chunks in [8usize, 2] {
            let r: Result<Vec<()>, String> = (0..4).map(|_| stream(&cc, &sl[0], chunks, None)).collect();
            say(r.is_ok(), format!("{label}: 4 x {} MiB streams, checked: {}", chunks * 2, r.err().unwrap_or_else(|| "identical bytes".into())));
        }
        // 2. multiplexing: 8 concurrent streams on this one client
        let cc_ref = &cc;
        let walls: Vec<Result<f64, String>> = std::thread::scope(|s| {
            let hs: Vec<_> = (0..8).map(|i| {
                let slot = &sl[i];
                s.spawn(move || {
                    let t0 = Instant::now();
                    stream(cc_ref, slot, 8, None).map(|_| t0.elapsed().as_secs_f64() * 1e3)
                })
            }).collect();
            hs.into_iter().map(|h| h.join().unwrap_or_else(|_| Err("panic".into()))).collect()
        });
        let mut ok: Vec<f64> = walls.iter().filter_map(|w| w.as_ref().ok().copied()).collect();
        ok.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let errs: Vec<&String> = walls.iter().filter_map(|w| w.as_ref().err()).collect();
        say(errs.is_empty(), format!("{label}: 8 concurrent 16 MiB streams on one connection, each checked: {} ok; per-stream wall ms min {:.1} median {:.1} max {:.1}{}",
            ok.len(), ok.first().copied().unwrap_or(0.0), ok.get(ok.len() / 2).copied().unwrap_or(0.0), ok.last().copied().unwrap_or(0.0),
            if errs.is_empty() { String::new() } else { format!("; errors {errs:?}") }));
        // 3. cancel mid-burst, then a checked stream on the same connection
        for after in [1usize, 4] {
            let mut fails = Vec::new();
            for _ in 0..10 {
                if let Err(e) = stream(&cc, &sl[0], 8, Some(after)) {
                    fails.push(e);
                }
                if let Err(e) = stream(&cc, &sl[0], 8, None) {
                    fails.push(format!("after the cancel: {e}"));
                }
            }
            say(fails.is_empty(), format!("{label}: 10 x (cancel after {after} of 8 chunks: CANCELLED; then a checked 16 MiB stream on the same connection){}",
                if fails.is_empty() { String::new() } else { format!(": {fails:?}") }));
        }
    }
    println!("{}", if bad == 0 { "BURST CHECK PASSED" } else { "BURST CHECK FAILED" });
    std::process::exit(if bad == 0 { 0 } else { 1 });
}
