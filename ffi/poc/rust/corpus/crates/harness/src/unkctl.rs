//! Decision 11's controls (WP5 steps 7-8), built only with `unknown-fields`.
#![allow(dead_code)]
use super::*;
use ak_abi::*;
use std::path::Path;

// ------------------------------------------------------------ decision 11's controls

/// WP5 step 7. (1) On every accept row whose root crosses the C ABI: each position's entry
/// zeroed in turn drops exactly that position (the all-armed value with that position's
/// bags cleared), the pull family delivers what push does, and map-entry bytes arrive
/// unless the entry position is zeroed. `--plant` skips the clearing, so (1) must FAIL on
/// the rows that carry unknowns. (2) A pre-allocated buffer is placed once: with no grow,
/// the second element that needs a buffer is refused (AK_ERR_CAPACITY) rather than given
/// the same one; with grow, it gets a different buffer.
pub fn unk_controls(manifest: &Path, only: &[String], plant: bool) -> i32 {
    let m = load(manifest);
    let dir = manifest.parent().unwrap();
    assert!(binding::ak_init_once() >= 0);
    let cx = unsafe { Cx { enc: ak_enc_ctx_new(), dec: binding::DecCtxs::new(), tcs: binding::Tcs::trusted() } };
    let (mut rows, mut with_unk, mut bad, mut pull_bad, mut entry_rows) = (0, 0, 0, 0, 0);
    let mut positions = 0usize;
    let mut changed = 0usize;
    for (id, row) in m["vectors"].as_object().unwrap() {
        if row["expect"] != "accept" || (!only.is_empty() && !only.iter().any(|o| id.starts_with(o.as_str()))) {
            continue;
        }
        let root = row["root"].as_str().unwrap();
        let bytes = std::fs::read(dir.join(row["file"].as_str().unwrap())).unwrap();
        let Some(r) = super::generated::dispatch::unk_controls(root, &cx, &bytes, plant) else { continue };
        let r = match r {
            Ok(r) => r,
            Err(e) => { println!("  ERR  {id}: {e}"); bad += 1; continue; }
        };
        rows += 1;
        positions += r.positions;
        changed += r.changed;
        if r.changed > 0 { with_unk += 1; }
        if r.entry_bytes > 0 { entry_rows += 1; }
        if !r.mismatched.is_empty() || !r.entry_mismatch.is_empty() {
            bad += 1;
            if bad <= 12 {
                println!("  FAIL {id} ({root}): positions {:?} did not drop exactly themselves; entry {:?}", r.mismatched, r.entry_mismatch);
            }
        }
        if !r.pull_equal { pull_bad += 1; println!("  FAIL {id}: pull != push"); }
        if id.starts_with("U-") && (r.changed > 0 || r.entry_bytes > 0) {
            println!("  {id:<34} {root:<26} positions {:>2}, changed by zeroing {:>2}, entry bytes {}", r.positions, r.changed, r.entry_bytes);
        }
    }
    println!("rows {rows} (accept, root in the ABI), {positions} (row, position) pairs; rows with unknowns at some position {with_unk} ({changed} pairs changed by zeroing); rows with map-entry bytes {entry_rows}");
    println!("discard mismatches: {bad} row(s); pull != push: {pull_bad} row(s){}", if plant { "   [PLANTED: clearing skipped]" } else { "" });
    let placed = if plant { true } else { prealloc_cases(&cx) };
    if bad == 0 && pull_bad == 0 && placed { 0 } else { 1 }
}

fn varint(mut n: u64, out: &mut Vec<u8>) {
    while n >= 0x80 { out.push((n as u8) | 0x80); n >>= 7; }
    out.push(n as u8);
}
fn ld(tag: u32, body: &[u8], out: &mut Vec<u8>) {
    varint(((tag as u64) << 3) | 2, out);
    varint(body.len() as u64, out);
    out.extend_from_slice(body);
}

/// Decision 11 rule 1's host side, instrumented: a grow that counts its calls (and its
/// FRESH calls, dst == NULL) through the ONE host pointer, then does what `unk_grow` does.
#[repr(C)]
#[derive(Default)]
struct GrowCount {
    calls: u32,
    fresh: u32,
}
unsafe extern "C" fn counting_grow(host: *mut std::ffi::c_void, want: i32, dst: *mut *mut u8, cap: *mut i32) -> i32 {
    let h = &mut *(host as *mut GrowCount);
    h.calls += 1;
    if (*dst).is_null() {
        h.fresh += 1;
    }
    binding::unk_grow(host, want, dst, cap)
}

const NOBUF: ak_unk_buf = ak_unk_buf { data: std::ptr::null_mut(), len: 0, cap: 0 };
fn alloc(n: usize) -> *mut u8 {
    unsafe { std::alloc::alloc(std::alloc::Layout::from_size_align(n, 1).unwrap()) }
}
fn free(p: *mut u8, n: usize) {
    unsafe { std::alloc::dealloc(p, std::alloc::Layout::from_size_align(n, 1).unwrap()) }
}
fn verdict(ok: bool) -> &'static str {
    if ok { "PASS" } else { "FAIL" }
}

/// WP5 step 8: the pool, in-place refill, oneof switch and wrong-root controls.
fn prealloc_cases(cx: &Cx) -> bool {
    let mut ok = true;
    ok &= pool_cases(cx);
    ok &= refill_cases();
    ok &= oneof_cases(cx);
    ok &= wrong_root_cases(cx);
    ok &= refused_parse_keeps_records(cx);
    ok &= decided_at_arm_time(cx);
    ok
}

/// POOL (rule 1): a batched repeated position (ListResultsResponse.results, leaf elements)
/// with n pre-allocated buffers: no grow for the first n elements that carry unknowns,
/// then grow; with no grow, AK_ERR_CAPACITY. Taken buffers are cleared IN the host's struct.
fn pool_cases(cx: &Cx) -> bool {
    let mut b = Vec::new();
    let mut runs = Vec::new();
    for (i, sid) in [&b"aa"[..], b"bb", b"cc"].iter().enumerate() {
        let mut e = Vec::new();
        ld(1, sid, &mut e);
        let run = vec![0xa0, 0x06, 7 + i as u8]; // field 100, varint
        e.extend_from_slice(&run);
        runs.push(run);
        ld(1, &e, &mut b);
    }
    let mut ok = true;
    let pz = ak_unk_pool { bufs: std::ptr::null_mut(), n: 0, grow: None };
    for (name, grow) in [("pool n=2, grow", true), ("pool n=2, no grow", false)] {
        let (p0, p1) = (alloc(64), alloc(64));
        let mut arr = [ak_unk_buf { data: p0 as _, len: 0, cap: 64 }, ak_unk_buf { data: p1 as _, len: 0, cap: 64 }];
        let mut gc = GrowCount::default();
        let mut o = binding::unk_opts_list_results_response(Some(usize::MAX));
        o.host = &mut gc as *mut GrowCount as *mut std::ffi::c_void;
        o.self_ = ak_unk_opts { buf: NOBUF, grow: None };
        o.results_created_at = pz;
        o.results_completed_at = pz;
        o.results = ak_unk_pool { bufs: arr.as_mut_ptr(), n: 2, grow: if grow { Some(counting_grow) } else { None } };
        let r = binding::decode_with_list_results_response_opts(cx.dec, &b, &mut o);
        let cleared = arr.iter().all(|x| x.data.is_null());
        let line = match (&r, grow) {
            (Ok(v), true) => {
                let pp: Vec<*const u8> = v.results.iter().map(|e| e.unknown_fields.as_ptr()).collect();
                let bags = v.results.iter().zip(&runs).all(|(e, r)| &e.unknown_fields == r);
                let pass = bags && pp[0] == p0 as *const u8 && pp[1] == p1 as *const u8
                    && pp[2] != p0 as *const u8 && pp[2] != p1 as *const u8 && gc.calls == 1 && gc.fresh == 1 && cleared;
                ok &= pass;
                format!("elements 0,1 in the pool's buffers, element 2 from grow; grow calls {} (fresh {}); host entries cleared: {cleared}; bags exact: {bags}  {}",
                        gc.calls, gc.fresh, verdict(pass))
            }
            (Err(e), false) => {
                let pass = *e == AK_ERR_CAPACITY && cleared && gc.calls == 0;
                ok &= pass;
                // Rule 3: the taken buffers stayed the host's (not delivered); free them.
                free(p0, 64);
                free(p1, 64);
                format!("rc {e} (AK_ERR_CAPACITY when the pool is exhausted and there is no grow); host entries cleared: {cleared}  {}", verdict(pass))
            }
            (r, _) => { ok = false; format!("unexpected {:?}  FAIL", r.as_ref().map(|_| ())) }
        };
        println!("  placement: {name:<24} {line}");
    }
    ok
}

/// IN-PLACE REFILL (rule 1): ListTasksDetailedResponse.tasks is NOT batched (its element has
/// runs of its own), so the host refills the pool in `new_tasks`, writing its own struct;
/// the element decoded next must use the refilled buffer. Without the refill the second
/// element finds nothing and no grow: AK_ERR_CAPACITY.
struct RefillHost {
    opts: *mut ak_dec_ListTasksDetailedResponse_opts,
    arr: *mut ak_unk_buf,
    refill: bool,
    placed: Vec<*mut u8>,
    got: Vec<(*const u8, Vec<u8>)>,
    n: i64,
}
unsafe extern "C" fn r_new(_c: *mut ak_dec_ctx, obj: *mut std::ffi::c_void) -> i64 {
    let h = &mut *(obj as *mut RefillHost);
    if h.refill && (*h.arr).data.is_null() {
        let p = alloc(64);
        h.placed.push(p);
        *h.arr = ak_unk_buf { data: p as _, len: 0, cap: 64 };
        let _ = h.opts;
    }
    h.n += 1;
    h.n - 1
}
unsafe extern "C" fn r_apply(_c: *mut ak_dec_ctx, obj: *mut std::ffi::c_void, _t: i64, f: *const ak_dfix_TaskDetailed) {
    let h = &mut *(obj as *mut RefillHost);
    let u = (*f).unknown;
    let bytes = if u.data.is_null() { Vec::new() } else { std::slice::from_raw_parts(u.data as *const u8, u.len as usize).to_vec() };
    h.got.push((u.data as *const u8, bytes));
}

fn refill_cases() -> bool {
    use facade::generated::core_native_retain as nr;
    let mut v = facade::ListTasksDetailedResponse::default();
    let mut runs = Vec::new();
    for i in 0..3u8 {
        let mut t = facade::TaskDetailed::default();
        t.id = format!("t{i}");
        t.unknown_fields = vec![0xa0, 0x06, 20 + i];
        runs.push(t.unknown_fields.clone());
        v.tasks.push(t);
    }
    let b = nr::encode_list_tasks_detailed_response(&v);
    let mut ok = true;
    for refill in [true, false] {
        let p0 = alloc(64);
        let mut arr = [ak_unk_buf { data: p0 as _, len: 0, cap: 64 }];
        let mut o: ak_dec_ListTasksDetailedResponse_opts = unsafe { std::mem::zeroed() };
        o.tasks = ak_unk_pool { bufs: arr.as_mut_ptr(), n: 1, grow: None };
        let mut h = RefillHost { opts: &mut o, arr: arr.as_mut_ptr(), refill, placed: vec![p0], got: Vec::new(), n: 0 };
        let rc = unsafe {
            let ctx = ak_dec_ctx_new_ListTasksDetailedResponse(&mut o);
            let mut vt: ak_dvt_ListTasksDetailedResponse = std::mem::zeroed();
            vt.new_tasks = Some(r_new);
            vt.apply_tasks = Some(r_apply);
            let rc = ak_decode_ListTasksDetailedResponse(ctx, &mut h as *mut RefillHost as *mut std::ffi::c_void, b.as_ptr(), b.len(), &vt);
            ak_dec_ctx_free(ctx);
            rc
        };
        let line = if refill {
            let pass = rc == AK_OK && h.got.len() == 3
                && h.got.iter().zip(&h.placed).all(|((p, _), q)| *p == *q as *const u8)
                && h.got.iter().zip(&runs).all(|((_, g), r)| g == r) && h.placed.len() == 3;
            ok &= pass;
            format!("rc {rc}; 3 elements, each in the buffer the host wrote into its struct in new_tasks (refills {}), bags exact  {}",
                    h.placed.len() - 1, verdict(pass))
        } else {
            let pass = rc == AK_ERR_CAPACITY;
            ok &= pass;
            format!("rc {rc} (without the refill: AK_ERR_CAPACITY at the second element)  {}", verdict(pass))
        };
        for p in &h.placed {
            free(*p, 64);
        }
        println!("  placement: {:<24} {line}", if refill { "in-place refill" } else { "no refill (control)" });
    }
    ok
}

/// ONEOF (rule 4): one position, one buffer. A probe whose body switches message members
/// keeps ONE buffer (one fresh grow), emptied on each switch, so the final member's bag is
/// that member's run only.
fn oneof_cases(cx: &Cx) -> bool {
    let run = |x: u8| vec![0xa0u8, 0x06, x];
    let mut ok = true;
    for (name, seq, want_last) in [
        ("oneof stamp -> nothing", vec![(13u32, 1u8), (14, 2)], 2u8),
        ("oneof stamp -> nothing -> stamp", vec![(13, 1), (14, 2), (13, 3)], 3),
    ] {
        let mut p = Vec::new();
        ld(1, b"p", &mut p);
        for (tag, x) in &seq {
            ld(*tag, &run(*x), &mut p);
        }
        let mut b = Vec::new();
        ld(1, &p, &mut b);
        let mut gc = GrowCount::default();
        let mut o = binding::unk_opts_list_probe_response(None);
        o.host = &mut gc as *mut GrowCount as *mut std::ffi::c_void;
        o.probes_body.grow = Some(counting_grow);
        let r = binding::decode_with_list_probe_response_opts(cx.dec, &b, &mut o);
        let line = match r {
            Ok(v) => {
                let bag = match v.probes.first().and_then(|p| p.body.as_ref()) {
                    Some(facade::ProbeBody::AsStamp(t)) => Some(t.unknown_fields.clone()),
                    Some(facade::ProbeBody::AsNothing(e)) => Some(e.unknown_fields.clone()),
                    _ => None,
                };
                let pass = bag.as_deref() == Some(&run(want_last)[..]) && gc.fresh == 1;
                ok &= pass;
                format!("final member's bag {:?} (want its own run only); fresh buffers {}  {}", bag, gc.fresh, verdict(pass))
            }
            Err(e) => { ok = false; format!("rc {e}  FAIL") }
        };
        println!("  placement: {name:<24} {line}");
    }
    // FIX-PLAN R-H8: a switch from a MESSAGE member (carrying unknowns) to a SCALAR member.
    // The core leaves the buffer in the now inactive member's slot, which the binding never
    // delivers; it must come back through `unk_reclaim` (exactly one), and nothing may
    // stay live after (0 leaked). The value is the scalar member, with no bag anywhere.
    {
        let mut p = Vec::new();
        ld(1, b"p", &mut p);
        ld(13, &run(1), &mut p);
        p.extend_from_slice(&[0x50, 0x07]); // as_int (10) = 7
        let mut b = Vec::new();
        ld(1, &p, &mut b);
        let mut gc = GrowCount::default();
        let mut o = binding::unk_opts_list_probe_response(None);
        o.host = &mut gc as *mut GrowCount as *mut std::ffi::c_void;
        o.probes_body.grow = Some(counting_grow);
        binding::unk_reclaim();
        let ctx = cx.dec.list_probe_response;
        let r0 = unsafe { ak_dec_reset_ListProbeResponse(ctx, &mut o) };
        let r = binding::decode_with_list_probe_response(cx.dec, &b);
        unsafe { ak_dec_reset_ListProbeResponse(ctx, std::ptr::null_mut()) };
        let reclaimed = binding::unk_reclaim();
        let leaked = binding::unk_reclaim();
        let line = match r {
            Ok(v) => {
                let body = v.probes.first().and_then(|p| p.body.clone());
                let bags: usize = v.probes.iter().map(|p| p.unknown_fields.len()).sum::<usize>() + v.unknown_fields.len();
                let pass = r0 == AK_OK && body == Some(facade::ProbeBody::AsInt(7)) && bags == 0
                    && gc.fresh == 1 && reclaimed == 1 && leaked == 0;
                ok &= pass;
                format!("body {:?}, bags delivered {} bytes; buffers grown {} (fresh), reclaimed from the inactive slot {}, live after {}  {}",
                        body, bags, gc.fresh, reclaimed, leaked, verdict(pass))
            }
            Err(e) => { ok = false; format!("rc {e}  FAIL") }
        };
        println!("  placement: {:<24} {line}", "oneof stamp -> int");
    }
    ok
}

/// ROOT-BOUND CONTEXTS (rule 6): a context bound to ListResultsResponse refuses another
/// root's decode, parse and reset; its own root still decodes after.
fn wrong_root_cases(cx: &Cx) -> bool {
    let ctx = cx.dec.list_results_response;
    let b: Vec<u8> = Vec::new();
    unsafe {
        let vt: ak_dvt_ListTasksDetailedResponse = std::mem::zeroed();
        let d = ak_decode_ListTasksDetailedResponse(ctx, std::ptr::null_mut(), b.as_ptr(), 0, &vt);
        let p = ak_parse_ListTasksDetailedResponse(ctx, b.as_ptr(), 0);
        let r = ak_dec_reset_ListTasksDetailedResponse(ctx, std::ptr::null_mut());
        let own = binding::decode_with_list_results_response(cx.dec, &b).is_ok();
        let pass = d == AK_ERR_INVALID_STATE && p == AK_ERR_INVALID_STATE && r == AK_ERR_INVALID_STATE && own;
        println!("  placement: {:<24} decode rc {d}, parse rc {p}, reset rc {r} (AK_ERR_INVALID_STATE = {AK_ERR_INVALID_STATE}); own root still decodes: {own}  {}",
                 "wrong root (refused)", verdict(pass));
        pass
    }
}

/// FIX-PLAN R-H10: a parse refused for the wrong root (-8) must leave the records of an
/// earlier, unread parse on that context intact: parse A, then parse B on A's context,
/// then A's records byte for byte (footprint and the record bytes).
fn refused_parse_keeps_records(cx: &Cx) -> bool {
    let ctx = cx.dec.list_results_response;
    // Two ResultRaw elements, each with a string field, so the parse leaves records.
    let mut el = Vec::new();
    ld(1, b"session", &mut el);
    let mut b = Vec::new();
    ld(1, &el, &mut b);
    ld(1, &el, &mut b);
    unsafe {
        let pa = ak_parse_ListResultsResponse(ctx, b.as_ptr(), b.len());
        let snap = |ctx| {
            let (mut p, mut n) = (std::ptr::null::<u8>(), 0usize);
            ak_bdr_ptr(ctx, &mut p, &mut n);
            (ak_bdr_footprint(ctx), if n == 0 { Vec::new() } else { std::slice::from_raw_parts(p, n).to_vec() })
        };
        let (fa, ra) = snap(ctx);
        let pb = ak_parse_ListTasksDetailedResponse(ctx, b.as_ptr(), b.len());
        let (fb, rb) = snap(ctx);
        let pass = pa >= 0 && !ra.is_empty() && pb == AK_ERR_INVALID_STATE && ra == rb;
        println!("  placement: {:<24} parse A rc {pa}, {} record bytes (footprint {fa}); parse B on A's context rc {pb}; after: {} record bytes (footprint {fb}), identical: {}  {}",
                 "refused parse keeps A", ra.len(), rb.len(), ra == rb, verdict(pass));
        ak_bdr_reset(ctx);
        pass
    }
}

/// FIX-PLAN R-H20 / ABI-v1 rule 1 as amended 2026-09-26: whether a decode retains is decided
/// when the context is reset. Options all zero at reset, an entry refilled AFTER the reset
/// and before the decode: the decode succeeds (rc 0) and delivers no bag.
fn decided_at_arm_time(cx: &Cx) -> bool {
    let ctx = cx.dec.list_results_response;
    let mut o = binding::unk_opts_list_results_response(None);
    o.self_.grow = None;
    o.results.grow = None;
    o.results_created_at.grow = None;
    o.results_completed_at.grow = None;
    // The root carries one unknown field (99, varint 1).
    let mut b = vec![0x98u8, 0x06, 0x01];
    ld(1, b"", &mut b);
    unsafe {
        let r0 = ak_dec_reset_ListResultsResponse(ctx, &mut o);
        o.self_.grow = Some(binding::unk_grow); // the refill, after the decision
        let r = binding::decode_with_list_results_response(cx.dec, &b);
        ak_dec_reset_ListResultsResponse(ctx, std::ptr::null_mut());
        let (rc, bag) = match &r {
            Ok(v) => (0, v.unknown_fields.len()),
            Err(e) => (*e, usize::MAX),
        };
        // The twin that must see a bag: the same entry armed BEFORE the reset.
        let mut o2 = binding::unk_opts_list_results_response(Some(1));
        o2.results_created_at.grow = None;
        o2.results_completed_at.grow = None;
        let r2 = binding::decode_with_list_results_response_opts(cx.dec, &b, &mut o2);
        let bag2 = r2.as_ref().map(|v| v.unknown_fields.len()).unwrap_or(usize::MAX);
        binding::unk_reclaim();
        let pass = r0 == AK_OK && rc == 0 && bag == 0 && bag2 == 3;
        println!("  placement: {:<24} reset with all-zero options rc {r0}; root entry refilled after the reset; decode rc {rc}, root bag {} bytes (want rc 0 and no bag); twin armed at reset: bag {} bytes (want 3)  {}",
                 "decided at arm time", if bag == usize::MAX { 0 } else { bag }, bag2, verdict(pass));
        pass
    }
}
