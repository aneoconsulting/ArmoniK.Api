//! FIX-PLAN D20: is the `utf8_skip` mask this build's binding passes the one intended?
//!
//! Decodes one `ListResultsResponse` whose single result's `session_id` is malformed UTF-8
//! (0xFF 0x41; the wire is otherwise well formed) through this build's generated binding,
//! the codec suite's core-ffi decode path. The committed binding passes `utf8_skip = 0`, so
//! the core must REJECT it (AK_ERR_TRANSCODE); `gen/d20_bench.sh`'s harness-only skip-all
//! build passes every bit set, so the core must ACCEPT it and deliver the two bytes as they
//! are. Exit 0 iff the outcome is the one `AK_D20_EXPECT` (reject | accept, default reject)
//! names: the first hypothesis about a variant that measures like another is that it is
//! not running.
fn main() {
    let want = std::env::var("AK_D20_EXPECT").unwrap_or_else(|_| "reject".into());
    let ctx = harness::arms::core_ffi_arm::Ctx::new();
    // results (1) { session_id (1) = FF 41 }
    let bad = [0x0Au8, 0x04, 0x0A, 0x02, 0xFF, 0x41];
    let r = harness::generated::binding::decode_with_list_results_response(ctx.dec, &bad);
    let got = match &r {
        Ok(v) => {
            println!("d20_toggle: ACCEPTED, {} result(s)", v.results.len());
            "accept"
        }
        Err(e) => {
            println!("d20_toggle: REJECTED, code {e} (AK_ERR_TRANSCODE = {})", ak_abi::AK_ERR_TRANSCODE);
            "reject"
        }
    };
    println!("d20_toggle: expected {want}: {}", if got == want { "ok" } else { "WRONG" });
    std::process::exit(if got == want { 0 } else { 1 });
}
