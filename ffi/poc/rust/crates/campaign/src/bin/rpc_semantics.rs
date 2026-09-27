//! ABI v1 section 9's call semantics, checked against the core (gate step 11f; the owner's
//! "streaming as built" and "the status number, on unary calls too"): every case on both
//! send paths (reference, framed). An in-process server on a Unix socket
//! (`server::TEST_PREFIX` paths). Prints one line per case; exit 1 on any failure.
//!
//!   status   a chosen non-OK code comes back as AK_ERR_RPC_STATUS with its number:
//!            unary blocking (ak_call_unary, ak_call_unary_enc), callback and queue
//!            completions, and a stream's ak_call_recv; the OK case gives grpc_status 0
//!   deadline a stream to a server that sleeps 3 s, deadline 300 ms -> DEADLINE_EXCEEDED
//!   metadata ASCII and "-bin" metadata on a stream, echoed by the server into its response
//!   cancel   ak_call_cancel unblocks a stream's pending recv (CANCELLED) and delivers a
//!            CANCELLED completion for a callback and a queue call to a sleeping server
//!   limits   send limit: a unary request and a stream message above it -> AK_ERR_LIMIT and
//!            nothing sent (the server sees no request / only the later small message);
//!            receive limit: a unary response above it -> AK_ERR_LIMIT (grpc_status 8), a
//!            stream's -> AK_ERR_RPC_STATUS with RESOURCE_EXHAUSTED (8)
//!   misuse   a second recv and a send after last: AK_ERR_INVALID_STATE, grpc_status untouched;
//!            the reserved kinds open nothing
use ak_abi::*;
use campaign::grid;
use campaign::server::{CAPTURE, TEST_PREFIX};
use std::sync::{Arc, Condvar, Mutex};

struct Client {
    rt: *mut ak_runtime,
    c: *mut ak_client,
}

impl Client {
    fn new(target: &str, framed: bool, max_send: u32, max_recv: u32) -> Client {
        unsafe {
            let rt = ak_runtime_new(2);
            let o = ak_client_opts {
                stream_window: 0,
                connection_window: 0,
                adaptive_window: -1,
                max_recv_message: max_recv,
                max_send_message: max_send,
                tcp_nagle: -1,
            };
            let c = ak_client_new_opts(rt, target.as_ptr(), target.len(), &o);
            assert!(!c.is_null());
            assert_eq!(ak_client_set_framed(c, framed as i32), AK_OK);
            Client { rt, c }
        }
    }
    fn unary(&self, path: &str, req: &[u8]) -> (i32, i32, usize) {
        unsafe {
            let mut out = ak_bytes::default();
            let mut gs = -99;
            let rc = ak_call_unary(self.c, path.as_ptr(), path.len(), req.as_ptr(), req.len(), &mut out, &mut gs);
            let n = out.len;
            ak_bytes_free(&mut out);
            (rc, gs, n)
        }
    }
    /// A stream: open with `opts`, send each message (rc per send), recv.
    fn stream(&self, path: &str, opts: Option<&ak_call_opts>, msgs: &[&[u8]]) -> (Vec<i32>, i32, i32, Vec<u8>) {
        unsafe {
            let h = ak_call_open(self.c, path.as_ptr(), path.len(), AK_CALL_CLIENT_STREAM,
                                 opts.map_or(std::ptr::null(), |o| o as *const _));
            assert!(!h.is_null(), "ak_call_open {path}");
            let mut rcs = Vec::new();
            for (i, m) in msgs.iter().enumerate() {
                rcs.push(ak_call_send(h, m.as_ptr(), m.len(), (i + 1 == msgs.len()) as i32));
            }
            if msgs.is_empty() {
                rcs.push(ak_call_send(h, std::ptr::null(), 0, 1));
            }
            let mut out = ak_bytes::default();
            let mut gs = -99;
            let rc = ak_call_recv(h, &mut out, &mut gs);
            let v = if out.len == 0 { Vec::new() } else { std::slice::from_raw_parts(out.ptr, out.len).to_vec() };
            ak_bytes_free(&mut out);
            ak_call_destroy(h);
            (rcs, rc, gs, v)
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        unsafe {
            ak_client_destroy(self.c);
            ak_runtime_destroy(self.rt);
        }
    }
}

struct Sig {
    m: Mutex<Option<(i32, i32)>>,
    cv: Condvar,
}

extern "C" fn on_complete(user: *mut std::ffi::c_void, comp: *mut ak_completion) {
    unsafe {
        let s = &*(user as *const Sig);
        *s.m.lock().unwrap() = Some(((*comp).status, (*comp).grpc_status));
        ak_bytes_free(&mut (*comp).bytes);
        s.cv.notify_all();
    }
}

fn cb_call(cl: &Client, path: &str, cancel_after: Option<std::time::Duration>) -> (i32, i32) {
    let sig = Arc::new(Sig { m: Mutex::new(None), cv: Condvar::new() });
    unsafe {
        let h = ak_call_unary_cb(cl.c, path.as_ptr(), path.len(), [].as_ptr(), 0, on_complete,
                                 Arc::as_ptr(&sig) as *mut std::ffi::c_void, 7);
        assert!(!h.is_null());
        if let Some(d) = cancel_after {
            std::thread::sleep(d);
            ak_call_cancel(h);
        }
        let mut g = sig.m.lock().unwrap();
        while g.is_none() {
            let (ng, to) = sig.cv.wait_timeout(g, std::time::Duration::from_secs(10)).unwrap();
            g = ng;
            if to.timed_out() {
                break;
            }
        }
        let r = g.unwrap_or((i32::MIN, i32::MIN));
        drop(g);
        ak_call_destroy(h);
        r
    }
}

fn q_call(cl: &Client, path: &str, cancel_after: Option<std::time::Duration>) -> (i32, i32) {
    unsafe {
        let q = ak_queue_new();
        let h = ak_call_unary_q(cl.c, path.as_ptr(), path.len(), [].as_ptr(), 0, q, 9);
        assert!(!h.is_null());
        if let Some(d) = cancel_after {
            std::thread::sleep(d);
            ak_call_cancel(h);
        }
        let mut comp = ak_completion { tag: 0, status: 0, grpc_status: 0, bytes: ak_bytes::default() };
        let rc = ak_queue_next(q, &mut comp, 10_000);
        let r = if rc == AK_QUEUE_OK { (comp.status, comp.grpc_status) } else { (i32::MIN, i32::MIN) };
        ak_bytes_free(&mut comp.bytes);
        ak_call_destroy(h);
        ak_queue_shutdown(q);
        ak_queue_destroy(q);
        r
    }
}

/// A valid M5 message (ids and a data chunk of `n` bytes), as the upload path's server wants.
fn m5(n: usize) -> Vec<u8> {
    let v = shapes_prost::shapes::UploadResultDataMessage {
        upload: Some(shapes_prost::shapes::UploadResultData {
            session_id: "s".into(),
            result_id: "r".into(),
            data_chunk: vec![7u8; n].into(),
            ..Default::default()
        }),
        ..Default::default()
    };
    prost::Message::encode_to_vec(&v)
}

fn main() {
    assert!(harness::generated::binding::ak_init_once() >= 0);
    let dir = std::env::temp_dir().join(format!("ak-rpc-semantics-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let sock = dir.join("grid.sock");
    let _server = campaign::server::spawn_in_process(sock.clone(), false);
    let target = format!("unix:{}", sock.display());
    let p = |t: &str| format!("{TEST_PREFIX}{t}");
    let mut bad = 0;
    let mut check = |ok: bool, what: String| {
        println!("{} {what}", if ok { "PASS" } else { "FAIL" });
        if !ok {
            bad += 1;
        }
    };
    let p22 = campaign::server::p22_response().len();
    for framed in [false, true] {
        let path_name = if framed { "framed" } else { "reference" };
        let cl = Client::new(&target, framed, 0, 0);
        // status: unary, all three deliveries, and the stream
        let (rc, gs, n) = cl.unary(grid::FETCH, &[]);
        check(rc == AK_OK && gs == 0 && n == p22, format!("[{path_name}] unary OK: rc {rc}, grpc_status {gs}, {n} B"));
        let (rc, gs, _) = cl.unary(&p("StatusU7"), &[]);
        check(rc == AK_ERR_RPC_STATUS && gs == 7, format!("[{path_name}] unary blocking, server status 7: rc {rc}, grpc_status {gs}"));
        let (rc, gs) = cb_call(&cl, &p("StatusU5"), None);
        check(rc == AK_ERR_RPC_STATUS && gs == 5, format!("[{path_name}] unary callback, server status 5: completion status {rc}, grpc_status {gs}"));
        let (rc, gs) = q_call(&cl, &p("StatusU14"), None);
        check(rc == AK_ERR_RPC_STATUS && gs == 14, format!("[{path_name}] unary queue, server status 14: completion status {rc}, grpc_status {gs}"));
        let (rc, gs) = q_call(&cl, grid::FETCH, None);
        check(rc == AK_OK && gs == 0, format!("[{path_name}] unary queue OK: completion status {rc}, grpc_status {gs}"));
        // ak_call_unary_enc with a chosen status (the context from the harness binding)
        {
            let ctx = harness::arms::core_ffi_arm::Ctx::new();
            let v = harness::arms_m2::armonik_arm::value(harness::arms_m2::P2_2);
            use campaign::Ops;
            campaign::generated::roots::R_ListTasksDetailedResponse::f_encode(&ctx, &v, false).expect("encode");
            let path = p("StatusU9");
            let mut out = ak_bytes::default();
            let mut gs = -99;
            let rc = unsafe { ak_call_unary_enc(cl.c, path.as_ptr(), path.len(), ctx.enc, &mut out, &mut gs) };
            unsafe { ak_bytes_free(&mut out) };
            check(rc == AK_ERR_RPC_STATUS && gs == 9, format!("[{path_name}] ak_call_unary_enc, server status 9: rc {rc}, grpc_status {gs}"));
        }
        let (rcs, rc, gs, _) = cl.stream(&p("StatusS6"), None, &[b"x"]);
        check(rcs.iter().all(|r| *r == AK_OK || *r == AK_ERR_HOST) && rc == AK_ERR_RPC_STATUS && gs == 6,
              format!("[{path_name}] stream, server status 6: sends {rcs:?}, recv {rc}, grpc_status {gs}"));
        // deadline
        let o = ak_call_opts { deadline_ms: 300, metadata: std::ptr::null(), n_metadata: 0 };
        let t0 = std::time::Instant::now();
        let (_, rc, gs, _) = cl.stream(&p("SleepS"), Some(&o), &[b"x"]);
        let ms = t0.elapsed().as_millis();
        check(rc == AK_ERR_RPC_STATUS && gs == 4 && ms < 2500,
              format!("[{path_name}] stream deadline 300 ms, server sleeps 3 s: recv {rc}, grpc_status {gs} (DEADLINE_EXCEEDED = 4) after {ms} ms"));
        // metadata round trip, and grpc-timeout on the wire
        let kv = [
            ak_kv { key: b"ak-echo".as_ptr(), key_len: 7, val: b"hello-ascii".as_ptr(), val_len: 11 },
            ak_kv { key: b"ak-echo-bin".as_ptr(), key_len: 11, val: [0u8, 255, 1, 10].as_ptr(), val_len: 4 },
        ];
        let o = ak_call_opts { deadline_ms: 5000, metadata: kv.as_ptr(), n_metadata: 2 };
        *CAPTURE.lock().unwrap() = Some(Vec::new());
        let (_, rc, gs, v) = cl.stream(&p("EchoS"), Some(&o), &[b"x"]);
        let reqs = CAPTURE.lock().unwrap().take().unwrap_or_default();
        let want: Vec<u8> = [&b"hello-ascii|"[..], &[0u8, 255, 1, 10][..]].concat();
        let timeout_hdr = reqs.first().and_then(|r| r.iter().find(|l| l.starts_with("grpc-timeout: ")).cloned());
        check(rc == AK_OK && gs == 0 && v == want && timeout_hdr.is_some(),
              format!("[{path_name}] stream metadata echoed (ascii + -bin): recv {rc}, grpc_status {gs}, response {:?}; {}", String::from_utf8_lossy(&v),
                      timeout_hdr.unwrap_or_else(|| "NO grpc-timeout header".into())));
        // invalid metadata refuses the open
        let badkv = [ak_kv { key: b"ak-echo".as_ptr(), key_len: 7, val: "caf\u{e9}".as_ptr(), val_len: 5 }];
        let o = ak_call_opts { deadline_ms: 0, metadata: badkv.as_ptr(), n_metadata: 1 };
        let path = p("EchoS");
        let h = unsafe { ak_call_open(cl.c, path.as_ptr(), path.len(), AK_CALL_CLIENT_STREAM, &o) };
        check(h.is_null(), format!("[{path_name}] a non-ASCII value under an ASCII key: ak_call_open NULL"));
        // cancel: a stream's pending recv, a callback call and a queue call
        unsafe {
            let path = p("SleepS");
            let h = ak_call_open(cl.c, path.as_ptr(), path.len(), AK_CALL_CLIENT_STREAM, std::ptr::null());
            let hv = h as usize;
            let t = std::thread::spawn(move || {
                let mut out = ak_bytes::default();
                let mut gs = -99;
                let rc = ak_call_recv(hv as *mut ak_call, &mut out, &mut gs);
                (rc, gs)
            });
            std::thread::sleep(std::time::Duration::from_millis(200));
            let waiting = !t.is_finished();
            ak_call_cancel(h);
            let (rc, gs) = t.join().unwrap();
            let rc2 = ak_call_send(h, b"x".as_ptr(), 1, 1);
            ak_call_destroy(h);
            check(waiting && rc == AK_ERR_RPC_STATUS && gs == 1 && rc2 == AK_ERR_HOST,
                  format!("[{path_name}] cancel unblocks a stream's pending recv: recv {rc}, grpc_status {gs} (CANCELLED = 1); a send after it {rc2}"));
        }
        let (rc, gs) = cb_call(&cl, &p("SleepU"), Some(std::time::Duration::from_millis(200)));
        check(rc == AK_ERR_RPC_STATUS && gs == 1, format!("[{path_name}] cancel a callback call: completion status {rc}, grpc_status {gs}"));
        let (rc, gs) = q_call(&cl, &p("SleepU"), Some(std::time::Duration::from_millis(200)));
        check(rc == AK_ERR_RPC_STATUS && gs == 1, format!("[{path_name}] cancel a queue call: completion status {rc}, grpc_status {gs}"));
        // misuse
        let (rcs, rc, gs, _) = cl.stream(&p("EchoS"), None, &[b"x"]);
        let _ = (rcs, rc, gs);
        unsafe {
            let path = p("EchoS");
            let h = ak_call_open(cl.c, path.as_ptr(), path.len(), AK_CALL_CLIENT_STREAM, std::ptr::null());
            let a = ak_call_send(h, b"x".as_ptr(), 1, 1);
            let b = ak_call_send(h, b"x".as_ptr(), 1, 1);
            let mut out = ak_bytes::default();
            let mut gs = -99;
            let r1 = ak_call_recv(h, &mut out, &mut gs);
            ak_bytes_free(&mut out);
            let mut gs2 = -99;
            let r2 = ak_call_recv(h, &mut out, &mut gs2);
            ak_call_destroy(h);
            check(a == AK_OK && b == AK_ERR_INVALID_STATE && r1 == AK_OK && gs == 0 && r2 == AK_ERR_INVALID_STATE && gs2 == -99,
                  format!("[{path_name}] misuse: send after last {b}, second recv {r2} with grpc_status untouched ({gs2})"));
            let r = [AK_CALL_SERVER_STREAM, AK_CALL_BIDI_STREAM, 0].map(|k| ak_call_open(cl.c, path.as_ptr(), path.len(), k, std::ptr::null()).is_null());
            check(r.iter().all(|x| *x), format!("[{path_name}] reserved kinds 2, 3 and kind 0 open nothing"));
        }
        drop(cl);
        // limits: send
        let cl = Client::new(&target, framed, 1024, 0);
        *CAPTURE.lock().unwrap() = Some(Vec::new());
        let (rc, gs, _) = cl.unary(grid::PUSH, &vec![0u8; 2048]);
        let seen = CAPTURE.lock().unwrap().take().map_or(0, |v| v.len());
        check(rc == AK_ERR_LIMIT && gs == -1 && seen == 0,
              format!("[{path_name}] send limit 1024, unary request 2048 B: rc {rc}, grpc_status {gs}, requests the server saw {seen}"));
        let small = m5(100);
        let (rcs, rc, gs, v) = cl.stream(grid::STREAM, None, &[&m5(2000), &small]);
        let got = if v.len() >= 8 { u64::from_le_bytes(v[..8].try_into().unwrap()) } else { u64::MAX };
        check(rcs == vec![AK_ERR_LIMIT, AK_OK] && rc == AK_OK && gs == 0 && got == 100,
              format!("[{path_name}] send limit 1024, stream message {} B then {} B: sends {rcs:?}, recv {rc}, the server received {got} data bytes (only the small one)",
                      m5(2000).len(), small.len()));
        drop(cl);
        // limits: receive
        let cl = Client::new(&target, framed, 0, 1024);
        let (rc, gs, _) = cl.unary(grid::FETCH, &[]);
        check(rc == AK_ERR_LIMIT && gs == 8, format!("[{path_name}] receive limit 1024, unary response {p22} B: rc {rc}, grpc_status {gs}"));
        let (rc, gs) = q_call(&cl, grid::FETCH, None);
        check(rc == AK_ERR_LIMIT && gs == 8, format!("[{path_name}] receive limit 1024, queue completion: status {rc}, grpc_status {gs}"));
        drop(cl);
        let cl = Client::new(&target, framed, 0, 16);
        let (_, rc, gs, _) = cl.stream(grid::STREAM_CHECK, None, &[&small]);
        check(rc == AK_ERR_RPC_STATUS && gs == 8,
              format!("[{path_name}] receive limit 16, stream response 40 B: recv {rc}, grpc_status {gs} (RESOURCE_EXHAUSTED = 8)"));
    }
    let _ = std::fs::remove_dir_all(&dir);
    println!("{}", if bad == 0 { "RPC SEMANTICS PASSED" } else { "RPC SEMANTICS FAILED" });
    std::process::exit(if bad == 0 { 0 } else { 1 });
}
