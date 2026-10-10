//! CAMPAIGN.md requirement 19 (as amended 2026-09-26, R-H31): boundary-call counts from a
//! COUNTING build (`--features count`). Machine-independent: the runner and the gate compare
//! the output with the committed `gen/crossings.txt` (or `gen/crossings-nounk.txt`) and
//! stop on any difference.
//!
//! Codec rows: every core-ffi case the codec suite times -- every input (the 16 payloads,
//! the content sets, the `U-*` rows), encode, the target's decode (`decode`, `decode-read`:
//! the FSM family since FIX-PLAN D24) and the labelled extras' decodes (`decode-push`,
//! `decode-pull`), per unknown-field mode. RPC rows: one call of cells B, C, D and E per direction and mode, over an
//! in-process server on a Unix socket.
//!
//! forward = every exported entry point the loop calls: the ones the core counts in its
//! contexts (and, for RPC, `ak_call_unary` / `ak_bytes_free` in its RPC counters) PLUS the
//! plain exports the binding tallies (`ak_enc_reset`, `ak_dec_reset_<Root>`, `ak_enc_take`,
//! `ak_dec_err`). `resets` = how many of the forward calls are resets: since FIX-PLAN D27 the
//! core resets each context in the first call of an operation, so the binding makes none per
//! operation (`ak_dec_reset_<Root>` only when the options pointer changes, a retain decode after
//! a drop one or the reverse; each count is taken warm, so none) and the column is 0.
//! reverse = every reverse call. Retain mode runs with no pre-placed buffer and the
//! binding's `unk_grow`, which grows GEOMETRICALLY (max(want, 2 x capacity, 64), capped at
//! INT32_MAX; optimisation U2, owner decision: geometric in every build, the counting build
//! included), so its grow crossings are fixed for a given input.
//! Each count is taken on a warm context (the case run once before, as the timed loop is).

use ak_abi::*;
use campaign::grid::{self, Conn};
use campaign::*;
use harness::arms::core_ffi_arm::Ctx;
use harness::generated::binding::host_calls_take;

struct Count<'a> {
    ctx: &'a Ctx,
    inp: &'a Input,
    out: &'a mut Vec<String>,
}

fn enc(ctx: &Ctx) -> (u64, u64) {
    let mut c = AkCounters::default();
    unsafe { ak_enc_counters(ctx.enc, &mut c) };
    (c.forward, c.reverse)
}
fn dec(d: *mut ak_dec_ctx) -> (u64, u64) {
    let mut c = AkCounters::default();
    unsafe { ak_dec_counters(d, &mut c) };
    (c.forward, c.reverse)
}

fn row(input: &str, dir: &str, mode: &str, (f, r): (u64, u64), (resets, other): (u64, u64)) -> String {
    format!("{:<48} {:<12} {:<10} {:>8} {:>8} {:>6}", input, dir, mode, f + resets + other, r, resets)
}

impl Visit for Count<'_> {
    fn visit<R: Ops>(&mut self) {
        let (c, w) = (self.ctx, &self.inp.bytes[..]);
        for &(m, retain) in MODES {
            if self.inp.encode {
                let v = R::n_decode(w, retain && self.inp.unknown_row).expect("value");
                R::f_encode(c, &v, retain).expect("encode");
                unsafe { ak_enc_counters_reset(c.enc) };
                host_calls_take();
                R::f_encode(c, &v, retain).expect("encode");
                self.out.push(row(&self.inp.id, "encode", m, enc(c), host_calls_take()));
            }
            // D24: core-ffi's decode is the FSM family (begin + next per event, no reverse call).
            let mut toks = Vec::new();
            R::f_fsm(c, w, retain, &mut toks).expect("decode (fsm)");
            unsafe { ak_dec_counters_reset(R::dec_ctx(c)) };
            host_calls_take();
            R::f_fsm(c, w, retain, &mut toks).expect("decode (fsm)");
            self.out.push(row(&self.inp.id, "decode", m, dec(R::dec_ctx(c)), host_calls_take()));
            // decode-read (the core grid's decode direction, CAMPAIGN 4.0): the same decode, then
            // every field read on the host's value, counted over both.
            unsafe { ak_dec_counters_reset(R::dec_ctx(c)) };
            host_calls_take();
            let v = R::f_fsm(c, w, retain, &mut toks).expect("decode (fsm)");
            std::hint::black_box(R::touch_f(&v));
            self.out.push(row(&self.inp.id, "decode-read", m, dec(R::dec_ctx(c)), host_calls_take()));
            // core-ffi-push (labelled extra, D24): the push family; its decode-read reads the
            // host's value only, so it crosses exactly as its decode.
            R::f_decode(c, w, retain).expect("decode (push)");
            unsafe { ak_dec_counters_reset(R::dec_ctx(c)) };
            host_calls_take();
            R::f_decode(c, w, retain).expect("decode (push)");
            self.out.push(row(&self.inp.id, "decode-push", m, dec(R::dec_ctx(c)), host_calls_take()));
            R::f_pull(c, w, retain, &mut toks).expect("pull");
            unsafe { ak_dec_counters_reset(R::dec_ctx(c)) };
            host_calls_take();
            R::f_pull(c, w, retain, &mut toks).expect("pull");
            self.out.push(row(&self.inp.id, "decode-pull", m, dec(R::dec_ctx(c)), host_calls_take()));
        }
    }
}

fn rpc_counters() -> (u64, u64) {
    let mut c = ak_rpc_counters { forward: 0, reverse: 0 };
    unsafe { ak_abi::ak_rpc_counters(&mut c) };
    (c.forward, c.reverse)
}

/// Per-call counts of cells B, C, D and E (requirement 19 as amended): one warm call, then
/// one counted call, per direction; transport crossings from the core's RPC counters, codec
/// crossings from the slot's contexts, the binding's plain exports from its tally.
fn rpc_rows(out: &mut Vec<String>) {
    assert_eq!(unsafe { ak_rpc_counting() }, 1, "the core does not count RPC crossings (--features count)");
    let dir = std::env::temp_dir().join(format!("ak-crossings-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let sock = dir.join("grid.sock");
    let _server = campaign::server::spawn_in_process(sock.clone(), false);
    let target = format!("unix:{}", sock.display());
    let want_a = campaign::server::p22_response().len() as u64;
    for cell in grid::CELLS.iter().filter(|c| matches!(grid::base(c), 'B' | 'C' | 'D' | 'E')) {
        let conn = Conn::open(cell, &target, false);
        let mode = grid::mode_of(cell).unwrap_or("default");
        for d in ["a", "a+read", "b"] {
            let sl = grid::slots(1);
            let call = grid::call_of(cell, &conn, d, sl, want_a);
            call.once(0).expect("warm call");
            unsafe {
                ak_rpc_counters_reset();
                ak_enc_counters_reset(sl[0].ctx.enc);
                ak_dec_counters_reset(sl[0].ctx.dec.list_tasks_detailed_response);
            }
            host_calls_take();
            call.once(0).expect("counted call");
            let (rf, rr) = rpc_counters();
            let (ef, er) = enc(&sl[0].ctx);
            let (df, dr) = dec(sl[0].ctx.dec.list_tasks_detailed_response);
            out.push(row(&format!("rpc:{}", grid::stem(cell)), d, mode, (rf + ef + df, rr + er + dr), host_calls_take()));
        }
        // U1-unary: direction c, per payload (M5 encode + Upload; nothing decoded).
        for pid in grid::C_PAYLOADS {
            let sl = grid::slots(1);
            let call = grid::call_of_c(cell, &conn, pid, sl, 0);
            call.once(0).expect("warm call");
            unsafe {
                ak_rpc_counters_reset();
                ak_enc_counters_reset(sl[0].ctx.enc);
            }
            host_calls_take();
            call.once(0).expect("counted call");
            let (rf, rr) = rpc_counters();
            let (ef, er) = enc(&sl[0].ctx);
            out.push(row(&format!("rpc:{}", grid::stem(cell)), &format!("c/{pid}"), mode, (rf + ef, rr + er), host_calls_take()));
        }
        // U2-stream: direction d, per size (open, a send per chunk, recv, free, destroy; the
        // encodes of C and D; nothing decoded).
        for &(label, chunks) in grid::D_PAYLOADS {
            let sl = grid::slots(1);
            let call = grid::call_of_d(cell, &conn, chunks, sl, (chunks * grid::CHUNK) as u64, false);
            call.once(0).expect("warm call");
            unsafe {
                ak_rpc_counters_reset();
                ak_enc_counters_reset(sl[0].ctx.enc);
            }
            host_calls_take();
            call.once(0).expect("counted call");
            let (rf, rr) = rpc_counters();
            let (ef, er) = enc(&sl[0].ctx);
            out.push(row(&format!("rpc:{}", grid::stem(cell)), &format!("d/{label}"), mode, (rf + ef, rr + er), host_calls_take()));
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

fn main() {
    let ctx = Ctx::new();
    let mut out = Vec::new();
    for inp in inputs(&[]) {
        campaign::generated::roots::with_root(&inp.root, &mut Count { ctx: &ctx, inp: &inp, out: &mut out });
    }
    // A build without counters reads zero everywhere: refuse rather than write zeros.
    assert!(out.iter().any(|l| !l.trim_end().ends_with("0        0      0")), "not a counting build (--features count)");
    rpc_rows(&mut out);
    println!("# CAMPAIGN req 19 (as amended 2026-09-26): forward = every exported entry point called (core-counted + the");
    println!("# binding's ak_enc_reset / ak_dec_reset_<Root> / ak_enc_take / ak_dec_err); resets = the resets among them (0 since D27, see below;");
    println!("# before it one ak_enc_reset per encode and one ak_dec_reset_<Root> per retain decode or pull). Retain: no pre-placed buffer,");
    println!("# geometric grow (unk_grow: max(want, 2 x capacity, 64), capped at INT32_MAX; U2, the owner's decision for every build).");
    println!("# decode, decode-read: core-ffi's decode, the FSM family since FIX-PLAN D24 (ak_fsm_begin_<Root> + one ak_fsm_next_<Root> per further");
    println!("# event, ak_dec_err after the last; no reverse call); decode-push: the push family (core-ffi-push, labelled extra; ak_decode_<Root> +");
    println!("# vtable reverse calls; its decode-read crosses as its decode); decode-pull: the pull family (core-ffi-pull, labelled extra).");
    println!("# decode-read: the decode, then every field of the host's value read (the core grid's decode direction, CAMPAIGN 4.0).");
    println!("# encode: the core-ffi encode, which is ALSO the op of end states reused-buffer and transport-ready-core (cell C's form: the context's");
    println!("# buffer is moved inside ak_call_unary_enc, counted in the rpc:C rows); core-native and incumbent-prod call no ak_* entry point.");
    println!("# rpc:<cell> rows: one call of cell B, C, D or E (P2.2; a = Fetch + decode, a+read = a + every field read, b = encode + Push), core RPC counters included;");
    println!("# C and D (and their framed and callback twins) decode the response with the FSM family (D24); B decodes with prost, E with core-native.");
    println!("# Bf, Cf, Df, Ef: the same cells on the framed send path (T1 option 3; ak_client_set_framed is called once at open, not per call).");
    println!("# c/P5.3, c/P5.4: U1-unary, one upload of M5 (encode + Upload; the response is empty and decoded by nobody).");
    println!("# d/4MiB, d/16MiB: U2-stream, one client-streamed upload in 2 MiB chunks (B/C/E: open, a send per chunk, recv, free, destroy).");
    println!("# The counts do not depend on the h2 variant (stock or h2-batch): h2 is inside the core, below every counted entry point.");
    println!("# B-cb, C-cb, E-cb (and Bf-cb, Cf-cb, Ef-cb): the callback cells (CAMPAIGN req 16 as amended, Rust's reference core cells): ak_call_unary_cb / ak_call_unary_enc_cb + ak_call_destroy + ak_bytes_free, one reverse (the completion); d: ak_call_send_cb / _enc_cb per chunk and ak_call_recv_cb, one reverse per completion.");
    println!("# FIX-PLAN D27 (reset on entry): the core resets each context in the first call of an operation; the binding calls no ak_enc_reset, and ak_dec_reset_<Root> only to set or change the options pointer (counted warm: the warm call armed the context), so the resets column is 0 on every row.");
    println!("# input                                            direction    mode        forward  reverse resets");
    for l in out {
        println!("{l}");
    }
}
