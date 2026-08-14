//! The resolved plans: what each shape decided, in the vocabulary the emitters and the item
//! rewriter read.
//!
//! Resolution fills these from the descriptor and the annotations; emission and `item` read them
//! and nothing else. Keeping them here rather than next to either half is what makes that split
//! checkable: a plan type mentions no `TokenStream`, and an emitter reaches for no
//! `DescriptorIndex`.
//!
//! That is also why the harvested proto comments live here, on the slot or variant they belong to.
//! `item::inject` used to look them up itself, matching Rust names to proto names a second time
//! with rules that were nearly, but not exactly, the resolvers'. Nearly cost real documentation:
//! inlined variant fields were looked up in the containing message and never found, and a
//! transparent enum's values were never looked up at all. Resolution knows the answer at the moment
//! it matches, so it writes it down.

use proc_macro2::Span;

use crate::descriptor::{Cardinality, FieldKind, FieldMeta};

pub(crate) struct MessagePlan {
    pub(crate) ident: syn::Ident,
    /// Full proto names the type stands for (several for unified types).
    pub(crate) proto_names: Vec<String>,
    /// Leading comment of the proto message, for the re-emitted item.
    pub(crate) docs: Vec<String>,
    /// Fields sorted by tag (canonical encode order). In `transparent` mode this holds exactly the
    /// single delegate field.
    pub(crate) fields: Vec<Slot>,
    pub(crate) generics: syn::Generics,
    pub(crate) fingerprint: u64,
    /// `#[armonik(transparent)]` on a struct: the type delegates its whole `prost::Message` impl to
    /// its single field.
    pub(crate) transparent: bool,
    /// Proto messages a `with` adapter flattens away, declared through `#[armonik(absorbs = ...)]`,
    /// so they have no Rust type of their own.
    pub(crate) absorbs: Vec<String>,
}

pub(crate) enum FieldAccess {
    Named(syn::Ident),
    Indexed(syn::Index),
}

/// How one [`Slot`] gets on the wire.
pub(crate) enum SlotCodec {
    /// An ordinary field, through the type's `ProtoField` impl; `adapter` is the
    /// `#[armonik(with = "...")]` type when present (which skips the shape checks by design).
    Field {
        ty: Box<syn::Type>,
        adapter: Option<Box<syn::Type>>,
    },
    /// A whole oneof of the message, routed to the flattened enum's own `prost::Message` impl;
    /// `tags` are the member field tags that reach it.
    Oneof { ty: Box<syn::Type>, tags: Vec<u32> },
    /// `#[armonik(present)]`: the member carries nothing but its own presence. A `bool` member
    /// encodes `true`, an empty-message member an empty message.
    Marker { empty_message: bool },
    /// `#[armonik(inline)]`: the member message's own fields, spread into the variant and framed
    /// here, since the message is absorbed and has no Rust type to delegate to.
    Inline { parts: Vec<Slot> },
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

/// One protobuf field, wherever it sits.
///
/// A struct's field, a whole-message enum's non-oneof field (replicated across every variant), the
/// member a variant carries, and one field of a member message spread into a variant under `inline`
/// are the same thing seen from four places, and used to be four structs: `FieldPlan`,
/// `SiblingPlan`, `InlinePart` and the `Payload` arm of an `OneofVariantShape`, of which the middle
/// two were field-for-field identical. A per-field concern — a new attribute key that changes the
/// encoding, a new check — was four edits and four chances to miss one, in a crate whose premise is
/// that field-level duplication is what rots.
pub(crate) struct Slot {
    /// How the value is reached: `self.name` on a struct, the field name bound by the pattern in a
    /// struct variant, the single element of a tuple variant. `None` for a `present` marker, which
    /// carries no value at all.
    pub(crate) access: Option<FieldAccess>,
    pub(crate) span: Span,
    /// Tag of the field, or the lowest member tag of a whole oneof, which is what orders it among
    /// its siblings.
    pub(crate) tag: u32,
    pub(crate) codec: SlotCodec,
    pub(crate) checks: Option<Expectation>,
    /// `TypeName.field_name` of the proto field, for diagnostics.
    pub(crate) proto_path: String,
    /// Leading comment of the proto field, which the re-emitted item carries as `#[doc]`. Empty
    /// where the proto says nothing, and where there is nothing to say it about: a whole oneof (the
    /// declaration carries no comment of its own), a transparent delegate, a generic field.
    pub(crate) docs: Vec<String>,
}

impl Slot {
    /// The Rust type carrying the value, for the shape assert. `None` where there is no value: a
    /// `present` marker, or an inlined member (whose parts carry their own).
    pub(crate) fn ty(&self) -> Option<&syn::Type> {
        match &self.codec {
            SlotCodec::Field { ty, .. } | SlotCodec::Oneof { ty, .. } => Some(ty),
            SlotCodec::Marker { .. } | SlotCodec::Inline { .. } => None,
        }
    }

    /// Whether this slot is the one reached through `field`, the `index`th field of its container.
    /// How the item rewriter finds the syn field a slot stands for, without rematching names.
    pub(crate) fn reaches(&self, field: &syn::Field, index: usize) -> bool {
        match (&self.access, &field.ident) {
            (Some(FieldAccess::Named(name)), Some(ident)) => name == ident,
            (Some(FieldAccess::Indexed(at)), None) => at.index as usize == index,
            _ => false,
        }
    }
}

/// One named variant and the proto value it stands for.
pub(crate) struct EnumValue {
    pub(crate) ident: syn::Ident,
    pub(crate) number: i32,
    /// Leading comment of the proto value, for the re-emitted item. Harvested here rather than by
    /// the rewriter, which is what a transparent enum's values were missing: their comments live on
    /// the enum at the end of the wrapper chain, which only resolution walks.
    pub(crate) docs: Vec<String>,
}

/// Plan for a protobuf enum (or a transparent single-enum-field wrapper).
pub(crate) struct EnumPlan {
    pub(crate) ident: syn::Ident,
    /// The catch-all variant (`Unknown`) and its payload struct, which the expansion emits.
    pub(crate) unknown_variant: syn::Ident,
    pub(crate) payload: syn::Ident,
    /// Leading comment of the proto enum, or in transparent mode of the outermost wrapper message.
    pub(crate) docs: Vec<String>,
    /// Named variants with their proto numbers.
    pub(crate) named: Vec<EnumValue>,
    /// Named variant covering 0, when there is one; otherwise the derive emits an `UNSPECIFIED`
    /// const based on the catch-all.
    pub(crate) zero_variant: Option<syn::Ident>,
    /// Whether a variant carries the standard `#[default]` attribute, in which case the user
    /// derives `Default` and the macro must not.
    pub(crate) has_std_default: bool,
    pub(crate) mode: EnumMode,
    pub(crate) fingerprint: u64,
    /// Intermediate wrapper messages the transparent chain flattens away, so they have no Rust type
    /// of their own.
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
    /// Leading comment of the proto message, for the re-emitted item.
    pub(crate) docs: Vec<String>,
    /// Whether the enum stands for the whole message (annotation without `oneof = ...`), in which
    /// case it gets `prost::Message` + `ProtoField` implementations.
    pub(crate) whole_message: bool,
    /// `message.oneof` this type stands for, when it stands for one oneof of a larger message.
    /// `None` for a whole-message enum, which is a message and records itself through `Msg::NAMES`.
    pub(crate) oneof_path: Option<String>,
    /// Non-oneof fields of the message, replicated in every variant (whole-message enums only;
    /// empty when the oneof is the only field).
    pub(crate) siblings: Vec<Slot>,
    pub(crate) variants: Vec<OneofVariant>,
    /// The attribute-less variant standing for "no member set", if any: a unit variant, or a struct
    /// variant carrying exactly the sibling fields when there are siblings.
    pub(crate) default_variant: Option<syn::Ident>,
    pub(crate) fingerprint: u64,
    /// Messages inlined into struct variants (their fields are spread into the variant), so they
    /// have no Rust type of their own.
    pub(crate) absorbs: Vec<String>,
}

pub(crate) struct OneofVariant {
    pub(crate) ident: syn::Ident,
    /// What the variant carries. Its `span` is the variant's, which is where a shape assert about
    /// the member points. Its `access` says how: a named field of a struct variant, the
    /// single element of a tuple variant, or nothing for a `present` marker.
    pub(crate) own: Slot,
}
