//! CAMPAIGN.md requirement 19 (as amended 2026-09-26, R-H31): boundary-call counts from a
//! COUNTING build (`--features count`). Machine-independent: the runner and the gate compare
//! the output with the committed `gen/crossings.txt` (or `gen/crossings-nounk.txt`) and
//! stop on any difference.
//!
//! Codec rows: every core-ffi case the codec suite times -- every input (the 16 payloads,
//! the content sets, the `U-*` rows), encode and push and pull decode, per unknown-field
//! mode. RPC rows: one call of cells B, C, D and E per direction and mode, over an
//! in-process server on a Unix socket.
//!
//! forward = every exported entry point the loop calls: the ones the core counts in its
//! contexts (and, for RPC, `ak_call_unary` / `ak_bytes_free` in its RPC counters) PLUS the
//! plain exports the binding tallies (`ak_enc_reset`, `ak_dec_reset_<Root>`, `ak_enc_take`,
//! `ak_dec_err`). `resets` = how many of the forward calls are resets: one `ak_enc_reset`
//! before every encode; ONE `ak_dec_reset_<Root>` before a retain decode or pull (arming the
//! options at their stable address; the binding leaves the context armed, optimisation U1),
//! and one before a drop decode only when a retaining decode left the context armed (the
//! counted drop call runs after a warm drop call, so it is not armed and counts none).
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
            R::f_decode(c, w, retain).expect("decode");
            unsafe { ak_dec_counters_reset(R::dec_ctx(c)) };
            host_calls_take();
            R::f_decode(c, w, retain).expect("decode");
            self.out.push(row(&self.inp.id, "decode", m, dec(R::dec_ctx(c)), host_calls_take()));
            let mut toks = Vec::new();
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
        let mode = cell.split_once('-').map(|x| x.1).unwrap_or("default");
        for d in ["a", "b"] {
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
    // Optimisation N5: the fallback inputs' decode counts (new is called for the element
    // that falls back, and only for it), after the same equality check the pre-check makes.
    let (_, f) = campaign::n5_checks(&ctx);
    assert!(f.is_empty(), "N5 fallback: {f:?}");
    for (id, wire) in campaign::n5_fallback_inputs() {
        for &(mname, retain) in campaign::MODES {
            use campaign::generated::roots::R_ListTasksDetailedResponse as M2;
            use campaign::Ops;
            let d = M2::dec_ctx(&ctx);
            host_calls_take();
            unsafe { ak_dec_counters_reset(d) };
            M2::f_decode(&ctx, &wire, retain).expect("N5 fallback input decodes");
            out.push(row(id, "decode", mname, dec(d), host_calls_take()));
        }
    }
    rpc_rows(&mut out);
    println!("# CAMPAIGN req 19 (as amended 2026-09-26): forward = every exported entry point called (core-counted + the");
    println!("# binding's ak_enc_reset / ak_dec_reset_<Root> / ak_enc_take / ak_dec_err); resets = ak_enc_reset before each");
    println!("# encode + one ak_dec_reset_<Root> before each retain decode or pull (the context stays armed, U1). Retain: no pre-placed buffer,");
    println!("# geometric grow (unk_grow: max(want, 2 x capacity, 64), capped at INT32_MAX; U2, the owner's decision for every build).");
    println!("# rpc:<cell> rows: one call of cell B, C, D or E (P2.2; a = Fetch + decode, b = encode + Push), core RPC counters included;");
    println!("# Bf, Cf, Df, Ef: the same cells on the framed send path (T1 option 3; ak_client_set_framed is called once at open, not per call).");
    println!("# c/P5.3, c/P5.4: U1-unary, one upload of M5 (encode + Upload; the response is empty and decoded by nobody).");
    println!("# Push decode (ak_decode_<R>_af, optimisation N5 apply-first): no new_<slot> for an element whose runs fit; N5-arena, N5-held: the two fallback inputs.");
    println!("# input                                            direction    mode        forward  reverse resets");
    for l in out {
        println!("{l}");
    }
}
