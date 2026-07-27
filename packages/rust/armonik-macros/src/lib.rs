//! Internal derive macros for the [`armonik`](https://crates.io/crates/armonik) crate.
//!
//! This crate is an implementation detail of `armonik`: the attribute grammar
//! and the emitted code offer no stability guarantee of their own, and the
//! expansions reference `armonik`-internal paths, so the derives only work
//! inside the `armonik` crate itself. It must only be used through the
//! `armonik` crate, which depends on it with an exact version pin.
//!
//! The derives read the protobuf descriptor set compiled by the `armonik`
//! build script (`$OUT_DIR/descriptor.bin`) at expansion time: field tags,
//! wire kinds and cardinalities are taken from the descriptors, and any
//! mismatch between a Rust type and its proto counterpart is a compile
//! error. A fingerprint const-assert is emitted with every expansion so a
//! stale expansion can never survive a descriptor change.
//!
//! See [`Message`](macro@Message) for messages and oneofs, and
//! [`Enum`](macro@Enum) for proto enums; the `#[armonik(...)]` grammar is
//! documented in their [message attributes](macro@Message#attributes) and
//! [enum attributes](macro@Enum#attributes) sections.

use proc_macro::TokenStream;
use syn::parse_macro_input;

mod attrs;
mod codegen;
mod descriptor;
mod errors;
mod expand;
mod kind;
mod resolve;

/// Derive `prost::Message` for an ArmoniK API type, validated against the
/// protobuf descriptors compiled by the `armonik` build script.
///
/// Tags, wire kinds, cardinalities and packedness are read from the
/// descriptor at expansion time — nothing is restated in the source — and
/// every disagreement between the Rust type and the proto message (unknown
/// field, uncovered proto field or oneof, kind or cardinality mismatch) is
/// a spanned compile error naming both sides. Proto enums are derived
/// separately with [`Enum`](macro@Enum).
///
/// Besides `prost::Message`, the derive emits:
/// - a `ProtoField` implementation, so the type composes as a field of
///   other derived messages (field types dispatch through that trait:
///   scalars, `String`, `bytes::Bytes`, `Vec<T>`, `Option<T>` for proto3
///   explicit presence, `prost_types::{Timestamp, Duration}`, and every
///   derived type);
/// - a fingerprint const-assert that fails the build if the expansion ever
///   goes stale against a newer descriptor;
/// - under the private `_differential` feature, the type's registration
///   into the differential-harness registry and its `Normalize` projection.
///
/// Every derived type must uphold the crate's zero-default invariant:
/// `Default::default()` **is** the proto zero value, so decoding an empty
/// message yields it (the differential harness enforces this).
///
/// # Shapes
///
/// **Plain struct** — [`message`](#message) names the proto message; Rust
/// fields are matched to proto fields **by name** ([`rename`](#rename)
/// when they differ; tuple structs must rename every field):
///
/// ```ignore
/// #[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
/// #[armonik(message = "armonik.api.grpc.v1.tasks.GetResultIdsResponse")]
/// pub struct Response {
///     #[armonik(with = "crate::codec::adapters::PairMap<1, 2>")]
///     pub task_results: HashMap<String, Vec<String>>,
/// }
/// ```
///
/// A proto field that belongs to a oneof cannot be mapped alone: declare
/// one Rust field named after the *oneof*, whose type is an
/// embedded-oneof enum (see [`oneof`](#oneof)).
///
/// **Whole-message enum** — [`message`](#message) on an enum stands for a
/// message whose single oneof is inferred. Variants are matched to oneof
/// members by snake_cased name; an attribute-less unit variant is the
/// "no member set" case and becomes the `Default`:
///
/// ```ignore
/// #[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
/// #[armonik(message = "armonik.api.grpc.v1.Output")]
/// pub enum Output {
///     #[default]
///     #[armonik(present)]
///     Ok,                           // member `ok`, carried by presence
///     Error { details: String },    // member `error`, message fields inlined
/// }
/// ```
///
/// Variant payloads take three forms: a single tuple payload
/// (`Variant(T)`), a struct variant inlining the fields of a message
/// member (as `Error` above; the member's message must not itself contain
/// a oneof), or a [`present`](#present) unit variant for a `bool` or
/// empty-message member whose only information is which member is set.
/// When the message has non-oneof fields, they are *siblings*: every
/// variant (including the attribute-less one) is a struct variant carrying
/// all of them next to its own payload field, which keeps the per-field
/// merge stateless and order-independent.
///
/// **Embedded oneof** — [`message`](#message) with [`oneof`](#oneof)
/// declares an enum for one oneof of a larger message, used as a field
/// (named after the oneof) of the struct deriving that message.
///
/// **Generic struct** — [`generic`](#generic) skips descriptor validation
/// (a generic type cannot name one proto message); every field carries an
/// explicit [`tag`](#tag), and the concrete instantiations are validated
/// by the differential harness instead:
///
/// ```ignore
/// #[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
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
/// ## message
///
/// `message = "full.proto.Name"`, on the type — the proto message the type
/// stands for, validated field by field against the descriptor. Repeatable
/// when one Rust type stands for several identical messages (unified
/// types), which must agree on every field.
///
/// ## oneof
///
/// `oneof = "name"`, on an enum — the enum stands for that oneof of the
/// message named by [`message`](#message), embedded in a struct as a field
/// named after the oneof. Rejected when the oneof covers the whole
/// message: drop it and use the whole-message enum shape
/// ([`message`](#message) alone), keeping the two shapes visually
/// distinct.
///
/// ## generic
///
/// `generic`, on a struct — skip descriptor validation: a generic type
/// cannot name one proto message. Every field carries an explicit
/// [`tag`](#tag), and the concrete instantiations are validated by the
/// differential harness instead.
///
/// ## rename
///
/// `rename = "proto_name"`, on a field or variant — the proto field or
/// oneof member name when it differs from the Rust one (fields and
/// members are otherwise matched by snake_cased name). Required on
/// tuple-struct fields.
///
/// ## tag
///
/// `tag = N`, on a field — the field's proto tag, cross-checked against
/// the descriptor (a mismatch is a compile error). In [`generic`](#generic)
/// mode there is no descriptor to read, so the tag is required and
/// authoritative.
///
/// ## with
///
/// `with = "path::To::Adapter"`, on a field or single-payload tuple
/// variant (in sibling variants: on the member payload field) — encode
/// through a custom `ProtoAdapter` instead of the type's `ProtoField`
/// implementation, for a Rust representation that differs structurally
/// from the proto shape (e.g. `PairMap` exposing repeated key/value pairs
/// as a `HashMap`). Skips the descriptor kind checks on purpose; the
/// differential harness covers the adapter, including its
/// `normalize_dynamic` projection.
///
/// ## present
///
/// `present`, on a unit variant — the oneof member is carried by presence
/// only: a `bool` member encodes `true` (an explicit `false` still
/// selects the variant), an empty-message member encodes an empty
/// message.
#[proc_macro_derive(Message, attributes(armonik))]
pub fn derive_message(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    expand::message(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Derive the wire representation of a protobuf enum for an ArmoniK API
/// type, validated against the protobuf descriptors compiled by the
/// `armonik` build script.
///
/// proto3 enums are open: unknown values must round-trip losslessly. The
/// derive therefore requires exactly one catch-all tuple variant whose
/// payload struct it emits itself:
///
/// ```ignore
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, armonik_macros::Enum)]
/// #[armonik(enum = "armonik.api.grpc.v1.task_status.TaskStatus")]
/// pub enum TaskStatus {
///     Creating,
///     Submitted,
///     // ...
///     /// Unspecified (0) or a value unknown to this crate version.
///     Other(OtherTaskStatus),   // the derive emits `struct OtherTaskStatus`
/// }
/// ```
///
/// Unit variants are matched to proto values **by name**: either the
/// prost-style short form (the value name with the enum-name prefix
/// stripped, PascalCased — `TASK_STATUS_CREATING` ⇒ `Creating`) or the
/// full proto value name via [`rename`](#rename). Every proto value needs
/// a variant (a compile error otherwise), except a conventional
/// `*_UNSPECIFIED = 0`, which the catch-all covers.
///
/// The payload struct's field is private, so a value of the catch-all
/// variant can only be produced by decoding or `From<i32>`, which
/// normalize known values to their named variants: no known value can
/// hide inside the catch-all, keeping derived `PartialEq`/`Hash`
/// semantically correct (raw access via `.value() -> i32`). The derive
/// also emits `From<i32>` and `From<Self> for i32` (a dataful enum cannot
/// be `as`-cast), an `UNSPECIFIED` associated const when the zero value
/// has no named variant, and `Default` (the zero value, per the crate's
/// zero-default invariant) unless a variant carries the std `#[default]`
/// attribute.
///
/// Enum-typed fields of derived messages are declared with
/// [`Message`](macro@Message); the derive checks the field type stands
/// for the proto enum named in the descriptor.
///
/// # Attributes
///
/// ## enum
///
/// `enum = "full.proto.Name"`, on the type — the proto enum the type
/// stands for. Repeatable when one Rust type stands for several identical
/// enums (unified types), which must agree on every value.
///
/// ## transparent
///
/// `transparent`, on the type — the enum stands for a chain of
/// single-field wrapper messages ending at an enum field, flattened into
/// the Rust enum; name the wrapper message(s) with [`message`](#message)
/// instead of [`enum`](#enum). The type additionally implements
/// `prost::Message` as the outermost wrapper, so it can stand for whole
/// RPC messages in stub signatures:
///
/// ```ignore
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, armonik_macros::Enum)]
/// #[armonik(transparent, message = "armonik.api.grpc.v1.applications.ApplicationField")]
/// pub enum ApplicationField {
///     // matched against the enum at the end of the wrapper chain
///     Unspecified,
///     Name,
///     // ...
///     Other(OtherApplicationField),
/// }
/// ```
///
/// ## message
///
/// `message = "full.proto.Name"`, on the type — with
/// [`transparent`](#transparent): the single-field wrapper message
/// standing for the enum. Repeatable; the wrapper tag paths must agree.
///
/// ## rename
///
/// `rename = "FULL_PROTO_VALUE_NAME"`, on a variant — the proto value
/// name when the prost-style short form does not match.
#[proc_macro_derive(Enum, attributes(armonik))]
pub fn derive_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    expand::enumeration(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
