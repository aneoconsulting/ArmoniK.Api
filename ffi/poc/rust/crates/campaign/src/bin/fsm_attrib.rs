//! FIX-PLAN D23 attribution (coordinator, 2026-10-09): why the FSM decode family is slower than
//! push and pull on some rows. MEASUREMENT ONLY; container instrumentation.
//!
//! One process, the three families interleaved round by round (arm order rotated per round),
//! process CPU time per op, the median and the p10..p90 of the per-round values. Arms:
//!
//!   push        core-ffi push decode into the facade (binding decode_with_*)
//!   pull        pull parse + walk in place + replay into the facade (parse_walk_with_*)
//!   fsm         FSM begin/next, each event dispatched into the facade (fsm_with_*)
//!   pull-core   ak_parse_<R> alone (no replay, no facade)                       [drop only]
//!   fsm-core    ak_fsm_begin/next_<R> to the end, empty dispatch (no facade)   [drop only]
//!   fsm-collect FSM events copied into a pull-format buffer, then pull's replay    [drop only]
//!
//! Inputs: the codec-suite rows named on the command line (prefixes, campaign::inputs), plus
//! synthetic probes:
//!   PK-ticks-N    ListMetricsResponse { 1 batch { ticks = N packed int64 } }, N = 16, 256, 4096
//!   PK-values-N   ... { values = N packed double }
//!   PK-flags-N    ... { flags = N packed bool (1-byte varints) }
//!   PK-codes-N    ... { codes = N packed int32, 3-byte varints }
//!   EV-empty-K    ListMetricsResponse { K empty batches }: 2K + 1 events, almost no decode work
//!
//!   fsm_attrib bench [--rounds R] [--round-ms MS] [--retain] ROW_PREFIX...
//!   fsm_attrib loop ROW ARM SECONDS            (one arm in a loop, for perf record)

use ak_abi::*;
use campaign::*;
use harness::arms::core_ffi_arm::Ctx;

type ParseF = unsafe extern "C" fn(*mut ak_dec_ctx, *const u8, usize) -> i32;
type BeginF = unsafe extern "C" fn(*mut ak_dec_ctx, *const u8, usize, *mut ak_fsm_ev) -> i32;
type NextF = unsafe extern "C" fn(*mut ak_dec_ctx, *mut ak_fsm_ev) -> i32;

fn core_fns(root: &str) -> (ParseF, BeginF, NextF) {
    macro_rules! r {
        ($($n:ident),*) => {
            match root {
                $(stringify!($n) => paste3!($n),)*
                _ => panic!("root {root}"),
            }
        };
    }
    macro_rules! paste3 {
        (ListResultsResponse) => { (ak_parse_ListResultsResponse as ParseF, ak_fsm_begin_ListResultsResponse as BeginF, ak_fsm_next_ListResultsResponse as NextF) };
        (ListTasksDetailedResponse) => { (ak_parse_ListTasksDetailedResponse as ParseF, ak_fsm_begin_ListTasksDetailedResponse as BeginF, ak_fsm_next_ListTasksDetailedResponse as NextF) };
        (ListProbeResponse) => { (ak_parse_ListProbeResponse as ParseF, ak_fsm_begin_ListProbeResponse as BeginF, ak_fsm_next_ListProbeResponse as NextF) };
        (ListTaskSummaryResponse) => { (ak_parse_ListTaskSummaryResponse as ParseF, ak_fsm_begin_ListTaskSummaryResponse as BeginF, ak_fsm_next_ListTaskSummaryResponse as NextF) };
        (UploadResultDataMessage) => { (ak_parse_UploadResultDataMessage as ParseF, ak_fsm_begin_UploadResultDataMessage as BeginF, ak_fsm_next_UploadResultDataMessage as NextF) };
        (ListMetricsResponse) => { (ak_parse_ListMetricsResponse as ParseF, ak_fsm_begin_ListMetricsResponse as BeginF, ak_fsm_next_ListMetricsResponse as NextF) };
        (DualResponse) => { (ak_parse_DualResponse as ParseF, ak_fsm_begin_DualResponse as BeginF, ak_fsm_next_DualResponse as NextF) };
    }
    r!(ListResultsResponse, ListTasksDetailedResponse, ListProbeResponse, ListTaskSummaryResponse,
       UploadResultDataMessage, ListMetricsResponse, DualResponse)
}

fn varint(out: &mut Vec<u8>, mut v: u64) {
    while v >= 0x80 {
        out.push((v as u8) | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}
fn ld(out: &mut Vec<u8>, field: u64, body: &[u8]) {
    varint(out, field << 3 | 2);
    varint(out, body.len() as u64);
    out.extend_from_slice(body);
}

fn synthetic() -> Vec<(String, &'static str, Vec<u8>)> {
    let mut v = Vec::new();
    for n in [16usize, 256, 4096] {
        let mut t = Vec::new();
        for i in 0..n { varint(&mut t, 1_700_000_000_000 + (i as u64) * 1000); }
        let mut batch = Vec::new();
        ld(&mut batch, 2, &t);
        let mut root = Vec::new();
        ld(&mut root, 1, &batch);
        v.push((format!("PK-ticks-{n}"), "ListMetricsResponse", root));
        let mut f = Vec::new();
        for i in 0..n { f.extend_from_slice(&(i as f64 * 0.25).to_le_bytes()); }
        let mut batch = Vec::new();
        ld(&mut batch, 3, &f);
        let mut root = Vec::new();
        ld(&mut root, 1, &batch);
        v.push((format!("PK-values-{n}"), "ListMetricsResponse", root));
    }
    for n in [16usize, 256, 4096] {
        for (name, field, val) in [("flags", 5u64, 1u64), ("codes", 4u64, 100_000u64)] {
            let mut t = Vec::new();
            for i in 0..n { varint(&mut t, if field == 5 { (i % 2) as u64 } else { val + i as u64 }); }
            let mut batch = Vec::new();
            ld(&mut batch, field, &t);
            let mut root = Vec::new();
            ld(&mut root, 1, &batch);
            v.push((format!("PK-{name}-{n}"), "ListMetricsResponse", root));
        }
    }
    for k in [1usize, 64, 1024] {
        let mut root = Vec::new();
        for _ in 0..k { ld(&mut root, 1, &[]); }
        v.push((format!("EV-empty-{k}"), "ListMetricsResponse", root));
    }
    v
}

/// One arm as a closure returning something to black-box.
type Op<'a> = Box<dyn FnMut() -> u64 + 'a>;

struct Row<'a> {
    id: String,
    root: String,
    bytes: Vec<u8>,
    ctx: &'a Ctx,
    retain: bool,
    arms: Vec<(&'static str, Op<'a>)>,
    events: usize,
}

struct Build<'a, 'b>(&'b mut Row<'a>);
impl<'a, 'b> Visit for Build<'a, 'b> {
    fn visit<R: Ops>(&mut self) {
        let row = &mut *self.0;
        let (ctx, retain) = (row.ctx, row.retain);
        let b: &'a [u8] = Box::leak(row.bytes.clone().into_boxed_slice());
        let dec = R::dec_ctx(ctx);
        // value checks before anything is timed
        let mut toks = Vec::new();
        let a = format!("{:?}", R::f_decode(ctx, b, retain).expect("push"));
        let p = format!("{:?}", R::f_pull(ctx, b, retain, &mut toks).expect("pull"));
        let f = format!("{:?}", R::f_fsm(ctx, b, retain, &mut toks).expect("fsm"));
        assert!(a == p && a == f, "{}: the three families disagree", row.id);
        row.arms.push(("push", Box::new(move || R::touch_f(&R::f_decode(ctx, b, retain).unwrap()))));
        let mut t1 = Vec::new();
        row.arms.push(("pull", Box::new(move || R::touch_f(&R::f_pull(ctx, b, retain, &mut t1).unwrap()))));
        let mut t2 = Vec::new();
        row.arms.push(("fsm", Box::new(move || R::touch_f(&R::f_fsm(ctx, b, retain, &mut t2).unwrap()))));
        if !retain {
            let c = format!("{:?}", R::f_fsm_collect(ctx, b, &mut toks, &mut Vec::new()).expect("fsm-collect"));
            assert!(c == a, "{}: fsm-collect disagrees", row.id);
            let (mut t3, mut buf) = (Vec::new(), Vec::new());
            row.arms.push(("fsm-collect", Box::new(move || R::touch_f(&R::f_fsm_collect(ctx, b, &mut t3, &mut buf).unwrap()))));
        }
        let (parse, begin, next) = core_fns(R::ROOT);
        // events per op (also checks the core-only loop ends on APPLY)
        let mut ev = ak_fsm_ev::default();
        let _ = R::f_pull(ctx, b, false, &mut toks); // leaves the context in drop mode
        let mut n = 0usize;
        unsafe {
            let mut rc = begin(dec, b.as_ptr(), b.len(), &mut ev);
            while rc > 0 {
                n += 1;
                if rc == AK_BDR_APPLY as i32 { break; }
                rc = next(dec, &mut ev);
            }
            assert!(rc == AK_BDR_APPLY as i32, "{}: fsm-core ended with {rc}", row.id);
        }
        row.events = n;
        if !retain {
            row.arms.push(("pull-core", Box::new(move || unsafe { parse(dec, b.as_ptr(), b.len()) as u64 })));
            row.arms.push(("fsm-core", Box::new(move || unsafe {
                let mut ev = ak_fsm_ev::default();
                let mut acc = 0u64;
                let mut rc = begin(dec, b.as_ptr(), b.len(), &mut ev);
                while rc > 0 {
                    acc = acc.wrapping_add(ev.n as u64);
                    if rc == AK_BDR_APPLY as i32 { break; }
                    rc = next(dec, &mut ev);
                }
                acc
            })));
        }
    }
}

fn pct(v: &mut [f64], p: f64) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[((v.len() - 1) as f64 * p).round() as usize]
}

fn fmt(ns: f64) -> String {
    if ns >= 1e6 { format!("{:.3} ms", ns / 1e6) } else if ns >= 1e3 { format!("{:.2} us", ns / 1e3) } else { format!("{:.1} ns", ns) }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let ctx: &'static Ctx = Box::leak(Box::new(Ctx::new()));
    let mut rounds = 31usize;
    let mut round_ms = 20u64;
    let mut retain = false;
    let mut sel: Vec<String> = Vec::new();
    let mode = a.get(1).cloned().unwrap_or_default();
    let mut i = 2;
    while i < a.len() {
        match a[i].as_str() {
            "--rounds" => { rounds = a[i + 1].parse().unwrap(); i += 1; }
            "--round-ms" => { round_ms = a[i + 1].parse().unwrap(); i += 1; }
            "--retain" => retain = true,
            x => sel.push(x.to_string()),
        }
        i += 1;
    }
    let mut rows_src: Vec<(String, String, Vec<u8>)> = Vec::new();
    let codec_sel: Vec<String> = sel.iter().filter(|s| !s.starts_with("PK-") && !s.starts_with("EV-")).cloned().collect();
    if !codec_sel.is_empty() {
        for inp in inputs(&codec_sel) {
            rows_src.push((inp.id.clone(), inp.root.clone(), inp.bytes.clone()));
        }
    }
    for (id, root, b) in synthetic() {
        if sel.iter().any(|s| id.starts_with(s.as_str())) {
            rows_src.push((id, root.to_string(), b));
        }
    }
    if mode == "dump" {
        // dump ROW FILE: the row's wire bytes, for offline wire statistics
        let (id, _root, b) = rows_src.into_iter().find(|r| r.0 == sel[0]).expect("row");
        std::fs::write(&sel[1], &b).unwrap();
        eprintln!("# {id}: {} bytes -> {}", b.len(), sel[1]);
        return;
    }
    if mode == "loop" {
        // loop ROW ARM SECONDS
        let (row_id, arm, secs) = (&sel[0], &sel[1], sel[2].parse::<f64>().unwrap());
        let (id, root, b) = rows_src.into_iter().find(|r| &r.0 == row_id).expect("row");
        let mut row = Row { id, root: root.clone(), bytes: b, ctx, retain, arms: Vec::new(), events: 0 };
        assert!(campaign::generated::roots::with_root(&root, &mut Build(&mut row)));
        let op = &mut row.arms.iter_mut().find(|x| x.0 == arm).expect("arm").1;
        let t0 = std::time::Instant::now();
        let mut n = 0u64;
        while t0.elapsed().as_secs_f64() < secs {
            for _ in 0..64 { std::hint::black_box(op()); }
            n += 64;
        }
        eprintln!("# loop {row_id} {arm}: {n} ops in {secs} s");
        return;
    }
    println!("# fsm_attrib bench: {} rounds of ~{} ms per arm, arms interleaved (order rotated per round), process CPU per op; mode {}; CONTAINER INSTRUMENTATION",
             rounds, round_ms, if retain { "retain" } else { "drop" });
    println!("{:<44} {:>8} {:>7} {:<10} {:>12} {:>12} {:>12} {:>12}", "row", "bytes", "events", "arm", "median", "p10", "p90", "per event");
    for (id, root, b) in rows_src {
        let mut row = Row { id: id.clone(), root: root.clone(), bytes: b.clone(), ctx, retain, arms: Vec::new(), events: 0 };
        assert!(campaign::generated::roots::with_root(&root, &mut Build(&mut row)));
        // calibrate iterations per round for each arm
        let mut iters = Vec::new();
        for (_, op) in row.arms.iter_mut() {
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
        let na = row.arms.len();
        let mut per: Vec<Vec<f64>> = vec![Vec::new(); na];
        for r in 0..rounds {
            for j in 0..na {
                let x = (j + r) % na;
                let op = &mut row.arms[x].1;
                let k = iters[x];
                let c0 = process_clock_ns();
                for _ in 0..k { std::hint::black_box(op()); }
                per[x].push((process_clock_ns() - c0) as f64 / k as f64);
            }
        }
        for (x, (name, _)) in row.arms.iter().enumerate() {
            let v = &mut per[x];
            let med = pct(v, 0.5);
            println!("{:<44} {:>8} {:>7} {:<10} {:>12} {:>12} {:>12} {:>12}", id, b.len(), row.events, name,
                     fmt(med), fmt(pct(v, 0.1)), fmt(pct(v, 0.9)), fmt(med / row.events as f64));
        }
    }
}
