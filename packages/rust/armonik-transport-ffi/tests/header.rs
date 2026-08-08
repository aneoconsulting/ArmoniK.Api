//! Checks on the generated C header, which is a committed artefact rather than a build output.
//!
//! `build.rs` regenerates `include/armonik_transport_ffi.h` on every build, so these run against
//! whatever the current sources produce.

const HEADER: &str = include_str!("../include/armonik_transport_ffi.h");

/// Names that legitimately appear as `#define` without the `AK_` prefix.
const ALLOWED_UNPREFIXED: &[&str] = &["ARMONIK_TRANSPORT_FFI_H"];

#[test]
fn every_macro_is_namespaced() {
    // A C header has no modules. `#define OK 0` would collide with a great deal of existing code,
    // so every macro this header defines carries the `AK_` prefix. The names are written that way in
    // the sources; this test is what stops one added later from arriving bare.
    let offenders: Vec<&str> = HEADER
        .lines()
        .filter_map(|line| line.strip_prefix("#define "))
        .filter_map(|rest| rest.split_whitespace().next())
        // Function-like macros would carry their parameter list; none exist today, but split on `(`
        // so one appearing later is still checked by name.
        .map(|name| name.split('(').next().unwrap_or(name))
        .filter(|name| !name.starts_with("AK_"))
        .filter(|name| !ALLOWED_UNPREFIXED.contains(name))
        .collect();

    assert!(
        offenders.is_empty(),
        "these macros need an `AK_` prefix: {offenders:?}"
    );
}

#[test]
fn every_entry_point_is_declared() {
    // The header is what a caller's own declarations are written against, so an entry point missing
    // from it is a function no caller can reach. Only `#[no_mangle] pub extern "C"` items are
    // emitted, so this catches one losing its attribute as much as a generation failure.
    for symbol in [
        "ak_abi_version",
        "ak_bytes_release",
        "ak_client_create",
        "ak_client_release",
    ] {
        assert!(
            HEADER.contains(symbol),
            "`{symbol}` is missing from the generated header"
        );
    }
}

#[test]
fn every_type_of_the_contract_is_declared() {
    // A type reaches the header only when a signature mentions it, and these two are part of the
    // contract without being an argument to anything yet. Dropping them would leave the preamble
    // describing a shape the header does not define.
    for declaration in [
        "typedef struct ak_bytes {",
        "typedef struct ak_bytes_in {",
        "enum ak_status",
        // Opaque: a handle is an address the caller passes back, and its fields are none of a
        // caller's business. The typedef is what lets one be declared at all.
        "typedef struct ak_client ak_client;",
    ] {
        assert!(
            HEADER.contains(declaration),
            "`{declaration}` is missing from the generated header"
        );
    }
}

#[test]
fn every_result_code_is_declared_with_its_value() {
    // A caller compares against these by name, and switches on them. One that never reaches the
    // header is a code the other side has to hard-wire as a number.
    for (name, value) in [
        ("AK_OK", 0),
        ("AK_NULL_ARGUMENT", -1),
        ("AK_INVALID_UTF8", -2),
        ("AK_INVALID_CONFIG", -3),
        ("AK_CONNECTION_FAILED", -4),
        ("AK_INVALID_HANDLE", -5),
        ("AK_INVALID_STATE", -6),
        ("AK_INTERNAL", -8),
        ("AK_INTERNAL_PANIC", -9),
        ("AK_CANCELLED", -10),
        ("AK_TIMEOUT", -11),
        ("AK_TRANSPORT", -12),
    ] {
        assert!(
            HEADER.contains(&format!("  {name} = {value},")),
            "`{name} = {value}` is missing from the generated header"
        );
    }

    assert!(
        HEADER.contains("#define AK_ABI_VERSION "),
        "the ABI revision is missing from the generated header"
    );
}

#[test]
fn no_code_claims_the_reserved_value() {
    // -7 is reserved. A code that took it would be indistinguishable, to a caller written against an
    // earlier revision, from one it was already treating as unknown.
    assert!(!HEADER.contains("= -7,"), "-7 is reserved");
}

#[test]
fn the_contract_the_signatures_cannot_carry_is_spelled_out() {
    // These are the rules a caller cannot infer from the signatures, and getting any of them wrong
    // is a memory bug, a hang or a wrong answer rather than a compile error. If the preamble is ever
    // trimmed, this is the reminder that the contract went with it.
    for phrase in [
        // Result codes.
        "AK_OK is 0 and means success; every failure is negative",
        "The value -7 is reserved and never returned",
        // Ownership, in both directions.
        "exactly one `ak_bytes_release`",
        "its `owner` is opaque, so never",
        "a view into memory this library does not own",
        // The blob encoding.
        "uint32 count",
        "NATIVE byte order",
        "kept in the order they were given",
        // Handles, which a caller has to know about before it hands one to a thread pool.
        "Handles are reference-counted, and thread-safe",
        // The reactor's three rules.
        "Nothing arrives unarmed",
        "COMPLETED exactly once, and last",
        "Never a callback during an inbound call",
        // The borrowed payload, whose cost is a use-after-free.
        "BORROWED for the duration of the invocation",
        // What a callback may not do.
        "must not block",
        "must not throw",
        "must not re-enter",
        // No promised order between independent events.
        "Two simultaneous events have no promised order",
        // And what a version mismatch may and may not mean.
        "The ABI is additive only",
        "What may never change",
    ] {
        assert!(
            HEADER.contains(phrase),
            "the header no longer documents {phrase:?}"
        );
    }
}

#[test]
fn nothing_in_the_header_says_what_it_was_generated_from() {
    // The cleaning pass in `build.rs`. Documentation written for one language reads as a leak in
    // another: a link that resolves to nothing, a heading from a convention this reader does not
    // follow, a path naming a module this reader cannot see. Whoever reads this file has a C
    // compiler and no way to look any of that up, so none of it may survive.
    for marker in [
        // Documentation links, and the module paths inside them.
        "[`",
        "`]",
        "::",
        // A section heading of the source language's documentation format.
        "# Safety",
        // The word for a source tree, where a contract has a library.
        "crate",
        // Types and modules that exist only on the other side.
        "Bytes",
        "armonik_transport",
        // And the language itself.
        "Rust",
        "rust",
    ] {
        assert!(
            !HEADER.contains(marker),
            "the header still carries {marker:?}, which the cleaning pass should have taken out"
        );
    }
}

#[test]
fn the_safety_requirements_survive_the_cleaning() {
    // Taking the heading out must not take the paragraph under it: what a caller has to guarantee is
    // exactly the part of the documentation a C caller most needs.
    assert!(
        HEADER.contains("Safety:"),
        "the safety requirements lost their label"
    );
    assert!(
        HEADER.contains("The zeroed value is always safe to pass here"),
        "the safety requirements lost their text"
    );
}
