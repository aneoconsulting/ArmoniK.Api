//! Smoke tests for the JSON schema of the flat option vocabulary.
//!
//! The schema is generated, never committed: `examples/generate_schema.rs` prints it for the
//! consumer that wants a file. What is pinned here are the invariants a consumer builds against,
//! not the byte-for-byte rendering, so a `schemars` upgrade that moves a keyword around does not
//! break anything that was not actually promised.

#![cfg(feature = "schema")]

use armonik_transport::reexports::schemars;

/// The generated schema, as plain JSON.
fn schema() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(armonik_transport::HttpConfig))
        .expect("a schema serialises to JSON")
}

/// Every property name declared anywhere in the schema: top level, `anyOf`/`allOf` branches, and
/// `$defs` alike, so the assertions hold whatever nesting `schemars` chooses.
fn property_names(value: &serde_json::Value, names: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, child) in object {
                if key == "properties" {
                    if let serde_json::Value::Object(properties) = child {
                        names.extend(properties.keys().cloned());
                    }
                }
                property_names(child, names);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                property_names(item, names);
            }
        }
        _ => {}
    }
}

/// Every `anyOf` array anywhere in the schema.
fn any_ofs(value: &serde_json::Value, found: &mut Vec<Vec<serde_json::Value>>) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, child) in object {
                if key == "anyOf" {
                    if let serde_json::Value::Array(alternatives) = child {
                        found.push(alternatives.clone());
                    }
                }
                any_ofs(child, found);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                any_ofs(item, found);
            }
        }
        _ => {}
    }
}

#[test]
fn every_option_appears_under_its_flat_name() {
    // The whole vocabulary, spelled exactly as deserialisation reads it: this list is what a
    // consumer generating an options class builds against.
    let schema = schema();
    let mut names = Vec::new();
    property_names(&schema, &mut names);

    for option in [
        "Endpoint",
        "CertPem",
        "KeyPem",
        "CaCertPath",
        "CertP12",
        "CertP12Password",
        "AllowUnsafeConnection",
        "OverrideTargetName",
        "ConnectTimeout",
        "Timeout",
        "PoolIdleTimeout",
        "RateLimit",
        "TcpKeepalive",
        "TcpKeepaliveInterval",
        "TcpKeepaliveRetries",
        "TcpNagleAlgorithm",
        "Http2KeepAliveInterval",
        "Http2KeepAliveTimeout",
        "Http2KeepAliveWhileIdle",
        "Http2MaxHeaderListSize",
        "UserAgent",
        "MaxAttempts",
        "InitialBackOff",
        "MaxBackOff",
        "BackOffMultiplier",
        "ProxyAddress",
        "ProxyUsername",
        "ProxyPassword",
    ] {
        assert!(names.iter().any(|name| name == option), "missing {option}");
    }
}

#[test]
fn the_schema_promises_nothing_that_is_not_read() {
    // A consumer generates its options class from this, so an option the schema declares and
    // deserialisation ignores is a field that silently does nothing. The retryable status codes
    // are set programmatically and no option reads them.
    let schema = schema();
    let mut names = Vec::new();
    property_names(&schema, &mut names);

    assert!(
        !names.iter().any(|name| name == "RetryableStatusCodes"),
        "`RetryableStatusCodes` is not read"
    );
}

/// Every `description` anywhere in the schema.
fn descriptions(value: &serde_json::Value, found: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(object) => {
            if let Some(serde_json::Value::String(description)) = object.get("description") {
                found.push(description.clone());
            }
            for child in object.values() {
                descriptions(child, found);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                descriptions(item, found);
            }
        }
        _ => {}
    }
}

#[test]
fn a_description_names_an_option_the_way_a_document_spells_it() {
    // A description reaches a generated options class verbatim, so a Rust field name in it names
    // an option no deployment can set: `allow_unsafe_connection` is spelled `AllowUnsafeConnection`
    // everywhere a source writes it. The prefix matters as much as the casing, which is why the
    // rendering goes through the embeddings rather than through the field name alone.
    let schema = schema();
    let mut found = Vec::new();
    descriptions(&schema, &mut found);

    assert!(!found.is_empty(), "no descriptions at all: {schema:#}");
    for description in &found {
        for quoted in description.split('`').skip(1).step_by(2) {
            let rust_field = quoted.starts_with(|first: char| first.is_ascii_lowercase())
                && quoted.contains('_')
                && quoted
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
            assert!(
                !rust_field,
                "`{quoted}` is a Rust field name, not an option: {description}"
            );
        }
    }
    assert!(
        found
            .iter()
            .any(|description| description.contains("`AllowUnsafeConnection`")),
        "no description resolved a link to its flat name: {found:#?}"
    );
}

#[test]
fn no_description_carries_a_rust_doc_link() {
    // A description reaches a generated options class verbatim. An intra-doc link resolves to
    // nothing there and its brackets read as broken markup, so the schema keeps the prose and
    // drops the path.
    let schema = schema();
    let mut found = Vec::new();
    descriptions(&schema, &mut found);

    assert!(!found.is_empty(), "no descriptions at all: {schema:#}");
    for description in &found {
        assert!(
            !description.contains("[`"),
            "a rustdoc link survives: {description}"
        );
        assert!(
            !description.contains("crate::"),
            "a crate path survives: {description}"
        );
    }
}

#[test]
fn the_prefixed_groups_keep_their_flat_spellings() {
    // `serde_with::with_prefix!` has no `schemars` integration, so without the prefix helper the
    // schema would list each group under its unprefixed field names.
    let schema = schema();
    let mut names = Vec::new();
    property_names(&schema, &mut names);

    for unprefixed in [
        "Keepalive",
        "KeepaliveInterval",
        "KeepaliveRetries",
        "NagleAlgorithm",
        "KeepAliveInterval",
        "KeepAliveTimeout",
        "KeepAliveWhileIdle",
        "MaxHeaderListSize",
        "Address",
        "Username",
        "Password",
    ] {
        assert!(
            !names.iter().any(|name| name == unprefixed),
            "`{unprefixed}` appears unprefixed"
        );
    }
}

/// The property names one alternative of an `anyOf` requires.
fn required_names(alternative: &serde_json::Value) -> Vec<String> {
    alternative
        .get("required")
        .and_then(serde_json::Value::as_array)
        .map(|names| {
            names
                .iter()
                .filter_map(|name| name.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn the_identity_alternatives_are_an_any_of() {
    // The identity comes as both PEM halves, as a PKCS#12 bundle, or not at all: the schema has to
    // spell the alternatives rather than flatten them into one bag of optional fields. Each shape
    // declares every identity option, since that is how deserialisation catches the two spellings
    // set at once, so what tells the shapes apart there and here alike is which ones they require.
    let schema = schema();
    let mut found = Vec::new();
    any_ofs(&schema, &mut found);

    let identity = found.iter().find(|alternatives| {
        let pem = alternatives.iter().any(|alternative| {
            let required = required_names(alternative);
            required.iter().any(|name| name == "CertPem")
                && required.iter().any(|name| name == "KeyPem")
        });
        let bundle = alternatives.iter().any(|alternative| {
            let required = required_names(alternative);
            let mut names = Vec::new();
            property_names(alternative, &mut names);
            required.iter().any(|name| name == "CertP12")
                && !required.iter().any(|name| name == "CertPem")
                && names.iter().any(|name| name == "CertP12Password")
        });
        pem && bundle
    });
    assert!(
        identity.is_some(),
        "no anyOf spells the PEM pair and the PKCS#12 bundle as alternatives: {schema:#}"
    );
}

#[test]
fn the_proxy_alternatives_are_an_any_of() {
    // Credentials come one of two ways, written into the URL or through the dedicated fields;
    // deserialisation refuses a mix, so the schema spells the two shapes rather than flattening
    // them into one bag a consumer could fill both halves of.
    let schema = schema();
    let mut found = Vec::new();
    any_ofs(&schema, &mut found);

    let proxy = found.iter().find(|alternatives| {
        let with_fields = alternatives.iter().any(|alternative| {
            let mut names = Vec::new();
            property_names(alternative, &mut names);
            names.iter().any(|name| name == "ProxyUsername")
        });
        let embedded = alternatives.iter().any(|alternative| {
            let mut names = Vec::new();
            property_names(alternative, &mut names);
            names.iter().any(|name| name == "ProxyAddress")
                && !names.iter().any(|name| name == "ProxyUsername")
        });
        with_fields && embedded
    });
    assert!(
        proxy.is_some(),
        "no anyOf spells the two proxy credential shapes: {schema:#}"
    );
}

#[test]
fn no_default_survives_anywhere_in_the_schema() {
    // A Rust field's `Default` serialises in the field's own type, not in the option's text form:
    // `false`, or a `Duration` as `{"secs":..,"nanos":..}`, on an option whose schema type is
    // string. Each option states its default in prose instead. The walk is recursive because
    // `schemars` is free to nest, and a stripping transform that stops reaching a branch would
    // otherwise resurface those values unnoticed.
    fn defaults(value: &serde_json::Value, path: String, found: &mut Vec<String>) {
        match value {
            serde_json::Value::Object(object) => {
                for (key, child) in object {
                    if key == "default" {
                        found.push(format!("{path}/{key} = {child}"));
                    }
                    defaults(child, format!("{path}/{key}"), found);
                }
            }
            serde_json::Value::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    defaults(item, format!("{path}/{index}"), found);
                }
            }
            _ => {}
        }
    }

    // The unit types are public and generate their own schemas, so the guarantee has to hold for
    // each on its own, not only for the configuration that embeds them.
    for (name, schema) in [
        ("HttpConfig", schema()),
        (
            "TcpConfig",
            serde_json::to_value(schemars::schema_for!(armonik_transport::TcpConfig))
                .expect("a schema serialises to JSON"),
        ),
        (
            "Http2Config",
            serde_json::to_value(schemars::schema_for!(armonik_transport::Http2Config))
                .expect("a schema serialises to JSON"),
        ),
    ] {
        let mut found = Vec::new();
        defaults(&schema, String::new(), &mut found);

        assert!(
            found.is_empty(),
            "{name}'s schema still carries defaults: {found:#?}"
        );
    }
}
