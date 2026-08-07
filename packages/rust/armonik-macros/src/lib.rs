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
//! [`message`](macro@message) covers messages and oneofs, [`enumeration`](macro@enumeration) proto
//! enums, [`service`](macro@service) the per-service RPC definitions. The `#[armonik(...)]` grammar
//! lives in the [message attributes](macro@message#attributes) and [enum
//! attributes](macro@enumeration#attributes) sections.

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
///   the type composes as a field of other derived messages (`ProtoField`
///   covers scalars, `String`, `bytes::Bytes`, `Vec<T>`, `Option<T>` for
///   proto3 explicit presence, `HashMap<K, V>`,
///   `prost_types::{Timestamp, Duration}`, and every derived type);
/// - a fingerprint const-assert that fails the build once the expansion goes
///   stale against a newer descriptor;
/// - under the private `_differential` feature, the type's registration into
///   `armonik::wire::REGISTRY`, with its `Normalize` projection and harness
///   hooks.
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
/// #[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
/// #[armonik(message = "armonik.api.grpc.v1.tasks.GetResultIdsResponse")]
/// pub struct Response {
///     #[armonik(with = "crate::codec::adapters::PairMap<1, 2>")]
///     pub task_results: HashMap<String, Vec<String>>,
/// }
/// ```
///
/// A proto field belonging to a oneof cannot be mapped alone: declare one
/// Rust field named after the *oneof*, typed as an embedded-oneof enum (see
/// [`oneof`](#oneof)).
///
/// **Whole-message enum**: [`message`](#message) on an enum stands for a
/// message whose single oneof is inferred. Variants match oneof members by
/// snake_cased name; an attribute-less unit variant is the "no member set"
/// case and becomes the `Default`:
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
/// Variant payloads take three forms: a single tuple payload (`Variant(T)`),
/// a struct variant inlining the fields of a message member (`Error` above;
/// that member's message must not itself contain a oneof), or a
/// [`present`](#present) unit variant for a `bool` or empty-message member
/// whose only information is that it is set. Non-oneof fields of the message
/// become *siblings*: every variant, including the attribute-less one, is a
/// struct variant carrying all of them next to its own payload field, which
/// keeps the per-field merge stateless and order-independent.
///
/// **Embedded oneof**: [`message`](#message) with [`oneof`](#oneof) declares
/// an enum for one oneof of a larger message, used as a field (named after
/// the oneof) of the struct deriving that message.
///
/// **Generic struct**: [`generic`](#generic) skips descriptor validation,
/// since a generic type cannot name one proto message. Every field carries an
/// explicit [`tag`](#tag), and the differential harness validates the
/// concrete instantiations instead:
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
/// `message = "full.proto.Name"`, on the type: the proto message the type
/// stands for, validated field by field. Repeatable when one Rust type stands
/// for several identical messages (unified types), which must agree on every
/// field.
///
/// ## oneof
///
/// `oneof = "name"`, on an enum: the enum stands for that oneof of the message
/// named by [`message`](#message), embedded in a struct as a field named after
/// the oneof. Rejected when the oneof covers the whole message; use the
/// whole-message enum shape ([`message`](#message) alone) there, so the two
/// shapes stay visually distinct.
///
/// ## generic
///
/// `generic`, on a struct: skip descriptor validation, since a generic type
/// cannot name one proto message. Every field carries an explicit
/// [`tag`](#tag), and the differential harness validates the concrete
/// instantiations instead.
///
/// ## rename
///
/// `rename = "proto_name"`, on a field or variant: the proto field or oneof
/// member name when it differs from the Rust one (the default match is by
/// snake_cased name). Required on tuple-struct fields.
///
/// ## tag
///
/// `tag = N`, on a field: the field's proto tag, cross-checked against the
/// descriptor. In [`generic`](#generic) mode there is no descriptor to read,
/// so the tag is required and authoritative.
///
/// ## with
///
/// `with = "path::To::Adapter"`, on a field or single-payload tuple variant
/// (in sibling variants, on the member payload field): encode through a custom
/// `ProtoAdapter` instead of the type's `ProtoField` impl, for a Rust
/// representation that differs structurally from the proto shape (e.g.
/// `PairMap` exposing repeated key/value pairs as a `HashMap`). Skips the
/// descriptor kind checks on purpose; the differential harness covers the
/// adapter, including its `normalize_dynamic` projection.
///
/// ## absorbs
///
/// `absorbs = "full.proto.Name"`, on a field or variant carrying a
/// [`with`](#with) adapter: the proto message the adapter flattens away (a
/// pair-entry, `VecWrapper` or `StringWrapper` message), which therefore has no
/// Rust type of its own. Registered as absorbed in `armonik::wire`, so the
/// build script prunes it from the stubs and the differential harness counts it
/// as covered through this parent. Repeatable. The other flatteners,
/// [`transparent`](macro@enumeration#transparent) chains and inline struct variants,
/// declare their absorbed messages automatically.
///
/// ## present
///
/// `present`, on a unit variant: the oneof member is carried by presence alone.
/// A `bool` member encodes `true` (an explicit `false` still selects the
/// variant), an empty-message member encodes an empty message.
///
/// ## transparent
///
/// `transparent`, on a single-field struct: the type delegates its whole
/// `prost::Message` impl to that one field, so it is wire-identical to the
/// field's message and can stand for a whole RPC message in the stub
/// signatures (the struct sibling of the `derive(Enum)` wrapper mode). Name the
/// inner message with [`message`](#message); the field is not matched against
/// the descriptor. Typically wraps a shared message per RPC site (e.g.
/// `struct Request { filter: TaskFilter }`), keeping request types injective
/// over RPCs.
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
/// type, validated against the protobuf descriptors compiled by the `armonik`
/// build script. An attribute macro like [`message`](macro@message): the item
/// is re-emitted with the proto documentation injected (the enum and each
/// matched value) and the `#[armonik(...)]` attributes stripped.
///
/// proto3 enums are open, so unknown values must round-trip losslessly. The
/// expansion requires exactly one catch-all tuple variant, whose payload
/// struct it emits itself:
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
/// Unit variants match proto values by name: either the prost-style short form
/// (the value name with the enum-name prefix stripped, PascalCased, so
/// `TASK_STATUS_CREATING` gives `Creating`) or the full proto value name via
/// [`rename`](#rename). Every proto value needs a variant, except a
/// conventional `*_UNSPECIFIED = 0`, which the catch-all covers.
///
/// The payload struct's field is private, so a catch-all value can only come
/// from decoding or `From<i32>`, both of which normalize known values to their
/// named variants. No known value can hide inside the catch-all, which keeps
/// the derived `PartialEq`/`Hash` semantically correct (raw access via
/// `.value() -> i32`). The expansion also emits `From<i32>` and
/// `From<Self> for i32` (a dataful enum cannot be `as`-cast), an `UNSPECIFIED`
/// associated const when the zero value has no named variant, and `Default`
/// (the zero value, per the crate's zero-default invariant) unless a variant
/// carries the std `#[default]` attribute.
///
/// The item is re-emitted `#[repr(i32)]`, each named variant carrying the proto
/// value it stands for as its discriminant, so a derived `PartialOrd`/`Ord`
/// (which compare discriminants) orders the type by proto value. The catch-all
/// stands for no single value and takes `i32::MIN`, so the zero value and the
/// unknown ones sort before every named value, and among themselves by the raw
/// value.
///
/// Enum-typed fields of derived messages are declared with
/// [`message`](macro@message), which checks that the field type stands for the
/// proto enum the descriptor names.
///
/// # Attributes
///
/// ## enum
///
/// `enum = "full.proto.Name"`, on the type: the proto enum the type stands
/// for. Repeatable when one Rust type stands for several identical enums
/// (unified types), which must agree on every value.
///
/// ## transparent
///
/// `transparent`, on the type: the enum stands for a chain of single-field
/// wrapper messages ending at an enum field, flattened into the Rust enum.
/// Name the wrapper messages with [`message`](#message) instead of
/// [`enum`](#enum). The type also implements `prost::Message` as the outermost
/// wrapper, so it can stand for whole RPC messages in stub signatures:
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
/// `message = "full.proto.Name"`, on the type, with
/// [`transparent`](#transparent): the single-field wrapper message standing
/// for the enum. Repeatable; the wrapper tag paths must agree.
///
/// ## rename
///
/// `rename = "FULL_PROTO_VALUE_NAME"`, on a variant: the proto value name when
/// the prost-style short form does not match.
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

/// Register a proto message name for a type alias, so generic instantiations
/// carrying no annotation of their own (the per-service `Sort = Sort<Field>`,
/// `Status = FilterStatus<T>`) are discovered by `armonik`'s build script and
/// the differential harness like any `#[armonik_macros::message]` type.
///
/// The alias is re-emitted verbatim, plus the `crate::register!` entry a
/// derive would emit for that proto name (into `armonik::wire::REGISTRY`, with
/// its `_differential` harness hooks). The aliased type must implement
/// `prost::Message`, and `Normalize` under `_differential`.
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

/// Carry a derived message's field reflection onto a type alias of it, so the
/// alias can stand for the message on a [`service!`](macro@service) rpc line
/// without losing what the convenience emission reads off it: a projection
/// (`=> field`) or an `auto` field count on the response side, the parameter
/// list on the request side.
///
/// The alias is re-emitted, plus renaming re-exports of the aliased struct's
/// `__armonik_fields_*` callback and `__armonik_ty_*` field aliases under the
/// alias's own stem (`Response` gives `response`), the names the emission
/// mangles from the path on the rpc line. Reflection is looked up in the module
/// named after the aliased type (`super::super::Count` in `super::super::count`,
/// per the crate's one-object-per-file convention); a right-hand side already
/// spelling the defining module is taken as is. Generic aliases are rejected,
/// since neither they nor the struct they instantiate carry reflection.
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

/// Internal continuation of the field-reflection callback for [`reflect`](macro@reflect):
/// re-exports one field type alias per reflected field. Only ever invoked by `reflect`-emitted
/// code; see `reflect.rs`.
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
/// response paths are relative to, and the full proto service name (the marker
/// is the Rust-facing name; the two differ for `Auth`/`Authentication` and
/// `HealthChecks`/`HealthChecksService`). Two optional header lines follow:
/// `unexposed(Method, ...);` lists RPCs the crate deliberately does not expose
/// (the router answers UNIMPLEMENTED for their paths), and `deprecated;` marks
/// every generated convenience method `#[deprecated]` (the `Submitter`
/// service).
///
/// Each rpc line is:
///
/// ```text
/// rpc Method([stream] req::Request) -> [stream] req::Response [as name] [=> ...] [manual];
/// ```
///
/// - `stream` sits where the proto puts it: schema syntax validated against
///   the descriptor's streaming flags, not a config field.
/// - The ergonomic name (server trait method, convenience method, telemetry
///   label) is the module segment of the request path; `as name` overrides it
///   when several RPCs share a module (`create_tasks::{Small,Large}Request`).
/// - `=> ...` controls what the convenience method returns; see
///   [Projection](#projection).
/// - `manual` emits no convenience method: the opt-out for custom wiring or a
///   wrong mechanical default (`worker::Process`, whose request would explode
///   into nine parameters). Client-streaming RPCs are required to carry it,
///   since a request stream has no single message to spread into parameters;
///   their entry point is `call_streaming`.
///
/// # What one invocation emits
///
/// - **ungated**: the service marker (docs harvested from the proto) with its
///   `Service` impl; one `Rpc` impl per line, with const asserts that the
///   request and response types implement the method's input and output
///   messages, and a fingerprint tripwire against stale expansions.
/// - **`_differential`**: the unexposed RPCs' message names, registered for the
///   coverage ratchet. Derived from `unexposed(...)`, so the two allowlists
///   cannot drift.
/// - **`_gen-server`**: the `<Marker>Service` trait (one method per RPC, docs
///   harvested, streaming shapes from the descriptor), the
///   `<Marker>ServiceExt::<marker>_server` wrapper, and the `Routes` table the
///   generic `Router` dispatches through.
/// - **`_gen-client`**: one convenience method per non-manual rpc line, built
///   by [`__emit_convenience`] from the request struct's fields through the
///   field-reflection callbacks the derives emit. Parameters mirror the fields
///   in declaration order (reorder the struct to reorder them), widened per
///   sugar class: `String`/`Bytes` to `impl Into`, `Vec<T>` to
///   `impl IntoIterator<Item = impl Into<T>>`, `HashMap<K, V>` to pair
///   iterators, `filter::Or` to nested iterators. Docs harvested.
///
/// # Projection
///
/// What the convenience method returns:
///
/// - *(default)* the response's fields decide: exactly one field yields that
///   field, several yield the whole response;
/// - `=> field`: that field of the response, mapped over the stream items on a
///   server-streaming RPC (`download` yielding `Bytes`);
/// - `=> *`: the whole response, always. Required when the response type is an
///   alias or an enum, which carry no field reflection;
/// - `=> ()`: discard it and return `()`.
///
/// # Validation
///
/// Schema facts are spanned errors at expansion time: the service and every
/// method exist in the descriptor, the `stream` keywords agree with its
/// streaming flags, no method is declared twice, no two methods share an
/// ergonomic name, and every method of the service is either declared or listed
/// in `unexposed(...)`. Type facts, that the named Rust types implement the
/// RPC's messages, are const-asserted over the codec's `NAMES`. A wrong sugar
/// inference is an ordinary type error in the generated code.
#[proc_macro]
pub fn service(input: TokenStream) -> TokenStream {
    let def = parse_macro_input!(input as service::ServiceDef);
    descriptor::index()
        .map_err(|message| syn::Error::new(proc_macro2::Span::call_site(), message))
        .and_then(|index| service::expand(def, &index))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Internal continuation of the field-reflection callbacks: builds one client convenience method
/// per RPC from the request struct's fields. Only ever invoked by `service!`-emitted code; see
/// `convenience.rs`.
#[doc(hidden)]
#[proc_macro]
pub fn __emit_convenience(input: TokenStream) -> TokenStream {
    convenience::expand(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

// ---- Expansion orchestration, shared by the three entry points ----

fn expand_message(input: DeriveInput) -> syn::Result<TokenStream2> {
    let index = load_index(&input)?;
    let entries = attrs::parse(&input.attrs)?;
    let has_oneof = entries
        .iter()
        .any(|entry| matches!(entry.item, AttrItem::Oneof(_)));
    let generic = entries
        .iter()
        .any(|entry| matches!(entry.item, AttrItem::Generic));
    // Enums are oneof-shaped: `message = ...` alone stands for a whole message with a single
    // inferred oneof, `oneof = ...` for one oneof of a message, embedded in a struct.
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

/// Field reflection for the `service!` convenience emission: a callback macro forwarding each
/// field's name and sugar class in declaration order (which is the generated method's parameter
/// order), plus flat per-field type aliases so the consuming proc macro can name field and element
/// types from another module. The aliases resolve the field's type tokens here, where they mean the
/// right thing. `__emit_convenience` is the consuming side.
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

/// How the generated signature widens a request field's type, and how the body converts it back.
/// Conservative: anything unrecognized passes through unchanged, and a whole method opts out with
/// `manual` on its rpc line.
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
        // The per-service `filter::Or`, whose sibling `Field` is the element type of the
        // nested-iterator sugar.
        "Or" => {
            let mut field = path.path.clone();
            field.segments.last_mut().expect("segment").ident =
                syn::Ident::new("Field", segment.ident.span());
            Sugar::Filters(field)
        }
        _ => Sugar::Plain,
    }
}

/// The proto value each variant stands for, which the re-emitted item carries as its discriminant
/// (see [`docs::tag_variants`]).
pub(crate) struct EnumTags {
    /// Named variants with their proto values.
    pub(crate) named: Vec<(syn::Ident, i32)>,
    /// The catch-all variant, which stands for no single value.
    pub(crate) other: syn::Ident,
}

fn expand_enumeration(input: DeriveInput) -> syn::Result<(TokenStream2, EnumTags)> {
    let index = load_index(&input)?;
    let plan = resolve::enum_plan(&input, &index).map_err(Errors::into_syn_error)?;
    let tags = EnumTags {
        named: plan.named.clone(),
        other: plan.other_variant.clone(),
    };
    let mut out = doc_anchors(&input, "enumeration");
    let mut absorbs = collect_absorbs(&input);
    absorbs.extend(plan.absorbs.iter().cloned());
    out.extend(codegen::enumeration(&plan));
    out.extend(absorbed(absorbs));
    Ok((out, tags))
}

/// Visit the attributes of the type itself and of every field, variant and variant field: the
/// common traversal for whole-input attribute scans.
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

/// The explicit `#[armonik(absorbs = "...")]` names on any field or variant of the input.
/// Transparent and inline ones are collected into the plan instead.
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
    let registrations = codegen::registrations(&item_type.ident, std::slice::from_ref(&name));
    Ok(quote::quote! {
        #item_type
        #registrations
    })
}

/// Hover-documentation anchors: re-emit every `#[armonik(...)]` key token of the input as an
/// anonymous import of the deriving macro, respanned onto the key. IDE hover on the otherwise-inert
/// keys then resolves to this crate's macro, the single home of the grammar documentation. The
/// anonymous `const` compiles to nothing.
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
