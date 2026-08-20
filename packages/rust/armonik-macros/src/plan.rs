//! The resolved plans: what resolution decided, in the vocabulary the emitters and the item
//! rewriter read.
//!
//! Resolution fills these from the descriptor and the annotations; emission and `item` read them
//! and nothing else. Keeping them here rather than next to either half is what makes that split
//! checkable: a plan type mentions no `TokenStream`, and an emitter reaches for no
//! `DescriptorIndex`.
//!
//! One plan for everything message-shaped ([`Ir`]): a message is shared fields plus an optional
//! discriminant, and every shape `#[armonik_macros::message]` accepts is that form at some
//! degenerate point. A plain struct has no discriminant; a transparent newtype is a single
//! whole-message delegate; a generic struct is a message that names no proto; an embedded oneof is
//! a discriminant alone; a whole-message enum has both. The shapes differ in how they are
//! *resolved*, never in what they are.
//!
//! The harvested proto comments live here too, on the slot or arm they belong to: resolution knows
//! which proto element a Rust name matched at the moment it matches it, so looking that up a second
//! time to attach the docs would mean a second copy of the matching rules.

use proc_macro2::Span;

use crate::descriptor::{Cardinality, FieldKind, FieldMeta};

/// One message-shaped expansion: what every shape of `#[armonik_macros::message]` resolves to, and
/// the wire half of a transparent enumeration.
pub(crate) struct Ir {
    pub(crate) ident: syn::Ident,
    pub(crate) generics: syn::Generics,
    pub(crate) fingerprint: u64,
    /// Full proto names the type stands for (several for unified types). Empty in
    /// [`generic`](Ir::generic) mode, which registers nothing.
    pub(crate) names: Vec<String>,
    /// `Some("message.oneof")` for an embedded oneof: a fragment of a message rather than one, so
    /// it gets the `Oneof` identity marker instead of `Msg`, and registers nothing.
    pub(crate) fragment_of: Option<String>,
    /// Leading comment of the proto message, for the re-emitted item.
    pub(crate) docs: Vec<String>,
    /// Proto messages a flattening construct swallowed into this type (a `with` adapter's
    /// `absorbs`, an inline variant's member message), so they have no Rust type of their own.
    pub(crate) absorbs: Vec<String>,
    /// `#[armonik(generic)]`: no descriptor was read, the tags are authoritative, and the
    /// `GenericFields` table is emitted so every `#[armonik_macros::alias]` instantiation can
    /// assert the fields against the message it registers under.
    pub(crate) generic: bool,
    /// Fields every alternative carries, sorted by tag: all fields of a struct, the non-oneof
    /// siblings of a whole-message enum, nothing for an embedded oneof.
    pub(crate) shared: Vec<Slot>,
    /// The oneof, when the type is an enum. `None` is a struct: one alternative, owning nothing.
    pub(crate) discr: Option<Discr>,
}

/// The discriminant of an enum-shaped message: one arm per oneof member, plus the optional
/// attribute-less "no member set" arm, which owns nothing and is selected by no tag.
pub(crate) struct Discr {
    /// Arms in member tag order.
    pub(crate) arms: Vec<Arm>,
    pub(crate) default_arm: Option<syn::Ident>,
}

/// One named variant and the member slot it owns beyond the shared fields.
pub(crate) struct Arm {
    pub(crate) ident: syn::Ident,
    /// What the arm carries. Its `span` is the variant's, which is where a shape assert about the
    /// member points. Its `access` says how: a named field of a struct variant, the single element
    /// of a tuple variant, or nothing for a `present` marker.
    pub(crate) own: Slot,
}

pub(crate) enum FieldAccess {
    Named(syn::Ident),
    Indexed(syn::Index),
}

/// How one [`Slot`] gets on the wire.
pub(crate) enum SlotCodec {
    /// A leaf value, through the type's `ProtoField` impl; `adapter` is the codec substitution
    /// when there is one (which skips the shape checks by design): the `#[armonik(with = "...")]`
    /// type, or the `BoolPresence`/`EmptyPresence` adapter a `#[armonik(present)]` marker picks,
    /// whose value type is `()` and whose slot binds nothing (`access: None`).
    Field {
        ty: Box<syn::Type>,
        adapter: Option<Box<syn::Type>>,
    },
    /// Whole-message delegation through the value's own `prost::Message` impl. `tags` are the
    /// field tags routed to it: the member tags of an embedded oneof, or `None` for every tag,
    /// which is a `transparent` newtype's single field, wire-identical to the whole message.
    Delegate {
        ty: Box<syn::Type>,
        tags: Option<Vec<u32>>,
    },
    /// `#[armonik(inline)]`: the member message's own fields, spread into the variant and framed
    /// here, since the message is absorbed and has no Rust type to delegate to.
    Group { parts: Vec<Slot> },
    /// A slot that failed to resolve, kept because the user wrote it: it has a shape but no proto
    /// meaning, so the emitter keeps the code that mentions it compiling (an `unimplemented!()`
    /// arm for a variant, a whole-body placeholder for a struct) while the recorded error fails
    /// the build. Nothing else is emitted for it: no tag, no assert, no docs.
    Poisoned,
}

/// What the descriptor says a checked field is: the shape assert is emitted straight from this, in
/// the descriptor's own vocabulary.
///
/// The descriptor's vocabulary, not the codec's: what a Rust type is allowed to be for it (a
/// singular message field may be `Option`; a map checks its key and value kinds) is an emission
/// rule, so it lives with the emitter.
pub(crate) struct Expectation {
    pub(crate) kind: FieldKind,
    pub(crate) cardinality: Cardinality,
}

impl Expectation {
    /// The expectation for a descriptor field. Whether a slot is checked at all is the caller's
    /// call, recorded in [`Slot::checks`]: a `with` adapter, a delegate and a generic field have
    /// nothing to check.
    pub(crate) fn of(field: &FieldMeta) -> Self {
        Self {
            kind: field.kind.clone(),
            cardinality: field.cardinality.clone(),
        }
    }
}

/// One protobuf field, wherever it sits.
///
/// A struct's field, a whole-message enum's non-oneof field (replicated across every variant), the
/// member a variant carries, and one field of a member message spread into a variant under `inline`
/// are one type seen from four places, so a new attribute key or a new check is one edit.
pub(crate) struct Slot {
    /// How the value is reached: `self.name` on a struct, the field name bound by the pattern in a
    /// struct variant, the single element of a tuple variant. `None` for a slot that carries no
    /// value at all (a `present` marker, an inlined member whose parts carry their own).
    pub(crate) access: Option<FieldAccess>,
    pub(crate) span: Span,
    /// Tag of the field, or the lowest routed tag of a delegate, which is what orders it among its
    /// siblings.
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
    /// A slot for one checked proto field: its tag, diagnostic path and harvested docs are the
    /// descriptor's; its codec and the check that survives it are the site's, because which
    /// substitution leaves a shape to check is what `resolve::payload_codec` decides.
    pub(crate) fn field(
        message: &str,
        meta: &FieldMeta,
        span: Span,
        access: FieldAccess,
        ty: syn::Type,
        adapter: Option<Box<syn::Type>>,
        checks: Option<Expectation>,
    ) -> Self {
        let codec = SlotCodec::Field {
            ty: Box::new(ty),
            adapter,
        };
        Self {
            access: Some(access),
            span,
            tag: meta.tag,
            codec,
            checks,
            proto_path: format!("{message}.{}", meta.name),
            docs: meta.docs.clone(),
        }
    }

    /// The Rust type carrying the value, for the shape assert. `None` where no type resolved: an
    /// inlined member (whose parts carry their own), a poisoned slot.
    pub(crate) fn ty(&self) -> Option<&syn::Type> {
        match &self.codec {
            SlotCodec::Field { ty, .. } | SlotCodec::Delegate { ty, .. } => Some(ty),
            SlotCodec::Group { .. } | SlotCodec::Poisoned => None,
        }
    }

    /// Whether resolution could give this slot no meaning. Read where a container degrades with
    /// what it holds: a struct whose field failed has no correct partial wire form, and a
    /// completeness pass over a container holding one would only restate the failure.
    pub(crate) fn is_poisoned(&self) -> bool {
        matches!(self.codec, SlotCodec::Poisoned)
    }

    /// A slot for something the user wrote that resolution could not give a meaning.
    pub(crate) fn poisoned(span: Span) -> Self {
        Self {
            access: None,
            span,
            tag: u32::MAX,
            codec: SlotCodec::Poisoned,
            checks: None,
            proto_path: String::new(),
            docs: Vec::new(),
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
    /// `None` when the enum failed to declare one: the items that need its names are then skipped
    /// or placeholder-bodied, and the recorded error fails the build.
    pub(crate) catch_all: Option<CatchAll>,
    /// Leading comment of the proto enum, or in transparent mode of the outermost wrapper message.
    pub(crate) docs: Vec<String>,
    /// Named variants with their proto numbers.
    pub(crate) named: Vec<EnumValue>,
    /// Variants the user wrote that resolution could not give a meaning: matched no proto value,
    /// or took a shape no variant may have. Kept so the matches over the enum stay exhaustive
    /// (each contributes an `unimplemented!()` arm) and so the payload struct a stray tuple
    /// variant names still exists.
    pub(crate) poisoned: Vec<PoisonedValue>,
    /// Named variant covering 0, when there is one; otherwise the derive emits an `UNSPECIFIED`
    /// const based on the catch-all.
    pub(crate) zero_variant: Option<syn::Ident>,
    /// Whether a variant carries the standard `#[default]` attribute, in which case the user
    /// derives `Default` and the macro must not.
    pub(crate) has_std_default: bool,
    /// The comparison traits the item derives even though the expansion emits them: an error, and
    /// the emitted impls that would collide are withheld (the derived ones satisfy the bounds).
    pub(crate) derived_comparisons: bool,
    /// Whether the item is an enum at all. When it is not, nothing variant-shaped can be said:
    /// the wire impl degrades to placeholder bodies and the value-level items are withheld, since
    /// every one of them matches over variants the item does not have.
    pub(crate) is_enum: bool,
    pub(crate) mode: EnumMode,
    pub(crate) fingerprint: u64,
    /// Intermediate wrapper messages the transparent chain flattens away, so they have no Rust type
    /// of their own.
    pub(crate) absorbs: Vec<String>,
}

/// The catch-all tuple variant and the payload struct the expansion emits for it.
pub(crate) struct CatchAll {
    pub(crate) variant: syn::Ident,
    pub(crate) payload: syn::Ident,
}

impl EnumPlan {
    /// Keep a variant resolution could give no value: the re-emitted item still names it, so every
    /// match over the enum needs an arm for it.
    pub(crate) fn poison(&mut self, ident: &syn::Ident, payload: Option<syn::Ident>) {
        self.poisoned.push(PoisonedValue {
            ident: ident.clone(),
            payload,
        });
    }
}

/// A variant that failed to resolve, and the payload struct it names when it is a single-payload
/// tuple variant (emitted so the re-emitted item still resolves).
pub(crate) struct PoisonedValue {
    pub(crate) ident: syn::Ident,
    pub(crate) payload: Option<syn::Ident>,
}

pub(crate) enum EnumMode {
    /// The Rust enum is a proto enum, an `int32` varint on the wire.
    Plain { names: Vec<String> },
    /// The Rust enum stands for proto message(s) wrapping an enum field through a chain of
    /// single-field wrappers; `path` holds the tags from the outermost wrapper down to the enum
    /// field, or `None` when the chain failed to resolve, in which case the wire impl is a
    /// placeholder.
    Transparent {
        names: Vec<String>,
        path: Option<Vec<u32>>,
    },
}
