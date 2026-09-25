//! FIX-PLAN WP4 item 7 / finding R-D6: the sticky error slot (ABI v1 section 5).
//!
//! "The codec checks the sticky slot after every upcall and unwinds: it stops, and the
//! entry point returns the code." The finding: encode entry points ignore `hdr.err`, so a
//! host callback that calls `ak_fail` and then returns AK_OK gets a SUCCESSFUL encode;
//! decode reads the slot only at the very end, so it goes on making upcalls (more `add`,
//! more `new`, `apply`) after the host said the operation failed.
//!
//! Every case below drives the real C ABI (the cdylib) with hand-written callbacks, counts
//! the upcalls made AFTER the host called `ak_fail`, and prints the entry point's return.
//! The gate: every case returns a negative code, and zero upcalls follow the failure.
//! Printed as data; `PASS`/`FAIL` per case is that rule applied, nothing more.

use std::ffi::c_void;

use ak_abi::*;
use harness::arms::core_ffi_arm::Ctx;

fn varint(mut n: u64, out: &mut Vec<u8>) {
    while n >= 0x80 {
        out.push((n as u8) | 0x80);
        n >>= 7;
    }
    out.push(n as u8);
}
fn ld(tag: u32, body: &[u8], out: &mut Vec<u8>) {
    varint(((tag as u64) << 3) | 2, out);
    varint(body.len() as u64, out);
    out.extend_from_slice(body);
}

/// Three ResultRaw elements, each with a session_id, then page=7.
fn m1_bytes() -> Vec<u8> {
    let mut b = Vec::new();
    for s in [&b"aa"[..], b"bb", b"cc"] {
        let mut e = Vec::new();
        ld(1, s, &mut e);
        ld(1, &e, &mut b);
    }
    b.extend([0x18, 7]); // page = 7 (tag 3, varint)
    b
}

/// Three TaskDetailed elements, each with an id and one parent_task_id.
fn m2_bytes() -> Vec<u8> {
    let mut b = Vec::new();
    for s in [&b"t1"[..], b"t2", b"t3"] {
        let mut e = Vec::new();
        ld(1, s, &mut e);
        ld(4, b"p", &mut e);
        ld(1, &e, &mut b);
    }
    b
}

/// Two results elements then an unknown field (15, varint) at the root.
fn m1_unknown_bytes() -> Vec<u8> {
    let mut b = Vec::new();
    let mut e = Vec::new();
    ld(1, b"aa", &mut e);
    ld(1, &e, &mut b);
    b.extend([0x78, 1]); // field 15 varint 1
    b
}

/// Shared observation state. `failed` flips when a callback calls `ak_fail`; every
/// upcall after that bumps `after`.
#[derive(Default)]
struct Obs {
    failed: bool,
    after: u32,
    upcalls: u32,
    apply_called: bool,
    tok_neg: bool,
}
impl Obs {
    fn up(&mut self) {
        self.upcalls += 1;
        if self.failed {
            self.after += 1;
        }
    }
}
unsafe fn obs<'a>(p: *const c_void) -> &'a mut Obs {
    &mut *(p as *mut Obs)
}
unsafe fn host_fail(ctx: *mut c_void, o: &mut Obs) {
    ak_fail(ctx, AK_ERR_HOST, b"planted".as_ptr(), 7);
    o.failed = true;
}

fn verdict(name: &str, rc: i64, o: &Obs, extra: &str) -> bool {
    let ok = rc < 0 && o.after == 0 && !o.apply_called_after_fail();
    println!(
        "{:<44} rc={:<4} upcalls={} after_fail={} apply_called={} {}  {}",
        name, rc, o.upcalls, o.after, o.apply_called, extra, if ok { "PASS" } else { "FAIL" }
    );
    ok
}
impl Obs {
    fn apply_called_after_fail(&self) -> bool {
        self.apply_called && self.failed
    }
}

// ------------------------------------------------------------------ encode

unsafe extern "C" fn e1_loop(ctx: *mut ak_enc_ctx, obj: *const c_void, _tok: i64) -> i32 {
    let o = obs(obj);
    o.up();
    host_fail(ctx as *mut c_void, o);
    AK_OK
}

fn enc_m1_loop_fails() -> bool {
    let c = Ctx::new();
    let mut o = Obs::default();
    let fix = ak_efix_ListResultsResponse { page: 7, total: 0, presence: 0 };
    let vt = ak_evt_ListResultsResponse { loop_results: Some(e1_loop) };
    let rc = unsafe {
        ak_enc_reset(c.enc);
        ak_encode_ListResultsResponse(&mut o as *mut Obs as *const c_void, c.enc, &vt, &fix)
    };
    let e = unsafe { ak_enc_err(c.enc) };
    verdict("E1 encode M1, loop fails, returns AK_OK", rc as i64, &o, &format!("ak_enc_err={e}"))
}

static mut E2_ELEMS: [ak_efix_TaskDetailed; 2] = [ak_efix_TaskDetailed::ZERO; 2];

unsafe extern "C" fn e2_loop_tasks(ctx: *mut ak_enc_ctx, obj: *const c_void, _tok: i64) -> i32 {
    let o = obs(obj);
    o.up();
    // The host hands the codec two elements; the codec calls back into the host for each
    // element's own repeated fields.
    let p = &raw const E2_ELEMS;
    ak_elemu_TaskDetailed(ctx, (*p).as_ptr(), 2, 0)
}
unsafe extern "C" fn e2_inner(ctx: *mut ak_enc_ctx, obj: *const c_void, _tok: i64) -> i32 {
    let o = obs(obj);
    o.up();
    if !o.failed {
        host_fail(ctx as *mut c_void, o);
    }
    AK_OK
}

fn enc_m2_inner_loop_fails() -> bool {
    let c = Ctx::new();
    let mut o = Obs::default();
    let fix = ak_efix_ListTasksDetailedResponse::ZERO;
    let evt = ak_evt_TaskDetailed {
        loop_parent_task_ids: Some(e2_inner),
        loop_data_dependencies: Some(e2_inner),
        loop_expected_output_ids: Some(e2_inner),
        loop_retry_of_ids: Some(e2_inner),
        loop_options_options: Some(e2_inner),
    };
    let vt = ak_evt_ListTasksDetailedResponse { loop_tasks: Some(e2_loop_tasks), elem_tasks: &evt };
    let rc = unsafe {
        ak_enc_reset(c.enc);
        ak_encode_ListTasksDetailedResponse(&mut o as *mut Obs as *const c_void, c.enc, &vt, &fix)
    };
    verdict("E2 encode M2, inner element loop fails", rc as i64, &o, "")
}

// ------------------------------------------------------------------ decode

unsafe extern "C" fn d_apply_m1(_c: *mut ak_dec_ctx, obj: *mut c_void, _f: *const ak_dfix_ListResultsResponse) {
    let o = obs(obj);
    o.up();
    o.apply_called = true;
}
unsafe extern "C" fn d1_add(ctx: *mut ak_dec_ctx, obj: *mut c_void, _t: i64, _e: *const ak_dfix_ResultRaw, _n: i32) {
    let o = obs(obj);
    o.up();
    host_fail(ctx as *mut c_void, o);
}

fn dec_m1_add_fails() -> bool {
    let c = Ctx::new();
    let mut o = Obs::default();
    let b = m1_bytes();
    let vt = ak_dvt_ListResultsResponse {
        apply: Some(d_apply_m1),
        add_results: Some(d1_add),
    };
    let rc = unsafe { ak_decode_ListResultsResponse(c.dec, &mut o as *mut Obs as *mut c_void, b.as_ptr(), b.len(), &vt) };
    verdict("D1 decode M1, add fails", rc as i64, &o, "")
}

/// WP5 step 7 (decision 11): the unknown-field upcall is now `grow`. It calls `ak_fail`
/// through the context it is given as its host and returns AK_OK; the core must stop.
struct GrowHost {
    ctx: *mut ak_dec_ctx,
    o: *mut Obs,
}
unsafe extern "C" fn d_unk_grow(sink: *mut c_void, _want: i32, _dst: *mut *mut u8, _cap: *mut i32) -> i32 {
    let h = &mut *(sink as *mut GrowHost);
    let o = &mut *h.o;
    o.up();
    host_fail(h.ctx as *mut c_void, o);
    AK_OK
}
unsafe extern "C" fn d_add_ok(_ctx: *mut ak_dec_ctx, obj: *mut c_void, _t: i64, _e: *const ak_dfix_ResultRaw, _n: i32) {
    obs(obj).up();
}

fn dec_m1_unknown_fails() -> bool {
    let c = Ctx::new();
    let mut o = Obs::default();
    let b = m1_unknown_bytes();
    let vt = ak_dvt_ListResultsResponse {
        apply: Some(d_apply_m1),
        add_results: Some(d_add_ok),
    };
    let mut h = GrowHost { ctx: c.dec, o: &mut o as *mut Obs };
    let z = ak_unk_opts { buf: ak_unk_buf { data: std::ptr::null_mut(), len: 0, cap: 0 }, grow: None };
    let opts = ak_dec_ListResultsResponse_opts {
        host: &mut h as *mut GrowHost as *mut c_void,
        self_: ak_unk_opts { grow: Some(d_unk_grow), ..z },
        results: z,
        results_created_at: z,
        results_completed_at: z,
    };
    let rc = unsafe {
        ak_dec_reset_ListResultsResponse(c.dec, &opts);
        let rc = ak_decode_ListResultsResponse(c.dec, &mut o as *mut Obs as *mut c_void, b.as_ptr(), b.len(), &vt);
        ak_dec_reset_ListResultsResponse(c.dec, std::ptr::null());
        rc
    };
    verdict("D3 decode M1, root unknown-field grow upcall fails", rc as i64, &o, "")
}

unsafe extern "C" fn d_apply_m2(_c: *mut ak_dec_ctx, obj: *mut c_void, _f: *const ak_dfix_ListTasksDetailedResponse) {
    let o = obs(obj);
    o.up();
    o.apply_called = true;
}
unsafe extern "C" fn d2_new(ctx: *mut ak_dec_ctx, obj: *mut c_void) -> i64 {
    let o = obs(obj);
    o.up();
    host_fail(ctx as *mut c_void, o);
    0
}
unsafe extern "C" fn d2_new_neg(_ctx: *mut ak_dec_ctx, obj: *mut c_void) -> i64 {
    let o = obs(obj);
    o.up();
    // A negative token WITHOUT ak_fail: the host could not make the element.
    o.tok_neg = true;
    o.failed = true;
    -1
}
unsafe extern "C" fn d2_new_ok(_ctx: *mut ak_dec_ctx, obj: *mut c_void) -> i64 {
    let o = obs(obj);
    o.up();
    o.upcalls as i64
}
unsafe extern "C" fn d2_apply_el(_c: *mut ak_dec_ctx, obj: *mut c_void, _t: i64, _f: *const ak_dfix_TaskDetailed) {
    obs(obj).up();
}
unsafe extern "C" fn d2_add_span(_c: *mut ak_dec_ctx, obj: *mut c_void, _t: i64, _e: *const ak_span, _n: i32) {
    obs(obj).up();
}
unsafe extern "C" fn d2_add_span_fail(ctx: *mut ak_dec_ctx, obj: *mut c_void, _t: i64, _e: *const ak_span, _n: i32) {
    let o = obs(obj);
    o.up();
    if !o.failed {
        host_fail(ctx as *mut c_void, o);
    }
}
unsafe extern "C" fn d2_add_map(_c: *mut ak_dec_ctx, obj: *mut c_void, _t: i64, _e: *const ak_dfix_TaskOptionsOptionsEntry, _n: i32) {
    obs(obj).up();
}

fn m2_vt(
    new: unsafe extern "C" fn(*mut ak_dec_ctx, *mut c_void) -> i64,
    add_parent: unsafe extern "C" fn(*mut ak_dec_ctx, *mut c_void, i64, *const ak_span, i32),
) -> ak_dvt_ListTasksDetailedResponse {
    ak_dvt_ListTasksDetailedResponse {
        apply: Some(d_apply_m2),
        new_tasks: Some(new),
        apply_tasks: Some(d2_apply_el),
        add_tasks_parent_task_ids: Some(add_parent),
        add_tasks_data_dependencies: Some(d2_add_span),
        add_tasks_expected_output_ids: Some(d2_add_span),
        add_tasks_retry_of_ids: Some(d2_add_span),
        add_tasks_options_options: Some(d2_add_map),
    }
}

fn dec_m2(name: &str, vt: ak_dvt_ListTasksDetailedResponse) -> bool {
    let c = Ctx::new();
    let mut o = Obs::default();
    let b = m2_bytes();
    let rc = unsafe {
        ak_decode_ListTasksDetailedResponse(c.dec, &mut o as *mut Obs as *mut c_void, b.as_ptr(), b.len(), &vt)
    };
    verdict(name, rc as i64, &o, if o.tok_neg { "(token -1, no ak_fail)" } else { "" })
}

fn main() {
    println!("# FIX-PLAN WP4 item 7 / R-D6: the sticky error slot, ABI v1 section 5");
    println!("# rule: rc < 0, and zero upcalls after the host's ak_fail (apply included)");
    let mut all = true;
    all &= enc_m1_loop_fails();
    all &= enc_m2_inner_loop_fails();
    all &= dec_m1_add_fails();
    all &= dec_m2("D2 decode M2, new_tasks fails", m2_vt(d2_new, d2_add_span));
    all &= dec_m2("D2b decode M2, inner add (parent_task_ids) fails", m2_vt(d2_new_ok, d2_add_span_fail));
    all &= dec_m1_unknown_fails();
    all &= dec_m2("D4 decode M2, new_tasks returns token -1", m2_vt(d2_new_neg, d2_add_span));
    println!("{}", if all { "ALL PASS" } else { "SOME FAIL" });
    std::process::exit(if all { 0 } else { 1 });
}
