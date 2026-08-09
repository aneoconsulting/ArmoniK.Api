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
//!
//! Being current is half of it. The other half is the ledger: every option the schema declares is
//! named in one of two lists here, applied or not applied, so an option added to the vocabulary
//! fails this crate's build until somebody decides which it is. That is what keeps an option from
//! reaching a caller's options class and then doing nothing at all.
//!
//! Both lists say what is true of this library as it stands, which is the only way a ledger is worth
//! reading. An option the client parses and holds but nothing acts on is not applied, and putting it
//! on the other list would turn "nobody has checked" into "a test says this is fine" - the one
//! failure a ledger exists to catch.

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

/// Every option this library applies, and what each one reaches.
///
/// Half of the ledger below. An option is here only if something in this library reads it and acts
/// on it: the note says what it reaches, so that "applied" is a claim a reader can go and check
/// rather than a name on a list. Carrying a value on the client is not applying it - an option
/// nothing acts on belongs in [`NOT_APPLIED`], however faithfully it was parsed.
const APPLIED: &[(&str, &str)] = &[
    (
        "Endpoint",
        "dialled by the connector, and the origin a request is addressed to",
    ),
    ("CertPem", "the client's certificate chain, for mTLS"),
    ("KeyPem", "the key of that chain"),
    ("CertP12", "the same identity as one PKCS#12 bundle"),
    ("CertP12Password", "opens that bundle"),
    (
        "CaCertPath",
        "read for the authority the server is verified against",
    ),
    (
        "AllowUnsafeConnection",
        "verifies no server certificate at all",
    ),
    (
        "OverrideTargetName",
        "moves the verified name and the origin off the endpoint",
    ),
    ("ConnectTimeout", "bounds opening a socket, and the tunnel"),
    ("PoolIdleTimeout", "closes an idle pooled connection"),
    ("TcpKeepalive", "the socket's keepalive"),
    ("TcpKeepaliveInterval", "between its probes"),
    ("TcpKeepaliveRetries", "before it gives up"),
    ("TcpNagleAlgorithm", "off is `TCP_NODELAY`"),
    ("Http2KeepAliveInterval", "between PING frames"),
    ("Http2KeepAliveTimeout", "waits for the PING to come back"),
    (
        "Http2KeepAliveWhileIdle",
        "PINGs a connection carrying no request",
    ),
    ("Http2MaxHeaderListSize", "bounds one request's headers"),
    (
        "ProxyAddress",
        "tunnelled through, under the connector's TLS",
    ),
    ("ProxyUsername", "authenticates that tunnel"),
    ("ProxyPassword", "likewise"),
];

/// Every option nothing in this library acts on, and why not.
///
/// The other half, and the one that has to stay honest for either to be worth anything. Two reasons
/// put an option here, and the note says which. Either the option belongs to a layer above this one,
/// or it is applied per request and nothing here sends a request: the client holds the value, and
/// holding a value changes no behaviour.
///
/// Whoever makes this library send a request moves the second group across, and the ledger fails
/// until they do. That is the ledger working, not a hole in it.
const NOT_APPLIED: &[(&str, &str)] = &[
    (
        "MaxAttempts",
        "a replay is a new request, and which failures are worth one is keyed on a `grpc-status` \
         this library never sees: it moves bytes, and the gRPC stack above it is what has a call to \
         retry",
    ),
    ("InitialBackOff", "the same schedule, for the same reason"),
    ("MaxBackOff", "likewise"),
    ("BackOffMultiplier", "likewise"),
    (
        "Timeout",
        "read onto the client as `ak_client::timeout`, and applied by whoever sends a request: \
         nothing below the sender can time a request out, and nothing here sends one",
    ),
    (
        "RateLimit",
        "read onto the client as the limiter in `ak_client::rate_limit`, whose permit is taken by \
         whoever sends a request; nothing here takes one",
    ),
    (
        "UserAgent",
        "read onto the client as `ak_client::user_agent`, and set on a request by whoever builds \
         one; nothing here builds one",
    ),
];

/// Every property name the schema declares, wherever it declares it: at the top level, inside an
/// `anyOf` or `allOf` branch, or under `$defs`.
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

/// The vocabulary, deduplicated: an alternative repeats the options it shares with its siblings.
fn vocabulary() -> Vec<String> {
    let schema: serde_json::Value =
        serde_json::from_str(COMMITTED).expect("the committed schema is JSON");
    let mut names = Vec::new();
    property_names(&schema, &mut names);
    names.sort_unstable();
    names.dedup();
    names
}

#[test]
fn every_option_of_the_vocabulary_is_either_applied_or_deliberately_not() {
    // The ledger. An option added to the vocabulary fails this until somebody decides which of the
    // two it is, which is the only way an option reaches a caller's options class and then does
    // nothing at all. Nothing here is allowed to be silently dropped.
    let undecided: Vec<String> = vocabulary()
        .into_iter()
        .filter(|option| {
            !APPLIED.iter().any(|(name, _)| name == option)
                && !NOT_APPLIED.iter().any(|(name, _)| name == option)
        })
        .collect();

    assert!(
        undecided.is_empty(),
        "these options are in the vocabulary and in neither list: {undecided:?}. Add each to \
         `APPLIED` with what it reaches, or to `NOT_APPLIED` with why this layer is the wrong one \
         to read it"
    );
}

#[test]
fn nothing_is_on_both_lists_and_nothing_is_on_a_list_twice() {
    let mut listed: Vec<&str> = APPLIED
        .iter()
        .chain(NOT_APPLIED)
        .map(|(name, _)| *name)
        .collect();
    let total = listed.len();
    listed.sort_unstable();
    listed.dedup();

    assert_eq!(
        listed.len(),
        total,
        "an option is listed more than once, so one of the two entries says nothing"
    );
}

#[test]
fn no_entry_of_either_list_has_left_the_vocabulary() {
    // The other direction, which is what catches an option renamed in the transport: an entry
    // naming nothing would go on satisfying the ledger while the option it stood for went unread.
    let vocabulary = vocabulary();
    let stale: Vec<&str> = APPLIED
        .iter()
        .chain(NOT_APPLIED)
        .map(|(name, _)| *name)
        .filter(|name| !vocabulary.iter().any(|option| option == name))
        .collect();

    assert!(
        stale.is_empty(),
        "these are listed but are no longer options: {stale:?}"
    );
}

#[test]
fn what_is_left_unapplied_is_the_retry_schedule_and_what_a_request_carries() {
    // The list spelled out, so that growing it is a decision somebody makes here rather than the
    // path of least resistance when an option turns out to be inconvenient. Two groups, and no
    // third: the retry schedule, which belongs to the gRPC stack above this library, and the three
    // options a request applies, which the client holds and nothing acts on because nothing here
    // sends a request. Shortening this list is the work; nothing may lengthen it quietly.
    let left_alone: Vec<&str> = NOT_APPLIED.iter().map(|(name, _)| *name).collect();

    assert_eq!(
        left_alone,
        [
            "MaxAttempts",
            "InitialBackOff",
            "MaxBackOff",
            "BackOffMultiplier",
            "Timeout",
            "RateLimit",
            "UserAgent",
        ],
        "an option outside those two groups is being left unapplied"
    );
}

#[test]
fn every_entry_says_what_it_does_or_why_it_does_not() {
    // A list of bare names would pass the ledger while telling a reviewer nothing. The note is the
    // part that can be argued with.
    for (name, note) in APPLIED.iter().chain(NOT_APPLIED) {
        assert!(!note.trim().is_empty(), "`{name}` carries no note");
    }
}
