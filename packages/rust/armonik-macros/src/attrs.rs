//! The `#[armonik(...)]` attribute grammar: one struct per site, each naming the keys that site
//! accepts.
//!
//! A key is an identifier, optionally `= <literal>` whose type the field's type picks; `darling`
//! reads the entries of every `#[armonik(...)]` on the site into the struct and names an unknown one
//! back with the keys the site does have. One struct per type, field and variant site, so a site
//! accepts exactly the keys it declares and cannot quietly start tolerating one by forgetting to
//! reject it. A scan that fails records into the [`Generator`] and answers `None`, like every other
//! step: what the site does with that is the site's own. The user-facing grammar documentation lives
//! on the two macros in `lib.rs`; keep it in sync.

use darling::util::Flag;
use darling::FromAttributes;

/// A key's value and the span of that value, for a diagnostic about what the value *says*: the
/// proto name that does not resolve, the adapter that does not fit. A key the site does not accept
/// is `darling`'s own business, and it spans that at the key.
pub(crate) use darling::util::SpannedValue;
use proc_macro2::{Span, TokenStream};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Attribute, LitStr, Token};

use crate::generator::Generator;

/// The span of a set flag, or `None`: what a site reasons about where the key both states a fact and
/// says where to report anything about it.
pub(crate) fn flagged(flag: Flag) -> Option<Span> {
    flag.is_present().then(|| flag.span())
}

/// Type level of `#[armonik_macros::message]`: the two keys the shape is picked from
/// (`resolve::resolve_message`).
#[derive(FromAttributes)]
#[darling(attributes(armonik))]
pub(crate) struct MessageAttrs {
    /// `generic`: no descriptor validation; fields carry explicit `tag`s.
    pub(crate) generic: Flag,
    /// `transparent`: single-field wrapper message flattened into its field.
    pub(crate) transparent: Flag,
}

/// Type level of `#[armonik_macros::oneof]`, which takes no key: the oneof it stands for is the
/// macro's argument, and the shape follows from that. Declared all the same, so that a key written
/// here is named back rather than ignored.
#[derive(FromAttributes)]
#[darling(attributes(armonik))]
pub(crate) struct OneofAttrs {}

/// Type level of `#[armonik_macros::enumeration]`.
#[derive(FromAttributes)]
#[darling(attributes(armonik))]
pub(crate) struct EnumerationAttrs {
    /// `transparent`: the enum is flattened out of the chain of single-field wrapper messages the
    /// macro's argument names, rather than standing for a proto enum directly.
    pub(crate) transparent: Flag,
}

/// A field standing for a proto field: a struct's own, or a struct variant's.
///
/// No `tag`: a descriptor-validated field takes its tag from the descriptor, and every one of the
/// `tag = ...` sites in the crate is inside an `#[armonik(generic)]` struct
/// ([`GenericFieldAttrs`]). Spelling one here only ever restated what the proto says.
#[derive(FromAttributes)]
#[darling(attributes(armonik))]
pub(crate) struct FieldAttrs {
    /// `rename = "proto_name"`: the proto field's name where it differs from the Rust field's.
    pub(crate) rename: Option<String>,
    /// `with = "path::to::Adapter"`: custom codec for a non-standard representation.
    pub(crate) with: Option<SpannedValue<syn::Type>>,
    /// `inlined`: the wrapper layer around the field's value gets no Rust type of its own; what it
    /// contains (the inner value, or a map) lives directly at the field.
    pub(crate) inlined: Flag,
}

/// A field of an `#[armonik(generic)]` struct, which names no proto message.
///
/// No `with`: the only check a generic type gets is the field-shape comparison at each
/// `#[armonik_macros::alias]`, which reads `ProtoField::SHAPE` per field. An adapter has no shape to
/// report -- it exists because the Rust representation is deliberately not the proto's -- so a field
/// carrying one would have nothing to put in `GenericFields::FIELDS`.
#[derive(FromAttributes)]
#[darling(attributes(armonik))]
pub(crate) struct GenericFieldAttrs {
    /// `tag = N`: the field's tag, authoritative here because there is no descriptor to take it
    /// from.
    pub(crate) tag: Option<u32>,
}

/// A variant of a oneof-shaped enum.
#[derive(FromAttributes)]
#[darling(attributes(armonik))]
pub(crate) struct VariantAttrs {
    /// `rename = "member_name"`: the oneof member's name where it differs from the snake-cased
    /// variant name.
    pub(crate) rename: Option<String>,
    /// `with = "path::to::Adapter"`: custom codec for the member carried whole.
    pub(crate) with: Option<SpannedValue<syn::Type>>,
    /// `present`: marker variant selected by the member's presence alone.
    pub(crate) present: Flag,
    /// `inlined`: the member's message layer gets no Rust type of its own; what it contains lives
    /// directly in the variant (a struct variant spreads its fields, a tuple variant carries its
    /// unwrapped inner value).
    pub(crate) inlined: Flag,
}

/// A variant of an `#[armonik_macros::enumeration]`, standing for one proto value.
#[derive(FromAttributes)]
#[darling(attributes(armonik))]
pub(crate) struct ValueAttrs {
    /// `rename = "FULL_PROTO_VALUE_NAME"`: the proto value's name where the prost-style short form
    /// does not match.
    pub(crate) rename: Option<String>,
}

/// The impl block of `#[armonik_macros::client]`.
#[derive(FromAttributes)]
#[darling(attributes(armonik))]
pub(crate) struct ClientAttrs {
    /// `service = "full.proto.Service"`: the proto service the block's methods belong to, which is
    /// what their documentation is looked up in.
    pub(crate) service: Option<LitStr>,
}

/// One method of a `#[armonik_macros::client]` impl block.
#[derive(FromAttributes)]
#[darling(attributes(armonik))]
pub(crate) struct MethodAttrs {
    /// `rpc = "MethodName"`: the RPC the method stands for.
    pub(crate) rpc: Option<LitStr>,
}

/// Read one site's `#[armonik(...)]` keys, or the combined error once the entries do not parse or
/// name a key the site does not accept. `darling` stays behind this: the grammar has one home.
pub(crate) fn read<T: FromAttributes>(attrs: &[Attribute]) -> syn::Result<T> {
    T::from_attributes(attrs).map_err(syn::Error::from)
}

/// The same for a step holding a [`Generator`]: `None`, with everything wrong recorded.
pub(crate) fn scan<T: FromAttributes>(attrs: &[Attribute], generator: &mut Generator) -> Option<T> {
    read(attrs).map_err(|error| generator.record(error)).ok()
}

/// The proto names a macro was given as its argument: `#[armonik_macros::message("a.B")]`, or
/// several for a unified type standing for identical messages. Empty where the macro was given
/// none, which only a `generic` type may be, and which resolution reports for itself.
///
/// The name is the macro's argument rather than an `#[armonik(...)]` key because it is what the
/// macro is *for*: every validated expansion needs one, so taking it as an argument is the grammar
/// saying so, and there is no key to forget.
pub(crate) fn proto_names(attr: TokenStream) -> syn::Result<Vec<(Span, String)>> {
    let names = Punctuated::<LitStr, Token![,]>::parse_terminated.parse2(attr)?;
    Ok(names
        .into_iter()
        .map(|name| (name.span(), name.value()))
        .collect())
}

/// Visit the attributes of the type itself and of every field, variant and variant field: the
/// common traversal for whole-input attribute scans.
pub(crate) fn for_each_site(input: &syn::DeriveInput, mut visit: impl FnMut(&[Attribute])) {
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

/// Span of every key token of every `#[armonik(...)]` attribute in `attrs`, for the
/// hover-documentation anchors (see `item::anchors`). Read as `key`/`key = value` entries and
/// nothing more, so an unknown key still gets its anchor; a malformed attribute is skipped, and the
/// site's own scan reports it.
pub(crate) fn key_spans(attrs: &[Attribute]) -> Vec<Span> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("armonik"))
        .filter_map(|attr| {
            attr.parse_args_with(Punctuated::<syn::Meta, Token![,]>::parse_terminated)
                .ok()
        })
        .flatten()
        .map(|entry| entry.path().span())
        .collect()
}

/// Remove the `#[armonik(...)]` attributes from one site: they are consumed by the expansion, and
/// what re-emits the item they were written on hands back an item without them.
pub(crate) fn strip(attrs: &mut Vec<Attribute>) {
    attrs.retain(|attr| !attr.path().is_ident("armonik"));
}

/// The name an identifier matches proto names by: the ident, minus any raw prefix.
pub(crate) fn unraw(ident: &syn::Ident) -> String {
    ident.to_string().trim_start_matches("r#").to_owned()
}

#[cfg(test)]
mod tests {
    //! Guards for the per-site key sets: which `#[armonik(...)]` keys a field or variant accepts.
    //! The full expansions only run inside the `armonik` crate (they read the build-script
    //! descriptor), and the differential harness only fuzzes valid input, so the two halves of a
    //! site -- the keys it reads, and the keys it refuses -- are pinned here, over the site with
    //! the most of both.

    use super::*;

    fn variant_attrs(attr: syn::Attribute) -> darling::Result<VariantAttrs> {
        VariantAttrs::from_attributes(&[attr])
    }

    #[test]
    fn collects_the_keys_of_the_site() {
        let scanned = variant_attrs(syn::parse_quote!(#[armonik(rename = "member", present)]))
            .expect("the oneof-variant site takes both keys");
        assert_eq!(scanned.rename.as_deref(), Some("member"));
        assert!(scanned.present.is_present());
        assert!(!scanned.inlined.is_present());
    }

    #[test]
    fn a_key_the_site_does_not_declare_is_rejected() {
        // `tag` belongs to generic-mode fields; a oneof variant takes its member from the
        // descriptor.
        let Err(error) = variant_attrs(syn::parse_quote!(#[armonik(tag = 1)])) else {
            panic!("a oneof variant does not take tag = ...");
        };
        assert!(
            error.to_string().contains("tag"),
            "the rejection names the key: {error}"
        );
    }
}
