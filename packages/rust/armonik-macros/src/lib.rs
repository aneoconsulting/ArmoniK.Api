//! Internal macros for the [`armonik`](https://crates.io/crates/armonik) crate.
//!
//! An implementation detail of `armonik`, which pins it to an exact version. The grammar and the
//! emitted code carry no stability guarantee, and the expansions reference `armonik`-internal
//! paths, so they only work inside that crate.
//!
//! Tags, wire kinds, cardinalities and documentation all come from the descriptor set the `armonik`
//! build script compiles (`$OUT_DIR/descriptor.bin`), read at expansion time; any mismatch with the
//! Rust type is a compile error. Every expansion const-asserts a descriptor fingerprint, so a stale
//! expansion cannot survive a descriptor change.
//!
//! [`message`](macro@message) covers messages, [`oneof`](macro@oneof) the oneofs embedded in them,
//! [`enumeration`](macro@enumeration) proto enums, [`service`](macro@service) the per-service RPC
//! definitions. The `#[armonik(...)]` grammar lives in the [message
//! attributes](macro@message#attributes) and [enum attributes](macro@enumeration#attributes)
//! sections.

use proc_macro::TokenStream;
use quote::ToTokens;
use syn::parse_macro_input;

mod attrs;
mod client;
mod descriptor;
mod emit;
mod enumeration;
mod generator;
mod item;
mod matcher;
mod names;
mod plan;
mod resolve;
mod service;

use generator::Generator;
use item::Kind;
use proc_macro2::TokenStream as TokenStream2;
use syn::DeriveInput;

/// Implement `prost::Message` for an ArmoniK API type, validated against the
/// protobuf descriptors compiled by the `armonik` build script.
///
/// The macro's argument is the full proto name of the message the type stands
/// for, or several names for a unified type standing for identical messages,
/// which must agree on every field. Every validated type has one, so it is the
/// macro's argument rather than a key that could be left out; a
/// [`generic`](#generic) type gives none.
///
/// Tags, wire kinds and cardinalities come from the descriptor, never from the
/// source. Every disagreement with the proto message (unknown field, uncovered
/// proto field or oneof, kind or cardinality mismatch) is a spanned error
/// naming both sides. Proto enums go through
/// [`enumeration`](macro@enumeration).
///
/// An attribute macro rather than a derive so the item can be re-emitted with
/// the proto documentation injected: the type, its fields, its oneof variants
/// and their inlined fields get `#[doc]`s harvested from the protos' leading
/// comments, the same harvest `service!` does for services. Hand-written doc
/// comments follow them, as Rust-specific notes. The `#[armonik(...)]`
/// attributes are consumed and stripped.
///
/// Besides `prost::Message`, the expansion emits:
/// - a `Msg` impl, which the codec's blanket `ProtoField` impl picks up, so
///   the type composes as a field of other derived messages;
/// - a fingerprint const-assert that fails the build once the expansion goes
///   stale against a newer descriptor;
/// - under `cfg(test)`, the type's registration into `armonik`'s
///   `differential::registrations::REGISTRY`, with its `Normalize`
///   projection and harness hooks.
///
/// Derived types uphold the crate's zero-default invariant:
/// `Default::default()` is the proto zero value, so decoding an empty message
/// yields it. The differential harness enforces it.
///
/// # Shapes
///
/// **Plain struct**: [`message`](#message) names the proto message, and Rust
/// fields match proto fields by name ([`rename`](#rename) when they differ;
/// tuple structs must rename every field):
///
/// ```ignore
/// #[armonik_macros::message("armonik.api.grpc.v1.tasks.GetResultIdsResponse")]
/// #[derive(Debug, Clone, Default, PartialEq, Eq)]
/// pub struct Response {
///     #[armonik(inlined)]
///     pub task_results: HashMap<String, Vec<String>>,
/// }
/// ```
///
/// A proto field belonging to a oneof cannot be mapped alone: declare one Rust
/// field named after the *oneof*, typed as a [`oneof`](macro@oneof) enum.
///
/// **Whole-message enum**: on an enum, the type stands for a message whose
/// single oneof is inferred. Variants match oneof members by snake_cased name;
/// one attribute-less variant, at most, is the "no member set" case, and is the
/// one to mark `#[default]` since that is the proto zero:
///
/// ```ignore
/// #[armonik_macros::message("armonik.api.grpc.v1.Output")]
/// #[derive(Debug, Clone, Default, PartialEq, Eq)]
/// pub enum Output {
///     #[default]
///     Invalid,                      // no member set
///     #[armonik(present)]
///     Ok,                           // member `ok`, carried by presence
///     #[armonik(inlined)]
///     Error { details: String },    // member `error`, message fields inlined
/// }
/// ```
///
/// Variant payloads take three forms: a single tuple payload (`Variant(T)`), an
/// [`inlined`](#inlined) struct variant spreading the fields of a message member
/// (`Error` above; that member's message must not itself contain a oneof), or a
/// [`present`](#present) unit variant for a `bool` or empty-message member whose
/// only information is that it is set. Non-oneof fields of the message become
/// *siblings*: every variant, including the attribute-less one, is a struct
/// variant carrying all of them next to its own payload field.
///
/// One oneof of a *larger* message is a fragment rather than a message, and has
/// a macro of its own: [`oneof`](macro@oneof).
///
/// **Generic struct**: [`generic`](#generic) skips descriptor validation, since
/// a generic type cannot name one proto message. Every field carries an explicit
/// [`tag`](#tag), and the concrete instantiations are validated instead:
///
/// ```ignore
/// #[armonik_macros::message]
/// #[derive(Debug, Clone, Default, PartialEq, Eq)]
/// #[armonik(generic)]
/// pub struct Sort<T> {
///     #[armonik(tag = 1)]
///     pub field: T,
///     #[armonik(tag = 2)]
///     pub direction: SortDirection,
/// }
/// ```
///
/// # Attributes
///
/// Everything not declared through `#[armonik(...)]` is inferred from the
/// descriptor.
///
/// ## generic
///
/// `generic`, on a struct: skip descriptor validation, since a generic type
/// cannot name one proto message. Fields take [`tag`](#tag) and nothing else:
/// each instantiation is checked at its [`alias`](macro@alias) by comparing
/// every field's `ProtoField` shape, which a [`with`](#with) adapter has none
/// of. Combines with neither [`message`](#message) nor
/// [`transparent`](#transparent), both of which name the message this says it
/// does not have.
///
/// ## rename
///
/// `rename = "proto_name"`, on a field or variant: the proto field or oneof
/// member name when it differs from the Rust one (the default match is by
/// snake_cased name). Required on tuple-struct fields.
///
/// ## tag
///
/// `tag = N`, on a field of a [`generic`](#generic) struct, where it is
/// required and authoritative. Rejected anywhere else: a descriptor-validated
/// field takes its tag from the descriptor, and spelling one would restate it.
///
/// ## with
///
/// `with = "path::To::Adapter"`, on a field or single-payload tuple variant
/// (in sibling variants, on the member payload field): encode through a
/// `ProtoAdapter` instead of the type's `ProtoField` impl, for a relation to
/// the proto shape that is semantic rather than structural -- `ErrorAdapter`,
/// reading an empty error string as success, is the one site. It skips the
/// descriptor kind checks on purpose, so only the differential harness covers
/// it, `normalize_dynamic` included. Structural reshaping is
/// [`inlined`](#inlined)'s, and stays checked. Not accepted in
/// [`generic`](#generic) mode, whose only check is that comparison.
///
/// ## present
///
/// `present`, on a unit variant: the oneof member is carried by presence alone.
/// A `bool` member encodes `true` (an explicit `false` still selects the
/// variant), an empty-message member encodes an empty message.
///
/// ## inlined
///
/// `inlined`: a proto message gets no Rust type of its own; what it contains
/// lives directly at the annotated site, and is registered as absorbed. What
/// the site absorbs is read off its shape.
///
/// **On a struct variant**, the member message's fields, spread into the
/// variant: the leftover fields (the ones that are not the message's own
/// non-oneof fields) are the *member message's* fields, rather than one field
/// carrying the member whole.
///
/// ```ignore
/// #[armonik_macros::message("armonik.api.grpc.v1.Output")]
/// pub enum Output {
///     Invalid,
///     #[armonik(present)]
///     Ok,
///     #[armonik(inlined)]                // `details` is `Output.Error.details`,
///     Error { details: String },         // not an `Output.Error` carried whole
/// }
/// ```
///
/// Spelled rather than inferred: `Variant { token, request: T }` is genuinely
/// ambiguous between the two readings. Without `inlined`, one leftover field
/// carries the member and several are an error naming this key; with it, a
/// leftover matching no field of the member message is an error listing the
/// ones that exist.
///
/// On a struct variant, `inlined` combines with none of [`present`](#present),
/// [`with`](#with), or a message that has non-oneof fields. The last is the one
/// worth spelling out: every variant of such an enum carries those fields, so an
/// inlined member's own fields would land in the same variant, sharing a binding
/// namespace with tags drawn from two different messages. Carry the member whole
/// in a field of its own there.
///
/// **On a field, a tuple variant, or a member payload field**, the field's
/// message layer, unwrapped: a singular single-field wrapper
/// (`CreationStatusList { repeated CreationStatus creation_statuses = 1; }` as
/// `Vec<Status>`, through `Wrapper<Own, N>` with `N` from the descriptor). The
/// Rust type is shape-checked against the unwrapped form, unlike
/// [`with`](#with), which is trusted.
///
/// A *repeated* key/value pair message takes no key at all, because nothing has
/// to be unwrapped: `map<K, V>` is encoded as `repeated Entry { K key = 1; V
/// value = 2; }`, so a `HashMap` field already carries those bytes through its
/// own `ProtoField`. A repeated two-field message at tags 1 and 2 is therefore
/// shape-checked against *either* representation, `Vec<Entry>` or a map of the
/// pair's two fields, each checked whole (a `repeated` value stays repeated, and
/// a key names its enum), and the Rust type is what picks: `IdStatus { string
/// task_id = 1; TaskStatus status = 2; }` as `HashMap<String, TaskStatus>`,
/// which drops entry order and collapses duplicate keys. Only that reading
/// leaves the pair without a Rust type, so only there is it registered as
/// absorbed.
///
/// ## transparent
///
/// `transparent`, on a single-field struct: the type delegates its whole
/// `prost::Message` impl to that one field, so it is wire-identical to the
/// field's message and can stand for a whole RPC message (the struct sibling of
/// the [`enumeration`](macro@enumeration#transparent) wrapper mode). Name the
/// inner message with [`message`](#message); the field is not matched against
/// the descriptor. Typically wraps a shared message per RPC site (e.g.
/// `struct Request { filter: TaskFilter }`), keeping request types injective
/// over RPCs.
///
#[proc_macro_attribute]
pub fn message(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attr = TokenStream2::from(attr);
    expand(input, |input, index, generator| {
        let Some(names) = proto_names(attr, generator) else {
            return;
        };
        let ir = resolve::resolve_message(input, index, &names, generator);
        // Read before `rewrite` mutates the item: `anchors` has to see the `#[armonik(...)]` keys
        // it points at, which `rewrite` strips.
        generator.emit(item::anchors(input, Kind::Message));
        emit::message(&ir, generator);
        generator.emit(emit::absorbed_registrations(&ir.absorbs));
        item::rewrite(input, &ir);
    })
}

/// Implement `prost::Message` for one oneof of an ArmoniK API message: a Rust
/// enum that the message's own type carries in the field named after that
/// oneof.
///
/// The macro's argument is the oneof's path, `full.proto.Message.oneof_name`.
/// The last segment is the oneof and what precedes it is the message; that path
/// is what the expansion emits into the type's `Oneof` impl and what the
/// carrying struct const-asserts, which makes standing one message's oneof in
/// for another's a compile error rather than a silent re-encoding: two
/// tag-compatible oneofs are a byte-level bijection.
///
/// A fragment of a message rather than a message: no `Msg` impl and no
/// registration, both of which belong to the struct deriving the message. A
/// oneof that covers its whole message is rejected here: that is the
/// whole-message enum shape, under [`message`](macro@message).
///
/// ```ignore
/// #[armonik_macros::message("armonik.api.grpc.v1.tasks.FilterField")]
/// #[derive(Debug, Clone, Default, PartialEq, Eq)]
/// pub struct Field {
///     pub field: TaskField,
///     #[armonik(rename = "value_condition")]
///     pub condition: Condition,     // the fragment, named after the oneof
/// }
///
/// #[armonik_macros::oneof("armonik.api.grpc.v1.tasks.FilterField.value_condition")]
/// #[derive(Debug, Clone, Default, PartialEq, Eq)]
/// pub enum Condition {
///     #[default]
///     Invalid,                      // no member set
///     #[armonik(rename = "filter_string")]
///     String(FilterString),
///     #[armonik(rename = "filter_number")]
///     Number(FilterNumber),
/// }
/// ```
///
/// # Variants
///
/// One per oneof member, matched by snake_cased name, plus one attribute-less
/// variant, at most, standing for "no member set" -- the proto zero, so it is
/// the one to mark `#[default]`. A payload takes one of three forms: a tuple
/// payload, an `inlined` struct variant spreading a member message's fields, or
/// a `present` unit variant for a member whose only information is that it is
/// set. There are no sibling fields to carry: those belong to the struct.
///
/// # Attributes
///
/// None at type level: the oneof is the macro's argument. On variants and their
/// fields, the grammar is the [message attributes](macro@message#attributes).
#[proc_macro_attribute]
pub fn oneof(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attr = TokenStream2::from(attr);
    expand(input, |input, index, generator| {
        let Some(paths) = proto_names(attr, generator) else {
            return;
        };
        let ir = resolve::resolve_oneof(input, index, &paths, generator);
        // Read before `rewrite` strips the `#[armonik(...)]` keys the anchors point at.
        generator.emit(item::anchors(input, Kind::Oneof));
        emit::message(&ir, generator);
        generator.emit(emit::absorbed_registrations(&ir.absorbs));
        item::rewrite(input, &ir);
    })
}

/// Implement the wire representation of a protobuf enum for an ArmoniK API
/// type, validated against the protobuf descriptors compiled by the `armonik`
/// build script. The item is re-emitted with the proto documentation injected
/// (the enum and each matched value) and the `#[armonik(...)]` attributes
/// stripped.
///
/// The macro's argument is the full proto name of the enum the type stands for,
/// or several for a unified type standing for identical enums; with
/// [`transparent`](#transparent) it names the single-field wrapper messages
/// instead, whose tag paths must agree.
///
/// proto3 enums are open, so unknown values must round-trip losslessly. The
/// expansion requires exactly one catch-all tuple variant, whose payload
/// struct it emits itself:
///
/// ```ignore
/// #[armonik_macros::enumeration("armonik.api.grpc.v1.task_status.TaskStatus")]
/// #[derive(Debug, Clone, Copy)]
/// pub enum TaskStatus {
///     Creating,
///     Submitted,
///     // ...
///     /// Unspecified (0) or a value unknown to this crate version.
///     Unknown(UnknownTaskStatus),   // the expansion emits `struct UnknownTaskStatus`
/// }
/// ```
///
/// Unit variants match proto values by name: either the prost-style short form
/// (the value name with the enum-name prefix stripped, PascalCased, so
/// `TASK_STATUS_CREATING` gives `Creating`) or the full proto value name via
/// [`rename`](#rename). Every proto value needs a variant, except the zero
/// one, which the catch-all may cover instead; the expansion then names it with
/// an `UNSPECIFIED` associated const.
///
/// The payload struct's field is private, so a catch-all value can only come
/// from decoding or `From<i32>`, both of which normalize known values to their
/// named variants (raw access via `.value() -> i32`). The expansion also emits
/// `From<i32>` and `From<Self> for i32`, an `UNSPECIFIED` associated const
/// when the zero value has no named variant,
/// and `Default` (the zero value, per the crate's zero-default invariant)
/// unless a variant carries the std `#[default]` attribute.
///
/// # What not to derive
///
/// `PartialEq`, `Eq`, `PartialOrd`, `Ord` and `Hash` are emitted, in terms of
/// the proto value, and deriving any of them is a spanned error. One value has
/// two spellings, the named variant and the catch-all holding its number, and
/// they are one value; the derived versions would make them differ, and would
/// order the catch-all by where it sits in the declaration rather than by what
/// it holds. `Serialize` and `Deserialize` are emitted for the same reason:
/// a derived `Deserialize` is generated in the module that owns the payload's
/// private field, so it builds the catch-all directly, without normalizing.
///
/// Named values serialize as their variant name and the catch-all as a plain
/// integer; deserializing accepts a name, an integer, or the `{"Unknown": 4}`
/// object `derive(Deserialize)` writes. Reading three shapes means
/// `deserialize_any`, so the self-describing formats work and the ones that
/// need the type to drive the parse (bincode, postcard) do not.
///
/// Emitting the comparison traits makes the type non-structural-match, so the
/// `UNSPECIFIED` const is compared with `==` rather than matched on.
///
/// Enum-typed fields of derived messages are declared with
/// [`message`](macro@message), which checks that the field type stands for the
/// proto enum the descriptor names.
///
/// # Attributes
///
/// ## transparent
///
/// `transparent`, on the type: the enum stands for a chain of single-field
/// wrapper messages ending at an enum field, flattened into the Rust enum.
/// Name the wrapper messages with [`message`](#message) instead of
/// [`enum`](#enum). The type also implements `prost::Message` as the outermost
/// wrapper, so it can stand for whole RPC messages:
///
/// ```ignore
/// #[armonik_macros::enumeration]
/// #[derive(Debug, Clone, Copy)]
/// #[armonik(transparent, message = "armonik.api.grpc.v1.applications.ApplicationField")]
/// pub enum ApplicationField {
///     // matched against the enum at the end of the wrapper chain
///     Unspecified,
///     Name,
///     // ...
///     Unknown(UnknownApplicationField),
/// }
/// ```
///
/// ## rename
///
/// `rename = "FULL_PROTO_VALUE_NAME"`, on a variant: the proto value name when
/// the prost-style short form does not match.
#[proc_macro_attribute]
pub fn enumeration(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attr = TokenStream2::from(attr);
    expand(input, |input, index, generator| {
        let Some(names) = proto_names(attr, generator) else {
            return;
        };
        let plan = enumeration::resolve_enumeration(input, index, &names, generator);
        generator.emit(item::anchors(input, Kind::Enumeration));
        let wire = enumeration::wire(&plan, generator);
        generator.emit(wire);
        // `items` is the value-level half: the payload struct, the two `i32` conversions,
        // `UNSPECIFIED` and `Default`. Called here rather than from anything shared, because only
        // an enumeration has them.
        generator.emit(enumeration::items(&plan));
        generator.emit(emit::absorbed_registrations(&plan.absorbs));
        item::rewrite_enum(input, &plan);
    })
}

/// The shared entry point of the two attribute macros: parse the item, load the descriptor, run
/// the expansion, and always re-emit the item, which `f` mutates in place (doc injection,
/// attribute strip) and only ever *adds* to, emitting the implementations into the [`Generator`].
///
/// The one home of the failure policy: there is none. Every step records what failed and degrades
/// to what it can still say, the recorded errors become `compile_error!`s after the item and the
/// impls, and a poisoned expansion resolves everywhere it is used while never building.
fn expand(
    input: TokenStream,
    f: impl FnOnce(&mut DeriveInput, &descriptor::DescriptorIndex, &mut Generator),
) -> TokenStream {
    let mut input = parse_macro_input!(input as syn::DeriveInput);
    let mut generator = Generator::new();
    // A descriptor that fails to load is a build-environment failure, not a mistake in the item:
    // nothing can resolve, so nothing but the error is worth emitting.
    match crate::descriptor::index() {
        Ok(index) => f(&mut input, &index, &mut generator),
        Err(message) => generator.error(input.ident.span(), message),
    }
    generator.finish(input.into_token_stream()).into()
}

/// The proto names the macro was given, or `None` once the malformed argument is recorded.
///
/// `None` stops the expansion rather than resolving against no name: the item is re-emitted either
/// way, and resolving would add "missing the proto message" to a mistake that is already reported.
fn proto_names(
    attr: TokenStream2,
    generator: &mut Generator,
) -> Option<Vec<(proc_macro2::Span, String)>> {
    match attrs::proto_names(attr) {
        Ok(names) => Some(names),
        Err(error) => {
            generator.record(error);
            None
        }
    }
}

/// Register a proto message name for a type alias, so generic instantiations
/// carrying no annotation of their own (the per-service `Sort = Sort<Field>`,
/// `Status = FilterStatus<T>`) are discovered by `armonik`'s build script and
/// the differential harness like any `#[armonik_macros::message]` type.
///
/// The alias is re-emitted verbatim, plus the `crate::register!` entry a
/// derive would emit for that proto name (into `armonik`'s
/// `differential::registrations::REGISTRY`, with its harness hooks). The
/// aliased type must implement `prost::Message`, and `Normalize` under
/// `cfg(test)`.
///
/// ```ignore
/// #[armonik_macros::alias("armonik.api.grpc.v1.tasks.ListTasksRequest.Sort")]
/// pub type Sort = super::Sort<Field>;
/// ```
#[proc_macro_attribute]
pub fn alias(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = TokenStream2::from(item);
    expand_alias(attr.into(), item.clone())
        // The alias survives its own failure: the item is re-emitted, only the registration is
        // withheld, and the one real error is the only one reported.
        .unwrap_or_else(|error| {
            let error = error.into_compile_error();
            quote::quote!(#item #error)
        })
        .into()
}

/// Declare the RPCs of one proto service, validated against the protobuf
/// descriptor at expansion time. One invocation per service owns RPC identity,
/// the server trait and the router table for it.
///
/// ```ignore
/// crate::rpc::service! {
///     Results in crate::results @ "armonik.api.grpc.v1.results.Results";
///
///     rpc ListResults(list::Request) -> list::Response;
///     rpc DownloadResultData(download::Request) -> stream download::Response;
///     rpc UploadResultData(stream upload::Request) -> upload::Response;
///     rpc WatchResults(stream watch::Request) -> stream watch::Response;
/// }
/// ```
///
/// # Grammar
///
/// The header names the marker type to emit, the module the request and
/// response paths are relative to, and the full proto service name (the marker
/// is the Rust-facing name; the two differ for `Auth`/`Authentication` and
/// `HealthChecks`/`HealthChecksService`). Two optional header lines follow:
/// `unexposed(Method, ...);` lists RPCs the crate deliberately does not expose
/// (the router answers UNIMPLEMENTED for their paths), and `deprecated;` marks
/// the generated client alias `#[deprecated]` (the `Submitter` service).
///
/// Each rpc line is:
///
/// ```text
/// rpc Method([stream] req::Request) -> [stream] req::Response [as name];
/// ```
///
/// - `stream` sits where the proto puts it: schema syntax validated against
///   the descriptor's streaming flags, not a config field.
/// - `as name` names the **server** side: the `<Marker>Service` trait method
///   that handles this RPC, the router entry that dispatches into it, and the
///   telemetry label. It defaults to the module segment of the request path
///   (`list::Request` gives `list`), and is spelled only where two RPCs share a
///   request module and would otherwise collide in the trait, which is what
///   `create_tasks::{Small,Large}Request` do.
///
///   It says nothing about the client. Client methods are hand-written in
///   `client/*.rs` and name their own RPC through
///   [`client`](macro@client)'s `#[armonik(rpc = "...")]`, so the two sides are
///   named independently.
///
/// The client methods are *not* declared here. They live in `client/*.rs`,
/// written out, and are tied back to these declarations by
/// [`client`](macro@client) so that a test can prove every RPC has one. That is
/// the point: a signature that is written down cannot move when a field is
/// added to the proto message behind it.
///
/// # What one invocation emits
///
/// - **ungated**: the service marker (docs harvested from the proto) with its
///   `Service` impl; one `Rpc` impl per line, with const asserts that the
///   request and response types implement the method's input and output
///   messages, and a fingerprint tripwire against stale expansions.
/// - **under `cfg(test)`**: the unexposed RPCs' message names, registered for
///   the coverage ratchet (derived from `unexposed(...)`, so the two allowlists
///   cannot drift), and every declared RPC, registered for the client-coverage
///   check that [`client`](macro@client) is the other half of.
/// - **`_gen-server`**: the `<Marker>Service` trait (one method per RPC, docs
///   harvested, streaming shapes from the descriptor), the
///   `<Marker>ServiceExt::<marker>_server` wrapper, and the `Routes` table the
///   generic `Router` dispatches through.
/// - **`_gen-client`**: the `Client` type alias, with the service's harvested
///   docs.
///
/// # Validation
///
/// Schema facts are spanned errors at expansion time: the service and every
/// method exist in the descriptor, the `stream` keywords agree with its
/// streaming flags, no method is declared twice, no two methods share an
/// handler name, and every method of the service is either declared or listed
/// in `unexposed(...)`. Type facts, that the named Rust types implement the
/// RPC's messages, are const-asserted over the codec's `NAMES`.
#[proc_macro]
pub fn service(input: TokenStream) -> TokenStream {
    let def = parse_macro_input!(input as service::ServiceDef);
    service::expand(def)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Link the hand-written client methods of one service to the RPCs they stand for.
///
/// The methods in `client/*.rs` are written by hand, so their signatures are stable against schema
/// changes by construction. This attribute supplies the two things they cannot supply themselves:
/// it prepends each method the RPC's documentation, harvested from the proto rather than copied,
/// and it registers the method so a test can prove every declared RPC has one.
///
/// ```ignore
/// #[armonik_macros::client]
/// #[armonik(service = "armonik.api.grpc.v1.sessions.Sessions")]
/// impl<T: super::Channel> super::ServiceClient<services::Sessions, T> {
///     #[armonik(rpc = "GetSession")]
///     pub async fn get(&mut self, session_id: impl Into<String>) -> Result<Raw, RequestError> {
///         Ok(self.call(get::Request { session_id: session_id.into() }).await?.session)
///     }
///
///     client_method!(CreateSession: create(partition_ids: iter<String>)
///         -> create::Request => session_id: String);
/// }
/// ```
///
/// # Attributes
///
/// ## service
///
/// `service = "full.proto.Service"`, on the impl block: the proto service its methods belong to,
/// which is where their documentation is looked up. Spelled here rather than read off
/// `ServiceClient<Sessions, T>` because a proc macro cannot resolve a path to the service it names.
///
/// ## rpc
///
/// `rpc = "MethodName"`, on a method: the RPC it stands for. A `client_method!` invocation says the
/// same thing by leading with the RPC name.
///
/// # Failure
///
/// Every failure re-emits the block, with the error beside it rather than instead of it. An
/// attributed item reaches the IDE only through its expansion, so a macro that answers a malformed
/// input with nothing but `compile_error!` withdraws every method in the block from completion and
/// go-to-definition on every keystroke that leaves it briefly unparseable.
#[proc_macro_attribute]
pub fn client(attr: TokenStream, input: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        let error = syn::Error::new(
            proc_macro2::Span::call_site(),
            "#[armonik_macros::client] takes no arguments; \
             configure it with #[armonik(service = \"...\")]",
        )
        .into_compile_error();
        let input = TokenStream2::from(input);
        return quote::quote!(#input #error).into();
    }
    client::expand(input.into()).into()
}

// ---- Shared by the entry points ----

/// `#[armonik_macros::alias("proto.Name")]` on a `type` alias: re-emit the alias and register
/// `(proto name, Rust path)` the way a derive would, so generic instantiations carrying no
/// annotation of their own are still harvested. No descriptor validation; the differential harness
/// covers the concrete instantiation like any generic type.
fn expand_alias(attr: TokenStream2, item: TokenStream2) -> syn::Result<TokenStream2> {
    let proto: syn::LitStr = syn::parse2(attr).map_err(|err| {
        syn::Error::new(
            err.span(),
            "#[alias(...)] takes a single string literal: the full proto message name",
        )
    })?;
    let item_type: syn::ItemType = syn::parse2(item)?;
    let name = proto.value();

    // Resolved rather than taken on trust: a typo is a spanned error here instead of four failing
    // harness tests, none of which names this line.
    let index = descriptor::index().map_err(|message| syn::Error::new(proto.span(), message))?;
    let Some(meta) = index.messages.get(&name) else {
        return Err(matcher::not_found(proto.span(), "message", &name));
    };

    let asserts = alias_asserts(&item_type, &name, meta)?;
    let registrations =
        emit::registrations(&plan::respan(&item_type.ident), std::slice::from_ref(&name));
    Ok(quote::quote! {
        #item_type
        #asserts
        #registrations
    })
}

/// The field asserts a generic instantiation gets, standing in for the ones its declaration cannot
/// have: `#[armonik(generic)]` skips descriptor validation because a generic type names no proto
/// message, so this is where its fields are finally checked against one.
///
/// Empty for the two shapes it cannot speak for: an alias that instantiates nothing (there is no
/// `GenericFields` to read, and the aliased type validated itself), and a message with a oneof
/// (whose members are not fields in this sense). Neither exists today; both are skipped rather than
/// rejected, since an alias is a registration first and a check second.
fn alias_asserts(
    item_type: &syn::ItemType,
    name: &str,
    meta: &descriptor::MessageMeta,
) -> syn::Result<TokenStream2> {
    let syn::Type::Path(path) = item_type.ty.as_ref() else {
        return Ok(TokenStream2::new());
    };
    let instantiated =
        path.path.segments.last().is_some_and(|segment| {
            matches!(segment.arguments, syn::PathArguments::AngleBracketed(_))
        });
    if !instantiated || !meta.oneofs.is_empty() {
        return Ok(TokenStream2::new());
    }

    use syn::spanned::Spanned as _;

    let ty = &item_type.ty;
    let span = item_type.ty.span();
    // The assert is anchored on the `=`: punctuation on the alias's own line, so a failure renders
    // that line while hover on the aliased type finds nothing of the expansion under it (see
    // `plan::At`). `span` stays the type's, for the diagnostics.
    let anchor = item_type.eq_token.span();
    let mut fields: Vec<&descriptor::FieldMeta> = meta.fields.iter().collect();
    fields.sort_by_key(|field| field.tag);
    let mut expects = Vec::new();
    for field in fields {
        let expect = plan::Expectation::of(field);
        let path = format!("{name}.{}", field.name);
        match emit::expect_literal(&expect, &path, span) {
            Ok(literal) => {
                let tag = field.tag;
                expects.push(quote::quote! { (#tag, #literal) });
            }
            Err(error) => return Ok(error),
        }
    }

    Ok(quote::quote_spanned! { anchor =>
        const _: () = crate::codec::assert_generic_fields::<#ty>(&[#(#expects),*]);
    })
}
