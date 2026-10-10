//! FIX-PLAN D27 (owner, 2026-10-10): reset on entry, the core's behaviour. The checks, on the
//! codec suite's inputs (the 16 shapes, the content sets, every `U-*` row the shapes roots
//! carry), every unknown-field mode of this build (gate step 11h):
//!
//!   ABANDON  an FSM decode stopped before its end event (after begin only, and after half of
//!            its events), then a fresh FSM, push and pull decode on the SAME context: each
//!            equals a reference decode on a context of its own;
//!   ERR      a failed decode (the input less its last byte) in each family: the context's error
//!            slot (`ak_dec_err`) right after the call, after operations on other contexts, and
//!            after the next decode on this one (codec errors are returned, the slot reads 0);
//!   ENC      an encode's output read after the call (`ak_enc_take`'s bytes) equals core-native's,
//!            still after a decode and an encode on other contexts, and the next encode on this
//!            context (another value, then the first again) writes exactly its own bytes;
//!   HOSTERR  (full build) a decode the HOST fails (every unknown-field grow calls `ak_fail`):
//!            the slot holds the host's code after the call and after operations on other
//!            contexts, and the next decode on the context clears it;
//!   REARM    (full build) every unknown-field position re-read on each decode entry, the host
//!            rewriting its options in place with no reset (`binding::roe_rearm_check_<root>`).
//!
//! The controls are gate step 11h's plants (the re-arm, or the encode reset, removed from the
//! core in a shadow tree): each must make this binary fail. Exit 1 on any failure.

use ak_abi::*;
use campaign::*;
use harness::arms::core_ffi_arm::Ctx;
use harness::generated::binding;

/// begin, then up to `steps` nexts, stopping before the end event. Returns (last rc, calls).
fn fsm_partial(root: &str, ctx: *mut ak_dec_ctx, b: &[u8], steps: usize) -> (i32, usize) {
    macro_rules! go {
        ($begin:ident, $next:ident) => {{
            unsafe {
                let mut ev = ak_fsm_ev::default();
                let mut rc = $begin(ctx, b.as_ptr(), b.len(), &mut ev);
                let mut n = 1usize;
                while rc > 0 && rc != AK_BDR_APPLY as i32 && n <= steps {
                    rc = $next(ctx, &mut ev);
                    n += 1;
                }
                (rc, n)
            }
        }};
    }
    match root {
        "ListResultsResponse" => go!(ak_fsm_begin_ListResultsResponse, ak_fsm_next_ListResultsResponse),
        "ListTasksDetailedResponse" => go!(ak_fsm_begin_ListTasksDetailedResponse, ak_fsm_next_ListTasksDetailedResponse),
        "ListProbeResponse" => go!(ak_fsm_begin_ListProbeResponse, ak_fsm_next_ListProbeResponse),
        "ListTaskSummaryResponse" => go!(ak_fsm_begin_ListTaskSummaryResponse, ak_fsm_next_ListTaskSummaryResponse),
        "UploadResultDataMessage" => go!(ak_fsm_begin_UploadResultDataMessage, ak_fsm_next_UploadResultDataMessage),
        "ListMetricsResponse" => go!(ak_fsm_begin_ListMetricsResponse, ak_fsm_next_ListMetricsResponse),
        "DualResponse" => go!(ak_fsm_begin_DualResponse, ak_fsm_next_DualResponse),
        r => panic!("root {r}"),
    }
}

#[cfg(feature = "unknown-fields")]
fn fail_decode(root: &str, c: binding::DecCtxs, b: &[u8], fam: u32) -> i32 {
    match root {
        "ListResultsResponse" => binding::roe_fail_decode_list_results_response(c, b, fam),
        "ListTasksDetailedResponse" => binding::roe_fail_decode_list_tasks_detailed_response(c, b, fam),
        "ListProbeResponse" => binding::roe_fail_decode_list_probe_response(c, b, fam),
        "ListTaskSummaryResponse" => binding::roe_fail_decode_list_task_summary_response(c, b, fam),
        "UploadResultDataMessage" => binding::roe_fail_decode_upload_result_data_message(c, b, fam),
        "ListMetricsResponse" => binding::roe_fail_decode_list_metrics_response(c, b, fam),
        "DualResponse" => binding::roe_fail_decode_dual_response(c, b, fam),
        r => panic!("root {r}"),
    }
}

#[cfg(feature = "unknown-fields")]
fn rearm(root: &str, c: binding::DecCtxs, b: &[u8]) -> Result<(usize, usize), String> {
    match root {
        "ListResultsResponse" => binding::roe_rearm_check_list_results_response(c, b),
        "ListTasksDetailedResponse" => binding::roe_rearm_check_list_tasks_detailed_response(c, b),
        "ListProbeResponse" => binding::roe_rearm_check_list_probe_response(c, b),
        "ListTaskSummaryResponse" => binding::roe_rearm_check_list_task_summary_response(c, b),
        "UploadResultDataMessage" => binding::roe_rearm_check_upload_result_data_message(c, b),
        "ListMetricsResponse" => binding::roe_rearm_check_list_metrics_response(c, b),
        "DualResponse" => binding::roe_rearm_check_dual_response(c, b),
        r => panic!("root {r}"),
    }
}

#[derive(Default)]
struct Tally {
    checks: usize,
    fails: Vec<String>,
    lines: Vec<String>,
    abandoned: usize,
    no_abandon: usize,
    err_cases: usize,
    enc_cases: usize,
    rearm_cmp: usize,
    hosterr: usize,
    rearm_bite: usize,
    rearm_inputs: usize,
}

struct V<'a> {
    inp: &'a Input,
    t: &'a mut Tally,
}

impl V<'_> {
    fn chk(&mut self, ok: bool, what: String) {
        self.t.checks += 1;
        if !ok {
            self.t.fails.push(format!("{} {}", self.inp.id, what));
        }
    }
}

impl Visit for V<'_> {
    fn visit<R: Ops>(&mut self) {
        let w = &self.inp.bytes[..];
        let id = self.inp.id.clone();
        for &(m, retain) in MODES {
            // Contexts of their own for this (input, mode): the one under test, a reference,
            // and an "other" one for the operations in between.
            let (c, refc, other) = (Ctx::new(), Ctx::new(), Ctx::new());
            let mut toks = Vec::new();
            let reference = R::f_decode(&refc, w, retain);
            let want = format!("{:?}", reference);

            // ---- ABANDON: begin only, and half the events, then fresh decodes.
            let full = R::f_fsm(&c, w, retain, &mut toks); // warm (and armed, in retain)
            self.chk(format!("{:?}", full) == want, format!("{m} fsm == push before any abandon"));
            let (_, events) = fsm_partial(R::ROOT, R::dec_ctx(&c), w, usize::MAX);
            if events < 2 {
                self.t.no_abandon += 1;
            }
            for steps in [0usize, events / 2] {
                if events < 2 || steps + 1 >= events {
                    continue;
                }
                let (rc, calls) = fsm_partial(R::ROOT, R::dec_ctx(&c), w, steps);
                let stopped_early = rc > 0 && rc != AK_BDR_APPLY as i32;
                let a = format!("{:?}", R::f_fsm(&c, w, retain, &mut toks));
                let (rc2, _) = fsm_partial(R::ROOT, R::dec_ctx(&c), w, steps);
                let p = format!("{:?}", R::f_decode(&c, w, retain));
                let (rc3, _) = fsm_partial(R::ROOT, R::dec_ctx(&c), w, steps);
                let l = format!("{:?}", R::f_pull(&c, w, retain, &mut toks));
                let ok = stopped_early && rc2 > 0 && rc3 > 0 && a == want && p == want && l == want;
                self.chk(ok, format!("{m} abandoned after {calls} call(s): fsm/push/pull afterwards equal the reference"));
                self.t.abandoned += 1;
                self.t.lines.push(format!("ABANDON {id} {m} calls {calls} of {events}: fsm {} push {} pull {}",
                    a == want, p == want, l == want));
            }

            // ---- ERR: a failed decode's detail, after the call, after other operations, after
            // the next decode.
            if w.len() > 1 {
                let tr = &w[..w.len() - 1];
                for fam in ["push", "pull", "fsm"] {
                    let r = match fam {
                        "push" => R::f_decode(&c, tr, retain),
                        "pull" => R::f_pull(&c, tr, retain, &mut toks),
                        _ => R::f_fsm(&c, tr, retain, &mut toks),
                    };
                    let rc = match &r { Err(e) => *e, Ok(_) => 0 };
                    if rc == 0 {
                        continue;
                    }
                    let e1 = unsafe { ak_dec_err(R::dec_ctx(&c)) };
                    // Operations on OTHER contexts: a decode and, where the input encodes, an encode.
                    let v = R::f_decode(&other, w, retain);
                    if let (true, Ok(v)) = (self.inp.encode, &v) {
                        let _ = R::f_encode(&other, v, retain);
                    }
                    let e2 = unsafe { ak_dec_err(R::dec_ctx(&c)) };
                    let ok_next = R::f_fsm(&c, w, retain, &mut toks).is_ok();
                    let e3 = unsafe { ak_dec_err(R::dec_ctx(&c)) };
                    self.chk(e1 == e2 && ok_next && e3 == AK_OK,
                        format!("{m} {fam}: error detail {e1} after the call, {e2} after other operations, {e3} after the next decode"));
                    self.t.err_cases += 1;
                    self.t.lines.push(format!("ERR {id} {m} {fam}: rc {rc} detail after call {e1}, after other operations {e2}, after the next decode {e3}"));
                }
            }

            // ---- ENC: the output readable after the call, until the next encode on it.
            if self.inp.encode {
                if let Ok(v) = R::n_decode(w, retain && self.inp.unknown_row) {
                    let native = |x: &R::F| {
                        let mut e = ak_rt::Enc::new(facade::generated::core_native::SITES);
                        R::n_encode(x, &mut e, retain);
                        e.buf.to_vec()
                    };
                    let (want_v, want_d) = (native(&v), native(&R::F::default()));
                    let n = R::f_encode(&c, &v, retain);
                    let a = unsafe { binding::encoded(c.enc) }.to_vec();
                    let _ = R::f_fsm(&c, w, retain, &mut toks);
                    let _ = R::f_encode(&other, &v, retain);
                    let b = unsafe { binding::encoded(c.enc) }.to_vec();
                    let _ = R::f_encode(&c, &R::F::default(), retain);
                    let d = unsafe { binding::encoded(c.enc) }.to_vec();
                    let _ = R::f_encode(&c, &v, retain);
                    let e = unsafe { binding::encoded(c.enc) }.to_vec();
                    let ok = n.is_ok() && a == want_v && b == want_v && d == want_d && e == want_v;
                    self.chk(ok, format!("{m} encode output: after the call {}, after other operations {}, next encode {} {}",
                        a == want_v, b == want_v, d == want_d, e == want_v));
                    self.t.enc_cases += 1;
                    self.t.lines.push(format!("ENC {id} {m}: {} B; after the call {}, after other operations {}, next encode (default value) {}, then the value again {}",
                        a.len(), a == want_v, b == want_v, d == want_d, e == want_v));
                }
            }

            // ---- HOSTERR (full build): the host's error on the sticky slot.
            #[cfg(feature = "unknown-fields")]
            if retain {
                for fam in 0..3u32 {
                    let h = Ctx::new();
                    let rc = fail_decode(R::ROOT, h.dec, w, fam);
                    if rc == 0 {
                        continue; // no grow asked: the input has no unknown field
                    }
                    let e1 = unsafe { ak_dec_err(R::dec_ctx(&h)) };
                    let v = R::f_decode(&other, w, retain);
                    if let (true, Ok(v)) = (self.inp.encode, &v) {
                        let _ = R::f_encode(&other, v, retain);
                    }
                    let e2 = unsafe { ak_dec_err(R::dec_ctx(&h)) };
                    let ok_next = R::f_decode(&h, w, false).is_ok();
                    let e3 = unsafe { ak_dec_err(R::dec_ctx(&h)) };
                    let fam_s = ["push", "pull", "fsm"][fam as usize];
                    self.chk(rc == binding::ROE_HOST_FAIL && e1 == rc && e2 == rc && ok_next && e3 == AK_OK,
                        format!("HOSTERR {fam_s}: rc {rc}, detail {e1} after the call, {e2} after other operations, {e3} after the next decode"));
                    self.t.hosterr += 1;
                    self.t.lines.push(format!("HOSTERR {id} {fam_s}: rc {rc}, detail after call {e1}, after other operations {e2}, after the next decode {e3}"));
                }
            }

            // ---- REARM (full build): every position re-read on each decode entry.
            #[cfg(feature = "unknown-fields")]
            if retain {
                let fresh = Ctx::new();
                self.t.checks += 1;
                match rearm(R::ROOT, fresh.dec, w) {
                    Ok((n, bite)) => {
                        self.t.rearm_cmp += n;
                        self.t.rearm_bite += bite;
                        self.t.rearm_inputs += 1;
                    }
                    Err(e) => self.t.fails.push(format!("{id} REARM {e}")),
                }
            }
        }
    }
}

fn main() {
    let mut t = Tally::default();
    for inp in inputs(&[]) {
        campaign::generated::roots::with_root(&inp.root, &mut V { inp: &inp, t: &mut t });
    }
    println!("# D27 reset-on-entry checks, build {}", if cfg!(feature = "unknown-fields") { "full (drop, retain)" } else { "no-unknown" });
    for l in &t.lines {
        println!("{l}");
    }
    println!("# ABANDON: {} abandoned decodes followed by fresh fsm, push and pull decodes; {} (input, mode) pairs with a single event (nothing to abandon)", t.abandoned, t.no_abandon);
    println!("# ERR: {} failed decodes (input less its last byte, three families)", t.err_cases);
    println!("# ENC: {} encodes read after the call", t.enc_cases);
    if cfg!(feature = "unknown-fields") {
        println!("# HOSTERR: {} decodes failed by the host (inputs with an unknown field, three families)", t.hosterr);
        println!("# REARM: {} inputs, {} comparisons (5 per family and position), {} (input, family, position) cases where zeroing the position changes the value", t.rearm_inputs, t.rearm_cmp, t.rearm_bite);
    }
    println!("# {} checks, {} failures", t.checks, t.fails.len());
    for f in &t.fails {
        println!("FAIL {f}");
    }
    if t.fails.is_empty() {
        println!("ROE CHECKS PASSED");
    } else {
        std::process::exit(1);
    }
}
