//! The re-emitted item: the `#[armonik_macros::message]` / `#[armonik_macros::enumeration]`
//! attribute macros hand the annotated type back with `#[doc]` attributes extracted from the
//! protos' comments (type, fields, oneof variants, enum values), the `#[armonik(...)]` attributes
//! stripped, and the hover anchors that make those attributes documented.
//!
//! Only an attribute macro may rewrite the item, which is the whole reason these are attributes:
//! the proto prose becomes uncopyable, as it already is for the services. Injected docs come first,
//! hand-written ones after, for Rust-specific notes.

use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use crate::attrs;
use crate::plan::{EnumPlan, Ir, Slot, SlotCodec};

/// Which macro is expanding, for the hover anchors.
#[derive(Clone, Copy)]
pub(crate) enum Kind {
    Message,
    Enumeration,
}

impl Kind {
    /// The macro's own name, for the hover anchors.
    fn derive(self) -> &'static str {
        match self {
            Kind::Message => "message",
            Kind::Enumeration => "enumeration",
        }
    }
}

/// Re-emit the item of a `#[armonik_macros::message]` type: the plan's docs injected,
/// `#[armonik(...)]` stripped, the serde line added.
///
/// Infallible: everything it needs is already in the plan.
pub(crate) fn rewrite(input: &mut DeriveInput, ir: &Ir) {
    inject(input, ir);
    strip(input);
    input.attrs.push(serde_derive());
}

/// The same, minus the serde line: an enumeration's `Serialize`/`Deserialize` are emitted by hand
/// (`shape::enumeration::serde`), because the derived pair spells the catch-all as an object and
/// builds it without normalizing. The comparison traits are emitted for the same reason, so nothing
/// here has to reach a proto value either.
pub(crate) fn rewrite_enum(input: &mut DeriveInput, plan: &EnumPlan) {
    inject_enum(input, plan);
    strip(input);
}

/// Hover-documentation anchors: every `#[armonik(...)]` key token of the input, re-emitted as an
/// anonymous import of the deriving macro respanned onto the key. IDE hover on the otherwise-inert
/// keys then resolves to this crate's macro, the single home of the grammar documentation. The
/// anonymous `const` compiles to nothing.
///
/// Reads the pristine item, so it has to run before [`rewrite`] strips the attributes it points at.
pub(crate) fn anchors(input: &DeriveInput, kind: Kind) -> TokenStream {
    let mut spans = Vec::new();
    attrs::for_each_site(input, |attrs| spans.extend(attrs::key_spans(attrs)));
    if spans.is_empty() {
        return TokenStream::new();
    }
    let uses = spans.iter().map(|span| {
        let derive = syn::Ident::new(kind.derive(), *span);
        quote! {
            {
                #[allow(unused_imports)]
                use ::armonik_macros::#derive as _;
            }
        }
    });
    quote! {
        const _: () = {
            #(#uses)*
        };
    }
}

/// The `serde` line every one of these types carries.
///
/// The `#[derive(...)]` above it is deliberately *not* emitted: it varies across ten shapes, and
/// hiding it would take the derive set off the type.
fn serde_derive() -> syn::Attribute {
    syn::parse_quote!(
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    )
}

/// Apply the docs the plan harvested: on the type, its fields, its oneof variants and their
/// sibling or inlined fields.
///
/// A pure applier. It matches nothing: every `#[doc]` below was decided by resolution, which is
/// what makes the inlined-variant and transparent-enum cases work at all. Slots are found through
/// [`Slot::reaches`], the access path resolution already recorded.
fn inject(input: &mut DeriveInput, ir: &Ir) {
    prepend(&mut input.attrs, &ir.docs);
    match (&mut input.data, &ir.discr) {
        (syn::Data::Struct(data), None) => {
            apply(
                data.fields.iter_mut(),
                &ir.shared.iter().collect::<Vec<_>>(),
            );
        }
        (syn::Data::Enum(data), Some(discr)) => {
            for variant in &mut data.variants {
                let arm = discr
                    .arms
                    .iter()
                    .find(|candidate| candidate.ident == variant.ident);
                if let Some(arm) = arm {
                    prepend(&mut variant.attrs, &arm.own.docs);
                }
                // Every variant carries the shared fields; the "no member set" one carries only
                // those. A member is reached either through its own field, or, under `inline`,
                // through one field per part.
                let mut slots: Vec<&Slot> = ir.shared.iter().collect();
                if let Some(arm) = arm {
                    match &arm.own.codec {
                        SlotCodec::Group { parts } => slots.extend(parts),
                        _ => slots.push(&arm.own),
                    }
                }
                apply(variant.fields.iter_mut(), &slots);
            }
        }
        _ => {}
    }
}

/// The same for an enumeration: the type, then one variant per proto value.
fn inject_enum(input: &mut DeriveInput, plan: &EnumPlan) {
    prepend(&mut input.attrs, &plan.docs);
    let syn::Data::Enum(data) = &mut input.data else {
        return;
    };
    for variant in &mut data.variants {
        if let Some(value) = plan.named.iter().find(|value| value.ident == variant.ident) {
            prepend(&mut variant.attrs, &value.docs);
        }
    }
}

fn apply<'a>(fields: impl Iterator<Item = &'a mut syn::Field>, slots: &[&Slot]) {
    for (index, field) in fields.enumerate() {
        if let Some(slot) = slots.iter().find(|slot| slot.reaches(field, index)) {
            prepend(&mut field.attrs, &slot.docs);
        }
    }
}

/// Put the harvested docs *before* the existing attributes, so hand-written doc comments read as
/// additional notes after the proto prose.
fn prepend(attrs: &mut Vec<syn::Attribute>, docs: &[String]) {
    for line in docs.iter().rev() {
        attrs.insert(0, syn::parse_quote!(#[doc = #line]));
    }
}

/// Visit every field of the item, wherever it sits: a struct's own, or one of a variant's.
fn for_each_field(input: &mut DeriveInput, visit: impl FnMut(&mut syn::Field)) {
    match &mut input.data {
        syn::Data::Struct(data) => data.fields.iter_mut().for_each(visit),
        syn::Data::Enum(data) => data
            .variants
            .iter_mut()
            .flat_map(|variant| variant.fields.iter_mut())
            .for_each(visit),
        syn::Data::Union(_) => {}
    }
}

/// Remove every `#[armonik(...)]` attribute: they were consumed by the expansion and are not
/// registered anywhere once the item is re-emitted.
fn strip(input: &mut DeriveInput) {
    fn retain(attrs: &mut Vec<syn::Attribute>) {
        attrs.retain(|attr| !attr.path().is_ident("armonik"));
    }
    retain(&mut input.attrs);
    if let syn::Data::Enum(data) = &mut input.data {
        for variant in &mut data.variants {
            retain(&mut variant.attrs);
        }
    }
    for_each_field(input, |field| retain(&mut field.attrs));
}
