//! Internal macros for the [`armonik`](https://crates.io/crates/armonik) crate.
//!
//! This crate is an implementation detail of `armonik`: the attribute grammar
//! and the emitted code offer no stability guarantee of their own, and the
//! expansions reference `armonik`-internal paths, so the derives only work
//! inside the `armonik` crate itself. It must only be used through the
//! `armonik` crate, which depends on it with an exact version pin.
//!
//! The macros read the protobuf descriptor set compiled by the `armonik`
//! build script (`$OUT_DIR/descriptor.bin`) at expansion time: field tags,
//! wire kinds, cardinalities and the documentation are taken from the
//! descriptors, and any mismatch between a Rust type and its proto
//! counterpart is a compile error. A fingerprint const-assert is emitted
//! with every expansion so a stale expansion can never survive a descriptor
//! change.
//!
//! See [`message`](macro@message) for messages and oneofs,
//! [`enumeration`](macro@enumeration) for proto enums, and
//! [`service`](macro@service) for the per-service RPC definitions; the
//! `#[armonik(...)]` grammar is documented in the
//! [message attributes](macro@message#attributes) and
//! [enum attributes](macro@enumeration#attributes) sections.

use proc_macro::TokenStream;
use syn::parse_macro_input;

mod attrs;
mod callback;
mod codegen;
mod convenience;
mod descriptor;
mod docs;
mod reflect;
mod resolve;
mod service;

use attrs::{AttrItem, Errors};
use descriptor::DescriptorIndex;
use proc_macro2::TokenStream as TokenStream2;
use syn::DeriveInput;

/// Implement `prost::Message` for an ArmoniK API type, validated against the
/// protobuf descriptors compiled by the `armonik` build script.
///
/// Tags, wire kinds and cardinalities are read from the descriptor at
/// expansion time — nothing is restated in the source — and
/// every disagreement between the Rust type and the proto message (unknown
/// field, uncovered proto field or oneof, kind or cardinality mismatch) is
/// a spanned compile error naming both sides. Proto enums are handled
/// separately by [`enumeration`](macro@enumeration).
///
/// An attribute macro rather than a derive so the item can be **re-emitted
/// with the proto documentation injected**: the type, its fields, its oneof
/// variants and their inlined fields receive `#[doc]`s extracted from the
/// protos' leading comments — the same harvest `service!` does for services —
/// and hand-written doc comments follow them as Rust-specific notes. The
/// `#[armonik(...)]` attributes are consumed and stripped.
///
/// Besides `prost::Message`, the expansion emits:
/// - a `Msg` implementation (picked up by the codec's blanket `ProtoField`
///   impl), so the type composes as a field of other derived messages
///   (field types dispatch through `ProtoField`: scalars, `String`,
///   `bytes::Bytes`, `Vec<T>`, `Option<T>` for proto3 explicit presence,
///   `HashMap<K, V>`, `prost_types::{Timestamp, Duration}`, and every
///   derived type);
/// - a fingerprint const-assert that fails the build if the expansion ever
///   goes stale against a newer descriptor;
/// - under the private `_registry` feature, the type's registration into
///   `armonik::wire::REGISTRY` (under the private `_differential` feature), with its
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
/// has no Rust type of its own. Registered as absorbed in `armonik::wire`
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
/// against the descriptor. Typically used to wrap a shared message per RPC
/// site (e.g. `struct Request { filter: TaskFilter }`), keeping request
/// types injective over RPCs.
///
#[proc_macro_attribute]
pub fn message(attr: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    if !attr.is_empty() {
        return syn::Error::new(
            input.ident.span(),
            "#[armonik_macros::message] takes no arguments",
        )
        .into_compile_error()
        .into();
    }
    docs::expand(input, docs::Mode::Message)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Implement the wire representation of a protobuf enum for an ArmoniK API
/// type, validated against the protobuf descriptors compiled by the
/// `armonik` build script. Like [`message`](macro@message), an attribute
/// macro: the item is re-emitted with the proto documentation injected (the
/// enum and each matched value) and the `#[armonik(...)]` attributes
/// stripped.
///
/// proto3 enums are open: unknown values must round-trip losslessly. The
/// expansion therefore requires exactly one catch-all tuple variant whose
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
#[proc_macro_attribute]
pub fn enumeration(attr: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    if !attr.is_empty() {
        return syn::Error::new(
            input.ident.span(),
            "#[armonik_macros::enumeration] takes no arguments",
        )
        .into_compile_error()
        .into();
    }
    docs::expand(input, docs::Mode::Enumeration)
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
/// derive would emit for that proto name (into `armonik::wire::REGISTRY`,
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

/// Carry a derived message's field reflection onto a **type alias** of it, so
/// the alias can stand for the message in a [`service!`](macro@service) rpc
/// line without losing what the convenience emission reads off it: a
/// projection (`=> field`) or an `auto` field count on the response side, the
/// parameter list on the request side.
///
/// The alias is re-emitted, plus renaming re-exports of the aliased struct's
/// `__armonik_fields_*` callback and `__armonik_ty_*` field aliases under the
/// alias's own stem (`Response` gives `response`), which are the names the
/// emission mangles from the path on the rpc line. The reflection is looked up
/// in the module named after the aliased type (`super::super::Count` in
/// `super::super::count`, the crate's one-object-per-file convention); a
/// right-hand side already spelling the defining module is taken as is.
/// Generic aliases carry no reflection (neither does the struct they
/// instantiate) and are rejected.
///
/// ```ignore
/// #[armonik_macros::reflect]
/// pub type Response = super::super::Count;   // rpc line: `=> values` now resolves
/// ```
#[proc_macro_attribute]
pub fn reflect(attr: TokenStream, item: TokenStream) -> TokenStream {
    reflect::expand_attribute(attr.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Internal continuation of the field-reflection callback for
/// [`reflect`](macro@reflect): re-exports one field type alias per reflected
/// field. Only ever invoked by `reflect`-emitted code; see `reflect.rs`.
#[doc(hidden)]
#[proc_macro]
pub fn __emit_reflect(input: TokenStream) -> TokenStream {
    reflect::expand(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Declare the RPCs of one proto service, validated against the protobuf
/// descriptor at expansion time. One invocation per service owns that
/// service end to end: RPC identity, the server trait and router table, and
/// the client convenience methods.
///
/// ```ignore
/// crate::rpc::service! {
///     Results in crate::results @ "armonik.api.grpc.v1.results.Results";
///     unexposed(WatchResults);
///
///     rpc ListResults(list::Request) -> list::Response;
///     rpc GetOwnerTaskId(get_owner_task_id::Request) -> get_owner_task_id::Response => result_task;
///     rpc DownloadResultData(download::Request) -> stream download::Response;
///     rpc UploadResultData(stream upload::Request) -> upload::Response;
///     rpc GetServiceConfiguration(get_service_configuration::Request)
///         -> get_service_configuration::Response => *;
/// }
/// ```
///
/// # Grammar
///
/// The header names the marker type to emit, the module the request and
/// response paths are relative to, and the **full proto service name** (the
/// marker is the Rust-facing name; the two differ for `Auth`/`Authentication`
/// and `HealthChecks`/`HealthChecksService`). Two optional header lines
/// follow: `unexposed(Method, ...);` lists RPCs the crate deliberately does
/// not expose (the router answers UNIMPLEMENTED for their paths), and
/// `deprecated;` marks every generated convenience method `#[deprecated]`
/// (the `Submitter` service).
///
/// Each rpc line is:
///
/// ```text
/// rpc Method([stream] req::Request) -> [stream] req::Response [as name] [=> …] [manual];
/// ```
///
/// - `stream` sits where the proto puts it — it is schema syntax, validated
///   against the descriptor's streaming flags, not a config field.
/// - The ergonomic name (server trait method, convenience method, telemetry
///   label) is the module segment of the request path; `as name` overrides it
///   when several RPCs share a module (`create_tasks::{Small,Large}Request`).
/// - `=> …` controls what the convenience method returns; see
///   [Projection](#projection).
/// - `manual` emits no convenience method — the opt-out for custom wiring or
///   a wrong mechanical default (e.g. `worker::Process`, whose request would
///   explode into nine parameters). Client-streaming RPCs are *required* to
///   carry it: a request stream has no single message to spread into
///   parameters, so nothing can be derived, and the entry point is
///   `call_streaming`.
///
/// # What one invocation emits
///
/// - **ungated**: the service marker (docs harvested from the proto) with its
///   `Service` impl; one `Rpc` impl per line, with const asserts that the
///   request and response types implement the method's input and output
///   messages, and a fingerprint tripwire against stale expansions.
/// - **`_differential`**: the unexposed RPCs' message names, registered for
///   the coverage ratchet — derived from `unexposed(...)`, so the two
///   allowlists cannot drift.
/// - **`_gen-server`**: the `<Marker>Service` trait (one method per RPC, docs
///   harvested, streaming shapes from the descriptor), the
///   `<Marker>ServiceExt::<marker>_server` wrapper, and the `Routes` table
///   the generic `Router` dispatches through.
/// - **`_gen-client`**: one convenience method per non-manual rpc line, built
///   by [`__emit_convenience`] from the *request struct's fields* through the
///   field-reflection callbacks the derives emit: parameters mirror the
///   fields in declaration order (reorder the struct to change the parameter
///   order), widened per sugar class (`String`/`Bytes` → `impl Into`,
///   `Vec<T>` → `impl IntoIterator<Item = impl Into<T>>`, `HashMap<K, V>` →
///   pair iterators, `filter::Or` → nested iterators), docs harvested.
///
/// # Projection
///
/// What the convenience method returns:
///
/// - *(default)* the response's fields decide: exactly one field → that
///   field, several → the whole response;
/// - `=> field` — that field of the response (on a server-streaming RPC:
///   mapped over the stream items, e.g. `download` yielding `Bytes`);
/// - `=> *` — the whole response, always (required when the response type is
///   an alias or an enum, which carry no field reflection);
/// - `=> ()` — discard it and return `()`.
///
/// # Validation
///
/// Expansion validates the schema facts as spanned errors: the service and
/// every method exist in the descriptor, the `stream` keywords agree with its
/// streaming flags, no method is declared twice, no two methods share an
/// ergonomic name, and every method of the service is declared or listed in
/// `unexposed(...)`. The type facts — that the named Rust types implement the
/// RPC's messages — are const-asserted over the codec's `NAMES`, and a wrong
/// sugar inference in a convenience method is an ordinary type error in the
/// generated code.
#[proc_macro]
pub fn service(input: TokenStream) -> TokenStream {
    let def = parse_macro_input!(input as service::ServiceDef);
    descriptor::index()
        .map_err(|message| syn::Error::new(proc_macro2::Span::call_site(), message))
        .and_then(|index| service::expand(def, &index))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Internal continuation of the field-reflection callbacks: builds one client
/// convenience method per RPC from the request struct's fields. Only ever
/// invoked by `service!`-emitted code; see `convenience.rs`.
#[doc(hidden)]
#[proc_macro]
pub fn __emit_convenience(input: TokenStream) -> TokenStream {
    convenience::expand(input.into())
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
    let mut out = doc_anchors(&input, "message");
    let mut absorbs = collect_absorbs(&input);
    if has_oneof || (matches!(input.data, syn::Data::Enum(_)) && !generic) {
        let plan = resolve::oneof_plan(&input, &index).map_err(Errors::into_syn_error)?;
        absorbs.extend(plan.absorbs.iter().cloned());
        out.extend(codegen::oneof(&plan));
    } else {
        let plan = resolve::message_plan(&input, &index).map_err(Errors::into_syn_error)?;
        out.extend(codegen::message(&plan));
        out.extend(reflection(&input));
    }
    out.extend(absorbed(absorbs));
    Ok(out)
}

/// Field reflection for the `service!` convenience emission: a callback macro
/// forwarding each field's name and sugar class (declaration order — the
/// generated methods' parameter order), plus flat per-field type aliases so
/// the consuming proc macro can name field and element types from another
/// module (the aliases resolve the field's type tokens *here*, where they
/// mean the right thing). See `__emit_convenience` for the consuming side.
fn reflection(input: &DeriveInput) -> TokenStream2 {
    let syn::Data::Struct(data) = &input.data else {
        return TokenStream2::new();
    };
    let syn::Fields::Named(fields) = &data.fields else {
        return TokenStream2::new();
    };
    if !input.generics.params.is_empty() {
        return TokenStream2::new();
    }

    let snake = service::snake(&input.ident.to_string());
    let fields_macro = quote::format_ident!("__armonik_fields_{snake}");

    let mut units = Vec::new();
    let mut aliases = Vec::new();
    let mut alias = |suffix: &String, ty: &dyn quote::ToTokens| {
        let name = quote::format_ident!("__armonik_ty_{snake}_{suffix}");
        aliases.push(quote::quote! {
            #[doc(hidden)]
            #[allow(non_camel_case_types, dead_code)]
            pub(crate) type #name = #ty;
        });
    };
    for field in &fields.named {
        let name = field.ident.as_ref().expect("named");
        let ty = &field.ty;
        let class = sugar(ty);
        alias(&name.to_string(), &ty);
        match &class {
            Sugar::Iter(elem) => alias(&format!("{name}_elem"), elem),
            Sugar::Filters(elem) => alias(&format!("{name}_elem"), elem),
            Sugar::Pairs(key, value) => {
                alias(&format!("{name}_key"), key);
                alias(&format!("{name}_value"), value);
            }
            Sugar::Plain | Sugar::Into => {}
        }
        let class = match class {
            Sugar::Plain => quote::quote!(plain),
            Sugar::Into => quote::quote!(into),
            Sugar::Iter(_) => quote::quote!(iter),
            Sugar::Pairs(..) => quote::quote!(pairs),
            Sugar::Filters(_) => quote::quote!(filters),
        };
        units.push(quote::quote!([#name #class]));
    }

    quote::quote! {
        #[doc(hidden)]
        macro_rules! #fields_macro {
            ($($cont:tt)::* ! { $($ctx:tt)* }) => {
                $($cont)::* ! { $($ctx)* fields { #(#units)* } }
            };
        }
        #[doc(hidden)]
        pub(crate) use #fields_macro;

        #(#aliases)*
    }
}

/// The sugar class of a convenience-method parameter, from the request
/// field's Rust type: how the generated signature widens it and how the body
/// converts it back. Conservative: anything unrecognized is passed through
/// unchanged, and a whole method can opt out with `manual` on its rpc line.
#[allow(clippy::large_enum_variant)] // transient parse-time value, a handful per struct
enum Sugar {
    Plain,
    Into,
    Iter(syn::Type),
    Pairs(syn::Type, syn::Type),
    Filters(syn::Path),
}

fn sugar(ty: &syn::Type) -> Sugar {
    let syn::Type::Path(path) = ty else {
        return Sugar::Plain;
    };
    let Some(segment) = path.path.segments.last() else {
        return Sugar::Plain;
    };
    let arg = |index: usize| match &segment.arguments {
        syn::PathArguments::AngleBracketed(args) => {
            args.args.iter().nth(index).and_then(|arg| match arg {
                syn::GenericArgument::Type(ty) => Some(ty.clone()),
                _ => None,
            })
        }
        _ => None,
    };
    match segment.ident.to_string().as_str() {
        "String" | "Bytes" => Sugar::Into,
        "Vec" => match arg(0) {
            // `Vec<u8>` is a payload, not a collection of convertibles.
            Some(syn::Type::Path(elem)) if elem.path.is_ident("u8") => Sugar::Into,
            Some(elem) => Sugar::Iter(elem),
            None => Sugar::Plain,
        },
        "HashMap" => match (arg(0), arg(1)) {
            (Some(key), Some(value)) => Sugar::Pairs(key, value),
            _ => Sugar::Plain,
        },
        // The per-service filter type: `filter::Or`, whose sibling `Field` is
        // the element type of the nested-iterator sugar.
        "Or" => {
            let mut field = path.path.clone();
            field.segments.last_mut().expect("segment").ident =
                syn::Ident::new("Field", segment.ident.span());
            Sugar::Filters(field)
        }
        _ => Sugar::Plain,
    }
}

fn expand_enumeration(input: DeriveInput) -> syn::Result<TokenStream2> {
    let index = load_index(&input)?;
    let plan = resolve::enum_plan(&input, &index).map_err(Errors::into_syn_error)?;
    let mut out = doc_anchors(&input, "enumeration");
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
    let registrations = codegen::registrations(&item_type.ident, std::slice::from_ref(&name));
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
