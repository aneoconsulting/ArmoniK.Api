//! Golden tests for the JSON schemas of the configuration.
//!
//! The committed files are what a consumer (e.g. a C# options-class generator) reads without
//! building this crate, so a drift between them and the code has to fail loudly here.

#![cfg(feature = "schema")]

/// Compare `generated` against the file at `path`, or rewrite the file when `UPDATE_SCHEMA` is
/// set, so a committed schema can only ever be this test's own output.
fn assert_matches_committed(generated: &schemars::Schema, path: &str) {
    let generated = serde_json::to_value(generated).expect("a schema serialises to JSON");

    if std::env::var_os("UPDATE_SCHEMA").is_some() {
        let mut pretty = serde_json::to_string_pretty(&generated).expect("a schema serialises");
        pretty.push('\n');
        std::fs::write(path, pretty).expect("write the schema file");
        return;
    }

    let committed = std::fs::read_to_string(path).unwrap_or_else(|error| {
        panic!(
            "could not read `{path}`: {error}; regenerate it with `UPDATE_SCHEMA=1 cargo test -p \
             armonik-transport --features schema --test schema`"
        )
    });
    let committed: serde_json::Value =
        serde_json::from_str(&committed).expect("the committed schema is JSON");

    assert_eq!(
        generated, committed,
        "`{path}` no longer matches the generated schema; regenerate it with `UPDATE_SCHEMA=1 \
         cargo test -p armonik-transport --features schema --test schema` and commit the result"
    );
}

#[test]
fn the_committed_flat_schema_matches_the_generated_one() {
    // The flat option vocabulary, exactly as deserialisation accepts it: what a C# options-class
    // generator consumes.
    assert_matches_committed(
        &schemars::schema_for!(armonik_transport::HttpConfig),
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/schema/http_config.flat.schema.json"
        ),
    );
}

#[test]
fn the_committed_structured_schema_matches_the_generated_one() {
    // The config's own shape: nested thematic units, and the TLS identity as a `oneOf` of its
    // variants.
    assert_matches_committed(
        &armonik_transport::HttpConfig::structured_schema(),
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/schema/http_config.schema.json"
        ),
    );
}
