//! The committed JSON schema of the option vocabulary a client is configured with.
//!
//! `include/http_config.schema.json` sits beside the C header and the C# declarations, and is the
//! third artefact of the same contract: the header says how to call `ak_client_create`, and this
//! says what may be inside the document handed to it.
//!
//! Unlike the other two it is not written by `build.rs`. It is generated from the transport's own
//! types, which a build script of this crate has no way to reach, so what keeps it current is the
//! comparison below rather than a rebuild. A change to the vocabulary therefore shows up as a diff
//! in a pull request, which is the point: whoever generates an options class in another language
//! reads this file, and a stale one is an option that silently does nothing.

use armonik_transport::reexports::schemars;

/// The artefact as it sits in the source tree.
const COMMITTED: &str = include_str!("../include/http_config.schema.json");

/// How the artefact is written, so the two are compared byte for byte rather than as JSON that
/// happens to mean the same thing: `examples/generate_schema.rs` prints it and nothing more.
const REGENERATE: &str =
    "cargo run -p armonik-transport --features schema --example generate_schema \
     > armonik-transport-ffi/include/http_config.schema.json";

/// The schema of the vocabulary as the transport describes it now.
fn generated() -> String {
    let schema = schemars::schema_for!(armonik_transport::HttpConfig);
    let mut text = serde_json::to_string_pretty(&schema).expect("a schema serialises to JSON");
    // `println!`'s newline, which is what makes the file end the way a text file does.
    text.push('\n');
    text
}

#[test]
fn the_committed_schema_is_the_one_the_transport_generates() {
    let generated = generated();

    // Compared as text rather than as parsed JSON: what a consumer reads is the file, so a
    // reordering that means the same thing is still a change to the artefact and belongs in the
    // diff. The first differing line is reported, because the whole document says little.
    let difference = COMMITTED
        .lines()
        .zip(generated.lines())
        .enumerate()
        .find(|(_, (committed, generated))| committed != generated);
    if let Some((line, (committed, generated))) = difference {
        panic!(
            "the committed schema is out of date at line {}:\n  committed: {committed}\n  \
             generated: {generated}\nregenerate it with:\n  {REGENERATE}",
            line + 1
        );
    }
    assert_eq!(
        COMMITTED.lines().count(),
        generated.lines().count(),
        "the committed schema is shorter or longer than the generated one; regenerate it with:\n  \
         {REGENERATE}"
    );
    assert_eq!(
        COMMITTED, generated,
        "the committed schema differs from the generated one only in line endings or in its final \
         newline; regenerate it with:\n  {REGENERATE}"
    );
}

#[test]
fn the_committed_schema_carries_no_carriage_returns() {
    // A `.gitattributes` pins the artefacts under `include/` to LF, on every platform, because a
    // checkout that introduced a CR here would make the comparison above fail for a reason that has
    // nothing to do with the vocabulary.
    assert!(
        !COMMITTED.contains('\r'),
        "the checkout translated the line endings of a pinned artefact"
    );
}
