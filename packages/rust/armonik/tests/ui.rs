//! Compile-fail suite for the `call` input diagnostics.
//!
//! `ServiceClient::call` takes one argument for four call kinds, and which inputs are valid depends
//! on the RPC. That is enforced by `IntoCall` having no impl for the wrong pairing, so what a caller
//! actually sees is the `#[diagnostic::on_unimplemented]` text on the trait -- the whole reason the
//! `M` marker parameter exists. Nothing else pins that text, and re-running the three misuse probes
//! by hand is how a regression in it shipped once: all three still errored, and the wording that
//! named the mistake was gone.
//!
//! Each case is a minimal `tests/ui/*.rs` paired with the exact `*.stderr` rustc prints for it.
//! Regenerate after a deliberate wording change with:
//!
//! ```sh
//! TRYBUILD=overwrite cargo test -p armonik --test ui
//! ```
//!
//! and read the diff: an unexpected line there is the review.
//!
//! # Toolchain sensitivity
//!
//! The snapshots are byte-compared, and rustc's rendering moves between channels, so they are
//! pinned against stable and skipped on nightly. `ARMONIK_UI=1` runs them anyway.

#![cfg(feature = "client")]

/// Whether this run should byte-compare the snapshots; see the toolchain note in the module docs.
fn snapshots_are_comparable() -> bool {
    if std::env::var_os("ARMONIK_UI").is_some_and(|value| value == "1") {
        return true;
    }
    !std::env::var("RUSTUP_TOOLCHAIN").is_ok_and(|toolchain| toolchain.starts_with("nightly"))
}

#[test]
fn ui() {
    if !snapshots_are_comparable() {
        eprintln!(
            "skipping the compile-fail snapshots: they are byte-compared and this is a nightly \
             toolchain (set ARMONIK_UI=1 to run them anyway)"
        );
        return;
    }
    trybuild::TestCases::new().compile_fail("tests/ui/*.rs");
}
