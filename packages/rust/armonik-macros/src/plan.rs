//! The resolved plans: what each shape decided, in the vocabulary the emitters read.
//!
//! Resolution fills these from the descriptor and the annotations; emission reads them and nothing
//! else. Keeping them here rather than next to either half is what makes that split checkable: a
//! plan type mentions no `TokenStream`, and an emitter reaches for no `DescriptorIndex`.

use proc_macro2::Span;

use crate::descriptor::{Cardinality, FieldKind, FieldMeta};

pub(crate) struct MessagePlan {
    pub(crate) ident: syn::Ident,
    /// Full proto names the type stands for (several for unified types).
    pub(crate) proto_names: Vec<String>,
    /// Fields sorted by tag (canonical encode order). In `transparent` mode this holds exactly the
    /// single delegate field.
    pub(crate) fields: Vec<FieldPlan>,
    pub(crate) generics: syn::Generics,
    pub(crate) fingerprint: u64,
    /// `#[armonik(transparent)]` on a struct: the type delegates its whole `prost::Message` impl to
    /// its single field.
    pub(crate) transparent: bool,
}

pub(crate) enum FieldAccess {
    Named(syn::Ident),
    Indexed(syn::Index),
}

pub(crate) enum FieldCodec {
    /// An ordinary field; `adapter` is the `#[armonik(with = "...")]` type when present (which
    /// skips the shape checks by design).
    Field { adapter: Option<Box<syn::Type>> },
    /// The field covers a whole oneof of the message and is encoded through `prost::Message`; `tags`
    /// are the member field tags routed to it.
    OneofGroup { tags: Vec<u32> },
}

/// What the descriptor says a checked field is: the shape assert is emitted straight from this, in
/// the descriptor's own vocabulary.
///
/// There used to be a `Card` mirroring [`Cardinality`] and a `FieldChecks` mirroring the codec's
/// `Expect`, with four functions whose whole job was to launder one into the other. The rules they
/// encoded (a singular message field may be `Option` in Rust; a map checks its key and value kinds
/// and names its value type) are not resolution decisions, so they live with the emitter instead.
pub(crate) struct Expectation {
    pub(crate) kind: FieldKind,
    pub(crate) cardinality: Cardinality,
}

impl Expectation {
    /// The expectation for a descriptor field, or `None` where there is nothing to check: a `with`
    /// adapter, a oneof group, a generic field.
    pub(crate) fn of(field: &FieldMeta) -> Option<Self> {
        Some(Self {
            kind: field.kind.clone(),
            cardinality: field.cardinality.clone(),
        })
    }
}

pub(crate) struct FieldPlan {
    pub(crate) access: FieldAccess,
    pub(crate) ty: syn::Type,
    pub(crate) span: Span,
    /// Tag of the field (or the lowest member tag for oneof groups), used for ordering.
    pub(crate) tag: u32,
    pub(crate) codec: FieldCodec,
    pub(crate) checks: Option<Expectation>,
    /// `TypeName.field_name` of the proto field, for diagnostics.
    pub(crate) proto_path: String,
}

/// Plan for a protobuf enum (or a transparent single-enum-field wrapper).
pub(crate) struct EnumPlan {
    pub(crate) ident: syn::Ident,
    /// The catch-all variant (`Unknown`) and its payload struct, which the expansion emits.
    pub(crate) unknown_variant: syn::Ident,
    pub(crate) payload: syn::Ident,
    /// Named variants with their proto numbers.
    pub(crate) named: Vec<(syn::Ident, i32)>,
    /// Named variant covering 0, when there is one; otherwise the derive emits an `UNSPECIFIED`
    /// const based on the catch-all.
    pub(crate) zero_variant: Option<syn::Ident>,
    /// Whether a variant carries the standard `#[default]` attribute, in which case the user
    /// derives `Default` and the macro must not.
    pub(crate) has_std_default: bool,
    pub(crate) mode: EnumMode,
    pub(crate) fingerprint: u64,
    /// Intermediate wrapper messages the transparent chain flattens away, so they have no Rust type
    /// of their own (see [`crate::codegen`]).
    pub(crate) absorbs: Vec<String>,
}

pub(crate) enum EnumMode {
    /// The Rust enum is a proto enum, an `int32` varint on the wire.
    Plain { names: Vec<String> },
    /// The Rust enum stands for proto message(s) wrapping an enum field through a chain of
    /// single-field wrappers; `path` holds the tags from the outermost wrapper down to the enum
    /// field.
    Transparent { names: Vec<String>, path: Vec<u32> },
}

/// Plan for a oneof-shaped enum: either a whole message whose fields are a single oneof plus
/// optional sibling fields (`message = ...` alone), or just the oneof `oneof_name` of the message,
/// to be embedded in a struct (`message = ...`
/// + `oneof = ...`).
pub(crate) struct OneofPlan {
    pub(crate) ident: syn::Ident,
    pub(crate) proto_name: String,
    /// Whether the enum stands for the whole message (annotation without `oneof = ...`), in which
    /// case it gets `prost::Message` + `ProtoField` implementations.
    pub(crate) whole_message: bool,
    /// Non-oneof fields of the message, replicated in every variant (whole-message enums only;
    /// empty when the oneof is the only field).
    pub(crate) siblings: Vec<SiblingPlan>,
    pub(crate) variants: Vec<OneofVariant>,
    /// The attribute-less variant standing for "no member set", if any: a unit variant, or a struct
    /// variant carrying exactly the sibling fields when there are siblings.
    pub(crate) default_variant: Option<syn::Ident>,
    pub(crate) fingerprint: u64,
    /// Messages inlined into struct variants (their fields are spread into the variant), so they
    /// have no Rust type of their own.
    pub(crate) absorbs: Vec<String>,
}

/// A non-oneof field of a whole-message enum, present in every variant under the same name and
/// type.
pub(crate) struct SiblingPlan {
    pub(crate) ident: syn::Ident,
    pub(crate) ty: syn::Type,
    pub(crate) span: Span,
    pub(crate) tag: u32,
    pub(crate) proto_path: String,
    pub(crate) checks: Option<Expectation>,
}

pub(crate) struct OneofVariant {
    pub(crate) ident: syn::Ident,
    pub(crate) span: Span,
    pub(crate) tag: u32,
    pub(crate) proto_path: String,
    pub(crate) shape: OneofVariantShape,
}

pub(crate) enum OneofVariantShape {
    /// The member value, carried by `Variant(T)`, or by the `binding` field of `Variant { payload,
    /// ...siblings }` in a whole-message enum with sibling fields. Encoded through the type's
    /// `ProtoField` impl or a `ProtoAdapter` (`#[armonik(with = "...")]`, which skips the shape
    /// checks by design).
    Payload {
        ty: Box<syn::Type>,
        adapter: Option<Box<syn::Type>>,
        checks: Box<Option<Expectation>>,
        binding: Option<syn::Ident>,
    },
    /// `#[armonik(present)]` unit variant selected by a `bool` member.
    MarkerBool,
    /// `#[armonik(present)]` unit variant selected by an empty-message member.
    MarkerMessage,
    /// `Variant { field, ... }` inlining the fields of the member's message.
    Inline { parts: Vec<InlinePart> },
}

pub(crate) struct InlinePart {
    pub(crate) ident: syn::Ident,
    pub(crate) ty: syn::Type,
    pub(crate) span: Span,
    pub(crate) tag: u32,
    pub(crate) proto_path: String,
    pub(crate) checks: Option<Expectation>,
}
