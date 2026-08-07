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

/// The property names one alternative of an `anyOf` declares.
fn alternative_properties(alternative: &serde_json::Value) -> Vec<String> {
    let mut names = Vec::new();
    property_names(alternative, &mut names);
    names
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
        "CaCert",
        "AllowUnsafeConnection",
        "OverrideTargetName",
        "ConnectTimeout",
        "Timeout",
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
        "Proxy",
        "ProxyUsername",
        "ProxyPassword",
    ] {
        assert!(names.iter().any(|name| name == option), "missing {option}");
    }

    // The PKCS#12 identity is not an option yet: the schema must not promise it before the code
    // reads it.
    assert!(
        !names.iter().any(|name| name == "CertP12"),
        "CertP12 is not read"
    );
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
    ] {
        assert!(
            !names.iter().any(|name| name == unprefixed),
            "`{unprefixed}` appears unprefixed"
        );
    }
}

#[test]
fn the_identity_alternatives_are_an_any_of() {
    // The identity comes as both PEM halves or not at all: the schema has to spell the
    // alternatives rather than flatten them into one bag of optional fields.
    let schema = schema();
    let mut found = Vec::new();
    any_ofs(&schema, &mut found);

    let identity = found.iter().find(|alternatives| {
        alternatives.iter().any(|alternative| {
            let names = alternative_properties(alternative);
            names.iter().any(|name| name == "CertPem") && names.iter().any(|name| name == "KeyPem")
        })
    });
    assert!(
        identity.is_some(),
        "no anyOf alternative declares CertPem and KeyPem: {schema:#}"
    );
}

#[test]
fn the_proxy_alternatives_are_an_any_of() {
    // Credentials come one of two ways, written into the URL or through the dedicated fields;
    // deserialisation refuses a mix, so the schema spells the two shapes.
    let schema = schema();
    let mut found = Vec::new();
    any_ofs(&schema, &mut found);

    let proxy = found.iter().find(|alternatives| {
        let with_fields = alternatives.iter().any(|alternative| {
            let names = alternative_properties(alternative);
            names.iter().any(|name| name == "ProxyUsername")
        });
        let embedded = alternatives.iter().any(|alternative| {
            let names = alternative_properties(alternative);
            names.iter().any(|name| name == "Proxy")
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
