//! A brand-new encode context encodes correctly with NO prior reset (ABI v1 does not require
//! one before the first encode). Owner-approved core fix (2026-10-03): since the framed default
//! (e8fe14868) `ak_enc_ctx_new` set `head = 5` but did not lay the 5-byte headroom down, so the
//! first encode on a fresh context came out 5 bytes short until an `ak_enc_reset`.
//!
//! The control: `ak_enc_ctx_new`, `ak_encode_ListResultsResponse` (page 7, total 3, no
//! results) with no reset, `ak_enc_take`: the bytes must equal prost's encoding of the same
//! value. Then the same context reset and encoded again (the path every host already took).
//! Exit 1 on any mismatch.
use ak_abi::*;
use std::ffi::c_void;

fn take(ctx: *mut ak_enc_ctx) -> Vec<u8> {
    let (mut p, mut n) = (std::ptr::null::<u8>(), 0usize);
    let rc = unsafe { ak_enc_take(ctx, &mut p, &mut n) };
    assert_eq!(rc, AK_OK, "ak_enc_take");
    if n == 0 { Vec::new() } else { unsafe { std::slice::from_raw_parts(p, n) }.to_vec() }
}

fn main() {
    // ak_init, as every host does before its first context.
    let _init = harness::arms::core_ffi_arm::Ctx::new();
    let want = prost::Message::encode_to_vec(&shapes_prost::shapes::ListResultsResponse { results: vec![], page: 7, total: 3 });
    let fix = ak_efix_ListResultsResponse { page: 7, total: 3, presence: 0 };
    let vt = ak_evt_ListResultsResponse { loop_results: None };
    let ctx = unsafe { ak_enc_ctx_new() };
    let rc = unsafe { ak_encode_ListResultsResponse(std::ptr::null::<c_void>(), ctx, &vt, &fix) };
    let fresh = take(ctx);
    unsafe { ak_enc_reset(ctx) };
    let rc2 = unsafe { ak_encode_ListResultsResponse(std::ptr::null::<c_void>(), ctx, &vt, &fix) };
    let after = take(ctx);
    unsafe { ak_enc_ctx_free(ctx) };
    let ok = rc >= 0 && fresh == want && rc2 >= 0 && after == want;
    println!("fresh context, no reset: rc {rc}, {} B {:02x?}; prost {} B {:02x?}; after a reset: rc {rc2}, {} B  {}",
             fresh.len(), fresh, want.len(), want, after.len(), if ok { "PASS" } else { "FAIL" });
    std::process::exit(if ok { 0 } else { 1 });
}
