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
mod resolve;

use attrs::{AttrItem, Errors};
use descriptor::DescriptorIndex;
use proc_macro2::TokenStream as TokenStream2;
use syn::DeriveInput;

/// Derive `prost::Message` for an ArmoniK API type, validated against the
/// protobuf descriptors compiled by the `armonik` build script.
///
/// Tags, wire kinds and cardinalities are read from the descriptor at
/// expansion time — nothing is restated in the source — and
/// every disagreement between the Rust type and the proto message (unknown
/// field, uncovered proto field or oneof, kind or cardinality mismatch) is
/// a spanned compile error naming both sides. Proto enums are derived
/// separately with [`Enum`](macro@Enum).
///
/// Besides `prost::Message`, the derive emits:
/// - a `Msg` implementation (picked up by the codec's blanket `ProtoField`
///   impl), so the type composes as a field of other derived messages
///   (field types dispatch through `ProtoField`: scalars, `String`,
///   `bytes::Bytes`, `Vec<T>`, `Option<T>` for proto3 explicit presence,
///   `HashMap<K, V>`, `prost_types::{Timestamp, Duration}`, and every
///   derived type);
/// - a fingerprint const-assert that fails the build if the expansion ever
///   goes stale against a newer descriptor;
/// - under the private `_registry` feature, the type's registration into
///   `armonik_types::wire::REGISTRY`, and under `_differential` its
///   `Normalize` projection and harness hooks.
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
/// ## absorbs
///
/// `absorbs = "full.proto.Name"`, on a field/variant carrying a
/// [`with`](#with) adapter — the proto message the adapter flattens away
/// (a pair-entry, `VecWrapper`, or `StringWrapper` message), which therefore
/// has no Rust type of its own. Harvested into `armonik_types::wire::ABSORBED`
/// so the build script prunes it from the stubs and the differential harness
/// counts it as covered through this parent. Repeatable. The other flatteners
/// — [`transparent`](macro@Enum#transparent) chains and inline struct variants
/// — declare their absorbed messages automatically.
///
/// ## present
///
/// `present`, on a unit variant — the oneof member is carried by presence
/// only: a `bool` member encodes `true` (an explicit `false` still
/// selects the variant), an empty-message member encodes an empty
/// message.
///
/// ## transparent
///
/// `transparent`, on a single-field struct — the type delegates its whole
/// `prost::Message` impl to that one field, so it is wire-identical to the
/// field's message and can stand for a whole RPC message in the stub
/// signatures (the struct sibling of the `derive(Enum)` wrapper mode). Name
/// the inner message with [`message`](#message); the field is not matched
/// against the descriptor. Typically paired with [`replace`](#replace) to
/// wrap a shared message (e.g. `struct Request { filter: TaskFilter }`).
///
/// ## replace
///
/// `replace(target = "synthetic.Name", service = "Service", method =
/// "Method", input | output)`, on the type — the type stands in for its
/// [`message`](#message) at one RPC site. `armonik`'s build script checks the
/// RPC's `input`/`output` slot still holds `message` (a drift guard against
/// proto changes), then rewrites that slot to the synthetic `target` message
/// (absent from the real schema) and extern-maps `target` to this type — so
/// RPCs sharing one proto message get distinct stub signatures pointing at
/// distinct Rust types, keeping `GrpcCall<Request>` unambiguous. Repeatable.
#[proc_macro_derive(Message, attributes(armonik))]
pub fn derive_message(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    expand_message(input)
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
    expand_enumeration(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Register a proto message name for a **type alias**, so the generic
/// instantiations that carry no annotation of their own (e.g. the per-service
/// `Sort = Sort<Field>` / `Status = FilterStatus<T>`) are auto-discovered by
/// `armonik`'s build script and the differential harness — the same way a
/// `#[derive(Message)]` type is.
///
/// The alias is re-emitted verbatim, plus the `crate::register!` entry a
/// derive would emit for that proto name (into `armonik_types::wire::REGISTRY`,
/// with its `_differential` harness hooks). The aliased type must implement
/// `prost::Message` (and, under `_differential`, `Normalize`).
///
/// ```ignore
/// #[armonik_macros::alias("armonik.api.grpc.v1.tasks.ListTasksRequest.Sort")]
/// pub type Sort = super::Sort<Field>;
/// ```
#[proc_macro_attribute]
pub fn alias(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_alias(attr.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

// ---- Expansion orchestration (shared by the three entry points) ----

fn expand_message(input: DeriveInput) -> syn::Result<TokenStream2> {
    let index = load_index(&input)?;
    let entries = attrs::parse(&input.attrs)?;
    let has_oneof = entries
        .iter()
        .any(|entry| matches!(entry.item, AttrItem::Oneof(_)));
    let generic = entries
        .iter()
        .any(|entry| matches!(entry.item, AttrItem::Generic));
    // Enums are oneof-shaped: `message = ...` alone stands for a whole
    // message with a single (inferred) oneof, `oneof = ...` for one oneof
    // of a message, embedded in a struct.
    let mut out = doc_anchors(&input, "Message");
    let mut absorbs = collect_absorbs(&input);
    if has_oneof || (matches!(input.data, syn::Data::Enum(_)) && !generic) {
        let plan = resolve::oneof_plan(&input, &index).map_err(Errors::into_syn_error)?;
        absorbs.extend(plan.absorbs.iter().cloned());
        out.extend(codegen::oneof(&plan));
    } else {
        let plan = resolve::message_plan(&input, &index).map_err(Errors::into_syn_error)?;
        out.extend(codegen::message(&plan));
    }
    out.extend(absorbed(absorbs));
    Ok(out)
}

fn expand_enumeration(input: DeriveInput) -> syn::Result<TokenStream2> {
    let index = load_index(&input)?;
    let plan = resolve::enum_plan(&input, &index).map_err(Errors::into_syn_error)?;
    let mut out = doc_anchors(&input, "Enum");
    let mut absorbs = collect_absorbs(&input);
    absorbs.extend(plan.absorbs.iter().cloned());
    out.extend(codegen::enumeration(&plan));
    out.extend(absorbed(absorbs));
    Ok(out)
}

/// Visit the attribute list of the type itself and of every field, variant,
/// and variant field — the common traversal for whole-input attribute scans.
fn for_each_attr_site(input: &DeriveInput, mut visit: impl FnMut(&[syn::Attribute])) {
    visit(&input.attrs);
    match &input.data {
        syn::Data::Struct(data) => {
            for field in &data.fields {
                visit(&field.attrs);
            }
        }
        syn::Data::Enum(data) => {
            for variant in &data.variants {
                visit(&variant.attrs);
                for field in &variant.fields {
                    visit(&field.attrs);
                }
            }
        }
        syn::Data::Union(_) => {}
    }
}

/// The explicit `#[armonik(absorbs = "...")]` names on any field/variant of
/// the input (auto-collected transparent/inline ones come from the plan).
fn collect_absorbs(input: &DeriveInput) -> Vec<String> {
    let mut out = Vec::new();
    for_each_attr_site(input, |attrs| {
        if let Ok(entries) = attrs::parse(attrs) {
            for entry in entries {
                if let AttrItem::Absorbs(lit) = entry.item {
                    out.push(lit.value());
                }
            }
        }
    });
    out
}

fn absorbed(mut names: Vec<String>) -> TokenStream2 {
    names.sort();
    names.dedup();
    codegen::absorbed_registrations(&names)
}

/// `#[armonik_macros::alias("proto.Name")]` on a `type` alias: re-emit the
/// alias and register `(proto name, Rust path)` the way a derive would, so
/// generic instantiations that carry no annotation of their own are still
/// harvested. No descriptor validation — the concrete instantiation is
/// covered by the differential harness like any generic type.
fn expand_alias(attr: TokenStream2, item: TokenStream2) -> syn::Result<TokenStream2> {
    let proto: syn::LitStr = syn::parse2(attr).map_err(|err| {
        syn::Error::new(
            err.span(),
            "#[alias(...)] takes a single string literal: the full proto message name",
        )
    })?;
    let item_type: syn::ItemType = syn::parse2(item)?;
    let name = proto.value();
    let registrations = codegen::registrations(&item_type.ident, std::slice::from_ref(&name), None);
    Ok(quote::quote! {
        #item_type
        #registrations
    })
}

/// Hover-documentation anchors: re-emit every `#[armonik(...)]` key token
/// of the input as an anonymous import of the deriving macro, respanned
/// onto the key. IDE hover on the otherwise-inert helper attribute keys
/// then resolves to this crate's derive — the single home of the grammar
/// documentation. The anonymous `const` compiles to nothing.
fn doc_anchors(input: &DeriveInput, derive: &str) -> TokenStream2 {
    let mut spans = Vec::new();
    for_each_attr_site(input, |attrs| spans.extend(attrs::key_spans(attrs)));
    if spans.is_empty() {
        return TokenStream2::new();
    }
    let uses = spans.iter().map(|span| {
        let derive = syn::Ident::new(derive, *span);
        quote::quote! {
            {
                #[allow(unused_imports)]
                use ::armonik_macros::#derive as _;
            }
        }
    });
    quote::quote! {
        const _: () = {
            #(#uses)*
        };
    }
}

fn load_index(input: &DeriveInput) -> syn::Result<std::sync::Arc<DescriptorIndex>> {
    descriptor::index().map_err(|message| syn::Error::new(input.ident.span(), message))
}

