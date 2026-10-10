//! reset-on-entry (owner, 2026-10-10): the measurement. MEASUREMENT ONLY; container
//! instrumentation; absolute times only.
//!
//! One process, built with `reset-on-entry` (the variant core and its binding) and the
//! default binding beside it (`harness::generated::binding_explicit`). Both paths run on the
//! SAME core: the explicit arm's contexts have the core's reset-on-entry switched off
//! (`ak_measure_{enc,dec}_set_roe(ctx, 0)`) and its binding resets as the default build does
//! (`ak_enc_reset` before each encode, `ak_dec_reset_<Root>` before each retained decode); the
//! roe arm's contexts reset on entry and its binding makes no such call. What the explicit arm
//! pays beyond the default build is one predictable branch per entry (the switch).
//!
//! Per input and mode (drop, retain), arms interleaved round by round (order rotated per
//! round), process CPU per op, median and p10..p90 of the per-round values:
//!   enc-rb   core-ffi encode, end state reused-buffer (also transport-ready-core's op)
//!   enc-tt   core-ffi encode, end state transport-ready-tonic (+ ak_enc_take_owned, dropped)
//!   dec-rd   FSM decode-read (binding fsm_with_<root>, every field read on the value)
//! each as `explicit` and `roe`. And in isolation: one `ak_enc_reset` (on a context that has
//! encoded the row), one `ak_dec_reset_<Root>` with the binding's stable options (retain).
//!
//! Inputs: the core grid's (16 shapes, P2.2 Latin-1 and wide, the seven U-* rows), or the
//! prefixes given.
//!
//!   roe_bench [--rounds R] [--round-ms MS] [PREFIX...]

#[cfg(not(all(feature = "reset-on-entry", feature = "unknown-fields")))]
fn main() {
    eprintln!("roe_bench needs the full build with --features reset-on-entry");
    std::process::exit(2);
}

#[cfg(all(feature = "reset-on-entry", feature = "unknown-fields"))]
mod imp {
    use ak_abi::*;
    use campaign::*;
    use harness::arms::core_ffi_arm::Ctx;
    use harness::generated::{binding, binding_explicit as bx};
    use std::any::Any;

    extern "C" {
        fn ak_measure_enc_set_roe(ctx: *mut ak_enc_ctx, on: i32);
        fn ak_measure_dec_set_roe(ctx: *mut ak_dec_ctx, on: i32);
    }

    /// The explicit-reset path's contexts: the default binding's, the core's switch off.
    pub struct Ex {
        pub enc: *mut ak_enc_ctx,
        pub dec: bx::DecCtxs,
        pub tcs: bx::Tcs,
    }
    impl Ex {
        fn new() -> Ex {
            unsafe {
                let enc = ak_enc_ctx_new();
                ak_measure_enc_set_roe(enc, 0);
                let dec = bx::DecCtxs::new();
                for c in [dec.list_results_response, dec.list_tasks_detailed_response, dec.list_probe_response,
                          dec.list_task_summary_response, dec.upload_result_data_message, dec.list_metrics_response,
                          dec.dual_response] {
                    ak_measure_dec_set_roe(c, 0);
                }
                Ex { enc, dec, tcs: bx::Tcs::trusted() }
            }
        }
    }

    type Op<'a> = Box<dyn FnMut() -> u64 + 'a>;

    pub struct Row<'a> {
        pub id: String,
        pub bytes: &'static [u8],
        pub encode: bool,
        pub unknown_row: bool,
        pub retain: bool,
        pub r: &'a Ctx,
        pub x: &'a Ex,
        pub arms: Vec<(String, Op<'a>)>,
    }

    pub struct Build<'a, 'b>(pub &'b mut Row<'a>);

    impl<'a, 'b> Visit for Build<'a, 'b> {
        fn visit<R: Ops>(&mut self) {
            let row = &mut *self.0;
            let (r, x, b, retain) = (row.r, row.x, row.bytes, row.retain);
            // Each root's concrete functions, both bindings.
            macro_rules! roots {
                ($($name:literal => $t:ty, $ez:ident, $euz:ident, $fw:ident, $fwu:ident;)*) => {
                    match R::ROOT {
                        $($name => {
                            let enc_x = move |v: &$t| if retain { bx::$euz(x.enc, v, &x.tcs) } else { bx::$ez(x.enc, v, &x.tcs) };
                            let enc_r = move |v: &$t| if retain { binding::$euz(r.enc, v, &r.tcs) } else { binding::$ez(r.enc, v, &r.tcs) };
                            let dec_x = move |t: &mut Vec<i64>| if retain { bx::$fwu(x.dec, b, t) } else { bx::$fw(x.dec, b, t) };
                            let dec_r = move |t: &mut Vec<i64>| if retain { binding::$fwu(r.dec, b, t) } else { binding::$fw(r.dec, b, t) };
                            // Correctness first: both paths agree, byte for byte and value for value.
                            let (mut t1, mut t2) = (Vec::new(), Vec::new());
                            let (vx, vr) = (dec_x(&mut t1), dec_r(&mut t2));
                            assert!(vx.is_ok() && vx == vr, "{}: the two decodes disagree", row.id);
                            let touch = |v: &$t| R::touch_f((v as &dyn Any).downcast_ref::<R::F>().unwrap());
                            row.arms.push(("dec-rd explicit".into(), Box::new(move || touch(&dec_x(&mut t1).unwrap()))));
                            row.arms.push(("dec-rd roe".into(), Box::new(move || touch(&dec_r(&mut t2).unwrap()))));
                            if row.encode {
                                let nv = R::n_decode(b, retain && row.unknown_row).expect("value");
                                let v: &'static $t = Box::leak(Box::new((&nv as &dyn Any).downcast_ref::<$t>().unwrap().clone()));
                                enc_x(v).expect("explicit encode");
                                let ax = unsafe { bx::encoded(x.enc) }.to_vec();
                                enc_r(v).expect("roe encode");
                                let ar = unsafe { binding::encoded(r.enc) }.to_vec();
                                assert!(ax == ar, "{}: the two encodes disagree", row.id);
                                row.arms.push(("enc-rb explicit".into(), Box::new(move || enc_x(v).unwrap() as u64)));
                                row.arms.push(("enc-rb roe".into(), Box::new(move || enc_r(v).unwrap() as u64)));
                                row.arms.push(("enc-tt explicit".into(), Box::new(move || {
                                    enc_x(v).unwrap();
                                    ffi_owned_body(x.enc).unwrap().len() as u64
                                })));
                                row.arms.push(("enc-tt roe".into(), Box::new(move || {
                                    enc_r(v).unwrap();
                                    ffi_owned_body(r.enc).unwrap().len() as u64
                                })));
                            }
                        })*
                        other => panic!("root {other}"),
                    }
                };
            }
            roots! {
                "ListResultsResponse" => facade::ListResultsResponse, encode_into_list_results_response_zeroed, encode_into_list_results_response_unk_zeroed, fsm_with_list_results_response, fsm_with_list_results_response_unk;
                "ListTasksDetailedResponse" => facade::ListTasksDetailedResponse, encode_into_list_tasks_detailed_response_zeroed, encode_into_list_tasks_detailed_response_unk_zeroed, fsm_with_list_tasks_detailed_response, fsm_with_list_tasks_detailed_response_unk;
                "ListProbeResponse" => facade::ListProbeResponse, encode_into_list_probe_response_zeroed, encode_into_list_probe_response_unk_zeroed, fsm_with_list_probe_response, fsm_with_list_probe_response_unk;
                "ListTaskSummaryResponse" => facade::ListTaskSummaryResponse, encode_into_list_task_summary_response_zeroed, encode_into_list_task_summary_response_unk_zeroed, fsm_with_list_task_summary_response, fsm_with_list_task_summary_response_unk;
                "UploadResultDataMessage" => facade::UploadResultDataMessage, encode_into_upload_result_data_message_zeroed, encode_into_upload_result_data_message_unk_zeroed, fsm_with_upload_result_data_message, fsm_with_upload_result_data_message_unk;
                "ListMetricsResponse" => facade::ListMetricsResponse, encode_into_list_metrics_response_zeroed, encode_into_list_metrics_response_unk_zeroed, fsm_with_list_metrics_response, fsm_with_list_metrics_response_unk;
                "DualResponse" => facade::DualResponse, encode_into_dual_response_zeroed, encode_into_dual_response_unk_zeroed, fsm_with_dual_response, fsm_with_dual_response_unk;
            }
        }
    }

    /// One `ak_dec_reset_<Root>` with the binding's options at a stable address (retain).
    fn dec_reset_op(root: &str, x: &'static Ex) -> Op<'static> {
        macro_rules! one {
            ($opts:ident, $reset:ident, $field:ident) => {{
                let o = Box::leak(Box::new(binding::$opts(None)));
                let c = x.dec.$field;
                Box::new(move || unsafe { $reset(c, &mut *o) as u64 })
            }};
        }
        match root {
            "ListResultsResponse" => one!(unk_opts_list_results_response, ak_dec_reset_ListResultsResponse, list_results_response),
            "ListTasksDetailedResponse" => one!(unk_opts_list_tasks_detailed_response, ak_dec_reset_ListTasksDetailedResponse, list_tasks_detailed_response),
            "ListProbeResponse" => one!(unk_opts_list_probe_response, ak_dec_reset_ListProbeResponse, list_probe_response),
            "ListTaskSummaryResponse" => one!(unk_opts_list_task_summary_response, ak_dec_reset_ListTaskSummaryResponse, list_task_summary_response),
            "UploadResultDataMessage" => one!(unk_opts_upload_result_data_message, ak_dec_reset_UploadResultDataMessage, upload_result_data_message),
            "ListMetricsResponse" => one!(unk_opts_list_metrics_response, ak_dec_reset_ListMetricsResponse, list_metrics_response),
            "DualResponse" => one!(unk_opts_dual_response, ak_dec_reset_DualResponse, dual_response),
            r => panic!("root {r}"),
        }
    }

    fn pct(v: &mut [f64], p: f64) -> f64 {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v[((v.len() - 1) as f64 * p).round() as usize]
    }

    fn fmt(ns: f64) -> String {
        if ns >= 1e6 { format!("{:.3} ms", ns / 1e6) } else if ns >= 1e3 { format!("{:.2} us", ns / 1e3) } else { format!("{:.1} ns", ns) }
    }

    /// Calibrate each arm to ~round_ms, then `rounds` rounds with the arm order rotated.
    fn run(id: &str, mode: &str, arms: &mut [(String, Op<'_>)], rounds: usize, round_ms: u64) {
        let mut iters = Vec::new();
        for (_, op) in arms.iter_mut() {
            let mut k = 1u64;
            loop {
                let c0 = process_clock_ns();
                for _ in 0..k { std::hint::black_box(op()); }
                let dt = process_clock_ns() - c0;
                if dt >= round_ms * 1_000_000 / 4 || k > 1 << 30 {
                    iters.push(((k as f64) * (round_ms as f64 * 1e6) / dt.max(1) as f64).max(1.0) as u64);
                    break;
                }
                k *= 2;
            }
        }
        let na = arms.len();
        let mut per: Vec<Vec<f64>> = vec![Vec::new(); na];
        for rr in 0..rounds {
            for j in 0..na {
                let a = (j + rr) % na;
                let k = iters[a];
                let op = &mut arms[a].1;
                let c0 = process_clock_ns();
                for _ in 0..k { std::hint::black_box(op()); }
                per[a].push((process_clock_ns() - c0) as f64 / k as f64);
            }
        }
        for (a, (name, _)) in arms.iter().enumerate() {
            let v = &mut per[a];
            println!("{:<46} {:<7} {:<16} {:>12} {:>12} {:>12}", id, mode, name, fmt(pct(v, 0.5)), fmt(pct(v, 0.1)), fmt(pct(v, 0.9)));
        }
    }

    pub fn main() {
        let a: Vec<String> = std::env::args().collect();
        let (mut rounds, mut round_ms) = (21usize, 20u64);
        let mut sel: Vec<String> = Vec::new();
        let mut i = 1;
        while i < a.len() {
            match a[i].as_str() {
                "--rounds" => { rounds = a[i + 1].parse().unwrap(); i += 1; }
                "--round-ms" => { round_ms = a[i + 1].parse().unwrap(); i += 1; }
                s => sel.push(s.to_string()),
            }
            i += 1;
        }
        let r: &'static Ctx = Box::leak(Box::new(Ctx::new()));
        assert!(binding::RESET_ON_ENTRY && binding::core_resets_on_entry() == 1);
        let x: &'static Ex = Box::leak(Box::new(Ex::new()));
        println!("# roe_bench: reset-on-entry core; arms explicit (default binding, the core's switch off on its contexts) and roe (variant binding); {rounds} rounds of ~{round_ms} ms per arm, interleaved, order rotated per round; process CPU per op; CONTAINER INSTRUMENTATION, absolute times only");
        println!("{:<46} {:<7} {:<16} {:>12} {:>12} {:>12}", "input", "mode", "arm", "median", "p10", "p90");
        let mut roots_seen: Vec<String> = Vec::new();
        for inp in inputs(&[]) {
            if !(if sel.is_empty() { core_codec_input(&inp.id) } else { sel.iter().any(|p| inp.id.starts_with(p.as_str())) }) {
                continue;
            }
            for (mode, retain) in [("drop", false), ("retain", true)] {
                let mut row = Row {
                    id: inp.id.clone(), bytes: Box::leak(inp.bytes.clone().into_boxed_slice()), encode: inp.encode,
                    unknown_row: inp.unknown_row, retain, r, x, arms: Vec::new(),
                };
                assert!(campaign::generated::roots::with_root(&inp.root, &mut Build(&mut row)));
                // ak_enc_reset alone, on a context that has just encoded this row (the roe arm's
                // encode left it holding the output).
                let enc = x.enc;
                row.arms.push(("ak_enc_reset".into(), Box::new(move || unsafe { ak_enc_reset(enc); 0 })));
                run(&inp.id, mode, &mut row.arms, rounds, round_ms);
            }
            if !roots_seen.contains(&inp.root) {
                roots_seen.push(inp.root.clone());
            }
        }
        // ak_dec_reset_<Root> alone, per root, with the binding's options (retain).
        for root in &roots_seen {
            let mut arms: Vec<(String, Op<'static>)> = vec![("ak_dec_reset (opts)".to_string(), dec_reset_op(root, x))];
            run(&format!("[{root}]"), "retain", &mut arms, rounds, round_ms);
        }
    }
}

#[cfg(all(feature = "reset-on-entry", feature = "unknown-fields"))]
fn main() {
    imp::main()
}
