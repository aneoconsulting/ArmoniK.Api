//! One module per shape a type can take, each holding both halves: how it is resolved against the
//! descriptor, and how it is emitted. Plus the dispatch that picks one, which lives here because
//! this is the module that owns the shapes.
//!
//! Shape-major, because a shape is the unit anyone reads. The plan/emit split is kept *within*
//! each module, and the modules around it keep it checkable: `plan` names no `TokenStream`, and
//! `emit` reads the descriptor's vocabulary (`FieldKind`, `Cardinality`) without ever walking a
//! `DescriptorIndex`.

use proc_macro2::{Span, TokenStream};

use crate::attrs::{self, AttrItem, Errors};
use crate::descriptor::DescriptorIndex;
use crate::plan::{EnumPlan, MessagePlan, OneofPlan};

pub(crate) mod enumeration;
pub(crate) mod generic;
pub(crate) mod oneof;
pub(crate) mod plain;
pub(crate) mod transparent;

/// What `#[armonik_macros::message]` resolved to. Both variants are message-shaped: they implement
/// `prost::Message`, never `ProtoField` directly, which is why the entry point needs no family
/// dispatch. `#[armonik_macros::enumeration]` resolves to an [`EnumPlan`] instead, and is the only
/// macro that straddles the two families.
pub(crate) enum Plan {
    Struct(MessagePlan),
    Oneof(OneofPlan),
}

impl Plan {
    /// Proto messages a flattening construct swallowed into this type, so they have no Rust type of
    /// their own.
    pub(crate) fn absorbs(&self) -> &[String] {
        match self {
            Plan::Struct(plan) => &plan.absorbs,
            Plan::Oneof(plan) => &plan.absorbs,
        }
    }

    pub(crate) fn emit(&self) -> TokenStream {
        match self {
            Plan::Struct(plan) => plain::message(plan),
            Plan::Oneof(plan) => oneof::oneof(plan),
        }
    }
}

/// Pick the shape `#[armonik_macros::message]` is standing for and resolve it.
///
/// The single home of that decision, and of the type-level attribute scan the shapes are chosen
/// from: a shape resolver is handed what it needs and never rescans.
pub(crate) fn resolve_message(input: &syn::DeriveInput) -> Result<Plan, Errors> {
    let index = index(input)?;
    let entries = attrs::parse(&input.attrs)?;

    let mut proto_names: Vec<(Span, String)> = Vec::new();
    let mut stray: Vec<Span> = Vec::new();
    let mut oneof_attr = false;
    // Spans, not flags: the two are mutually exclusive and the rejection points at one of them.
    let mut generic: Option<Span> = None;
    let mut transparent: Option<Span> = None;
    for entry in &entries {
        match &entry.item {
            AttrItem::Message(lit) => proto_names.push((entry.span, lit.value())),
            AttrItem::Oneof(_) => oneof_attr = true,
            AttrItem::Generic => generic = Some(entry.span),
            AttrItem::Transparent => transparent = Some(entry.span),
            _ => stray.push(entry.span),
        }
    }

    // Enums are oneof-shaped: `message = ...` alone stands for a whole message with a single
    // inferred oneof, `oneof = ...` for one oneof of a message, embedded in a struct. Dispatched on
    // before anything is reported, because a oneof rescans the type-level attributes for itself and
    // rejects a stray key in its own words.
    if oneof_attr || (matches!(input.data, syn::Data::Enum(_)) && generic.is_none()) {
        return oneof::oneof_plan(input, &index).map(Plan::Oneof);
    }

    let mut errors = Errors::new();
    if let (Some(_), Some(transparent_span)) = (generic, transparent) {
        errors.at(
            transparent_span,
            "generic and transparent cannot be combined: transparent flattens a single-field \
             wrapper message into the type, generic skips descriptor validation because a \
             generic type names no proto message, and there is no wrapper to flatten without one",
        );
        return Err(errors);
    }
    for span in stray {
        errors.at(
            span,
            "this armonik attribute is not valid at type level on a struct",
        );
    }
    if generic.is_some() {
        if !proto_names.is_empty() {
            errors.at(
                input.ident.span(),
                "#[armonik(generic)] types are not validated against the descriptor; \
                 remove the message attribute",
            );
            return Err(errors);
        }
        return generic::generic_plan(input, &index, errors).map(Plan::Struct);
    }
    if transparent.is_some() {
        return transparent::transparent_plan(input, &index, proto_names, errors).map(Plan::Struct);
    }
    plain::message_plan(input, &index, proto_names, errors).map(Plan::Struct)
}

/// Resolve `#[armonik_macros::enumeration]`. Thin, because a proto enum and a transparent wrapper
/// chain around one are two modes of a single plan rather than two shapes: `enum_plan` owns that
/// split, and the entry point reads the mode back off the plan to pick the wire impl.
pub(crate) fn resolve_enumeration(input: &syn::DeriveInput) -> Result<EnumPlan, Errors> {
    let index = index(input)?;
    enumeration::enum_plan(input, &index)
}

/// The compiled descriptor set, or a spanned error naming the type that wanted it.
///
/// Loaded here rather than by the entry points, so that a descriptor which fails to load reads as
/// the reason this type could not be resolved, and both macros stay free of `?`.
fn index(input: &syn::DeriveInput) -> Result<std::sync::Arc<DescriptorIndex>, Errors> {
    crate::descriptor::index()
        .map_err(|message| syn::Error::new(input.ident.span(), message).into())
}
