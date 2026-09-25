//! FIX-PLAN WP4 item 1 / finding R-D1: reproduction driver for the length-varint
//! wrap and its three claimed consequences, plus the R-D9 u32-truncation item.
//!
//! One case per process, selected by argv[1], so a hang or an abort in one case does
//! not hide the others. The shell driver (`gen/rd1_repro.sh`) runs each under `timeout`.
//!
//! The claims (R-D1), each driven through BOTH the C ABI (`ak_decode_*`, a real shared
//! boundary) and the core-native path (the same traversal emitted into the host, no
//! boundary):
//!   a  unknown-field skip whose length wraps: `pos` moves backwards, the root loop
//!      re-reads the same field forever (a hang).
//!   b  a string span with len 0xFFFF_FFFF is produced and the trailing flush!()/apply
//!      delivers collected groups AFTER a decode error, so a host reads out of bounds.
//!   c  a nested-message length makes `&buf0[off..off+n]` panic inside `extern "C"`.
//!   u32  a buffer longer than u32::MAX truncates span offsets/lengths to u32 (R-D9).
//!
//! The ffi (b) callbacks record span integers only and never dereference them, so the
//! bad length is OBSERVED rather than followed off the end of the buffer.

use std::ffi::c_void;

use ak_abi::{
    ak_dec_ctx, ak_dfix_ListResultsResponse, ak_dfix_ResultRaw, ak_dvt_ListResultsResponse,
    AK_ABI_VERSION,
};
use harness::arms::core_ffi_arm::Ctx;

/// A minimal-length base-128 varint (may be non-canonical / over-long on purpose).
fn varint(mut n: u64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let b = (n & 0x7f) as u8;
        n >>= 7;
        if n != 0 {
            out.push(b | 0x80);
        } else {
            out.push(b);
            break;
        }
    }
    out
}

/// A 10-byte over-long varint carrying the full 64-bit value, so the length is close to
/// 2^64 and the decoder's `pos + n` overflows.
fn varint10(n: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(10);
    for i in 0..10u32 {
        let mut b = ((n >> (7 * i)) & 0x7f) as u8;
        if i < 9 {
            b |= 0x80;
        }
        out.push(b);
    }
    out
}

fn tag(field: u32, wire: u32) -> Vec<u8> {
    varint(((field as u64) << 3) | wire as u64)
}

// ---- the finding's literal input for claim (a): field 15, wire 2, length ~2^64.
fn input_a() -> Vec<u8> {
    vec![0x7A, 0xF5, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01]
}

// ---- claim (b): one VALID results element, then a field whose length wraps so the
//      loop errors; the trailing flush!()/apply must not still deliver the element.
//      results element body: session_id="hi" (tag 1, wire 2, len 2).
fn input_b_delivery_after_error() -> Vec<u8> {
    let mut elem = Vec::new();
    elem.extend(tag(1, 2)); // ResultRaw.session_id
    elem.extend(varint(2));
    elem.extend(b"hi");
    let mut buf = Vec::new();
    buf.extend(tag(1, 2)); // ListResultsResponse.results
    buf.extend(varint(elem.len() as u64));
    buf.extend(&elem);
    // Now an unknown field (15, wire 2) whose length wraps 2^64: on unfixed code its
    // len_body passes the bounds check and moves pos, on fixed code it sets TRUNCATED.
    buf.extend(tag(15, 2));
    buf.extend(varint10(u64::MAX - 3)); // ~2^64
    buf
}

// ---- claim (b), the 0xFFFF_FFFF span: a results element whose session_id length wraps
//      so that pos+n overflows to <= len and the returned span carries len 0xFFFF_FFFF.
//      Element body = tag(1,2) then a 10-byte varint for a value whose low 32 bits are
//      0xFFFF_FFFF and whose pos+n wraps back inside the element buffer.
fn input_b_bad_span() -> Vec<u8> {
    // element buffer will be [tag(1,2)=0x0A][10 bytes varint]; after the varint es.pos = 11.
    // Pick n so that (11 + n) mod 2^64 <= element_len (11) and (n as u32) == 0xFFFF_FFFF.
    // n = 2^64 - 11 makes (11+n) mod 2^64 = 0; low 32 bits of (2^64-11) = 0xFFFF_FFF5.
    // To get low32 = 0xFFFF_FFFF with a small wrap, use n = 0xFFFF_FFFF_FFFF_FFFF (=2^64-1):
    // (11 + n) mod 2^64 = 10 <= 11 -> passes; n as u32 = 0xFFFF_FFFF. es.pos becomes 10.
    let n: u64 = u64::MAX;
    let mut elem = Vec::new();
    elem.extend(tag(1, 2)); // session_id, wire 2
    elem.extend(varint10(n));
    let mut buf = Vec::new();
    buf.extend(tag(1, 2)); // results
    buf.extend(varint(elem.len() as u64));
    buf.extend(&elem);
    buf
}

// ---- claim (c): a nested message length that wraps. results element (field 1, wire 2)
//      whose OWN length varint wraps 2^64, so `&buf0[off..off+n]` is built with a huge n.
fn input_c() -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend(tag(1, 2)); // results, wire 2 -> len_body then &buf0[off..off+n]
    buf.extend(varint10(u64::MAX)); // n = 2^64-1
    // some trailing bytes so pos+n can wrap to <= len for the element slice
    buf.extend([0u8; 8]);
    buf
}

// ---- the observing vtable: records span integers, never dereferences.
struct Obs {
    n_elems: usize,
    first_session_len: u32,
    first_session_off: u32,
    apply_called: bool,
}

unsafe extern "C" fn obs_apply(
    _ctx: *mut ak_dec_ctx,
    obj: *mut c_void,
    _fix: *const ak_dfix_ListResultsResponse,
) {
    let o = &mut *(obj as *mut Obs);
    o.apply_called = true;
}

unsafe extern "C" fn obs_add(
    _ctx: *mut ak_dec_ctx,
    obj: *mut c_void,
    _tok: i64,
    elems: *const ak_dfix_ResultRaw,
    n: i32,
) {
    let o = &mut *(obj as *mut Obs);
    if o.n_elems == 0 && n > 0 {
        let e = &*elems; // reading the ak_span integers, NOT the bytes they point at
        o.first_session_off = e.session_id.off;
        o.first_session_len = e.session_id.len;
    }
    o.n_elems += n as usize;
}

fn drive_ffi(label: &str, buf: &[u8]) {
    let ctx = Ctx::new();
    let mut obs = Obs {
        n_elems: 0,
        first_session_len: 0,
        first_session_off: 0,
        apply_called: false,
    };
    let vt = ak_dvt_ListResultsResponse {
        apply: Some(obs_apply),
        add_results: Some(obs_add),
    };
    let rc = unsafe {
        ak_abi::ak_decode_ListResultsResponse(
            ctx.dec.list_results_response,
            &mut obs as *mut _ as *mut c_void,
            buf.as_ptr(),
            buf.len(),
            &vt,
        )
    };
    println!(
        "[{label}] ffi   rc={rc} elems={} apply_called={} session_id.off={} session_id.len={} (0x{:08X})",
        obs.n_elems, obs.apply_called, obs.first_session_off, obs.first_session_len, obs.first_session_len
    );
}

fn drive_native(label: &str, buf: &[u8]) {
    // core-native is not extern "C": a panic here is a normal Rust panic (exit 101),
    // an out-of-bounds slice is still a defect, and a hang is still a hang.
    let r = facade::generated::core_native::decode_list_results_response(buf);
    match r {
        Ok(v) => println!("[{label}] native ok elems={} (Ok)", v.results.len()),
        Err(e) => println!("[{label}] native rc={e} (Err)"),
    }
}

fn main() {
    assert_eq!(
        harness::abi_version(),
        AK_ABI_VERSION,
        "codec ABI mismatch"
    );
    let case = std::env::args().nth(1).unwrap_or_default();
    match case.as_str() {
        "a-ffi" => drive_ffi("a", &input_a()),
        "a-native" => drive_native("a", &input_a()),
        "b-err-ffi" => drive_ffi("b-err", &input_b_delivery_after_error()),
        "b-err-native" => drive_native("b-err", &input_b_delivery_after_error()),
        "b-span-ffi" => drive_ffi("b-span", &input_b_bad_span()),
        "b-span-native" => drive_native("b-span", &input_b_bad_span()),
        "c-ffi" => drive_ffi("c", &input_c()),
        "c-native" => drive_native("c", &input_c()),
        // FIX-PLAN WP5 step 1: a decode error TWO levels below a group root. The old
        // emitter named every nested reader `cd`, so at depth two the propagation line was
        // `if cd.err != 0 { cd.err = cd.err; }` and the error vanished: the truncated
        // Duration below (tasks[0].options.max_duration.seconds: a varint key with no
        // value) decoded as a success. Must be an error in both arms.
        "nested2-ffi" | "nested2-native" => {
            let mut dur = Vec::new();
            dur.extend(tag(1, 0)); // Duration.seconds, varint -- and then nothing: truncated
            let mut opts = Vec::new();
            opts.extend(tag(2, 2)); // TaskOptions.max_duration
            opts.extend(varint(dur.len() as u64));
            opts.extend(&dur);
            let mut task = Vec::new();
            task.extend(tag(10, 2)); // TaskDetailed.options
            task.extend(varint(opts.len() as u64));
            task.extend(&opts);
            let mut buf = Vec::new();
            buf.extend(tag(1, 2)); // ListTasksDetailedResponse.tasks
            buf.extend(varint(task.len() as u64));
            buf.extend(&task);
            if case == "nested2-ffi" {
                let ctx = Ctx::new();
                let r = harness::generated::binding::decode_with_list_tasks_detailed_response(ctx.dec, &buf);
                println!("[nested2] ffi   {:?} (must be Err)", r.as_ref().map(|_| ()).map_err(|e| *e));
            } else {
                let r = facade::generated::core_native::decode_list_tasks_detailed_response(&buf);
                println!("[nested2] native {:?} (must be Err)", r.as_ref().map(|_| ()).map_err(|e| *e));
            }
        }
        "u32-ffi" => {
            // R-D9: a buffer longer than u32::MAX. We pass a small REAL buffer but claim a
            // length above u32::MAX. On unfixed code there is no entry guard, so the reader
            // runs off the buffer (UB / SIGSEGV). On fixed code the entry rejects it with
            // AK_ERR_LIMIT before any access. Kept behind its own case so its crash cannot
            // be confused with a-c.
            let real = input_a();
            let ctx = Ctx::new();
            let fake_len = (u32::MAX as usize) + 1;
            let mut obs = Obs {
                n_elems: 0,
                first_session_len: 0,
                first_session_off: 0,
                apply_called: false,
            };
            let vt = ak_dvt_ListResultsResponse {
                apply: Some(obs_apply),
                add_results: Some(obs_add),
            };
            let rc = unsafe {
                ak_abi::ak_decode_ListResultsResponse(
                    ctx.dec.list_results_response,
                    &mut obs as *mut _ as *mut c_void,
                    real.as_ptr(),
                    fake_len,
                    &vt,
                )
            };
            println!("[u32] ffi rc={rc} (AK_ERR_LIMIT is -5; a clean reject means the guard is present)");
        }
        other => {
            eprintln!("unknown case {other:?}");
            std::process::exit(2);
        }
    }
}
